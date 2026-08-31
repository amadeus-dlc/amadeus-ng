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
    DefinitionArtifactsClientImpl, IntentExecutionRepositoryImpl, IntentRepositoryImpl,
    WorkflowDefinitionRepositoryImpl, WorkflowDefinitionSqliteStore,
};
use core_command_interface_adapter::{UnscannedWorkspace, WorkspaceScanner};
use core_command_use_case::orchestration::{
    CommitVerdictUseCase, CreateIntentUseCase, DefineWorkflowUseCase, IntentRepository as _,
    ReportedTransition,
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
        Request::Next(input) => emit(next(&layout, *input).await),
        Request::Continue { token } => emit(resume(&layout, &token).await),
        Request::Report(args) => report(&layout, &args).await,
        // park はドメインのガードが upstream の拒否 3 態を表現しきれていないため未配線
        // （実測: autonomous は表現済み、Completed と「すでに park 済み」が `NotRunning` に
        // 畳まれ、しかも upstream は再 park を**成功**させる）。upstream の `handlePark` 自身が
        // park の失敗を stdout の error directive で返す（`aidlc-orchestrate.ts:5976`）ので、
        // 未配線もその層に合わせる — 自己防衛拒否ではない。
        Request::Park => emit(Ok((
            Directive::Error {
                message: "Cannot park the workflow: park is not wired in this build.".to_string(),
            },
            Vec::new(),
        ))),
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
fn emit(outcome: Result<(Directive, Vec<u8>), String>) -> Completion {
    let (directive, key) = match outcome {
        Ok(pair) => pair,
        Err(message) => (Directive::Error { message }, Vec::new()),
    };
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
        return emit_error("report requires --result <outcome>.".to_string());
    };
    let verdict = match Verdict::parse(raw) {
        Ok(verdict) => verdict,
        Err(unknown) => {
            return emit_error(wording::unknown_result(unknown.as_str()));
        }
    };
    // `resume` は遷移をコミットしない — ルーティングなので**ユースケースへ届く手前で**分岐する
    // （`coding-rules/use-case-rules.md` §3 / `ReportedTransition` の doc）。
    let Some(transition) = transition_of(verdict, args) else {
        return emit_error(
            "Resume is routed, not committed. Run a fresh `next --resume`.".to_string(),
        );
    };
    let Some(stage) = parse_stage(args) else {
        return emit_error("The --stage value is not a stage slug.".to_string());
    };
    let store = match store_path(layout) {
        Ok(store) => store,
        Err(message) => return emit_error(message),
    };
    let Ok(execution_id) = active_execution(&store).await else {
        return emit_error(
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
        return emit_error(wording::transition_rejected(&error.to_string()));
    }
    // 書いた事実をリードモデルへ落とす（U7 の責務「コマンド末尾の RMU 起動」）。
    // ここは握り潰さない — 描けなければ利用者には何も見えないままになる。
    if let Err(error) = catch_up(layout).await {
        return Completion::refused(wording::orchestrate_failure(&error));
    }
    emit(Ok((
        Directive::Done {
            reason: Some(format!("reported {raw}")),
        },
        Vec::new(),
    )))
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
fn emit_error(message: String) -> Completion {
    emit(Ok((Directive::Error { message }, Vec::new())))
}

/// 記録まわりの書込 3 手 — **合成ルート私有の試験用継ぎ目**。
///
/// これは**アーキテクチャのポートではない**。層をまたぐ契約でもドメインが要求する抽象でも
/// なく、`create_intent` が自分の配線の中で踏む I/O を差し替えられるようにするためだけの
/// ものである（だから `pub` にせず、この 1 モジュールに閉じている）。狙いは 1 つ:
/// 「書けなかったときに何と言って、どの終了コードで止まるか」を**本物のテスト**で踏むこと。
/// 実 I/O は [`RealRecords`] が持ち、失敗ダブルはこのファイルのテストだけが持つ。
trait Records {
    /// 記録ディレクトリ（監査シャードの置き場ごと）を用意する。
    fn create(&self, record: &Path) -> std::io::Result<()>;
    /// 状態ファイルの骨格を書く。
    fn write_state(&self, record: &Path, contents: &str) -> std::io::Result<()>;
    /// active-intent カーソルを据える。
    fn point_at(&self, layout: &Layout, record_dir_name: &str) -> std::io::Result<()>;
}

/// 実 I/O（本番の配線）。
struct RealRecords;

impl Records for RealRecords {
    fn create(&self, record: &Path) -> std::io::Result<()> {
        std::fs::create_dir_all(record.join("audit"))
    }

    fn write_state(&self, record: &Path, contents: &str) -> std::io::Result<()> {
        std::fs::write(record.join("aidlc-state.md"), contents)
    }

    fn point_at(&self, layout: &Layout, record_dir_name: &str) -> std::io::Result<()> {
        layout.point_at(record_dir_name)
    }
}

/// `intent-create` — 鋳造して記録ディレクトリを用意し、カーソルを据えてから投影する。
async fn create_intent(layout: &Layout, args: &IntentCreateArgs) -> Completion {
    create_intent_with(layout, args, &RealRecords).await
}

/// 記録まわりの書込を差し替えられる形（試験用の継ぎ目を通る本体）。
///
/// 失敗はすべて**最終的な診断文言**を `Err` で運ぶ。各失敗点に `return Completion::refused(..)`
/// を書き下すと、到達しない `return` が失敗点の数だけ積み上がって本筋が読めなくなるので、
/// 出口はここ 1 か所に畳む。
async fn create_intent_with(
    layout: &Layout,
    args: &IntentCreateArgs,
    records: &dyn Records,
) -> Completion {
    match mint_intent(layout, args, records).await {
        Ok(completion) => completion,
        Err(diagnostic) => Completion::refused(diagnostic),
    }
}

/// 鋳造の本筋（失敗は最終文言つきの `Err`）。
async fn mint_intent(
    layout: &Layout,
    args: &IntentCreateArgs,
    records: &dyn Records,
) -> Result<Completion, String> {
    let scope = args
        .scope()
        .ok_or_else(|| wording::orchestrate_failure("intent-create requires --scope <name>."))?;
    // `--review` は閉集合。外れた値で intent を作ってしまうと、状態ファイルへ意味の無い
    // レビュー上限が焼き込まれる（upstream は同じ位置で `die` する）。
    let review = args
        .review()
        .map(|raw| review_class(raw).ok_or_else(|| wording::unknown_review_class(raw)))
        .transpose()?;
    // UUIDv7 の綴りは両識別子の文法内なので実際には失敗しないが、`unwrap` は使わず
    // 拒否として素直に運ぶ（到達不能を騙る分岐より、届かない `Err` のほうが安全である）。
    let (Ok(intent_id), Ok(execution_id)) = (
        IntentId::parse(&uuid::Uuid::now_v7().to_string()),
        IntentExecutionId::parse(&uuid::Uuid::now_v7().to_string()),
    ) else {
        return Err(wording::orchestrate_failure("cannot mint an identifier"));
    };
    let now = Utc::now();

    // 記録ディレクトリとカーソルは**マシンローカルな構造**なので合成ルートが用意する。
    let name = record_name::compose(
        &now.format("%y%m%d").to_string(),
        args.label(),
        args.arguments(),
        &intent_id,
    )
    .map_err(|error| {
        fault(
            "cannot compose a record directory name",
            &format!("{error:?}"),
        )
    })?;
    let record = layout.intents_dir().join(name.as_str());
    records
        .create(&record)
        .map_err(|error| fault("cannot create the record directory", &error.to_string()))?;

    let scan = UnscannedWorkspace::new()
        .scan()
        .map_err(|error| fault("cannot scan the workspace", &format!("{error:?}")))?;
    let request = build_request(scope, args, review.as_deref());
    let store = store_path(layout)?;
    let (Ok(intents), Ok(executions), Ok(definitions)) = (
        IntentRepositoryImpl::open(&store),
        IntentExecutionRepositoryImpl::open(&store),
        WorkflowDefinitionRepositoryImpl::open(&store),
    ) else {
        return Err(wording::orchestrate_failure("cannot open the event store"));
    };
    let intents_reader = intents.reopened();
    // 鋳造の前に定義を確立しておく（ensure-defined）。ハーネス配布物の 3 入力を取り込み、
    // ストアに定義が無ければ確立し、内容版が違えば改訂する。同じなら何も書かない
    // （冪等は集約の `Unchanged` ガードが決める — `DefineWorkflowUseCase` の doc）。
    //
    // ここに置くのは、`intent-create` が**定義を読む最初の書込動詞**だからである。
    // クエリ側の動詞（`next` / `continue`）は自分のリードモデル読取でファイルを直接読むので
    // この前段を要しない（`coding-rules/cqrs-boundaries.md` 規則 6）。
    let definitions_reader = definitions.reopened();
    ensure_defined(layout, definitions, now).await?;
    let mut use_case = CreateIntentUseCase::new(definitions_reader, intents, executions);
    let definition_id =
        definition_id(layout).map_err(|message| wording::orchestrate_failure(&message))?;
    use_case
        .execute(
            intent_id.clone(),
            execution_id,
            &definition_id,
            request,
            scan,
            now,
        )
        .await
        .map_err(|error| wording::orchestrate_failure(&error.to_string()))?;
    // 骨格を書く — 投影は既存の行を**書き換える**ので、書き換え先が無いと 1 行も描けない
    // (`crate::scaffold` の doc / RMU の `ScaffoldMissing`)。
    let intent = intents_reader
        .find_by_id(&intent_id)
        .await
        .map_err(|error| fault("cannot read back the minted intent", &error.to_string()))?;
    let scaffold = crate::scaffold::compose(
        &intent,
        &layout.project_dir().to_string_lossy(),
        &now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
    );
    records
        .write_state(&record, &scaffold)
        .map_err(|error| fault("cannot write the state scaffold", &error.to_string()))?;
    records
        .point_at(layout, name.as_str())
        .map_err(|error| fault("cannot set the active-intent cursor", &error.to_string()))?;
    // カーソルを据えたので配置を取り直してから投影する（record が決まって初めて
    // 状態ファイルと監査シャードの置き場が決まる）。
    catch_up(&Layout::resolve(layout.project_dir()))
        .await
        .map_err(|error| wording::orchestrate_failure(&error))?;
    // 鋳造の結果は directive ではなく upstream の素の JSON 1 行である
    // (`aidlc-utility` 面は directive プロトコルに参加しない)。契約 JSON なので
    // 直列化はやはり canon-json を通す (BR1.7)。
    let mut created = ObjectMembers::new();
    created.insert("created", JsonValue::Bool(true));
    created.insert("record", JsonValue::String(name.as_str().to_string()));
    Ok(Completion::emitted(serialize(
        &JsonValue::Object(created),
        SerializationProfile::ContractCompact,
    )))
}

/// ハーネス配布物を取り込んで定義を確立・改訂する（ensure-defined）。
///
/// 3 入力を読むのは**取込境界**であり、集約の読取ではない — 集約は常にイベントストアから
/// 読む（オーナー裁定 2026-08-31。`coding-rules/cqrs-boundaries.md` 規則 4）。配布物が
/// 読めなければ intent は鋳造できないので、失敗は自己防衛拒否として surface する。
async fn ensure_defined(
    layout: &Layout,
    definitions: WorkflowDefinitionRepositoryImpl<WorkflowDefinitionSqliteStore>,
    now: chrono::DateTime<Utc>,
) -> Result<(), String> {
    DefineWorkflowUseCase::new(
        DefinitionArtifactsClientImpl::new(layout.definition_data_dir(), layout.scopes_dir()),
        definitions,
    )
    .execute(now)
    .await
    .map_err(|error| fault("cannot ingest the workflow definition", &error.to_string()))
}

fn build_request(scope: &str, args: &IntentCreateArgs, review: Option<&str>) -> StartRequest {
    let mut request = StartRequest::new(scope, args.arguments().unwrap_or_default());
    if let Some(depth) = args.depth() {
        request = request.with_depth(depth);
    }
    if let Some(strategy) = args.test_strategy() {
        request = request.with_test_strategy(strategy);
    }
    // review は正規化済みの値を受け取る（生の綴りは `review_class` が閉集合へ畳む）。
    if let Some(review) = review {
        request = request.with_review(review);
    }
    request
}

/// 失敗の診断文言を組む（`<何ができなかったか>: <原因>` — 出す面は `aidlc-utility` だが、
/// 接頭辞は既存の自己防衛拒否と揃える）。
fn fault(what: &str, cause: &str) -> String {
    wording::orchestrate_failure(&format!("{what}: {cause}"))
}

/// `--review` を upstream の閉集合へ畳む（`parseReviewOverride` `aidlc-utility.ts:155-162`）。
///
/// 小文字化してから照合し、外れた値は `None` を返す — 呼出側はそれを拒否として運ぶ
/// （upstream も `die` で止め、既定へ落としたりはしない）。
fn review_class(raw: &str) -> Option<String> {
    let value = raw.to_lowercase();
    matches!(value.as_str(), "adversarial" | "advisory" | "none").then_some(value)
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
    let reader = JournalReaderImpl::open(&store_path(layout)?)
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

/// 現在のスペースのストアパス。
///
/// **カーソルの値が空間名として成立しないなら拒否する。** 既定へ落とすと、record や状態
/// ファイルはカーソルの綴りで解決されたまま、イベントだけが `default` のストアへ入る —
/// 指定と違う置き場へ黙って書くことになる。
///
/// # Errors
///
/// `aidlc/active-space` の値が空間名の文法に合わない場合。
fn store_path(layout: &Layout) -> Result<StorePath, String> {
    let space = SpaceName::parse(layout.space())
        .map_err(|_| wording::invalid_active_space(layout.space()))?;
    Ok(StorePath::for_space(&layout.aidlc_root(), &space))
}

/// 監査シャード名のホスト部 — upstream は `node:os` の `hostname()` を読む
/// （`aidlc-lib.ts:4446`）。
///
/// 環境変数 `HOSTNAME` は当てにならない。対話シェルが export しない環境（macOS の
/// 既定はまさにこれ）では常に未設定で、シャードが `host-<cloneId>.md` に落ちて既存の
/// `<実ホスト>-<cloneId>.md` から孤立する。実測値を読むのが唯一の正しい観測である。
///
/// 正規化（小文字化・`[a-z0-9-]` 以外の連続を `-` へ圧縮・trim・48 文字上限・空なら
/// `"host"`）は [`ShardName::of`] が所有するので、ここは**生の観測値を渡すだけ**である。
fn host_name() -> String {
    observed_host(hostname::get().ok())
}

/// OS 呼び出しの結果を文字列へ畳む（取得できなければ空 — 空の畳み先は
/// [`ShardName::of`] が持つ `"host"` である）。OS 呼び出しから切り離してあるのは、
/// 観測値の扱いをテストで固定するためである。
fn observed_host(raw: Option<std::ffi::OsString>) -> String {
    raw.map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
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

#[cfg(test)]
mod tests {
    #![allow(clippy::panic)]

    use super::*;
    use core_command_domain::workspace::CloneId;

    /// 実在シャード名の形 — macOS の `<user>-Mac-Studio.lan` が既存シャードと同じ綴りへ
    /// 落ちる。ここが崩れると監査証跡が 2 つのファイルへ割れる。
    #[test]
    fn the_observed_host_composes_the_real_shard_name() {
        let clone_id = CloneId::parse("8fc90228c64e").expect("固定のクローン ID");
        let host = observed_host(Some(std::ffi::OsString::from("j5ik2o-Mac-Studio.lan")));
        assert_eq!(
            ShardName::of(&host, &clone_id).as_str(),
            "j5ik2o-mac-studio-lan-8fc90228c64e.md"
        );
    }

    /// ホスト名が読めなければ空を渡し、`"host"` への畳み込みは `ShardName` に任せる。
    #[test]
    fn an_unreadable_host_falls_back_through_the_shard_name() {
        let clone_id = CloneId::parse("8fc90228c64e").expect("固定のクローン ID");
        assert_eq!(observed_host(None), "");
        assert_eq!(
            ShardName::of(&observed_host(None), &clone_id).as_str(),
            "host-8fc90228c64e.md"
        );
    }

    /// 記録の書込 3 手のうち**1 手だけ**を失敗させるダブル。他は実 I/O を通すので、
    /// そこへ辿り着くまでの配線（鋳造・ストア・ユースケース）は本物のまま踏める。
    struct FailingAt(Step);

    #[derive(PartialEq, Eq)]
    enum Step {
        WriteState,
        PointAt,
    }

    impl Records for FailingAt {
        fn create(&self, record: &Path) -> std::io::Result<()> {
            RealRecords.create(record)
        }

        fn write_state(&self, record: &Path, contents: &str) -> std::io::Result<()> {
            if self.0 == Step::WriteState {
                return Err(std::io::Error::other("disk full"));
            }
            RealRecords.write_state(record, contents)
        }

        fn point_at(&self, layout: &Layout, record_dir_name: &str) -> std::io::Result<()> {
            if self.0 == Step::PointAt {
                return Err(std::io::Error::other("cursor is read-only"));
            }
            RealRecords.point_at(layout, record_dir_name)
        }
    }

    /// 鋳造が通るところまで揃えた最小ワークスペース（定義 3 入力 + memory + intents）。
    fn minimal_workspace() -> tempfile::TempDir {
        let root = tempfile::tempdir().expect("一時ディレクトリ");
        let data = root.path().join(".claude/tools/data");
        let scopes = root.path().join(".claude/scopes");
        std::fs::create_dir_all(&data).expect("data");
        std::fs::create_dir_all(&scopes).expect("scopes");
        std::fs::write(
            data.join("harness.json"),
            r#"{"name":"claude","harnessDir":".claude","rulesSubdir":"rules"}"#,
        )
        .expect("harness.json");
        std::fs::write(
            data.join("stage-graph.json"),
            r#"[{"slug":"state-init","number":"0.1","name":"State Init","phase":"initialization",
                 "execution":"ALWAYS","mode":"inline","lead_agent":"orchestrator","scopes":["classic"]},
                {"slug":"domain-design","number":"1.1","name":"Domain Design","phase":"inception",
                 "execution":"ALWAYS","mode":"inline","lead_agent":"orchestrator","scopes":["classic"]}]"#,
        )
        .expect("stage-graph.json");
        std::fs::write(
            data.join("scope-grid.json"),
            r#"{"classic":{"stages":{"state-init":"EXECUTE","domain-design":"EXECUTE"}}}"#,
        )
        .expect("scope-grid.json");
        std::fs::write(
            scopes.join("aidlc-classic.md"),
            "---\nname: classic\n---\n\n# Classic\n",
        )
        .expect("scope identity");
        let memory = root.path().join("aidlc/spaces/default/memory");
        std::fs::create_dir_all(&memory).expect("memory");
        std::fs::write(memory.join("org.md"), "# Org\n").expect("org.md");
        std::fs::create_dir_all(root.path().join("aidlc/spaces/default/intents")).expect("intents");
        root
    }

    /// `intent-create` の引数を実際のパーサから組む。
    fn intent_create_args(extra: &[&str]) -> IntentCreateArgs {
        let mut argv: Vec<String> = vec!["intent-create".to_string()];
        argv.extend(extra.iter().map(|arg| (*arg).to_string()));
        match parse(Face::Utility, &argv) {
            Request::IntentCreate(args) => args,
            other => panic!("intent-create へ行く: {other:?}"),
        }
    }

    /// 骨格を書けなければ**自己防衛拒否**で止まる — 投影は既存行を書き換えるので、骨格が
    /// 無いまま 0 で返すと以後の `next` が状態ファイルを読めないまま進んでしまう。
    #[tokio::test]
    async fn a_state_scaffold_that_cannot_be_written_is_refused() {
        let root = minimal_workspace();
        let layout = Layout::resolve(root.path());

        let completion = create_intent_with(
            &layout,
            &intent_create_args(&["--scope", "classic", "--label", "demo"]),
            &FailingAt(Step::WriteState),
        )
        .await;

        assert_eq!(completion.code(), 1);
        assert_eq!(completion.line(), None, "stdout には何も出さない");
        assert_eq!(
            completion.diagnostic(),
            Some("aidlc-orchestrate: cannot write the state scaffold: disk full")
        );
    }

    /// カーソルを据えられなければ拒否する — 据わっていない record は `next` から見えない。
    #[tokio::test]
    async fn a_cursor_that_cannot_be_set_is_refused() {
        let root = minimal_workspace();
        let layout = Layout::resolve(root.path());

        let completion = create_intent_with(
            &layout,
            &intent_create_args(&["--scope", "classic", "--label", "demo"]),
            &FailingAt(Step::PointAt),
        )
        .await;

        assert_eq!(completion.code(), 1);
        assert_eq!(
            completion.diagnostic(),
            Some("aidlc-orchestrate: cannot set the active-intent cursor: cursor is read-only")
        );
    }

    /// 報告された結末は、それぞれ別の遷移へ写り、材料（`--user-input` / `--reason`）を
    /// 落とさない。`resume` だけは遷移にならない（ルーティングなので手前で分岐する）。
    #[test]
    fn every_verdict_maps_to_its_transition_with_its_material() {
        assert!(matches!(
            transition_of(Verdict::AwaitingApproval, &report_args(&[])),
            Some(ReportedTransition::AwaitingApproval { artifacts }) if artifacts.is_empty()
        ));
        assert!(matches!(
            transition_of(Verdict::Forward, &report_args(&["--user-input", "go ahead"])),
            Some(ReportedTransition::Forward { user_input: Some(text) }) if text == "go ahead"
        ));
        assert!(matches!(
            transition_of(Verdict::Rejected, &report_args(&["--user-input", "not yet"])),
            Some(ReportedTransition::Rejected { feedback: Some(text) }) if text == "not yet"
        ));
        assert!(matches!(
            transition_of(Verdict::Revised, &report_args(&[])),
            Some(ReportedTransition::Revised)
        ));
        assert!(matches!(
            transition_of(Verdict::Skipped, &report_args(&["--reason", "out of scope"])),
            Some(ReportedTransition::Skipped { reason }) if reason == "out of scope"
        ));
        // 材料が無ければ空で運ぶ（`--reason` 無しの skipped は理由なしの skip）。
        assert!(matches!(
            transition_of(Verdict::Skipped, &report_args(&[])),
            Some(ReportedTransition::Skipped { reason }) if reason.is_empty()
        ));
        assert!(transition_of(Verdict::Resume, &report_args(&[])).is_none());
    }

    /// `report` の引数を実際のパーサから組む（テスト専用の抜け道を作らない）。
    fn report_args(extra: &[&str]) -> crate::cli::ReportArgs {
        let mut argv: Vec<String> = vec!["report".to_string()];
        argv.extend(extra.iter().map(|arg| (*arg).to_string()));
        match parse(Face::Orchestrate, &argv) {
            Request::Report(args) => args,
            other => panic!("report へ行く: {other:?}"),
        }
    }

    /// `--review` は大小を問わず閉集合へ畳み、外れた値は畳まない（既定へ落とさない）。
    #[test]
    fn the_review_class_folds_to_the_closed_set() {
        assert_eq!(review_class("Adversarial").as_deref(), Some("adversarial"));
        assert_eq!(review_class("ADVISORY").as_deref(), Some("advisory"));
        assert_eq!(review_class("none").as_deref(), Some("none"));
        assert_eq!(review_class("strict"), None);
        assert_eq!(review_class(""), None);
    }

    /// 鍵の失敗 3 態はそれぞれ別の逐語になる（原因を混ぜない）。
    #[test]
    fn the_key_failures_map_to_three_distinct_wordings() {
        use core_infrastructure::secret_file::SecretFileError;
        let key = SteeringKey::resolve(Path::new("/ws"), None);
        let path = key.path().to_path_buf();

        let corrupt = key_wording(&key, &SecretFileError::Corrupt { path: path.clone() });
        let unreadable = key_wording(
            &key,
            &SecretFileError::Unreadable {
                path: path.clone(),
                cause: std::io::Error::other("EACCES"),
            },
        );
        let uncreatable = key_wording(
            &key,
            &SecretFileError::Uncreatable {
                path,
                cause: std::io::Error::other("EROFS"),
            },
        );

        assert!(corrupt.contains("corrupt"), "{corrupt}");
        assert!(unreadable.contains("EACCES"), "{unreadable}");
        assert!(uncreatable.contains("EROFS"), "{uncreatable}");
        assert_ne!(corrupt, unreadable);
        assert_ne!(unreadable, uncreatable);
    }

    /// 実機の観測値は空ではない（env `HOSTNAME` 頼みだとここが落ちる環境がある）。
    #[test]
    fn the_running_machine_reports_a_host() {
        assert!(!host_name().is_empty(), "実機のホスト名が読めない");
    }
}
