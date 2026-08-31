//! 配線と実行 — 要求を組み立てたユースケースへ通し、結末を出口の形へ写す。
//!
//! `main.rs` はここを呼ぶだけである（カバレッジ除外はあの 1 ファイルだけなので、判断は
//! すべてこちらに置く）。
//!
//! # RMU をいつ回すか
//!
//! - **クエリ動詞（`next` / `continue`）の前**: リードモデルが最新でないと、直前の書込が
//!   見えないまま判断してしまう。
//! - **書込動詞（`report` / `intent-create`）の後**: 書いた事実を投影して
//!   `aidlc-state.md` と監査シャードへ落とす（U7 の責務「コマンド末尾の RMU 起動」）。
//!
//! **駆動ループはここに無い** — 回すのは `catch_up` の呼出 1 行だけで、バッチ・チェック
//! ポイント・エラー処理は RMU の中にある（`coding-rules/cqrs-boundaries.md` 禁止パターン
//! 「駆動ループを合成ルートに置く」）。
//!
//! # 2 層の出口
//!
//! ビジネス拒否は `error` directive として **stdout・exit 0**、自己防衛拒否は
//! **stderr・exit 1**（[`crate::presenter`] のモジュール doc）。[`Completion`] がその 2 つを
//! 表す。

use std::path::Path;

use chrono::Utc;
use core_command_domain::orchestration::{IntentExecutionId, IntentId, StartRequest, Verdict};
use core_command_domain::workflow_definition::StageSlug;
use core_command_domain::workspace::{ShardName, SpaceName, StorePath};
use core_command_interface_adapter::orchestration::{
    IntentExecutionRepositoryImpl, IntentRepositoryImpl, WorkflowDefinitionRepositoryImpl,
};
use core_command_interface_adapter::{UnscannedWorkspace, WorkspaceScanner};
use core_command_use_case::orchestration::{
    CommitVerdictUseCase, CreateIntentUseCase, IntentRepository as _, ReportedTransition,
};
use core_infrastructure::canon_json::{JsonValue, ObjectMembers, SerializationProfile, serialize};
use core_query_interface_adapter::{
    DefinitionPaths, ExecutionStateDaoImpl, MemoryRulesDaoImpl, WorkflowDefinitionDaoImpl,
    verify_continue_token,
};
use core_query_use_case::orchestration::{
    ContinueUseCase, Directive, NextTurnInput, NextUseCase, WorkspaceLayout,
};
use core_read_model_updater::orchestration::{
    GlobalSeqNr, JournalReader as _, JournalReaderImpl, ProjectionName, ProjectionTargets,
    ReadModelUpdater,
};

use crate::cli::{Face, IntentCreateArgs, Invocation, Request, parse};
use crate::layout::Layout;
use crate::presenter::{DIRECTIVE_MAX_BYTES, Presenter};
use crate::record_name;
use crate::steering::SteeringKey;
use crate::wording;

/// 投影の名前（チェックポイントの鍵）。
const PROJECTION: &str = "orchestration";

/// 1 回の起動の結末。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Completion {
    line: Option<String>,
    diagnostic: Option<String>,
    code: u8,
}

impl Completion {
    /// directive を 1 つ出して正常終了する（ビジネス拒否の `error` directive もこちら）。
    #[must_use]
    pub const fn emitted(line: String) -> Completion {
        Completion {
            line: Some(line),
            diagnostic: None,
            code: 0,
        }
    }

    /// 何も stdout へ出さず、stderr へ逐語を出して失敗する（自己防衛拒否）。
    #[must_use]
    pub const fn refused(diagnostic: String) -> Completion {
        Completion {
            line: None,
            diagnostic: Some(diagnostic),
            code: 1,
        }
    }

    /// stdout へ出す 1 行（改行は書く側が付ける）。
    #[must_use]
    pub fn line(&self) -> Option<&str> {
        self.line.as_deref()
    }

    /// stderr へ出す診断。
    #[must_use]
    pub fn diagnostic(&self) -> Option<&str> {
        self.diagnostic.as_deref()
    }

    /// 終了コード。
    #[must_use]
    pub const fn code(&self) -> u8 {
        self.code
    }
}

/// 起動名と引数を受けて 1 回分を実行する。
///
/// `cwd` はワークスペース根の既定値で、`--project-dir` があればそちらが勝つ。
pub async fn run(argv0: &str, args: &[String], cwd: &Path) -> Completion {
    let invocation = Invocation::strip_global_flags(args);
    let project_dir = invocation
        .project_dir()
        .map_or_else(|| cwd.to_path_buf(), std::path::PathBuf::from);
    let layout = Layout::resolve(&project_dir);
    match parse(Face::of(argv0), invocation.rest()) {
        Request::Next(input) => emit(&layout, next(&layout, *input).await),
        Request::Continue { token } => emit(&layout, resume(&layout, &token).await),
        Request::Report(args) => report(&layout, &args).await,
        // park はドメインのガードが upstream の拒否 3 態を表現しきれていないため未配線
        // （実測: autonomous は表現済み、Completed と「すでに park 済み」が `NotRunning` に
        // 畳まれ、しかも upstream は再 park を**成功**させる）。upstream の `handlePark` 自身が
        // park の失敗を stdout の error directive で返す（`aidlc-orchestrate.ts:5976`）ので、
        // 未配線もその層に合わせる — 自己防衛拒否ではない。
        Request::Park => emit(
            &layout,
            Ok((
                Directive::Error {
                    message: "Cannot park the workflow: park is not wired in this build."
                        .to_string(),
                },
                Vec::new(),
            )),
        ),
        Request::IntentCreate(args) => create_intent(&layout, &args).await,
        Request::UnknownOrchestrateVerb { given } => {
            Completion::refused(wording::unknown_orchestrate_subcommand(given.as_deref()))
        }
        Request::UnknownUtilityVerb { given } => {
            Completion::refused(wording::orchestrate_failure(&format!(
                "Unknown subcommand: {}",
                given.as_deref().unwrap_or("(none)")
            )))
        }
    }
}

/// directive を描いて出口の形にする。鍵の取得に失敗したらビジネス拒否へ倒す。
fn emit(layout: &Layout, outcome: Result<(Directive, Vec<u8>), String>) -> Completion {
    let (directive, key) = match outcome {
        Ok(pair) => pair,
        Err(message) => (Directive::Error { message }, Vec::new()),
    };
    let _ = layout;
    match Presenter::new(key).render(&directive) {
        Ok(line) => Completion::emitted(line),
        Err(_) => Completion::refused(wording::refusing_oversize_directive(DIRECTIVE_MAX_BYTES)),
    }
}

/// `next` — リードモデルを追いつかせてからラダーを回す。
async fn next(layout: &Layout, input: NextTurnInput) -> Result<(Directive, Vec<u8>), String> {
    catch_up_before_reading(layout).await;
    // 鍵は `next` だけが鋳造する（I8 の例外 1 — steering MAC キー）。
    let key = SteeringKey::resolve(layout.project_dir(), layout.record_dir());
    let bytes = key
        .mint_for_next()
        .map_err(|error| key_wording(&key, &error))?;
    let directive = use_case(layout).execute(&with_layout(layout, input));
    Ok((directive, bytes))
}

/// `continue` — 鍵は**読むだけ**。無ければ・壊れていれば fail-closed（I12）。
async fn resume(layout: &Layout, token: &str) -> Result<(Directive, Vec<u8>), String> {
    catch_up_before_reading(layout).await;
    let key = SteeringKey::resolve(layout.project_dir(), layout.record_dir());
    let bytes = match key.read_for_continue() {
        Ok(Some(bytes)) => bytes,
        // 鍵が無い = この継続は検証できない。原因は区別せず「fresh `next` からやり直せ」。
        Ok(None) => {
            return Ok((
                Directive::Error {
                    message: wording::INVALID_CONTINUATION_TOKEN.to_string(),
                },
                Vec::new(),
            ));
        }
        Err(error) => return Err(key_wording(&key, &error)),
    };
    let verified = verify_continue_token(&bytes, token).ok();
    let input = with_layout(layout, NextTurnInput::new());
    let directive = ContinueUseCase::new(
        definition_dao(layout),
        state_dao(layout),
        memory_dao(layout),
    )
    .execute(verified, &input);
    Ok((directive, bytes))
}

/// `report` — 報告された結末を 1 つの遷移としてコミットし、投影で読み面へ落とす。
///
/// **13 段ガードのうち実装しているのは verdict の正規化だけ**である。`--single` 分岐・
/// state-version ガード・turn-shape マーカー・completion-evidence などは後続 Bolt。
async fn report(layout: &Layout, args: &crate::cli::ReportArgs) -> Completion {
    let Some(raw) = args.result() else {
        return emit_error(layout, "report requires --result <outcome>.".to_string());
    };
    let verdict = match Verdict::parse(raw) {
        Ok(verdict) => verdict,
        Err(unknown) => {
            return emit_error(layout, wording::unknown_result(unknown.as_str()));
        }
    };
    // `resume` は遷移をコミットしない — ルーティングなので**ユースケースへ届く手前で**分岐する
    // （`coding-rules/use-case-rules.md` §3 / `ReportedTransition` の doc）。
    let Some(transition) = transition_of(verdict, args) else {
        return emit_error(
            layout,
            "Resume is routed, not committed. Run a fresh `next --resume`.".to_string(),
        );
    };
    let Some(stage) = parse_stage(args) else {
        return emit_error(layout, "The --stage value is not a stage slug.".to_string());
    };
    let store = store_path(layout);
    let Ok(execution_id) = active_execution(&store).await else {
        return emit_error(
            layout,
            "No workflow execution to report against. Run `next` first.".to_string(),
        );
    };
    let (Ok(executions), Ok(intents)) = (
        IntentExecutionRepositoryImpl::open(&store),
        IntentRepositoryImpl::open(&store),
    ) else {
        return Completion::refused(wording::orchestrate_failure("cannot open the event store"));
    };
    if let Err(error) = CommitVerdictUseCase::new(executions, intents)
        .execute(&execution_id, stage.as_ref(), transition, Utc::now())
        .await
    {
        return emit_error(layout, wording::transition_rejected(&error.to_string()));
    }
    // 書いた事実をリードモデルへ落とす（U7 の責務「コマンド末尾の RMU 起動」）。
    // ここは握り潰さない — 描けなければ利用者には何も見えないままになる。
    if let Err(error) = catch_up(layout).await {
        return Completion::refused(wording::orchestrate_failure(&error));
    }
    emit(
        layout,
        Ok((
            Directive::Done {
                reason: Some(format!("reported {raw}")),
            },
            Vec::new(),
        )),
    )
}

/// 報告された結末に材料を貼り付ける（`resume` はここに無い — 手前で分岐済み）。
fn transition_of(verdict: Verdict, args: &crate::cli::ReportArgs) -> Option<ReportedTransition> {
    Some(match verdict {
        Verdict::AwaitingApproval => ReportedTransition::AwaitingApproval {
            artifacts: Vec::new(),
        },
        Verdict::Forward => ReportedTransition::Forward {
            user_input: args.user_input().map(str::to_string),
        },
        Verdict::Rejected => ReportedTransition::Rejected {
            feedback: args.user_input().map(str::to_string),
        },
        Verdict::Revised => ReportedTransition::Revised,
        Verdict::Skipped => ReportedTransition::Skipped {
            reason: args.reason().unwrap_or_default().to_string(),
        },
        Verdict::Resume => return None,
    })
}

/// `--stage` を型付きの slug へ写す。省略は `None`（カーソルに作用する — 有無が契約）。
fn parse_stage(args: &crate::cli::ReportArgs) -> Option<Option<StageSlug>> {
    match args.stage() {
        None => Some(None),
        Some(raw) => StageSlug::parse(raw).ok().map(Some),
    }
}

/// この実行の識別子をジャーナルから引く。
///
/// **リードモデルは実行の識別子を記録していない**（`aidlc-state.md` にも `intents.json` にも
/// 欄が無い — 実測）。stage-1 は単一 intent・単一実行なので、ジャーナル先頭の実行行が
/// 唯一の実行を指す。複数 intent を扱うようになったら、ここは登録簿（A-1 の裁定待ち）に
/// 置き換わる。
async fn active_execution(store: &StorePath) -> Result<IntentExecutionId, ()> {
    let reader = JournalReaderImpl::open(store).map_err(|_| ())?;
    let batch = reader
        .events_after(GlobalSeqNr::ZERO)
        .await
        .map_err(|_| ())?;
    batch
        .executions()
        .first()
        .map(|entry| entry.execution_id().clone())
        .ok_or(())
}

/// ビジネス拒否を error directive として出す。
fn emit_error(layout: &Layout, message: String) -> Completion {
    emit(layout, Ok((Directive::Error { message }, Vec::new())))
}

/// `intent-create` — 鋳造して記録ディレクトリを用意し、カーソルを据えてから投影する。
async fn create_intent(layout: &Layout, args: &IntentCreateArgs) -> Completion {
    let Some(scope) = args.scope() else {
        return Completion::refused(wording::orchestrate_failure(
            "intent-create requires --scope <name>.",
        ));
    };
    // UUIDv7 の綴りは両識別子の文法内なので実際には失敗しないが、`unwrap` は使わず
    // 拒否として素直に運ぶ（到達不能を騙る分岐より、届かない `Err` のほうが安全である）。
    let (Ok(intent_id), Ok(execution_id)) = (
        IntentId::parse(&uuid::Uuid::now_v7().to_string()),
        IntentExecutionId::parse(&uuid::Uuid::now_v7().to_string()),
    ) else {
        return Completion::refused(wording::orchestrate_failure("cannot mint an identifier"));
    };
    let now = Utc::now();

    // 記録ディレクトリとカーソルは**マシンローカルな構造**なので合成ルートが用意する。
    let name = match record_name::compose(
        &now.format("%y%m%d").to_string(),
        args.label(),
        args.arguments(),
        &intent_id,
    ) {
        Ok(name) => name,
        Err(error) => {
            return Completion::refused(wording::orchestrate_failure(&format!(
                "cannot compose a record directory name: {error:?}"
            )));
        }
    };
    let record = layout.intents_dir().join(name.as_str());
    if let Err(error) = std::fs::create_dir_all(record.join("audit")) {
        return Completion::refused(wording::orchestrate_failure(&format!(
            "cannot create the record directory: {error}"
        )));
    }

    let scan = match UnscannedWorkspace::new().scan() {
        Ok(scan) => scan,
        Err(error) => {
            return Completion::refused(wording::orchestrate_failure(&format!(
                "cannot scan the workspace: {error:?}"
            )));
        }
    };
    let request = build_request(scope, args);
    let store = store_path(layout);
    let (Ok(intents), Ok(executions)) = (
        IntentRepositoryImpl::open(&store),
        IntentExecutionRepositoryImpl::open(&store),
    ) else {
        return Completion::refused(wording::orchestrate_failure("cannot open the event store"));
    };
    let intents_reader = intents.reopened();
    let mut use_case = CreateIntentUseCase::new(
        WorkflowDefinitionRepositoryImpl::new(layout.definition_data_dir(), layout.scopes_dir()),
        intents,
        executions,
    );
    let definition_id = match definition_id(layout) {
        Ok(id) => id,
        Err(message) => return Completion::refused(wording::orchestrate_failure(&message)),
    };
    if let Err(error) = use_case
        .execute(
            intent_id.clone(),
            execution_id,
            &definition_id,
            request,
            scan,
            now,
        )
        .await
    {
        return Completion::refused(wording::orchestrate_failure(&error.to_string()));
    }
    // 骨格を書く — 投影は既存の行を**書き換える**ので、書き換え先が無いと 1 行も描けない
    // (`crate::scaffold` の doc / RMU の `ScaffoldMissing`)。
    let scaffold = match intents_reader.find_by_id(&intent_id).await {
        Ok(intent) => crate::scaffold::compose(
            &intent,
            &layout.project_dir().to_string_lossy(),
            &now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        ),
        Err(error) => {
            return Completion::refused(wording::orchestrate_failure(&format!(
                "cannot read back the minted intent: {error}"
            )));
        }
    };
    if let Err(error) = std::fs::write(record.join("aidlc-state.md"), scaffold) {
        return Completion::refused(wording::orchestrate_failure(&format!(
            "cannot write the state scaffold: {error}"
        )));
    }
    if let Err(error) = layout.point_at(name.as_str()) {
        return Completion::refused(wording::orchestrate_failure(&format!(
            "cannot set the active-intent cursor: {error}"
        )));
    }
    // カーソルを据えたので配置を取り直してから投影する（record が決まって初めて
    // 状態ファイルと監査シャードの置き場が決まる）。
    if let Err(error) = catch_up(&Layout::resolve(layout.project_dir())).await {
        return Completion::refused(wording::orchestrate_failure(&error));
    }
    // 鋳造の結果は directive ではなく upstream の素の JSON 1 行である
    // (`aidlc-utility` 面は directive プロトコルに参加しない)。契約 JSON なので
    // 直列化はやはり canon-json を通す (BR1.7)。
    let mut created = ObjectMembers::new();
    created.insert("created", JsonValue::Bool(true));
    created.insert("record", JsonValue::String(name.as_str().to_string()));
    Completion::emitted(serialize(
        &JsonValue::Object(created),
        SerializationProfile::ContractCompact,
    ))
}

fn build_request(scope: &str, args: &IntentCreateArgs) -> StartRequest {
    let mut request = StartRequest::new(scope, args.arguments().unwrap_or_default());
    if let Some(depth) = args.depth() {
        request = request.with_depth(depth);
    }
    if let Some(strategy) = args.test_strategy() {
        request = request.with_test_strategy(strategy);
    }
    request
}

/// リードモデルを追いつかせる。
///
/// record がまだ無い（intent 未鋳造）ときは描く先が無いので何もしない — それは失敗では
/// なく fresh なワークスペースの正常な姿である。
async fn catch_up(layout: &Layout) -> Result<(), String> {
    let (Some(state_file), Some(audit_dir)) = (layout.state_file(), layout.audit_dir()) else {
        return Ok(());
    };
    let projection =
        ProjectionName::parse(PROJECTION).map_err(|error| format!("projection name: {error:?}"))?;
    let reader = JournalReaderImpl::open(&store_path(layout))
        .map_err(|error| format!("journal: {error}"))?;
    let clone_id = crate::clone_identity::load_or_mint(&layout.aidlc_root())
        .map_err(|error| format!("clone id: {error}"))?;
    let shard = ShardName::of(&host_name(), &clone_id);
    let targets = ProjectionTargets::new(state_file, audit_dir.join(shard.as_str()));
    ReadModelUpdater::new(reader, projection, targets)
        .catch_up()
        .await
        .map(|_| ())
        .map_err(|error| format!("projection: {error}"))
}

/// **読み手の前**の追いつき — 失敗しても倒れない。
///
/// 投影が遅れていても、読み手は「その時点のリードモデル」で答えを出せるほうが、動詞ごと
/// 落ちるより無害である（at-least-once の投影は次の呼出で追いつく）。**書込の後**は
/// この限りではない — そちらは書いた事実が読み面へ落ちないと利用者に何も見えないので、
/// 失敗を surface する。
async fn catch_up_before_reading(layout: &Layout) {
    let _ = catch_up(layout).await;
}

fn store_path(layout: &Layout) -> StorePath {
    let space = SpaceName::parse(layout.space()).unwrap_or_default();
    StorePath::for_space(&layout.aidlc_root(), &space)
}

fn host_name() -> String {
    std::env::var("HOSTNAME").unwrap_or_else(|_| "host".to_string())
}

fn definition_id(
    layout: &Layout,
) -> Result<core_command_domain::workflow_definition::WorkflowDefinitionId, String> {
    let _ = layout;
    core_command_domain::workflow_definition::WorkflowDefinitionId::parse("claude")
        .map_err(|error| format!("cannot resolve the definition id: {error:?}"))
}

fn use_case(
    layout: &Layout,
) -> NextUseCase<WorkflowDefinitionDaoImpl, ExecutionStateDaoImpl, MemoryRulesDaoImpl> {
    NextUseCase::new(
        definition_dao(layout),
        state_dao(layout),
        memory_dao(layout),
    )
}

fn definition_dao(layout: &Layout) -> WorkflowDefinitionDaoImpl {
    WorkflowDefinitionDaoImpl::new(DefinitionPaths::new(
        layout.definition_data_dir(),
        layout.scopes_dir(),
    ))
}

fn state_dao(layout: &Layout) -> ExecutionStateDaoImpl {
    ExecutionStateDaoImpl::new(
        layout
            .record_dir()
            .map(Path::to_path_buf)
            .unwrap_or_default(),
    )
}

fn memory_dao(layout: &Layout) -> MemoryRulesDaoImpl {
    MemoryRulesDaoImpl::new(layout.memory_dir())
}

/// ラダーへ渡す観測にワークスペース配置を載せる。
fn with_layout(layout: &Layout, input: NextTurnInput) -> NextTurnInput {
    input.with_layout(WorkspaceLayout::new(
        layout
            .record_dir()
            .map(|dir| dir.to_string_lossy().into_owned())
            .unwrap_or_default(),
        layout.stage_library_dir().to_string_lossy().into_owned(),
        layout.agent_dir().to_string_lossy().into_owned(),
    ))
}

/// 鍵の失敗を逐語文言へ写す（3 形 — upstream `aidlc-orchestrate.ts:2323/2331/2350`）。
fn key_wording(
    key: &SteeringKey,
    error: &core_infrastructure::secret_file::SecretFileError,
) -> String {
    use core_infrastructure::secret_file::SecretFileError;
    let path = key.path().to_string_lossy();
    match error {
        SecretFileError::Corrupt { .. } => wording::corrupt_key_file(&path),
        SecretFileError::Unreadable { cause, .. } => {
            wording::unreadable_key_file(&path, &cause.to_string())
        }
        SecretFileError::Uncreatable { cause, .. } => {
            wording::uncreatable_key_file(&path, &cause.to_string())
        }
    }
}
