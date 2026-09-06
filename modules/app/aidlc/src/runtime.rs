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
use core_command_domain::orchestration::{
    AutonomyMode, CommandError, IntentExecutionId, IntentId, ReportNoOp, ReportRefusal,
    ReportRequest, ReviewVerdict, SkeletonStance, StartRequest, TransitionStep, TransitionSteps,
    Verdict,
};
use core_command_domain::workflow_definition::{PRACTICES_DISCOVERY_SLUG, StageSlug};
use core_command_domain::workspace::{
    EventType, HumanTurns, PracticesPromotion, PromotionPlanError, ShardName, SpaceName,
    StateVersionClassification, StateVersionKind, StorePath,
};
use core_command_interface_adapter::orchestration::{
    CompiledDefinitionRepositoryImpl, IntentExecutionRepositoryImpl, IntentRepositoryImpl,
    WorkflowDefinitionRepositoryImpl, WorkflowDefinitionSqliteStore,
};
use core_command_interface_adapter::{UnscannedWorkspace, WorkspaceScanner};
use core_command_use_case::orchestration::{
    AutonomySwitchRequest, CommitError, CommitOutcome, CommitVerdictUseCase, CreateIntentError,
    CreateIntentUseCase, DefineWorkflowUseCase, IntentRepository as _, ParkError, ParkUseCase,
    PracticesPromotionRequest, PromotePracticesUseCase, RecordReviewUseCase,
    RecordSingleStageRunUseCase, RecordSkeletonStanceUseCase, ReviewLogError, ReviewLogKind,
    ReviewLogOutcome, ReviewLogRequest, SingleStageRunError, SkeletonStanceError,
    SwitchAutonomyError, SwitchAutonomyUseCase,
};
use core_infrastructure::canon_json::{
    JsonValue, Number, ObjectMembers, SerializationProfile, serialize,
};
use core_query_interface_adapter::{ReadModelDaos, StateFileDaoImpl, verify_continue_token};
use core_query_use_case::orchestration::{
    Directive, FindDefinitionStageUseCase, FindExecutionUseCase, FindStateFileUseCase,
    NextTurnInput, StageSlugView,
};
use core_read_model_updater::orchestration::{
    JournalReaderImpl, ProjectionName, ProjectionTargets, ReadModelUpdater, SteeringSource,
};

use crate::cli::{Face, IntentCreateArgs, Invocation, Request, parse};
use crate::execution_cursor::{ExecutionCursor, ExecutionCursorError};
use crate::layout::Layout;
use crate::presenter::{DIRECTIVE_MAX_BYTES, Presenter};
use crate::record_name;
use crate::steering::SteeringKey;
use crate::turn;
use crate::wording;

/// 投影の名前（チェックポイントの鍵）。
const PROJECTION: &str = "orchestration";
/// 初回の構造化面だけの進捗。ファイル側の未反映イベントを飛ばさない。
const STRUCTURED_PROJECTION: &str = "orchestration-structured";

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
        Request::Park => park(&layout).await,
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
        Request::LogReview(args) => log_review(&layout, &args).await,
        Request::LogNotWired { verb } => Completion::refused(wording::log_verb_not_wired(&verb)),
        Request::UnknownLogVerb { given } => {
            Completion::refused(wording::unknown_log_subcommand(given.as_deref()))
        }
        Request::StatePracticesPromote(args) => practices_promote(&layout, &args).await,
        Request::StateNotWired { verb } => {
            Completion::refused(wording::state_verb_not_wired(&verb))
        }
        Request::UnknownStateVerb { given } => {
            Completion::refused(wording::unknown_state_subcommand(given.as_deref()))
        }
        Request::BoltSetAutonomy(args) => set_autonomy(&layout, &args).await,
        Request::BoltNotWired { verb } => Completion::refused(wording::bolt_verb_not_wired(&verb)),
        Request::UnknownBoltVerb { given } => {
            Completion::refused(wording::unknown_bolt_subcommand(given.as_deref()))
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

/// `next` — 初回だけ定義を準備し、リードモデルを追いつかせてから構造化面を引く。
async fn next(layout: &Layout, input: NextTurnInput) -> Result<(Directive, Vec<u8>), String> {
    // 鍵は `next` だけが鋳造する（I8 の例外 1 — steering MAC キー）。
    let key = SteeringKey::resolve(layout.project_dir(), layout.record_dir());
    let bytes = key
        .mint_for_next()
        .map_err(|error| key_wording(&key, &error))?;

    // 配布物やストアを読まなくても答えが決まる拒否・ユーティリティは、初回準備より先に返す。
    if let Some(directive) = turn::pre_guard(&input) {
        return Ok((directive, bytes));
    }
    if layout.record_dir().is_none() {
        prepare_definition_for_first_read(layout).await?;
    }
    catch_up_before_reading(layout).await?;
    Ok((turn::next(layout, &input), bytes))
}

/// `continue` — 鍵は**読むだけ**。無ければ・壊れていれば fail-closed（I12）。
async fn resume(layout: &Layout, token: &str) -> Result<(Directive, Vec<u8>), String> {
    catch_up_before_reading(layout).await?;
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
    Ok((turn::resume(layout, verified.as_ref()), bytes))
}

/// `report` — 13 段ガードを順に通し、決まった遷移をコミットして投影で読み面へ落とす。
///
/// 段の順序は upstream `handleReport`（ピン `:5464-5927`）と同順である。**状態の値で決まる
/// 分岐はここに 1 つも無い** — 合成ルートが持つのは値の有無・既知値・env で決まる構文的な段
/// だけで、対象の解決から先の判断は集約のクエリ `report_dispatch` に閉じている（設計 §1）。
///
/// | 段 | ここ | 集約 |
/// | --- | --- | --- |
/// | 1 state-version guard | ○ | — |
/// | 2 `--single` / 3 `--skeleton-stance` | ○（構文段） | ○（受理と記録） |
/// | 4 resume ルーティング | ○ | — |
/// | 5 `--result` の有無・既知値 | ○ | — |
/// | 6 実行カーソルの有無 | ○ | — |
/// | 7〜10・13・forward 表 | — | ○ |
/// | 11 completion-evidence | 未配線（slice 2） | — |
/// | 12 practices 受領証 | — | ○（`approve_gate` の段 12 — b49） |
async fn report(layout: &Layout, args: &crate::cli::ReportArgs) -> Completion {
    // 読む面（状態ファイル・実行行）が最新でないと段 1 と段 4 が古い値で判断する。
    // 書いた事実を落とすのはコミットの後（末尾の `catch_up`）である。
    if let Err(message) = catch_up_before_reading(layout).await {
        return Completion::refused(message);
    }

    // 段 1 — state-version guard。**すべての report 経路**に効かせるので最初に通す。
    if let Some(refusal) = state_version_guard(layout) {
        return emit_error(refusal);
    }
    let store = match store_path(layout) {
        Ok(store) => store,
        Err(message) => return emit_error(message),
    };
    // 段 2 — `--single`。本流の遷移サブコマンドへ落ちることを構造的に不可能にするため、
    // 主経路より先に解決する（I10）。
    if args.is_single() {
        return single_report(layout, args, &store).await;
    }
    // 段 3 — `--skeleton-stance`。stance の報告は verdict を持たないので段 5 より先。
    if let Some(stance) = args.skeleton_stance() {
        return skeleton_stance_report(layout, stance, &store).await;
    }
    // 段 4 — 再開の選択。遷移をコミットしないのでルーティングだけで終わる。
    let raw = args.result();
    if raw.is_some_and(|value| Verdict::parse(value) == Ok(Verdict::Resume)) {
        return resume_report(layout, args);
    }
    // 段 5 — verdict は必須で、閉集合の外は硬いエラーである。
    let Some(raw) = raw else {
        return emit_error(wording::report_requires_result());
    };
    let verdict = match Verdict::parse(raw) {
        Ok(verdict) => verdict,
        Err(unknown) => return emit_error(wording::unknown_result(unknown.as_str())),
    };
    // 段 7 の構文半分 — 空白だけの `--stage` は「無い」と同じ（upstream の `trim()`）。
    let explicit = explicit_stage(args);
    let stage = match explicit {
        None => None,
        Some(raw) => match StageSlug::parse(raw) {
            Ok(slug) => Some(slug),
            Err(_) => return emit_error(wording::reported_stage_not_in_graph(raw)),
        },
    };
    // 段 9 の構文半分 — `skipped` は明示・非空の `--stage` を要する（集約より前）。
    if verdict == Verdict::Skipped && stage.is_none() {
        return emit_error(wording::SKIP_REQUIRES_EXPLICIT_STAGE.to_string());
    }
    // 段 6 — 実行カーソルが無い = まだ鋳造していない（fresh なワークスペースの正常な姿）。
    let execution_id = match active_execution(layout) {
        Ok(Some(cursor)) => cursor.execution_id().clone(),
        Ok(None) => return emit_error(wording::REPORT_WITHOUT_STATE.to_string()),
        // 在るのに読めない・壊れているは**不在と混ぜない** — 畳むと壊れた record の上で
        // 作業が続く。
        Err(error) => return emit_error(wording::unreadable_execution_cursor(&error.to_string())),
    };
    // 段 13 の env — 判定そのものは集約が持つ（ここは観測を載せるだけ）。
    let request = ReportRequest::new(
        verdict,
        stage,
        args.user_input().map(str::to_string),
        args.reason().map(str::to_string),
        human_presence_guard(),
    );
    let (
        Ok(intent_execution_repository),
        Ok(intent_repository),
        Ok(workflow_definition_repository),
    ) = (
        IntentExecutionRepositoryImpl::open(&store),
        IntentRepositoryImpl::open(&store),
        WorkflowDefinitionRepositoryImpl::open(&store),
    )
    else {
        return Completion::refused(wording::orchestrate_failure("cannot open the event store"));
    };
    // 段 11 のレビュー方針は定義から引く（Approve 段だけが読む — b48）。
    let outcome = CommitVerdictUseCase::new(
        intent_execution_repository,
        intent_repository,
        workflow_definition_repository,
    )
    .execute(&execution_id, request, Utc::now())
    .await;
    let directive = match outcome {
        Ok(outcome) => outcome,
        Err(error) => return emit_error(commit_refusal(raw, &error)),
    };
    // 書いた事実をリードモデルへ落とす（U7 の責務「コマンド末尾の RMU 起動」）。
    // ここは握り潰さない — 描けなければ利用者には何も見えないままになる。
    after_projection(layout, || {
        emit(Ok((committed_directive(raw, &directive), Vec::new())))
    })
    .await
}

/// 段 1 — 状態ファイルの `State Version` を分類し、`ok` 以外の逐語を返す。
///
/// 状態ファイルが無ければ何も言わない（`None`）— upstream の `loadStateFileIfPresent` が
/// `null` を返す場合と同じで、鋳造前のワークスペースは正常な姿である。**0 バイトは「在る」**
/// なので分類にかかる（ピン `:5479-5481`）。
fn state_version_guard(layout: &Layout) -> Option<String> {
    let state_file = layout.state_file()?;
    let found = FindStateFileUseCase::new(StateFileDaoImpl::new(&state_file)).execute();
    let content = match found {
        Ok(Some(content)) => content,
        Ok(None) => return None,
        Err(error) => {
            return Some(wording::read_model_unreadable(
                &state_file.to_string_lossy(),
                &error.kind().to_string(),
            ));
        }
    };
    let classified = StateVersionClassification::classify(&content);
    let version = classified.version().unwrap_or_default();
    match classified.kind() {
        StateVersionKind::Ok => None,
        StateVersionKind::Unparseable => Some(wording::INCOMPATIBLE_STATE_UNPARSEABLE.to_string()),
        StateVersionKind::Past => Some(wording::incompatible_state_past(version)),
        StateVersionKind::Future => Some(wording::incompatible_state_future(version)),
    }
}

/// 段 2 — `--single`。隔離実行の疑似ワークフロー ID 付き対をコミットする（#73 / I10）。
///
/// 段の順序は upstream `handleSingleReport`（ピン `:5261-5361`）と同順である:
/// result 必須 → FORWARD のみ → `--stage` 必須 → 未知 → initialization →（evidence は
/// slice 2）→ 記録。**本流の遷移サブコマンドへは 1 度も落ちない** — 打つコマンドは
/// `record_single_stage_run` だけで、その適用はフレーム空である。
async fn single_report(
    layout: &Layout,
    args: &crate::cli::ReportArgs,
    store: &StorePath,
) -> Completion {
    let Some(raw) = args.result() else {
        return emit_error(wording::single_requires_result());
    };
    if Verdict::parse(raw) != Ok(Verdict::Forward) {
        return emit_error(wording::single_unknown_result(raw));
    }
    // 空判定は trim しない — upstream の `flags.stage.length === 0` と同じである。
    let Some(stage) = args.stage().filter(|value| !value.is_empty()) else {
        return emit_error(wording::SINGLE_REPORT_REQUIRES_STAGE.to_string());
    };
    let Ok(slug) = StageSlug::parse(stage) else {
        return emit_error(wording::unknown_stage(stage));
    };
    // 隔離実行の対も**その intent の記録の中で起きた事実**なので、記録が要る。
    let execution_id = match active_execution(layout) {
        Ok(Some(cursor)) => cursor.execution_id().clone(),
        Ok(None) => {
            return emit_error(wording::single_pair_failed(
                stage,
                wording::SINGLE_WITHOUT_EXECUTION,
            ));
        }
        Err(error) => {
            return emit_error(wording::single_pair_failed(
                stage,
                &wording::unreadable_execution_cursor(&error.to_string()),
            ));
        }
    };
    let (Ok(intent_execution_repository), Ok(intent_repository)) = (
        IntentExecutionRepositoryImpl::open(store),
        IntentRepositoryImpl::open(store),
    ) else {
        return Completion::refused(wording::orchestrate_failure("cannot open the event store"));
    };
    if let Err(error) =
        RecordSingleStageRunUseCase::new(intent_execution_repository, intent_repository)
            .execute(&execution_id, &slug, Utc::now())
            .await
    {
        return emit_error(single_run_refusal(stage, &error));
    }
    // 書いた事実をリードモデルへ落とす（監査 2 行はこの投影で台帳に現れる）。
    after_projection(layout, || {
        emit(Ok((
            Directive::Done {
                reason: Some(wording::single_run_committed(stage)),
            },
            Vec::new(),
        )))
    })
    .await
}

/// 隔離実行の拒否を逐語へ写す。
///
/// 逐語を選ぶのは**出す側**である（`coding-rules/error-handling.md`）。集約が運ぶのは
/// `InvalidTarget` という材料だけなので、initialization の逐語はここで当てる。
fn single_run_refusal(stage: &str, error: &SingleStageRunError) -> String {
    match error {
        SingleStageRunError::UnknownStage { .. } => wording::unknown_stage(stage),
        SingleStageRunError::Command {
            error: CommandError::InvalidTarget(_),
            ..
        } => wording::SINGLE_INIT.to_string(),
        other => wording::single_pair_failed(stage, &chained(other)),
    }
}

/// 段 3 — `--skeleton-stance`。分類の往復の受け口（#73）。
///
/// 値検証 → state 必須 → 記録 → 投影 → 「`next` をもう一度」の print、という順序は
/// upstream `handleSkeletonStanceReport`（ピン `:4943-5008`）と同順である。
async fn skeleton_stance_report(layout: &Layout, stance: &str, store: &StorePath) -> Completion {
    let Ok(stance) = SkeletonStance::parse(stance) else {
        return emit_error(wording::unknown_skeleton_stance(stance));
    };
    if !state_file_present(layout) {
        return emit_error(wording::SKELETON_STANCE_WITHOUT_STATE.to_string());
    }
    let execution_id = match active_execution(layout) {
        Ok(Some(cursor)) => cursor.execution_id().clone(),
        // 不在 = まだ鋳造していない。状態ファイルだけが在る形も同じ答えである。
        Ok(None) => return emit_error(wording::SKELETON_STANCE_WITHOUT_STATE.to_string()),
        // 在るのに読めない・壊れているは**不在と混ぜない**（`report` 段 6 と同じ規律）。
        // 現在地を名乗れないので、材料をそのまま出す。
        Err(error) => return emit_error(wording::unreadable_execution_cursor(&error.to_string())),
    };
    let (Ok(intent_execution_repository), Ok(intent_repository)) = (
        IntentExecutionRepositoryImpl::open(store),
        IntentRepositoryImpl::open(store),
    ) else {
        return Completion::refused(wording::orchestrate_failure("cannot open the event store"));
    };
    // 成功の逐語は現在地を名乗る（upstream は状態ファイルの `Current Stage` を読む）。
    // 媒体の失敗はここより前で surface 済みなので、行が引けないのは「記録する先が無い」である。
    let current_stage = match current_execution_view(layout) {
        Ok(Some((stage, _))) => stage,
        // 不在 = 記録する先が無い。
        Ok(None) => return emit_error(wording::SKELETON_STANCE_WITHOUT_STATE.to_string()),
        // 媒体の失敗は不在と混ぜない — 材料をそのまま出す。
        Err(message) => return emit_error(message),
    };
    if let Err(error) =
        RecordSkeletonStanceUseCase::new(intent_execution_repository, intent_repository)
            .execute(&execution_id, stance, Utc::now())
            .await
    {
        return emit_error(skeleton_stance_refusal(&current_stage, &error));
    }
    after_projection(layout, || {
        emit(Ok((
            Directive::Print {
                message: wording::skeleton_stance_recorded(stance.as_str(), &current_stage),
            },
            Vec::new(),
        )))
    })
    .await
}

/// stance の記録の拒否を逐語へ写す。
///
/// 拒否の材料（現在地と scope）は**封筒が運んできたもの**を使う — 拒否した瞬間に集約が見て
/// いた値だからである。`fallback` は行から引いた現在地で、封筒が名乗れないときにだけ使う。
fn skeleton_stance_refusal(fallback: &str, error: &SkeletonStanceError) -> String {
    match error {
        // 集約が返す `InvalidTarget` は「現在地が skeleton-gate ステージでない」だけである。
        SkeletonStanceError::Command {
            stage,
            scope,
            error: CommandError::InvalidTarget(_),
        } => wording::not_the_skeleton_gate(
            stage.as_ref().map_or(fallback, StageSlug::as_str),
            scope,
        ),
        other => wording::skeleton_stance_failed(fallback, &chained(other)),
    }
}

/// 段 4 — 再開の選択をルーティングする（遷移はコミットしない）。
fn resume_report(layout: &Layout, args: &crate::cli::ReportArgs) -> Completion {
    if explicit_stage(args).is_some() {
        return emit_error(wording::RESUME_TAKES_NO_STAGE.to_string());
    }
    let Some(user_input) = args.user_input().filter(|text| !text.trim().is_empty()) else {
        return emit_error(wording::RESUME_REQUIRES_USER_INPUT.to_string());
    };
    if !state_file_present(layout) {
        return emit_error(wording::RESUME_WITHOUT_STATE.to_string());
    }
    let (stage, scope) = match current_execution_view(layout) {
        Ok(Some(view)) => view,
        // 不在 = 現在地を名乗れない（鋳造前・投影前）。
        Ok(None) => return emit_error(wording::RESUME_WITHOUT_CURRENT_STAGE.to_string()),
        // 媒体の失敗は不在と混ぜない — 材料をそのまま出す。
        Err(message) => return emit_error(message),
    };
    // 数字の応答キーを先に正規化してから意味で照合する（写像の持ち主はエンジンである —
    // ピン `:5417-5424`）。
    let raw = user_input.trim().to_lowercase();
    let choice = match raw.as_str() {
        "1" => "resume from last checkpoint",
        "2" => "redo the current stage",
        "3" => "jump to a stage",
        "4" => "start fresh",
        other => other,
    };
    let message = if choice.contains("redo") {
        wording::resume_redo(&stage, &scope)
    } else if choice.contains("jump") {
        wording::RESUME_JUMP.to_string()
    } else if choice.contains("fresh") || choice.contains("start over") {
        wording::RESUME_START_FRESH.to_string()
    } else if choice.contains("resume")
        || choice.contains("checkpoint")
        || choice.contains("continue")
    {
        wording::resume_from_checkpoint(&stage)
    } else {
        // 拒否は**正規化前の生値**を埋める（upstream も `flags.userInput` をそのまま出す）。
        return emit_error(wording::unrecognized_resume_choice(user_input));
    };
    emit(Ok((Directive::Print { message }, Vec::new())))
}

/// 空白だけの `--stage` は「無い」と同じ（upstream の `flags.stage?.trim()`）。
fn explicit_stage(args: &crate::cli::ReportArgs) -> Option<&str> {
    args.stage()
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

/// 段 13 の env — `AIDLC_SKIP_HUMAN_PRESENCE_GUARD=1` でだけガードが外れる。
fn human_presence_guard() -> bool {
    std::env::var("AIDLC_SKIP_HUMAN_PRESENCE_GUARD")
        .ok()
        .as_deref()
        != Some("1")
}

/// 状態ファイルが**在る**か（0 バイトも「在る」— ピン `:5479-5481`）。
fn state_file_present(layout: &Layout) -> bool {
    layout.state_file().is_some_and(|state_file| {
        FindStateFileUseCase::new(StateFileDaoImpl::new(&state_file))
            .execute()
            .is_ok_and(|found| found.is_some())
    })
}

/// 実行行から現在地 slug と scope を引く。
///
/// **失敗と不在を混ぜない**（`report` 段 6 と同じ規律）。媒体を開けない・引けないは
/// `Err(<診断文言>)` として材料を運び、`Ok(None)` は**不在**だけを意味する —
/// 実行カーソルが無い（まだ鋳造していない）、実行行がまだ投影されていない、
/// 行に `cursor_slug` が無い、の 3 形である。呼び出し側は不在を各段の逐語へ、
/// 失敗をそのまま error directive へ写す。
fn current_execution_view(layout: &Layout) -> Result<Option<(String, String)>, String> {
    let store = store_path(layout)?;
    let unreadable =
        |cause: &str| wording::read_model_unreadable(&store.as_path().to_string_lossy(), cause);
    let execution_id = match active_execution(layout) {
        Ok(Some(cursor)) => cursor.execution_id().clone(),
        Ok(None) => return Ok(None),
        Err(error) => return Err(wording::unreadable_execution_cursor(&error.to_string())),
    };
    let daos = ReadModelDaos::open(store.as_path())
        .map_err(|error| unreadable(&error.kind().to_string()))?;
    let found = FindExecutionUseCase::new(daos.execution())
        .execute(execution_id.as_str())
        .map_err(|error| unreadable(&error.kind().to_string()))?;
    let Some(found) = found else { return Ok(None) };
    let Some(stage) = found.cursor_slug() else {
        return Ok(None);
    };
    Ok(Some((stage.to_string(), found.scope().to_string())))
}

/// 成功 3 形を directive へ写す（`raw` は報告された生の語）。
fn committed_directive(raw: &str, outcome: &CommitOutcome) -> Directive {
    match outcome {
        CommitOutcome::Committed {
            stage,
            scope,
            steps,
        } => committed_transition(raw, stage.as_str(), scope, steps),
        CommitOutcome::NoOp { scope, no_op, .. } => no_op_directive(scope, no_op),
    }
}

/// コミットした段の列を逐語へ写す。
///
/// gate 系 3 段は `print`（`Recorded <result> for "<slug>".`）、読み飛ばしと前進は `done`。
fn committed_transition(raw: &str, stage: &str, scope: &str, steps: &TransitionSteps) -> Directive {
    // 段の同定は名前付きクエリで行う（スライスの形合わせをやめる — BR5.5）。
    let recorded = [
        TransitionStep::GateStart,
        TransitionStep::Reject,
        TransitionStep::Revise,
    ]
    .into_iter()
    .any(|step| steps.is_single(step));
    if recorded {
        return Directive::Print {
            message: wording::recorded_result(raw, stage),
        };
    }
    if steps.is_single(TransitionStep::Skip) {
        return Directive::Done {
            reason: Some(wording::committed_skip(stage, scope)),
        };
    }
    Directive::Done {
        reason: Some(wording::committed_transition(
            &steps
                .fold_left(Vec::new(), |mut names, step| {
                    names.push(step.subcommand());
                    names
                })
                .join(" + "),
            stage,
            scope,
        )),
    }
}

/// no-op 3 形を directive へ写す。
fn no_op_directive(scope: &str, no_op: &ReportNoOp) -> Directive {
    match no_op {
        ReportNoOp::AlreadyAwaiting { stage } => Directive::Print {
            message: wording::already_awaiting_approval(stage.as_str()),
        },
        ReportNoOp::AlreadyCompletedMovedOn { stage, current } => Directive::Done {
            reason: Some(wording::already_completed_moved_on(
                stage.as_str(),
                current.as_str(),
                scope,
            )),
        },
        ReportNoOp::WorkflowAlreadyCompleted { stage } => Directive::Done {
            reason: Some(wording::workflow_already_completed(stage.as_str(), scope)),
        },
    }
}

/// 失敗を逐語へ写す（`raw` は報告された生の語 — upstream も `flags.result` を埋める）。
fn commit_refusal(raw: &str, error: &CommitError) -> String {
    match error {
        CommitError::Refused(refusal) => report_refusal(raw, refusal),
        // 段 11 — レビュアー受領証の欠落だけは `aidlc-state.ts approve` の stderr 逐語を
        // 包み文の中に置く（upstream も spawn 先の出力をそのまま挟む — b46 の既存形）。
        // 段 12 — 昇格受領証の欠落は **orchestrate 自身の** error directive である
        // （upstream は `aidlc-state approve` を spawn する前に断るので、包み文に入らない）。
        CommitError::Transition {
            error: CommandError::PracticesReceiptMissing(_),
            ..
        } => wording::PRACTICES_RECEIPT_MISSING.to_string(),
        CommitError::Transition {
            step,
            stage,
            error: CommandError::ReviewReceiptMissing { reviewer, .. },
        } => wording::transition_rejected_by(
            step.subcommand(),
            stage.as_str(),
            &wording::reviewer_precondition(stage.as_str(), reviewer),
        ),
        CommitError::Transition { step, stage, error } => {
            wording::transition_rejected_by(step.subcommand(), stage.as_str(), &chained(error))
        }
        CommitError::UnwiredTransition { step, stage } => {
            wording::transition_not_wired(step.subcommand(), stage.as_str())
        }
        // 再構成・計画取得の失敗は upstream に対応する逐語が無い（あちらはファイルを読む
        // だけである）— 中継形に材料を載せる。
        other => wording::transition_rejected(&chained(other)),
    }
}

/// 集約の判断による拒否 13 形を逐語へ写す。
fn report_refusal(raw: &str, refusal: &ReportRefusal) -> String {
    match refusal {
        ReportRefusal::UnknownStage { named } => wording::reported_stage_not_in_graph(named),
        ReportRefusal::RoutedVerdict { .. } => wording::RESUME_IS_ROUTED.to_string(),
        ReportRefusal::SkipNotConditional { stage, execution } => {
            wording::skip_not_conditional(stage.as_str(), execution.as_str())
        }
        ReportRefusal::SkipRequiresReason { .. } => wording::SKIP_REQUIRES_REASON.to_string(),
        ReportRefusal::SkipMustNameCursor { named, current } => {
            wording::skip_must_name_cursor(named.as_str(), current.as_str())
        }
        ReportRefusal::SkipPrecondition { stage, actual } => {
            wording::skip_precondition(stage.as_str(), actual.spelling())
        }
        ReportRefusal::UngatedStage { stage, .. } => wording::ungated_stage(stage.as_str(), raw),
        ReportRefusal::GatePrecondition {
            stage,
            verdict,
            actual,
        } => gate_precondition(*verdict, stage.as_str(), actual.spelling()),
        ReportRefusal::RejectRequiresFeedback { stage } => {
            wording::reject_requires_feedback(stage.as_str())
        }
        ReportRefusal::HumanPresence { stage, .. } => {
            wording::human_presence_required(raw, stage.as_str())
        }
        ReportRefusal::ForwardCommitsCompletionsOnly { stage, actual } => {
            wording::forward_commits_completions_only(stage.as_str(), actual.spelling())
        }
        ReportRefusal::StillPending { stage } => wording::still_pending(stage.as_str()),
        ReportRefusal::InProgressRequiresExplicitStage { stage } => {
            wording::in_progress_requires_explicit_stage(stage.as_str())
        }
    }
}

/// gate 系 3 語の前提違反は語ごとに文言が違う（ピン `:5706` / `:5717` / `:5732`）。
fn gate_precondition(verdict: Verdict, stage: &str, state: &str) -> String {
    match verdict {
        Verdict::AwaitingApproval => wording::gate_open_precondition(stage, state),
        Verdict::Rejected => wording::gate_reject_precondition(stage, state),
        // 集約が `GatePrecondition` に載せる verdict は gate 系 3 語だけである。
        _ => wording::gate_revise_precondition(stage, state),
    }
}

/// `aidlc-log review` — レビュー受領証の対を記録する（b48 / B10）。
///
/// 段の順序は upstream `handleReview`（ピン `3c3146cf` `aidlc-log.ts:900-1168`）と同順である:
/// フラグ文法 → `--stage` 必須 → `--reviewer` 必須 → セレクタ拒否 → アクティブ intent →
/// (`--unit` / `--single` の未配線拒否) → 依頼形なら `--iteration`、判定形なら
/// `--retry-pending` 併用 → `--iteration` → `--verdict` の閉集合 → 記録。
///
/// **失敗はすべて stderr + exit 1** である（`Completion::refused`）— upstream の `error()` は
/// directive を出さない。`ERROR_LOGGED` 行は本 build では描かない（逸脱台帳）。
async fn log_review(layout: &Layout, args: &crate::cli::ReviewArgs) -> Completion {
    // 段 0 — フラグ文法そのものの違反（値が必要なフラグに値が無い）。
    if let Some(refusal) = args.parse_error() {
        return Completion::refused(refusal.to_string());
    }
    let Some(stage) = args.stage() else {
        return Completion::refused(wording::REVIEW_REQUIRES_STAGE.to_string());
    };
    let Some(reviewer) = args.reviewer() else {
        return Completion::refused(wording::REVIEW_REQUIRES_REVIEWER.to_string());
    };
    if args.intent().is_some() || args.space().is_some() {
        return Completion::refused(wording::REVIEW_TAKES_NO_SELECTORS.to_string());
    }
    let store = match store_path(layout) {
        Ok(store) => store,
        Err(message) => return Completion::refused(message),
    };
    let execution_id = match active_execution(layout) {
        Ok(Some(cursor)) => cursor.execution_id().clone(),
        // 不在 = まだ鋳造していない。upstream の `activeIntent` が空振りした形である。
        Ok(None) => return Completion::refused(wording::REVIEW_WITHOUT_INTENT.to_string()),
        // 在るのに読めない・壊れているは**不在と混ぜない**（`report` 段 6 と同じ規律）。
        Err(error) => {
            return Completion::refused(wording::unreadable_execution_cursor(&error.to_string()));
        }
    };
    // 未配線の 2 面（own wording — upstream には対応する拒否が無い）。
    if args.unit().is_some() {
        return Completion::refused(wording::REVIEW_UNIT_NOT_WIRED.to_string());
    }
    if args.is_single() {
        return Completion::refused(wording::REVIEW_SINGLE_NOT_WIRED.to_string());
    }
    // 通し番号と依頼形／判定形の分岐は slug の解析より**先**である。upstream の
    // `handleReview` は依頼形を `:983-985`、判定形を `:1124-1134` で検査し、どちらも
    // 宣言・一致を読む `loadContext` の前に置く。
    let (iteration, kind) = match review_log_input(args) {
        Ok(input) => input,
        Err(refusal) => return Completion::refused(refusal),
    };
    // slug の文法違反は「グラフがその名前を知らない」と同じ答えである（upstream の
    // `loadStageGraphAll().find(...)` は空振りするだけ）。
    let Ok(slug) = StageSlug::parse(stage) else {
        return Completion::refused(wording::stage_has_no_declared_reviewer(stage));
    };
    let (
        Ok(intent_execution_repository),
        Ok(intent_repository),
        Ok(workflow_definition_repository),
    ) = (
        IntentExecutionRepositoryImpl::open(&store),
        IntentRepositoryImpl::open(&store),
        WorkflowDefinitionRepositoryImpl::open(&store),
    )
    else {
        return Completion::refused(wording::orchestrate_failure("cannot open the event store"));
    };
    let request = ReviewLogRequest::new(slug, reviewer, iteration, kind);
    let recorded = RecordReviewUseCase::new(
        intent_execution_repository,
        intent_repository,
        workflow_definition_repository,
    )
    .execute(&execution_id, &request, Utc::now())
    .await;
    let outcome = match recorded {
        Ok(outcome) => outcome,
        Err(error) => return Completion::refused(review_refusal(stage, reviewer, args, &error)),
    };
    // 書いた事実をリードモデルへ落とす（監査 1 行はこの投影で台帳に現れる）。
    after_projection(layout, || {
        Completion::emitted(review_log_line(stage, outcome))
    })
    .await
}

/// 通し番号と、依頼形／判定形の分岐（分岐は `--verdict` の有無だけで決まる — upstream `:983`）。
///
/// `--iteration` は**両方の形**でここが検査する。upstream は依頼形を `:983-985`、判定形を
/// `:1124-1134` で検査し、いずれも宣言・一致を読む `loadContext` より前に置くので、
/// 文法違反の slug と通し番号の欠落が重なったときは通し番号の逐語が先に出る。
fn review_log_input(args: &crate::cli::ReviewArgs) -> Result<(u32, ReviewLogKind), String> {
    let Some(raw) = args.verdict() else {
        let Some(iteration) = positive_iteration(args.iteration()) else {
            return Err(wording::REVIEW_REQUEST_REQUIRES_ITERATION.to_string());
        };
        return Ok((
            iteration,
            ReviewLogKind::Request {
                retry_pending: args.is_retry_pending(),
            },
        ));
    };
    if args.is_retry_pending() {
        return Err(wording::REVIEW_RETRY_WITH_VERDICT.to_string());
    }
    // upstream は `--iteration` を検査してから `--verdict` を閉集合に当てる（`:1124-1134`）。
    let Some(iteration) = positive_iteration(args.iteration()) else {
        return Err(wording::REVIEW_COMPLETED_REQUIRES_ITERATION.to_string());
    };
    let verdict = ReviewVerdict::parse(raw)
        .map_err(|unknown| wording::unknown_review_verdict(unknown.as_str()))?;
    Ok((iteration, ReviewLogKind::Verdict(verdict)))
}

/// `--iteration` の閉じた文法 — upstream の `/^[1-9][0-9]*$/` と同じである。
///
/// `u32` に収まらない巨大値は飽和させる。JS の `Number()` はそのまま巨大な値になり
/// 「予算超過」で断られるので、飽和させても同じ答えに落ちる。
fn positive_iteration(raw: Option<&str>) -> Option<u32> {
    let raw = raw?;
    let mut chars = raw.chars();
    let first = chars.next()?;
    if !('1'..='9').contains(&first) || !chars.all(|c| c.is_ascii_digit()) {
        return None;
    }
    Some(raw.parse::<u32>().unwrap_or(u32::MAX))
}

/// 成功の素の JSON 1 行（upstream `:1112-1117` / `:1168`）。
///
/// `aidlc-log` 面は directive プロトコルに参加しないので、`aidlc-utility` と同じく
/// 契約 JSON をそのまま出す（直列化は canon-json を通す — BR1.7）。
fn review_log_line(stage: &str, outcome: ReviewLogOutcome) -> String {
    let mut emitted = ObjectMembers::new();
    match outcome {
        ReviewLogOutcome::Requested { retry } => {
            emitted.insert(
                "emitted",
                JsonValue::String(EventType::ReviewRequested.as_str().to_string()),
            );
            emitted.insert("stage", JsonValue::String(stage.to_string()));
            if retry {
                emitted.insert("retry", JsonValue::String("pending-request".to_string()));
            }
        }
        ReviewLogOutcome::Completed => {
            emitted.insert(
                "emitted",
                JsonValue::String(EventType::ReviewCompleted.as_str().to_string()),
            );
            emitted.insert("stage", JsonValue::String(stage.to_string()));
        }
    }
    serialize(
        &JsonValue::Object(emitted),
        SerializationProfile::ContractCompact,
    )
}

/// 受領証の記録の拒否を逐語へ写す。
///
/// 逐語を選ぶのは**出す側**である（`coding-rules/error-handling.md`）。集約が運ぶのは
/// 材料だけなので、`NoPendingReview` が判定形と retry 形で言い回しを分けるのもここである。
fn review_refusal(
    stage: &str,
    reviewer: &str,
    args: &crate::cli::ReviewArgs,
    error: &ReviewLogError,
) -> String {
    match error {
        // 「定義がその slug を知らない」と「宣言が無い」は upstream では同じ文言である。
        ReviewLogError::UnknownStage(_)
        | ReviewLogError::Command {
            error: CommandError::UnknownStage(_) | CommandError::NoDeclaredReviewer(_),
            ..
        } => wording::stage_has_no_declared_reviewer(stage),
        ReviewLogError::Command {
            error: CommandError::ReviewerMismatch { declared, .. },
            ..
        } => wording::reviewer_does_not_match(stage, reviewer, declared),
        ReviewLogError::Command {
            error:
                CommandError::ReviewBudgetExceeded {
                    ordinal, budget, ..
                },
            ..
        } => wording::review_budget_exceeded(stage, *ordinal, *budget),
        ReviewLogError::Command {
            error:
                CommandError::ReviewOutOfSequence {
                    iteration,
                    expected,
                    ..
                },
            ..
        } => wording::review_out_of_sequence(stage, *iteration, *expected),
        ReviewLogError::Command {
            error: CommandError::NoPendingReview { iteration, .. },
            ..
        } => {
            if args.is_retry_pending() {
                wording::review_retry_without_request(stage, *iteration)
            } else {
                wording::review_completed_without_request(stage, *iteration)
            }
        }
        other => wording::review_log_failed(stage, &chained(other)),
    }
}

/// `aidlc-state practices-promote` — 承認された実践をメモリ層の正本へ書き写す（b49 / B10）。
///
/// 段の順序は upstream `handlePracticesPromote`（ピン `3c3146cf` `aidlc-state.ts:3511-3770`）と
/// 同順である: フラグ文法 →（`--target-dir` の未配線拒否）→ アクティブ intent →
/// Step 1 ensemble 証跡 → Step 2 ドラフト読取 → Step 3 正本読取 → Step 4 昇格内容の計算 →
/// 記録 → 投影 → stdout JSON 1 行。
///
/// **失敗はすべて stderr + exit 1** である（`Completion::refused`）— upstream の `error()` は
/// directive を出さない。失敗時の `PRACTICES_OVERRIDE` 行は本 build では描かない（逸脱台帳）。
async fn practices_promote(layout: &Layout, args: &crate::cli::PromoteArgs) -> Completion {
    // 段 0 — 2 つの必須フラグ（upstream `:3520-3524`）。
    let (Some(team_practices), Some(discovered_rules)) =
        (args.team_practices(), args.discovered_rules())
    else {
        return Completion::refused(wording::PROMOTE_USAGE.to_string());
    };
    // 未配線の 1 面（own wording — 書込先は投影が持つ）。
    if args.target_dir().is_some() {
        return Completion::refused(wording::PROMOTE_TARGET_DIR_NOT_WIRED.to_string());
    }
    let store = match store_path(layout) {
        Ok(store) => store,
        Err(message) => return Completion::refused(message),
    };
    let execution_id = match active_execution(layout) {
        Ok(Some(cursor)) => cursor.execution_id().clone(),
        // 不在 = まだ鋳造していない。
        Ok(None) => return Completion::refused(wording::PROMOTE_WITHOUT_INTENT.to_string()),
        // 在るのに読めない・壊れているは**不在と混ぜない**（`report` 段 6 と同じ規律）。
        Err(error) => {
            return Completion::refused(wording::unreadable_execution_cursor(&error.to_string()));
        }
    };
    // 定義の面（practices ステージの宣言と support agents）を読む前に投影を追いつかせる。
    if let Err(message) = catch_up_before_reading(layout).await {
        return Completion::refused(message);
    }
    let support_agents = match practices_support_agents(layout, &store) {
        Ok(agents) => agents,
        Err(message) => return Completion::refused(message),
    };
    // Step 1 — hub-and-spoke の証跡を書込の直前に取り直す（ゲート開放後に消えうる）。
    if let Err(message) = verify_ensemble(team_practices, discovered_rules, &support_agents) {
        return Completion::refused(message);
    }
    // Step 2 / 3 — ドラフト 2 本と正本 2 本を読む（どちらも fail-closed）。
    let team_md_path = layout.memory_dir().join("team.md");
    let project_md_path = layout.memory_dir().join("project.md");
    let drafts = read_pair(
        Path::new(team_practices),
        Path::new(discovered_rules),
        &[
            wording::promote_team_practices_not_found(team_practices),
            wording::promote_discovered_rules_not_found(discovered_rules),
        ],
        wording::promote_unreadable_drafts,
    );
    let (team_practices_draft, discovered_rules_draft) = match drafts {
        Ok(pair) => pair,
        Err(message) => return Completion::refused(message),
    };
    let targets = read_pair(
        &team_md_path,
        &project_md_path,
        &[
            wording::promote_team_md_not_found(&team_md_path.to_string_lossy()),
            wording::promote_project_md_not_found(&project_md_path.to_string_lossy()),
        ],
        wording::promote_unreadable_targets,
    );
    let (team_md, project_md) = match targets {
        Ok(pair) => pair,
        Err(message) => return Completion::refused(message),
    };
    // Step 4 — 昇格内容の計算は純関数である（判断ではなく計算 — 設計 §1）。
    let occurred_at = Utc::now();
    let promotion = match PracticesPromotion::plan(
        &team_practices_draft,
        &discovered_rules_draft,
        &team_md,
        &project_md,
        occurred_at.date_naive(),
    ) {
        Ok(promotion) => promotion,
        Err(error) => return Completion::refused(promotion_plan_refusal(&error)),
    };

    let (Ok(intent_execution_repository), Ok(intent_repository)) = (
        IntentExecutionRepositoryImpl::open(&store),
        IntentRepositoryImpl::open(&store),
    ) else {
        return Completion::refused(wording::orchestrate_failure("cannot open the event store"));
    };
    let request = PracticesPromotionRequest::new(promotion.clone(), args.affirming_user());
    if let Err(error) = PromotePracticesUseCase::new(intent_execution_repository, intent_repository)
        .execute(&execution_id, &request, occurred_at)
        .await
    {
        return Completion::refused(wording::promote_failed(&chained(&error)));
    }
    // 書いた事実をリードモデルへ落とす（メモリ層 2 本・状態ファイル・監査行はこの投影で
    // 現れる）。
    after_projection(layout, || {
        Completion::emitted(promote_line(
            &promotion,
            &occurred_at,
            &team_md_path.to_string_lossy(),
            &project_md_path.to_string_lossy(),
        ))
    })
    .await
}

/// practices-discovery ステージの support agents（グラフに無ければ拒否）。
///
/// クエリ側の `read_definition_stage` を slug で 1 引当する — 「そのステージがグラフに在るか」
/// は**行が引けたかどうか**であり、判断はここに無い（`coding-rules/cqrs-boundaries.md`）。
fn practices_support_agents(layout: &Layout, store: &StorePath) -> Result<Vec<String>, String> {
    let unreadable =
        |cause: &str| wording::read_model_unreadable(&store.as_path().to_string_lossy(), cause);
    let definition_id = definition_id(layout)?;
    let daos = ReadModelDaos::open(store.as_path())
        .map_err(|error| unreadable(&error.kind().to_string()))?;
    let found = FindDefinitionStageUseCase::new(daos.definition_stage())
        .execute(definition_id.as_str(), PRACTICES_DISCOVERY_SLUG)
        .map_err(|error| unreadable(&error.kind().to_string()))?;
    let Some(row) = found else {
        return Err(wording::promote_failed(wording::PROMOTE_STAGE_ABSENT));
    };
    crate::directive_drawing::strings("support_agents", row.support_agents())
}

/// Step 1 — ドラフト 2 本が同じディレクトリに在り、宣言された support agents の
/// contributions が揃っていることを確かめる（upstream `:3564-3586`）。
fn verify_ensemble(
    team_practices: &str,
    discovered_rules: &str,
    support_agents: &[String],
) -> Result<(), String> {
    let draft_dir = Path::new(team_practices).parent();
    if Path::new(discovered_rules).parent() != draft_dir {
        return Err(wording::promote_failed(
            wording::PROMOTE_DRAFTS_MUST_SHARE_DIR,
        ));
    }
    let draft_dir = draft_dir.unwrap_or_else(|| Path::new(""));
    let missing: Vec<String> = support_agents
        .iter()
        .filter_map(|agent| {
            let contribution = draft_dir.join("contributions").join(format!("{agent}.md"));
            match std::fs::read_to_string(&contribution) {
                Err(_) => Some(wording::promote_missing_contribution(agent)),
                Ok(text) => {
                    let first = text.split('\n').next().unwrap_or_default().trim();
                    (first != format!("**Collaborator:** {agent}"))
                        .then(|| wording::promote_missing_identity_marker(agent))
                }
            }
        })
        .collect();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(wording::promote_failed(
            &wording::promote_incomplete_ensemble(&missing.join("; ")),
        ))
    }
}

/// 2 本のファイルを読む — 不在は個別の逐語、読めないのは共通の逐語（upstream の 2 段）。
fn read_pair(
    first: &Path,
    second: &Path,
    not_found: &[String; 2],
    unreadable: fn(&str) -> String,
) -> Result<(String, String), String> {
    for (path, message) in [(first, &not_found[0]), (second, &not_found[1])] {
        if !path.exists() {
            return Err(wording::promote_failed(message));
        }
    }
    let read = |path: &Path| {
        std::fs::read_to_string(path)
            .map_err(|error| wording::promote_failed(&unreadable(&error.to_string())))
    };
    Ok((read(first)?, read(second)?))
}

/// 昇格内容の計算の拒否を逐語へ写す（見出し不在の 2 形）。
fn promotion_plan_refusal(error: &PromotionPlanError) -> String {
    match error {
        PromotionPlanError::TeamHeadingMissing(heading) => {
            wording::promote_failed(&wording::promote_replace_section_failed(heading))
        }
        PromotionPlanError::ProjectHeadingMissing(heading) => wording::promote_failed(
            &wording::promote_append_failed(heading.trim_start_matches("## ")),
        ),
        // 見出しは固定 5 種を順に 1 度ずつ見るので構成不能だが、変種を握り潰さず
        // 置換先の見出し名を材料にして断る (`error-handling.md` — 無言の失敗にしない)。
        PromotionPlanError::DuplicateSection(heading) => {
            wording::promote_failed(&wording::promote_replace_section_failed(heading))
        }
    }
}

/// 成功の素の JSON 1 行（upstream `:3759-3769` の鍵順）。
fn promote_line(
    promotion: &PracticesPromotion,
    occurred_at: &chrono::DateTime<Utc>,
    team_md: &str,
    project_guardrails: &str,
) -> String {
    let mut emitted = ObjectMembers::new();
    emitted.insert(
        "emitted",
        JsonValue::String(EventType::PracticesAffirmed.as_str().to_string()),
    );
    emitted.insert(
        "sections_written",
        JsonValue::Array(
            promotion
                .sections_written()
                .into_iter()
                .map(|heading| JsonValue::String(heading.to_string()))
                .collect(),
        ),
    );
    emitted.insert(
        "mandated_appended",
        JsonValue::Number(Number::PosInt(u64::from(promotion.mandated_appended()))),
    );
    emitted.insert(
        "forbidden_appended",
        JsonValue::Number(Number::PosInt(u64::from(promotion.forbidden_appended()))),
    );
    emitted.insert(
        "affirmed_at",
        JsonValue::String(occurred_at.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)),
    );
    emitted.insert("team_md", JsonValue::String(team_md.to_string()));
    emitted.insert(
        "project_guardrails",
        JsonValue::String(project_guardrails.to_string()),
    );
    serialize(
        &JsonValue::Object(emitted),
        SerializationProfile::ContractCompact,
    )
}

/// `aidlc-bolt set-autonomy` — Construction の自律モードを切り替える（b50 / I11）。
///
/// 段の順序は upstream `handleSetAutonomy`（ピン `3c3146cf` `aidlc-bolt.ts:799-859`）と
/// 同順である: フラグ文法 → `--mode` 必須 → `--mode` の閉集合 → アクティブ intent →
/// （投影の追いつき）→ 監査台帳の読取 → 状態ファイルの欄検査 → 記録（昇格の presence ガードは
/// 集約の中）→ 投影 → stdout JSON 1 行。
///
/// upstream は presence 検査・監査追記・状態書込を 1 つの `withAuditLock` に囲うが、こちらは
/// **1 つのジャーナル追記**がその原子性を担う（ADR-007 のロック退役）— 2 つの付与が同じ turn を
/// 消費する競合は、楽観 version が弾く。
///
/// **判断はここに 1 つも無い**。合成ルートが持つのは値の有無・既知値・env で決まる構文段と、
/// 外部の材料（監査台帳の `HUMAN_TURN` 行）の読取だけで、昇格の可否は集約のガードに閉じている
/// （設計 §1）。
///
/// **失敗はすべて stderr + exit 1** である（`Completion::refused`）— upstream の `error()` は
/// directive を出さない。
async fn set_autonomy(layout: &Layout, args: &crate::cli::SetAutonomyArgs) -> Completion {
    // 段 0 — フラグ文法そのものの違反（値が必要なフラグに値が無い）。
    if let Some(refusal) = args.parse_error() {
        return Completion::refused(refusal.to_string());
    }
    let Some(raw_mode) = args.mode() else {
        return Completion::refused(wording::SET_AUTONOMY_REQUIRES_MODE.to_string());
    };
    // 閉集合の検査は境界型が持つ（b26 以来消費者の無かった `AutonomyMode::parse` の着地）。
    let Ok(mode) = AutonomyMode::parse(raw_mode) else {
        return Completion::refused(wording::invalid_autonomy_mode(raw_mode));
    };
    let store = match store_path(layout) {
        Ok(store) => store,
        Err(message) => return Completion::refused(message),
    };
    let execution_id = match active_execution(layout) {
        Ok(Some(cursor)) => cursor.execution_id().clone(),
        // 不在 = まだ鋳造していない。
        Ok(None) => return Completion::refused(wording::SET_AUTONOMY_WITHOUT_INTENT.to_string()),
        // 在るのに読めない・壊れているは**不在と混ぜない**（`report` 段 6 と同じ規律）。
        Err(error) => {
            return Completion::refused(wording::unreadable_execution_cursor(&error.to_string()));
        }
    };
    // 状態ファイルの欄を最新の投影で検査するため、読む前に追いつかせる。
    if let Err(message) = catch_up_before_reading(layout).await {
        return Completion::refused(message);
    }
    // 外部の材料 — `HUMAN_TURN` はフックが監査シャードへ直接書く一次の事実であり、我々の
    // 投影ではない。読んで値オブジェクトにするだけで、**判断はしない**（設計 §1）。
    let turns = HumanTurns::find_in(&audit_ledger(layout));
    // 状態ファイルの欄検査（upstream `setFieldStrict` を書込前に通す形の写し — 構文段）。
    if let Some(refusal) = autonomy_field_guard(layout) {
        return Completion::refused(refusal);
    }
    let (Ok(intent_execution_repository), Ok(intent_repository)) = (
        IntentExecutionRepositoryImpl::open(&store),
        IntentRepositoryImpl::open(&store),
    ) else {
        return Completion::refused(wording::orchestrate_failure("cannot open the event store"));
    };
    let request = AutonomySwitchRequest::new(mode, turns, human_presence_guard());
    if let Err(error) = SwitchAutonomyUseCase::new(intent_execution_repository, intent_repository)
        .execute(&execution_id, &request, Utc::now())
        .await
    {
        return Completion::refused(switch_autonomy_refusal(&error));
    }
    // 書いた事実をリードモデルへ落とす（状態ファイルの `Construction Autonomy Mode` と
    // 監査行 `AUTONOMY_MODE_SET` はこの投影で現れる）。
    after_projection(layout, || Completion::emitted(set_autonomy_line(mode))).await
}

/// 監査シャードの連結バッファ（record が無ければ空）。
///
/// 列挙とファイル読取は投影側のヘルパが持つ（11-workspace §2.3 — シャードの I/O は投影の
/// 責務であり、ドメインへは連結済みのバッファが渡る）。合成ルートは両側と RMU を知ってよい
/// 唯一の場所である（`coding-rules/cqrs-boundaries.md`）。
fn audit_ledger(layout: &Layout) -> String {
    layout
        .audit_dir()
        .map(|dir| core_read_model_updater::workspace::read_all_audit_shards(&dir))
        .unwrap_or_default()
}

/// 状態ファイルに `Construction Autonomy Mode` 欄が在るか（upstream `setFieldStrict` の検査）。
///
/// 逸脱台帳 #2 の M12 修正により誕生が欄を書くので、ここに掛かるのは**手編集で欄を消した
/// ときだけ**である。状態ファイルそのものが無い（まだ鋳造していない）ときは検査しない —
/// upstream の `readStateFile` はそこで別の失敗になるが、こちらは実行カーソルの段で既に
/// 断っている。
fn autonomy_field_guard(layout: &Layout) -> Option<String> {
    let state_file = layout.state_file()?;
    let found = FindStateFileUseCase::new(StateFileDaoImpl::new(&state_file)).execute();
    let content = match found {
        Ok(Some(content)) => content,
        Ok(None) => return None,
        Err(error) => {
            return Some(wording::read_model_unreadable(
                &state_file.to_string_lossy(),
                &error.kind().to_string(),
            ));
        }
    };
    let prefix = format!("- **{}**:", wording::CONSTRUCTION_AUTONOMY_MODE_FIELD);
    if content.lines().any(|line| line.starts_with(&prefix)) {
        None
    } else {
        Some(wording::state_field_not_found(
            wording::CONSTRUCTION_AUTONOMY_MODE_FIELD,
        ))
    }
}

/// 切替の拒否を逐語へ写す。
///
/// 逐語を選ぶのは**出す側**である（`coding-rules/error-handling.md`）。集約が運ぶのは
/// `HumanPresenceRequired` という材料だけなので、I11 の長い逐語はここで当てる。
fn switch_autonomy_refusal(error: &SwitchAutonomyError) -> String {
    match error {
        SwitchAutonomyError::Command(CommandError::HumanPresenceRequired) => {
            wording::HUMAN_PRESENCE_REQUIRED.to_string()
        }
        other => wording::switch_autonomy_failed(&chained(other)),
    }
}

/// 成功の素の JSON 1 行（upstream `:852-857` の鍵順）。
fn set_autonomy_line(mode: AutonomyMode) -> String {
    let mut emitted = ObjectMembers::new();
    emitted.insert(
        "emitted",
        JsonValue::String(EventType::AutonomyModeSet.as_str().to_string()),
    );
    emitted.insert("mode", JsonValue::String(mode.as_state_field().to_string()));
    emitted.insert("state_updated", JsonValue::Bool(true));
    serialize(
        &JsonValue::Object(emitted),
        SerializationProfile::ContractCompact,
    )
}

/// `park` — 実行を現在地で止め、投影してから止まった位置を名乗る。
///
/// upstream の `handlePark`（`aidlc-orchestrate.ts:8233-8261`）と同じ二段構えである:
/// 変更は `aidlc-state.ts park` に閉じ、失敗はその出力を `Cannot park the workflow: <detail>`
/// に包んだ **error directive（stdout・exit 0）** で返し、成功したら**状態ファイルを読み直して**
/// `Parked At Stage` を名乗る。こちらもコマンド → RMU → クエリの経路で同じ形になる。
async fn park(layout: &Layout) -> Completion {
    let store = match store_path(layout) {
        Ok(store) => store,
        Err(message) => return emit_error(message),
    };
    let execution_id = match active_execution(layout) {
        Ok(Some(cursor)) => cursor.execution_id().clone(),
        // 不在 = まだ鋳造していない（fresh なワークスペースの正常な姿）。
        Ok(None) => return emit_error(wording::park_refused(wording::PARK_WITHOUT_EXECUTION)),
        // 在るのに読めない・壊れているは**不在と混ぜない**（`report` と同じ規律）。
        Err(error) => {
            return emit_error(wording::park_refused(
                &wording::unreadable_execution_cursor(&error.to_string()),
            ));
        }
    };
    let (Ok(intent_execution_repository), Ok(intent_repository)) = (
        IntentExecutionRepositoryImpl::open(&store),
        IntentRepositoryImpl::open(&store),
    ) else {
        return Completion::refused(wording::orchestrate_failure("cannot open the event store"));
    };
    if let Err(error) = ParkUseCase::new(intent_execution_repository, intent_repository)
        .execute(&execution_id, Utc::now())
        .await
    {
        return emit_error(park_refusal(&error));
    }
    // 書いた事実をリードモデルへ落とす。ここは握り潰さない — 描けなければ park した位置も
    // 読めない（`report` と同じ規律）。
    after_projection(layout, || match parked_directive(&store, &execution_id) {
        Ok(directive) => emit(Ok((directive, Vec::new()))),
        Err(refusal) => refusal,
    })
    .await
}

/// 集約の拒否 2 形だけを upstream 逐語へ写し、それ以外は中継形の材料にする。
///
/// 逐語を選ぶのは**出す側**の仕事である（`coding-rules/error-handling.md`）。ドメインは
/// `RefusedUnderAutonomy` / `NotRunning` という材料しか運ばない。
fn park_refusal(error: &ParkError) -> String {
    match error {
        ParkError::Command(CommandError::RefusedUnderAutonomy) => {
            wording::park_refused(wording::PARK_REFUSED_AUTONOMOUS)
        }
        ParkError::Command(CommandError::NotRunning) => {
            wording::park_refused(wording::PARK_NOTHING_TO_PARK)
        }
        other => wording::park_refused(&chained(other)),
    }
}

/// 投影された行から park した位置を引いて `parked` directive を組む。
///
/// upstream も mutation のあとに状態ファイルの `Parked At Stage` を読み直す — こちらは同じ
/// 事実を**構造化リードモデル**（`read_execution.parked_at_slug`）から引く。行が無い・slug が
/// 無い・引けないのは**投影直後には起こりえない**（起きたら実装の穴）ので、利用者向けの
/// ビジネス拒否ではなく自己防衛拒否として `Err` を返す。
fn parked_directive(
    store: &StorePath,
    execution_id: &IntentExecutionId,
) -> Result<Directive, Completion> {
    let broken = |detail: &str| {
        Completion::refused(wording::orchestrate_failure(&format!(
            "the park marker was not projected: {detail}"
        )))
    };
    let daos =
        ReadModelDaos::open(store.as_path()).map_err(|error| broken(&error.kind().to_string()))?;
    let found = FindExecutionUseCase::new(daos.execution())
        .execute(execution_id.as_str())
        .map_err(|error| broken(&error.kind().to_string()))?
        .ok_or_else(|| broken("the execution row is missing"))?;
    let slug = found
        .parked_at_slug()
        .ok_or_else(|| broken("the row carries no parked stage"))?;
    let stage = StageSlugView::parse(slug).map_err(|_| broken("the parked stage is not a slug"))?;
    Ok(Directive::Parked {
        message: wording::parked(stage.as_str()),
        stage,
    })
}

/// この record が指す実行を**実行カーソルから**引く。
///
/// **リードモデルは実行の識別子を記録していない**（`aidlc-state.md` にも `intents.json` にも
/// 欄が無い — 実測）。かつてはその穴をジャーナル先頭の実行行で埋めていたが、それは
/// 「実行はワークスペースにただ 1 つ」という仮定に乗っており、2 本目が生まれた瞬間に静かに
/// 別の実行へ報告する。いまは鋳造が [`ExecutionCursor`] を record に据えるので、
/// **どの実行を握っているかは record 自身が答える**（`b43` 設計 §1）。
///
/// record そのものが無い（intent 未鋳造）ときは `Ok(None)` — 不在は失敗ではない。
fn active_execution(layout: &Layout) -> Result<Option<ExecutionCursor>, ExecutionCursorError> {
    layout.record_dir().map_or(Ok(None), ExecutionCursor::read)
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
    /// 実行カーソルを record に据える。
    fn write_execution_cursor(
        &self,
        record: &Path,
        cursor: &ExecutionCursor,
    ) -> Result<(), ExecutionCursorError>;
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

    fn write_execution_cursor(
        &self,
        record: &Path,
        cursor: &ExecutionCursor,
    ) -> Result<(), ExecutionCursorError> {
        cursor.write(record)
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
    // 鋳造した対をそのまま record の実行カーソルにする（`report` はこれで実行を解決する）。
    let cursor = ExecutionCursor::new(execution_id.clone(), intent_id.clone());
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
    let (
        Ok(intent_repository),
        Ok(intent_execution_repository),
        Ok(workflow_definition_repository),
    ) = (
        IntentRepositoryImpl::open(&store),
        IntentExecutionRepositoryImpl::open(&store),
        WorkflowDefinitionRepositoryImpl::open(&store),
    )
    else {
        return Err(wording::orchestrate_failure("cannot open the event store"));
    };
    let reopened_intent_repository = intent_repository.reopened();
    // 鋳造の前に定義を確立しておく（ensure-defined）。ハーネス配布物の 3 入力を取り込み、
    // ストアに定義が無ければ確立し、内容版が違えば改訂する。同じなら何も書かない
    // （冪等は集約の `Unchanged` ガードが決める — `DefineWorkflowUseCase` の doc）。
    //
    // ここに置くのは、`intent-create` が**定義を読む最初の書込動詞**だからである。
    // クエリ側の動詞（`next` / `continue`）は自分のリードモデル読取でファイルを直接読むので
    // この前段を要しない（`coding-rules/cqrs-boundaries.md` 規則 6）。
    let reopened_workflow_definition_repository = workflow_definition_repository.reopened();
    let definition_id =
        definition_id(layout).map_err(|message| wording::orchestrate_failure(&message))?;
    let compiled_definition_id =
        compiled_definition_id(layout).map_err(|message| wording::orchestrate_failure(&message))?;
    ensure_defined(
        layout,
        workflow_definition_repository,
        &compiled_definition_id,
        &definition_id,
        now,
    )
    .await?;
    let mut use_case = CreateIntentUseCase::new(
        reopened_workflow_definition_repository,
        intent_repository,
        intent_execution_repository,
    );
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
        .map_err(|error| mint_failure_wording(&error))?;
    // 骨格を書く — 投影は既存の行を**書き換える**ので、書き換え先が無いと 1 行も描けない
    // (`crate::scaffold` の doc / RMU の `ScaffoldMissing`)。
    let intent = reopened_intent_repository
        .find_by_id(&intent_id)
        .await
        .map_err(|error| diagnose("cannot read back the minted intent", &error))?;
    let scaffold = crate::scaffold::compose(
        &intent,
        &layout.project_dir().to_string_lossy(),
        &now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
    );
    records
        .write_state(&record, &scaffold)
        .map_err(|error| fault("cannot write the state scaffold", &error.to_string()))?;
    // 実行カーソルは active-intent カーソルより**先に**据える — 逆にすると、record を
    // 解決できるのに「どの実行か」が答えられない瞬間が生まれ、その隙の `report` が
    // 「まだ鋳造していない」と誤読する。
    records
        .write_execution_cursor(&record, &cursor)
        .map_err(|error| fault("cannot write the execution cursor", &error.to_string()))?;
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
    workflow_definition_repository: WorkflowDefinitionRepositoryImpl<WorkflowDefinitionSqliteStore>,
    compiled_definition_id: &core_command_domain::workflow_definition::CompiledDefinitionId,
    definition_id: &core_command_domain::workflow_definition::WorkflowDefinitionId,
    now: chrono::DateTime<Utc>,
) -> Result<(), String> {
    DefineWorkflowUseCase::new(
        CompiledDefinitionRepositoryImpl::new(layout.definition_data_dir(), layout.scopes_dir()),
        workflow_definition_repository,
    )
    .execute(compiled_definition_id, definition_id, now)
    .await
    .map_err(|error| diagnose("cannot read the compiled definition", &error))
}

/// 最初の `next` が読む定義行を、intent の記録を作る前に用意する。
///
/// 書込ユースケースと RMU を起動するのは合成ルートの前処理であり、クエリ側ユースケースは
/// 引き続き構造化リードモデルを読むだけである。intent が生まれる前は Markdown の投影先が
/// 無いため、この時点では定義イベントから構造化面だけを描く。
async fn prepare_definition_for_first_read(layout: &Layout) -> Result<(), String> {
    let store = store_path(layout)?;
    let workflow_definition_repository = WorkflowDefinitionRepositoryImpl::open(&store)
        .map_err(|error| diagnose("cannot open the workflow definition repository", &error))?;
    let definition_id =
        definition_id(layout).map_err(|message| wording::orchestrate_failure(&message))?;
    let compiled_definition_id =
        compiled_definition_id(layout).map_err(|message| wording::orchestrate_failure(&message))?;
    ensure_defined(
        layout,
        workflow_definition_repository,
        &compiled_definition_id,
        &definition_id,
        Utc::now(),
    )
    .await?;

    let projection = ProjectionName::parse(STRUCTURED_PROJECTION)
        .map_err(|error| format!("projection name: {error:?}"))?;
    let mut journal_reader = JournalReaderImpl::open(&store)
        .map_err(|error| diagnose("cannot open the definition journal", &error))?;
    ReadModelUpdater::catch_up_structured(&mut journal_reader, &projection)
        .await
        .map(|_| ())
        .map_err(|error| format!("definition projection: {error}"))
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

/// 失敗を `source` 連鎖の**末端まで辿って**診断 1 行に畳む。
///
/// 封筒型（`DefineWorkflowError` / `RepositoryError`）は分類だけを `Display` に書き、
/// 「どのファイルがどう壊れていたか」という実材料は `Error::source` の連鎖に載せる
/// （裁定 6 — エラーは契約の一部なので、内部実装がバレる分類を契約に含めない）。辿らないと
/// 利用者には「壊れていた」しか届かない。
///
/// 家風として封筒の `Display` は子の文言を内包する（`"definition artifacts: {error}"`）ので、
/// **既に描かれている末尾は二度書かない** — 同じ文言が `caused by` で 2 度並ぶのを防ぐ。
fn diagnose(what: &str, error: &dyn std::error::Error) -> String {
    fault(what, &chained(error))
}

/// 鋳造の失敗をユースケースの変種から最終文言へ写す（材料は use-case、文言は出す側 — 家風）。
///
/// 部分失敗（intent 着地後に実行の永続化だけが倒れた）だけは特別扱いする — 変種が運ぶ
/// 孤児 intent の識別子と復旧手順を利用者向けの文言に組む（issue #77 の先行改善。恒久対応は
/// doctor の検出・修復）。それ以外は従来どおり連鎖診断 1 行。
fn mint_failure_wording(error: &CreateIntentError) -> String {
    match error {
        CreateIntentError::ExecutionRepository { orphan, .. } => {
            wording::orphaned_intent(orphan.as_str(), &chained(error))
        }
        _ => wording::orchestrate_failure(&chained(error)),
    }
}

/// 失敗と `source` 連鎖の末端までを 1 つの文字列に畳む（`what` を冠さない形）。
///
/// 既に「何をしようとして失敗したか」が文言に含まれている経路（ユースケースの失敗をそのまま
/// 出す場合）はこちらを使う。連鎖の辿り方は [`diagnose`] と同一である。
fn chained(error: &dyn std::error::Error) -> String {
    let mut rendered = error.to_string();
    let mut source = error.source();
    while let Some(cause) = source {
        let text = cause.to_string();
        if !rendered.ends_with(&text) {
            rendered.push_str(" caused by ");
            rendered.push_str(&text);
        }
        source = cause.source();
    }
    rendered
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
    let mut journal_reader = JournalReaderImpl::open(&store_path(layout)?)
        .map_err(|error| format!("journal: {error}"))?;
    let clone_id = crate::clone_identity::load_or_mint(&layout.aidlc_root())
        .map_err(|error| format!("clone id: {error}"))?;
    let shard = ShardName::of(&host_name(), &clone_id);
    let targets = ProjectionTargets::new(
        state_file,
        audit_dir.join(shard.as_str()),
        layout.memory_dir(),
    );
    journal_reader
        .restore_missing_files(&projection, &targets)
        .map_err(|error| format!("projection restoration: {error}"))?;
    // 参照入力 (memory 層) はジャーナルとは別の入口である — 規則の編集はイベントを
    // 伴わないので、読取先を明示的に渡す。
    let steering = SteeringSource::new(layout.memory_dir());
    ReadModelUpdater::new(journal_reader, projection, targets, steering)
        .catch_up()
        .await
        .map(|_| ())
        .map_err(|error| format!("projection: {error}"))
}

/// 更新結果は公開が完了してから作る。すべての更新動詞が同じ失敗境界を通る。
async fn after_projection(layout: &Layout, published: impl FnOnce() -> Completion) -> Completion {
    match catch_up(layout).await {
        Ok(()) => published(),
        Err(cause) => Completion::refused(wording::orchestrate_failure(&cause)),
    }
}

/// 読む前に投影と復旧を完了する。失敗を隠して古い指示を返したり、次の書込へ進めたりしない。
async fn catch_up_before_reading(layout: &Layout) -> Result<(), String> {
    catch_up(layout)
        .await
        .map_err(|cause| wording::orchestrate_failure(&cause))
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

/// 定義の系譜名 — `harness.json` の `name`（ADR-008）。出荷ハーネスは `claude` で、現状の
/// 合成ルートはその固定値を使う。2 つの識別子（[`definition_id`] / [`compiled_definition_id`]）
/// の源はここ 1 箇所であり、`Layout` から読む形にしても両者が食い違う余地は無い。
const fn harness_name(_layout: &Layout) -> &'static str {
    "claude"
}

fn definition_id(
    layout: &Layout,
) -> Result<core_command_domain::workflow_definition::WorkflowDefinitionId, String> {
    core_command_domain::workflow_definition::WorkflowDefinitionId::parse(harness_name(layout))
        .map_err(|error| format!("cannot resolve the definition id: {error:?}"))
}

/// 配布束の識別子 — 系譜は [`definition_id`] と同じ name（集約ごとに自前の ID 型を持ち、
/// 突合せは合成ルートが同じ源 [`harness_name`] から両方を鋳造することで成立する）。
fn compiled_definition_id(
    layout: &Layout,
) -> Result<core_command_domain::workflow_definition::CompiledDefinitionId, String> {
    core_command_domain::workflow_definition::CompiledDefinitionId::parse(harness_name(layout))
        .map_err(|error| format!("cannot resolve the compiled definition id: {error:?}"))
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
    use core_command_domain::workflow_definition::WorkflowDefinitionId;
    use core_command_domain::workspace::CloneId;
    use core_command_use_case::orchestration::RepositoryError;

    /// 連鎖の途中に居る封筒 (自分の `Display` に子の文言を内包しない形)。
    #[derive(Debug)]
    struct Envelope {
        label: &'static str,
        cause: Box<dyn std::error::Error + Send + Sync>,
    }

    impl std::fmt::Display for Envelope {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str(self.label)
        }
    }

    impl std::error::Error for Envelope {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            Some(self.cause.as_ref())
        }
    }

    // ---- b48: レビュー受領証の私有ヘルパ ----

    /// `--iteration` は upstream の `/^[1-9][0-9]*$/` と同じ文法である。
    #[test]
    fn the_iteration_grammar_matches_the_upstream_regexp() {
        assert_eq!(positive_iteration(Some("1")), Some(1));
        assert_eq!(positive_iteration(Some("12")), Some(12));
        // 欠落・空・0 始まり・符号・非数字・空白は全部外である。
        for raw in [
            None,
            Some(""),
            Some("0"),
            Some("01"),
            Some("+1"),
            Some("-1"),
            Some("1x"),
            Some(" 1"),
        ] {
            assert_eq!(positive_iteration(raw), None, "{raw:?}");
        }
        // `u32` に収まらない値は飽和する（予算超過で断られるので答えは同じである）。
        assert_eq!(positive_iteration(Some("99999999999")), Some(u32::MAX));
    }

    /// 受領証の拒否は材料ごとに逐語が分かれ、それ以外は中継形へ落ちる。
    #[test]
    fn the_review_refusal_falls_back_to_the_relay_form_for_anything_else() {
        let args = crate::cli::ReviewArgs::default();
        let message = review_refusal(
            "domain-design",
            "aidlc-quality-agent",
            &args,
            &ReviewLogError::CorruptReviewOverride("Adversarial".to_string()),
        );
        assert_eq!(
            message,
            "Failed to record the review receipt for \"domain-design\": \
corrupt review override: Adversarial"
        );
    }

    /// 定義がその slug を知らない場合は「宣言が無い」と同じ逐語である。
    #[test]
    fn a_slug_the_definition_does_not_know_reads_as_no_declared_reviewer() {
        let args = crate::cli::ReviewArgs::default();
        let stage = StageSlug::parse("nowhere").expect("文法内の slug");
        assert_eq!(
            review_refusal(
                "nowhere",
                "aidlc-quality-agent",
                &args,
                &ReviewLogError::UnknownStage(stage),
            ),
            "Cannot record review: stage \"nowhere\" has no declared reviewer."
        );
    }

    /// 成功の JSON は動詞で 2 形（呼び直しだけ `retry` を足す）。
    #[test]
    fn the_review_log_line_adds_the_retry_only_for_a_retry() {
        assert_eq!(
            review_log_line(
                "domain-design",
                ReviewLogOutcome::Requested { retry: false }
            ),
            r#"{"emitted":"REVIEW_REQUESTED","stage":"domain-design"}"#
        );
        assert_eq!(
            review_log_line("domain-design", ReviewLogOutcome::Requested { retry: true }),
            r#"{"emitted":"REVIEW_REQUESTED","stage":"domain-design","retry":"pending-request"}"#
        );
        assert_eq!(
            review_log_line("domain-design", ReviewLogOutcome::Completed),
            r#"{"emitted":"REVIEW_COMPLETED","stage":"domain-design"}"#
        );
    }

    #[test]
    fn the_diagnostic_walks_the_chain_to_the_material_at_the_end() {
        // 封筒は分類しか `Display` に書かない (裁定 6)。辿らなければ「壊れていた」しか
        // 利用者に届かない。
        let error = Envelope {
            label: "corrupt",
            cause: Box::new(std::io::Error::other(
                "stage graph at /w/g.json is not valid JSON",
            )),
        };

        assert_eq!(
            diagnose("cannot ingest the workflow definition", &error),
            wording::orchestrate_failure(
                "cannot ingest the workflow definition: corrupt \
                 caused by stage graph at /w/g.json is not valid JSON"
            )
        );
    }

    #[test]
    fn a_wording_that_already_carries_its_cause_is_not_written_twice() {
        // 家風として封筒の `Display` が子の文言を内包することがある
        // (`"definition artifacts: {error}"`)。そのときは `caused by` で重ねない。
        let inner = std::io::Error::other("io: NotFound at /w/g.json");
        let error = Envelope {
            label: "definition artifacts: io: NotFound at /w/g.json",
            cause: Box::new(inner),
        };

        assert_eq!(
            diagnose("cannot ingest the workflow definition", &error),
            wording::orchestrate_failure(
                "cannot ingest the workflow definition: \
                 definition artifacts: io: NotFound at /w/g.json"
            )
        );
    }

    #[test]
    fn a_partial_mint_failure_names_the_orphan_and_the_recovery_path() {
        // intent 着地後に実行の永続化だけが倒れた部分失敗 (issue #77 の先行改善)。
        // 利用者は「どの intent が孤児か」「次に何をすればよいか」をこの文言だけで知る。
        let orphan = IntentId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000").expect("孤児の id");
        let error = CreateIntentError::ExecutionRepository {
            orphan,
            error: RepositoryError::Conflict {
                expected: 0,
                actual: 1,
            },
        };

        let text = mint_failure_wording(&error);
        assert!(
            text.contains("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000"),
            "孤児 intent の識別子を名指す: {text}"
        );
        assert_eq!(
            text.matches("conflict").count(),
            1,
            "ポートの失敗文言を二重に描かない (chained の ends_with 重複抑止が効く形): {text}"
        );
        assert!(
            text.contains("Re-run intent-create"),
            "復旧手順 (再実行で新しい intent が鋳造される) を言う: {text}"
        );
        assert!(
            text.contains("issue #77"),
            "恒久対応 (doctor の検出・修復) の追跡先を言う: {text}"
        );
    }

    #[test]
    fn a_non_partial_mint_failure_keeps_the_plain_diagnostic() {
        // 部分失敗でない鋳造の失敗は従来どおりの診断 1 行 — 孤児の話をしない。
        let error = CreateIntentError::DefinitionRepository(RepositoryError::NotFound {
            id: WorkflowDefinitionId::parse("claude").expect("定義 id"),
        });
        assert_eq!(
            mint_failure_wording(&error),
            wording::orchestrate_failure(&chained(&error))
        );
    }

    #[test]
    fn the_mint_failure_reaches_the_material_the_repository_hid() {
        // 鋳造の失敗はユースケースの文言をそのまま出す経路である（`what` を冠さない）。
        // `RepositoryError::Corrupt` が `source` に載せた実材料まで届くことを、**出す文字列
        // そのもの**で固定する。
        let error = CreateIntentError::DefinitionRepository(RepositoryError::Corrupt {
            id: WorkflowDefinitionId::parse("claude").expect("定義 id"),
            seq_nr: Some(1),
            source: Box::new(std::io::Error::other("undecodable payload")),
        });

        assert_eq!(
            wording::orchestrate_failure(&chained(&error)),
            wording::orchestrate_failure(
                "definition repository: corrupt: aggregate claude, seq_nr 1 \
                 caused by undecodable payload"
            )
        );
    }

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

    /// 記録の書込 4 手のうち**1 手だけ**を失敗させるダブル。他は実 I/O を通すので、
    /// そこへ辿り着くまでの配線（鋳造・ストア・ユースケース）は本物のまま踏める。
    struct FailingAt(Step);

    #[derive(PartialEq, Eq)]
    enum Step {
        /// どの手も失敗させない（ダブルが素通しであることを固定するための対照）。
        Nothing,
        WriteState,
        WriteExecutionCursor,
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

        fn write_execution_cursor(
            &self,
            record: &Path,
            cursor: &ExecutionCursor,
        ) -> Result<(), ExecutionCursorError> {
            if self.0 == Step::WriteExecutionCursor {
                return Err(ExecutionCursorError::Io {
                    kind: std::io::ErrorKind::PermissionDenied,
                    path: record.join(".aidlc-execution"),
                });
            }
            RealRecords.write_execution_cursor(record, cursor)
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

    /// 後段ガードの直前まで、実ストアで鋳造と事前復旧を完了する。
    async fn recovered_test_layout(root: &tempfile::TempDir) -> Layout {
        let completion = create_intent(
            &Layout::resolve(root.path()),
            &intent_create_args(&["--scope", "classic", "--label", "race"]),
        )
        .await;
        assert_eq!(completion.code(), 0, "{completion:?}");
        let layout = Layout::resolve(root.path());
        catch_up_before_reading(&layout)
            .await
            .expect("競合前の復旧は成功する");
        layout
    }

    fn journal_count_at(path: &Path) -> i64 {
        rusqlite::Connection::open(path)
            .expect("真実記録を開く")
            .query_row("SELECT COUNT(*) FROM journal", [], |row| row.get(0))
            .expect("イベント件数")
    }

    fn assert_late_read_error(completion: &Completion, expected: &str) {
        assert_eq!(
            completion.code(),
            0,
            "後段の読取拒否はerror directive: {completion:?}"
        );
        assert_eq!(completion.diagnostic(), None);
        let value: serde_json::Value =
            serde_json::from_str(completion.line().expect("error directive")).expect("JSON");
        assert_eq!(
            value.get("kind").and_then(serde_json::Value::as_str),
            Some("error")
        );
        assert_eq!(
            value.get("message").and_then(serde_json::Value::as_str),
            Some(expected)
        );
    }

    /// 復旧成功はその後のファイル差替えを防げない。後段の2ガードも読取不能を区別する。
    #[tokio::test]
    async fn a_state_file_replaced_after_recovery_keeps_its_path_and_cause() {
        let root = minimal_workspace();
        let layout = recovered_test_layout(&root).await;
        let store = store_path(&layout).expect("store");
        let before = journal_count_at(store.as_path());
        let state = layout.state_file().expect("投影先");
        assert_eq!(state_version_guard(&layout), None);
        assert_eq!(autonomy_field_guard(&layout), None);
        let original = std::fs::read(&state).expect("公開済み状態");
        std::fs::remove_file(&state).expect("別の利用者が状態を置き換える");
        std::fs::create_dir(&state).expect("同名ディレクトリ");
        let expected = format!(
            "Read model not readable at {}: {}. Start a workflow (intent-create) to build it, then run `next` again.",
            state.display(),
            std::io::ErrorKind::IsADirectory
        );
        assert_eq!(state_version_guard(&layout), Some(expected.clone()));
        assert_eq!(autonomy_field_guard(&layout), Some(expected));
        assert!(state.is_dir(), "後段ガードは障害を上書きしない");
        assert_eq!(journal_count_at(store.as_path()), before);
        std::fs::remove_dir(&state).expect("障害除去");
        std::fs::write(&state, original).expect("公開済み状態を戻す");
        assert_eq!(state_version_guard(&layout), None);
        assert_eq!(autonomy_field_guard(&layout), None);
    }

    /// 復旧後に別接続が実行表を壊しても、再開とstanceは不在の逐語や成功へ畳まない。
    #[tokio::test]
    async fn a_read_table_broken_after_recovery_stops_resume_and_stance() {
        let root = minimal_workspace();
        let layout = recovered_test_layout(&root).await;
        let store = store_path(&layout).expect("store");
        let before = journal_count_at(store.as_path());
        assert_eq!(
            current_execution_view(&layout).expect("競合前は読める"),
            Some(("domain-design".into(), "classic".into()))
        );
        let concurrent = rusqlite::Connection::open(store.as_path()).expect("別SQLite接続");
        concurrent
            .execute_batch(
                "DROP TABLE read_execution; CREATE TABLE read_execution (id TEXT PRIMARY KEY);",
            )
            .expect("後段読取の前に表を破損");
        let expected = format!(
            "Read model not readable at {}: {}. Start a workflow (intent-create) to build it, then run `next` again.",
            store.as_path().display(),
            std::io::ErrorKind::Other
        );
        assert_eq!(current_execution_view(&layout), Err(expected.clone()));
        assert_late_read_error(
            &resume_report(
                &layout,
                &report_args(&["--result", "resumed", "--user-input", "1"]),
            ),
            &expected,
        );
        assert_late_read_error(
            &skeleton_stance_report(&layout, "on", &store).await,
            &expected,
        );
        assert_eq!(
            journal_count_at(store.as_path()),
            before,
            "後段で拒否してイベントを残さない"
        );
    }

    /// 復旧後にストアの置き場が塞がれた場合も、現在地の不在とは区別する。
    #[tokio::test]
    async fn a_store_blocked_after_recovery_is_named_by_the_late_query() {
        let root = minimal_workspace();
        let layout = recovered_test_layout(&root).await;
        let store = store_path(&layout).expect("store");
        let before = journal_count_at(store.as_path());
        let saved = store.as_path().with_extension("preserved.sqlite");
        std::fs::rename(store.as_path(), &saved).expect("真実記録を保持した障害注入");
        std::fs::create_dir(store.as_path()).expect("ストアを塞ぐ");
        let expected = format!(
            "Read model not readable at {}: {}. Start a workflow (intent-create) to build it, then run `next` again.",
            store.as_path().display(),
            std::io::ErrorKind::Other
        );
        assert_eq!(current_execution_view(&layout), Err(expected.clone()));
        assert_late_read_error(
            &resume_report(
                &layout,
                &report_args(&["--result", "resumed", "--user-input", "1"]),
            ),
            &expected,
        );
        for completion in [
            single_report(
                &layout,
                &report_args(&[
                    "--single",
                    "--result",
                    "approved",
                    "--stage",
                    "domain-design",
                ]),
                &store,
            )
            .await,
            skeleton_stance_report(&layout, "on", &store).await,
        ] {
            assert_eq!(completion.code(), 1, "{completion:?}");
            assert_eq!(completion.line(), None);
            assert_eq!(
                completion.diagnostic(),
                Some("aidlc-orchestrate: cannot open the event store")
            );
        }
        assert!(store.as_path().is_dir());
        assert_eq!(journal_count_at(&saved), before);
    }

    /// スコープの表を別接続が破損した場合、昇格先のステージ不在として隠さない。
    #[tokio::test]
    async fn a_definition_table_broken_after_recovery_keeps_the_promotions_read_error() {
        let root = minimal_workspace();
        for relative in [
            ".claude/tools/data/stage-graph.json",
            ".claude/tools/data/scope-grid.json",
        ] {
            let path = root.path().join(relative);
            let content = std::fs::read_to_string(&path).expect("合成定義");
            std::fs::write(
                path,
                content.replace("domain-design", "practices-discovery"),
            )
            .expect("昇格ステージを持つ合成定義");
        }
        let layout = recovered_test_layout(&root).await;
        let store = store_path(&layout).expect("store");
        let before = journal_count_at(store.as_path());
        assert_eq!(
            practices_support_agents(&layout, &store).expect("競合前は宣言を読める"),
            Vec::<String>::new()
        );
        let concurrent = rusqlite::Connection::open(store.as_path()).expect("別SQLite接続");
        concurrent.execute_batch("DROP TABLE read_definition_stage; CREATE TABLE read_definition_stage (id TEXT PRIMARY KEY);").expect("昇格先の読取前に表を破損");
        let expected = format!(
            "Read model not readable at {}: {}. Start a workflow (intent-create) to build it, then run `next` again.",
            store.as_path().display(),
            std::io::ErrorKind::Other
        );
        assert_eq!(practices_support_agents(&layout, &store), Err(expected));
        assert_eq!(journal_count_at(store.as_path()), before);
    }

    /// 隔離実行とstanceでも、コミット後の公開失敗を成功にせず次の復旧へ委ねる。
    #[tokio::test]
    async fn specialized_reports_surface_publication_failure_after_the_event_commit() {
        for single in [true, false] {
            let root = minimal_workspace();
            let graph = root.path().join(".claude/tools/data/stage-graph.json");
            let content = std::fs::read_to_string(&graph).expect("合成定義");
            std::fs::write(
                graph,
                content.replace("\"phase\":\"inception\"", "\"phase\":\"construction\""),
            )
            .expect("skeleton gateを持つ定義");
            let layout = recovered_test_layout(&root).await;
            let store = store_path(&layout).expect("store");
            let before = journal_count_at(store.as_path());
            let concurrent = rusqlite::Connection::open(store.as_path()).expect("別SQLite接続");
            let checkpoint = || {
                concurrent.query_row("SELECT last_global_seq FROM amadeus_projection_checkpoint WHERE projection='orchestration'", [], |row| row.get::<_, i64>(0)).expect("公開位置")
            };
            let before_position = checkpoint();
            concurrent.execute_batch(&format!("CREATE TRIGGER fail_late_publication BEFORE INSERT ON amadeus_publication WHEN NEW.target_position > {before_position} BEGIN SELECT RAISE(ABORT,'publication unavailable'); END;")).expect("コミット後の公開を失敗させる");
            let completion = if single {
                single_report(
                    &layout,
                    &report_args(&[
                        "--single",
                        "--result",
                        "approved",
                        "--stage",
                        "domain-design",
                    ]),
                    &store,
                )
                .await
            } else {
                skeleton_stance_report(&layout, "on", &store).await
            };
            assert_eq!(completion.code(), 1, "{completion:?}");
            assert_eq!(completion.line(), None);
            assert_eq!(
                completion.diagnostic(),
                Some(
                    format!(
                        "aidlc-orchestrate: projection: read: io: Other at {}",
                        store.as_path().display()
                    )
                    .as_str()
                )
            );
            assert_eq!(
                journal_count_at(store.as_path()),
                before + 1,
                "真実はコミット済み"
            );
            assert_eq!(checkpoint(), before_position, "未公開の位置へ進めない");
            concurrent
                .execute_batch("DROP TRIGGER fail_late_publication")
                .expect("公開障害を除去");
            catch_up_before_reading(&layout)
                .await
                .expect("保存済みイベントから回復");
            assert_eq!(
                journal_count_at(store.as_path()),
                before + 1,
                "復旧でイベントを増やさない"
            );
            assert_eq!(checkpoint(), before_position + 1);
        }
    }

    /// 読み面が正常でも集約snapshotが壊れていれば、各reportは真実への追記を拒否する。
    #[tokio::test]
    async fn reports_refuse_a_corrupt_aggregate_snapshot_without_mutation() {
        let root = minimal_workspace();
        let layout = recovered_test_layout(&root).await;
        let store = store_path(&layout).unwrap();
        let execution = active_execution(&layout)
            .unwrap()
            .unwrap()
            .execution_id()
            .as_str()
            .to_owned();
        let database = rusqlite::Connection::open(store.as_path()).unwrap();
        let snapshot: Vec<u8> = database
            .query_row(
                "SELECT payload FROM snapshot WHERE aid=?1",
                [&execution],
                |row| row.get(0),
            )
            .unwrap();
        let before = journal_count_at(store.as_path());
        let state = std::fs::read(layout.state_file().unwrap()).unwrap();
        let audit = audit_ledger(&layout);
        assert_eq!(
            database
                .execute(
                    "UPDATE snapshot SET payload=X'00' WHERE aid=?1",
                    [&execution]
                )
                .unwrap(),
            1
        );
        for completion in [
            report(&layout, &report_args(&["--result", "awaiting-approval"])).await,
            single_report(
                &layout,
                &report_args(&[
                    "--single",
                    "--result",
                    "approved",
                    "--stage",
                    "domain-design",
                ]),
                &store,
            )
            .await,
            skeleton_stance_report(&layout, "on", &store).await,
        ] {
            assert_eq!(completion.code(), 0, "{completion:?}");
            let value: serde_json::Value =
                serde_json::from_str(completion.line().unwrap()).unwrap();
            assert_eq!(
                value.get("kind").and_then(serde_json::Value::as_str),
                Some("error")
            );
            let message = value
                .get("message")
                .and_then(serde_json::Value::as_str)
                .unwrap();
            assert!(message.contains("corrupt"), "{message}");
            assert!(message.contains(&execution), "{message}");
            assert_eq!(journal_count_at(store.as_path()), before);
            assert_eq!(std::fs::read(layout.state_file().unwrap()).unwrap(), state);
            assert_eq!(audit_ledger(&layout), audit);
        }
        database
            .execute(
                "UPDATE snapshot SET payload=?1 WHERE aid=?2",
                rusqlite::params![snapshot, execution],
            )
            .unwrap();
        let completion = single_report(
            &layout,
            &report_args(&[
                "--single",
                "--result",
                "approved",
                "--stage",
                "domain-design",
            ]),
            &store,
        )
        .await;
        let value: serde_json::Value = serde_json::from_str(completion.line().unwrap()).unwrap();
        assert_eq!(
            value.get("kind").and_then(serde_json::Value::as_str),
            Some("done")
        );
        assert_eq!(journal_count_at(store.as_path()), before + 1);
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

    /// 集約が拒んだ 2 形以外は、失敗の `Display` をそのまま中継形の材料にする。
    ///
    /// upstream の `handlePark` も spawn した subcommand の出力を丸ごと挟むだけなので、
    /// 逐語を持たない失敗はここで言い換えない。
    #[test]
    fn a_park_failure_outside_the_two_verbatim_forms_is_relayed_as_material() {
        let execution_id =
            IntentExecutionId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000").expect("UUIDv7");
        let error = ParkError::Repository(RepositoryError::NotFound {
            id: execution_id.clone(),
        });

        assert_eq!(
            park_refusal(&error),
            format!("Cannot park the workflow: repository: not found: {execution_id}")
        );
    }

    /// 封筒が運ぶ原因連鎖（`Error::source`）も中継形へ載せる — `Corrupt` は分類しか
    /// `Display` に書かず、実材料（`undecodable payload` 等）は連鎖にあるからである
    /// （裁定 6。upstream も spawn の stderr を丸ごと挟むので診断材料は落とさない）。
    #[test]
    fn a_park_failure_relays_its_source_chain_into_the_detail() {
        let execution_id =
            IntentExecutionId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000").expect("UUIDv7");
        let error = ParkError::Repository(RepositoryError::Corrupt {
            id: execution_id,
            seq_nr: Some(3),
            source: Box::new(std::io::Error::other("undecodable payload")),
        });

        let detail = park_refusal(&error);

        assert!(
            detail.starts_with("Cannot park the workflow: repository: "),
            "{detail}"
        );
        assert!(
            detail.ends_with(" caused by undecodable payload"),
            "{detail}"
        );
    }

    /// 集約の拒否 2 形は upstream 逐語へ写す（材料の `Display` は表に出ない）。
    #[test]
    fn the_two_aggregate_refusals_map_onto_the_upstream_verbatim() {
        assert_eq!(
            park_refusal(&ParkError::Command(CommandError::RefusedUnderAutonomy)),
            wording::park_refused(wording::PARK_REFUSED_AUTONOMOUS)
        );
        assert_eq!(
            park_refusal(&ParkError::Command(CommandError::NotRunning)),
            wording::park_refused(wording::PARK_NOTHING_TO_PARK)
        );
    }

    /// park の直後に投影が読めないのは**実装の穴**なので、自己防衛拒否で止まる。
    ///
    /// 3 形（開けない・引けない・行が無い）はいずれも CLI からは作れない状態である。ここでは
    /// 引当の口だけを直接叩いて、どれもビジネス拒否へ流れないことを固定する。
    #[tokio::test]
    async fn a_park_marker_that_was_not_projected_is_a_self_defence_refusal() {
        let root = minimal_workspace();
        let space = SpaceName::parse("default").expect("既定の空間名");
        let store = StorePath::for_space(&root.path().join("aidlc"), &space);
        let execution_id =
            IntentExecutionId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000").expect("UUIDv7");

        // ストアそのものが無い = 引当の口が開かない。
        let refusal = parked_directive(&store, &execution_id).expect_err("開けない");
        assert_eq!(refusal.code(), 1);
        assert!(
            refusal
                .diagnostic()
                .unwrap_or_default()
                .starts_with("aidlc-orchestrate: the park marker was not projected: "),
            "{refusal:?}"
        );

        // 開けるが引けない = 表が無い（空ファイルは SQLite にとって空のデータベースである）。
        std::fs::create_dir_all(root.path().join("aidlc/spaces/default/intents")).expect("intents");
        std::fs::write(store.as_path(), b"").expect("空のストア");
        let refusal = parked_directive(&store, &execution_id).expect_err("表が無い");
        assert_eq!(refusal.code(), 1);
        assert_eq!(refusal.line(), None, "stdout には何も出さない");

        // 表も行もあるが park していない = マーカーの列が空。
        let args = intent_create_args(&["--scope", "classic", "--label", "demo"]);
        let layout = Layout::resolve(root.path());
        std::fs::remove_file(store.as_path()).expect("空のストアを退ける");
        let minted = create_intent(&layout, &args).await;
        assert_eq!(minted.code(), 0, "{minted:?}");
        // 配置はカーソルを解決時に読むので、鋳造のあとに引き直す。
        let minted_layout = Layout::resolve(root.path());
        let record = minted_layout.record_dir().expect("カーソルが据わっている");
        let minted_id = ExecutionCursor::read(record)
            .expect("実行カーソルは読める")
            .expect("実行カーソルは据わっている")
            .execution_id()
            .clone();
        let refusal = parked_directive(&store, &minted_id).expect_err("park していない");
        assert_eq!(refusal.code(), 1);

        // 行そのものが無い = 別の実行を名指している。
        let refusal = parked_directive(&store, &execution_id).expect_err("行が無い");
        assert_eq!(refusal.code(), 1);
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

    /// 実行カーソルを据えられなければ拒否する — 据わっていない record は「どの実行か」を
    /// 答えられず、以後の `report` が「まだ鋳造していない」と誤読する。
    #[tokio::test]
    async fn an_execution_cursor_that_cannot_be_written_is_refused() {
        let root = minimal_workspace();
        let layout = Layout::resolve(root.path());

        let completion = create_intent_with(
            &layout,
            &intent_create_args(&["--scope", "classic", "--label", "demo"]),
            &FailingAt(Step::WriteExecutionCursor),
        )
        .await;

        assert_eq!(completion.code(), 1);
        assert_eq!(completion.line(), None, "stdout には何も出さない");
        let diagnostic = completion.diagnostic().unwrap_or_default().to_string();
        assert!(
            diagnostic.starts_with("aidlc-orchestrate: cannot write the execution cursor: "),
            "{diagnostic}"
        );
        assert!(
            diagnostic.contains("io: PermissionDenied at "),
            "{diagnostic}"
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

    /// 構文的な段は `ReportArgs` の値だけで決まる — 空白だけの `--stage` は「無い」と同じ。
    #[test]
    fn a_blank_explicit_stage_reads_as_absent() {
        assert_eq!(explicit_stage(&report_args(&[])), None);
        assert_eq!(explicit_stage(&report_args(&["--stage", "  "])), None);
        assert_eq!(
            explicit_stage(&report_args(&["--stage", " domain-design "])),
            Some("domain-design")
        );
    }

    /// gate 系 3 語の前提違反は語ごとに別の逐語になる（同じ材料でも文言が違う）。
    #[test]
    fn each_gate_verdict_has_its_own_precondition_wording() {
        assert_eq!(
            gate_precondition(Verdict::AwaitingApproval, "domain-design", "revising"),
            "Stage \"domain-design\" is revising; only an in-progress stage can open a gate."
        );
        assert_eq!(
            gate_precondition(Verdict::Rejected, "domain-design", "revising"),
            "Stage \"domain-design\" is revising; only an active or awaiting-approval stage can be rejected."
        );
        assert_eq!(
            gate_precondition(Verdict::Revised, "domain-design", "in-progress"),
            "Stage \"domain-design\" is in-progress; only a revising stage can re-enter its gate."
        );
    }

    use core_command_domain::workflow_definition::ExecutionKind;
    use core_command_domain::workspace::CheckboxState;

    /// 拒否 13 形が upstream の逐語へ 1:1 で写る（`raw` は報告された生の語）。
    ///
    /// 合成ルートは**材料を文言にするだけ**である — どれを返すかを決めたのは集約である。
    #[test]
    fn every_report_refusal_renders_its_upstream_wording() {
        let slug = |value: &str| StageSlug::parse(value).expect("フィクスチャの slug");
        let cases: [(ReportRefusal, &str); 13] = [
            (
                ReportRefusal::UnknownStage {
                    named: "nope".to_string(),
                },
                "Internal: reported stage \"nope\" is not in the compiled graph — cannot commit its transition.",
            ),
            (
                ReportRefusal::RoutedVerdict {
                    verdict: Verdict::Resume,
                },
                "Resume is routed, not committed. Run a fresh `next --resume`.",
            ),
            (
                ReportRefusal::SkipNotConditional {
                    stage: slug("domain-design"),
                    execution: ExecutionKind::Always,
                },
                "Stage \"domain-design\" is execution: ALWAYS; only a CONDITIONAL stage can report skipped.",
            ),
            (
                ReportRefusal::SkipRequiresReason {
                    stage: slug("domain-design"),
                },
                "report --result skipped requires a nonblank --reason <text>.",
            ),
            (
                ReportRefusal::SkipMustNameCursor {
                    named: slug("domain-design"),
                    current: slug("contract-design"),
                },
                "Cannot skip stage \"domain-design\": Current Stage is \"contract-design\". A skip report must name the active stage exactly.",
            ),
            (
                ReportRefusal::SkipPrecondition {
                    stage: slug("domain-design"),
                    actual: CheckboxState::Pending,
                },
                "Stage \"domain-design\" is pending; only an active, revising, or interrupted skipped stage can be routed as skipped.",
            ),
            (
                ReportRefusal::UngatedStage {
                    stage: slug("state-init"),
                    verdict: Verdict::Rejected,
                },
                "Stage \"state-init\" is an ungated initialization stage; it cannot report rejected.",
            ),
            (
                ReportRefusal::GatePrecondition {
                    stage: slug("domain-design"),
                    verdict: Verdict::AwaitingApproval,
                    actual: CheckboxState::Revising,
                },
                "Stage \"domain-design\" is revising; only an in-progress stage can open a gate.",
            ),
            (
                ReportRefusal::RejectRequiresFeedback {
                    stage: slug("domain-design"),
                },
                "report --result rejected for \"domain-design\" requires nonblank --user-input or --reason feedback.",
            ),
            (
                ReportRefusal::HumanPresence {
                    stage: slug("domain-design"),
                    verdict: Verdict::Forward,
                },
                "report --result rejected for \"domain-design\" requires --user-input with the human's exact approval choice.",
            ),
            (
                ReportRefusal::ForwardCommitsCompletionsOnly {
                    stage: slug("domain-design"),
                    actual: CheckboxState::Skipped,
                },
                "Stage \"domain-design\" is skipped; report commits forward completions only.",
            ),
            (
                ReportRefusal::StillPending {
                    stage: slug("contract-design"),
                },
                "Stage \"contract-design\" is still pending. Run the stage before reporting it complete.",
            ),
            (
                ReportRefusal::InProgressRequiresExplicitStage {
                    stage: slug("domain-design"),
                },
                "Stage \"domain-design\" is still in-progress. To approve a gated stage that has not entered awaiting-approval, report the acted directive explicitly with --stage \"domain-design\" so the engine cannot mistake a freshly advanced Current Stage for the completed one.",
            ),
        ];
        for (refusal, expected) in cases {
            // `raw` は報告された生の語である — upstream も `flags.result` を埋める（正規化した
            // `Verdict` からは `approved` と `completed` を区別して戻せない）。
            assert_eq!(
                report_refusal("rejected", &refusal),
                expected,
                "{refusal:?}"
            );
        }
    }

    /// 集約コマンドの拒否は upstream の中継形になり、失敗した段と対象を名乗る。
    #[test]
    fn a_refused_transition_is_relayed_with_the_step_that_failed() {
        let stage = StageSlug::parse("domain-design").expect("slug");
        assert_eq!(
            commit_refusal(
                "approved",
                &CommitError::Transition {
                    step: TransitionStep::Approve,
                    stage,
                    error: CommandError::NotRunning,
                }
            ),
            "Transition rejected by aidlc-state.ts approve for \"domain-design\": not running"
        );
        // 材料が空なら upstream は `.` で閉じる。
        assert_eq!(
            wording::transition_rejected_by("skip", "domain-design", ""),
            "Transition rejected by aidlc-state.ts skip for \"domain-design\"."
        );
    }

    /// この build に無い段は、読み替えずに名指しして断る（b42 で撤去した 2 段）。
    #[test]
    fn an_unwired_step_is_named_rather_than_re_read() {
        let stage = StageSlug::parse("state-init").expect("slug");
        assert_eq!(
            commit_refusal(
                "completed",
                &CommitError::UnwiredTransition {
                    step: TransitionStep::CompleteWorkflow,
                    stage: stage.clone(),
                }
            ),
            "Cannot commit complete-workflow for \"state-init\": the complete-workflow transition is not wired in this build."
        );
        assert_eq!(
            commit_refusal(
                "completed",
                &CommitError::UnwiredTransition {
                    step: TransitionStep::Advance,
                    stage,
                }
            ),
            "Cannot commit advance for \"state-init\": the advance transition is not wired in this build."
        );
    }

    /// 再構成・計画取得の失敗は upstream に対応する逐語が無いので中継形に材料を載せる。
    #[test]
    fn a_rehydration_failure_falls_back_to_the_relay_form() {
        let message = commit_refusal(
            "approved",
            &CommitError::Repository(
                core_command_use_case::orchestration::RepositoryError::NotFound {
                    id: IntentExecutionId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000")
                        .expect("UUIDv7"),
                },
            ),
        );
        assert!(
            message.starts_with("Transition rejected: repository: not found: "),
            "{message}"
        );
    }

    /// 成功 3 形 — gate 系は `print`、読み飛ばしと前進は `done`。
    #[test]
    fn every_commit_outcome_renders_its_directive() {
        let stage = StageSlug::parse("domain-design").expect("slug");
        let committed = |steps: TransitionSteps| CommitOutcome::Committed {
            stage: stage.clone(),
            scope: "classic".to_string(),
            steps,
        };
        assert!(matches!(
            committed_directive("awaiting-approval", &committed(TransitionSteps::single(TransitionStep::GateStart))),
            Directive::Print { message } if message == "Recorded awaiting-approval for \"domain-design\"."
        ));
        assert!(matches!(
            committed_directive("rejected", &committed(TransitionSteps::single(TransitionStep::Reject))),
            Directive::Print { message } if message == "Recorded rejected for \"domain-design\"."
        ));
        assert!(matches!(
            committed_directive("revised", &committed(TransitionSteps::single(TransitionStep::Revise))),
            Directive::Print { message } if message == "Recorded revised for \"domain-design\"."
        ));
        assert!(matches!(
            committed_directive("skipped", &committed(TransitionSteps::single(TransitionStep::Skip))),
            Directive::Done { reason: Some(reason) }
                if reason == "Committed skip for \"domain-design\" (scope: classic). State routed forward; run next to continue."
        ));
        assert!(matches!(
            committed_directive("approved", &committed(TransitionSteps::single(TransitionStep::Approve))),
            Directive::Done { reason: Some(reason) }
                if reason == "Committed approve for \"domain-design\" (scope: classic). State advanced; run next to continue."
        ));
        assert!(matches!(
            committed_directive(
                "approved",
                &committed(TransitionSteps::recovered_approval())
            ),
            Directive::Done { reason: Some(reason) }
                if reason == "Committed gate-start + approve for \"domain-design\" (scope: classic). State advanced; run next to continue."
        ));
    }

    /// no-op 3 形 — 既開ゲートは `print`、残り 2 つは `done`。
    #[test]
    fn every_no_op_renders_its_directive() {
        let stage = StageSlug::parse("domain-design").expect("slug");
        let current = StageSlug::parse("contract-design").expect("slug");
        let no_op = |no_op: ReportNoOp| CommitOutcome::NoOp {
            stage: stage.clone(),
            scope: "classic".to_string(),
            no_op,
        };
        assert!(matches!(
            committed_directive("awaiting-approval", &no_op(ReportNoOp::AlreadyAwaiting { stage: stage.clone() })),
            Directive::Print { message } if message == "Stage \"domain-design\" is already awaiting approval."
        ));
        assert!(matches!(
            committed_directive(
                "approved",
                &no_op(ReportNoOp::AlreadyCompletedMovedOn { stage: stage.clone(), current })
            ),
            Directive::Done { reason: Some(reason) }
                if reason == "Stage \"domain-design\" is already completed and the workflow has moved on to \"contract-design\" (scope: classic); idempotent re-report, no transition needed."
        ));
        let Directive::Done {
            reason: Some(reason),
        } = committed_directive(
            "approved",
            &no_op(ReportNoOp::WorkflowAlreadyCompleted {
                stage: stage.clone(),
            }),
        )
        else {
            panic!("done を期待した")
        };
        assert!(
            reason.starts_with(
                "Workflow is already completed at \"domain-design\" (scope: classic); no transition was needed."
            ),
            "{reason}"
        );
        assert!(
            reason.contains("If this input is genuinely NEW, unrelated work"),
            "{reason}"
        );
    }

    /// 段 13 の env — 立てていなければガードは効いたままである。
    #[test]
    fn the_human_presence_guard_is_on_unless_the_environment_switches_it_off() {
        assert!(human_presence_guard());
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

    /// どの手も失敗させないダブルは素通しである — 他の 3 つのテストが観測する失敗が
    /// ダブル自身ではなく**注入した 1 手**から来ていることの対照になる。
    #[tokio::test]
    async fn the_records_double_is_transparent_when_no_step_is_injected() {
        let root = minimal_workspace();
        let layout = Layout::resolve(root.path());

        let completion = create_intent_with(
            &layout,
            &intent_create_args(&["--scope", "classic", "--label", "demo"]),
            &FailingAt(Step::Nothing),
        )
        .await;

        assert_eq!(completion.code(), 0, "{completion:?}");
        assert_eq!(completion.diagnostic(), None);
        assert!(
            Layout::resolve(root.path()).record_dir().is_some(),
            "カーソルは実 I/O で据わる"
        );
    }

    /// `--project-dir` が無ければ、渡された作業ディレクトリがワークスペース根になる。
    #[tokio::test]
    async fn the_working_directory_is_the_workspace_root_when_no_project_dir_is_given() {
        let root = minimal_workspace();

        let completion = run(
            "aidlc-utility",
            &[
                "intent-create".to_string(),
                "--scope".to_string(),
                "classic".to_string(),
                "--label".to_string(),
                "demo".to_string(),
            ],
            root.path(),
        )
        .await;

        assert_eq!(completion.code(), 0, "{completion:?}");
        assert!(
            Layout::resolve(root.path()).record_dir().is_some(),
            "cwd の下に record が生まれる"
        );
    }

    /// 定義が名乗らない scope では鋳造できない（ユースケースの拒否をそのまま運ぶ）。
    #[tokio::test]
    async fn minting_an_intent_with_a_scope_the_definition_does_not_declare_is_refused() {
        let root = minimal_workspace();
        let layout = Layout::resolve(root.path());

        let completion = create_intent(
            &layout,
            &intent_create_args(&["--scope", "nope", "--label", "demo"]),
        )
        .await;

        assert_eq!(completion.code(), 1);
        assert_eq!(completion.line(), None, "stdout には何も出さない");
        assert!(
            completion
                .diagnostic()
                .unwrap_or_default()
                .starts_with("aidlc-orchestrate: "),
            "{completion:?}"
        );
    }

    /// イベントストアを開けなければ、報告は自己防衛拒否で止まる。
    #[tokio::test]
    async fn a_report_against_an_unopenable_event_store_is_refused() {
        let root = minimal_workspace();
        let layout = Layout::resolve(root.path());
        create_intent(
            &layout,
            &intent_create_args(&["--scope", "classic", "--label", "demo"]),
        )
        .await;
        // ストアの置き場をディレクトリで塞ぐ（開く時点で潰える）。
        let store = root
            .path()
            .join("aidlc/spaces/default/intents/.aidlc-store.sqlite");
        std::fs::remove_file(&store).expect("ストア");
        std::fs::create_dir(&store).expect("塞ぐ");

        let completion = report(
            &Layout::resolve(root.path()),
            &report_args(&["--result", "approved"]),
        )
        .await;

        assert_eq!(completion.code(), 1);
        assert!(
            completion
                .diagnostic()
                .is_some_and(|message| message.contains("journal: io:")),
            "{completion:?}"
        );
    }

    /// 公開先の異常が既にあるときは、書込を始める前に拒否する。
    #[tokio::test]
    async fn an_unwritable_projection_prevents_the_report_commit() {
        let root = minimal_workspace();
        let layout = Layout::resolve(root.path());
        create_intent(
            &layout,
            &intent_create_args(&["--scope", "classic", "--label", "demo"]),
        )
        .await;
        let layout = Layout::resolve(root.path());
        let database = rusqlite::Connection::open(store_path(&layout).expect("store").as_path())
            .expect("database");
        let event_count = || {
            database
                .query_row("SELECT COUNT(*) FROM journal", [], |row| {
                    row.get::<_, i64>(0)
                })
                .expect("events")
        };
        let before = event_count();
        // 公開先を塞ぎ、先行する復旧確認で新たなコマンド書込を拒否させる。
        let audit_dir = layout.audit_dir().expect("監査ディレクトリ");
        std::fs::remove_dir_all(&audit_dir).expect("監査ディレクトリ");
        std::fs::write(&audit_dir, "not a directory").expect("塞ぐ");

        let completion = report(
            &layout,
            // 報告自体は正当でも、先行する復旧を完了できなければ書き込まない。
            &report_args(&[
                "--result",
                "approved",
                "--user-input",
                "A",
                "--stage",
                "domain-design",
            ]),
        )
        .await;

        assert_eq!(completion.code(), 1, "{completion:?}");
        assert_eq!(
            event_count(),
            before,
            "公開先が壊れた状態で新しいイベントを書かない"
        );
        assert!(
            completion
                .diagnostic()
                .unwrap_or_default()
                .starts_with("aidlc-orchestrate: "),
            "{completion:?}"
        );
    }

    /// 定義の下準備でストアを開けなければ、`next` は逐語の拒否になる。
    #[tokio::test]
    async fn a_first_next_that_cannot_open_the_store_reports_the_failure() {
        let root = minimal_workspace();
        let store = root
            .path()
            .join("aidlc/spaces/default/intents/.aidlc-store.sqlite");
        std::fs::create_dir_all(&store).expect("塞ぐ");

        let completion = run("aidlc-orchestrate", &["next".to_string()], root.path()).await;

        assert_eq!(
            completion.code(),
            0,
            "ビジネス経路は exit 0: {completion:?}"
        );
        let line = completion.line().unwrap_or_default();
        assert!(line.contains("\"error\""), "{line}");
    }

    /// 空間名として成立しない active-space では、下準備の時点でストアの所在が決まらない。
    #[tokio::test]
    async fn a_first_next_under_an_invalid_active_space_is_refused_by_name() {
        let root = minimal_workspace();
        std::fs::write(root.path().join("aidlc/active-space"), "../escape\n").expect("space");

        let completion = run("aidlc-orchestrate", &["next".to_string()], root.path()).await;

        assert_eq!(completion.code(), 0, "{completion:?}");
        let line = completion.line().unwrap_or_default();
        assert!(line.contains("is not a valid space name"), "{line}");
    }

    /// `continue` の鍵が無ければ、原因を区別せず fail-closed の逐語になる。
    #[tokio::test]
    async fn a_continue_without_a_key_fails_closed() {
        let root = minimal_workspace();

        let completion = run(
            "aidlc-orchestrate",
            &["continue".to_string(), "not-a-token".to_string()],
            root.path(),
        )
        .await;

        assert_eq!(
            completion.code(),
            0,
            "ビジネス拒否は exit 0: {completion:?}"
        );
        assert!(
            completion
                .line()
                .unwrap_or_default()
                .contains("Invalid steering continuation token"),
            "{completion:?}"
        );
    }

    /// 受理されない `--result` は directive の error として stdout に出る（自己防衛拒否ではない）。
    #[tokio::test]
    async fn an_unknown_result_is_emitted_as_a_business_error() {
        let root = minimal_workspace();
        let layout = Layout::resolve(root.path());

        let completion = report(&layout, &report_args(&["--result", "ok"])).await;

        assert_eq!(completion.code(), 0, "{completion:?}");
        assert!(
            completion
                .line()
                .unwrap_or_default()
                .contains("Unknown --result"),
            "{completion:?}"
        );
    }

    /// 配布物を読めなければ定義を確立できず、鋳造はそこで止まる。
    #[tokio::test]
    async fn an_unreadable_compiled_definition_stops_the_mint() {
        let root = minimal_workspace();
        std::fs::remove_file(root.path().join(".claude/tools/data/stage-graph.json"))
            .expect("配布物");
        let layout = Layout::resolve(root.path());

        let completion = create_intent(
            &layout,
            &intent_create_args(&["--scope", "classic", "--label", "demo"]),
        )
        .await;

        assert_eq!(completion.code(), 1);
        assert!(
            completion
                .diagnostic()
                .unwrap_or_default()
                .contains("cannot read the compiled definition"),
            "{completion:?}"
        );
    }

    /// ジャーナルを開けなければ、古い読み面から通常指示を返さない。
    #[tokio::test]
    async fn a_blocked_store_stops_the_catch_up_before_reading() {
        let root = minimal_workspace();
        let layout = Layout::resolve(root.path());
        create_intent(
            &layout,
            &intent_create_args(&["--scope", "classic", "--label", "demo"]),
        )
        .await;
        let store = root
            .path()
            .join("aidlc/spaces/default/intents/.aidlc-store.sqlite");
        std::fs::remove_file(&store).expect("ストア");
        std::fs::create_dir(&store).expect("塞ぐ");

        let completion = run("aidlc-orchestrate", &["next".to_string()], root.path()).await;

        assert_eq!(completion.code(), 0, "{completion:?}");
        assert!(
            completion
                .line()
                .unwrap_or_default()
                .contains("journal: io:"),
            "{completion:?}"
        );
    }

    /// `--scope` の無い `intent-create` は、必須フラグを名指して拒む。
    #[tokio::test]
    async fn minting_without_a_scope_names_the_missing_flag() {
        let root = minimal_workspace();
        let layout = Layout::resolve(root.path());

        let completion = create_intent(&layout, &intent_create_args(&["--label", "demo"])).await;

        assert_eq!(completion.code(), 1);
        assert_eq!(
            completion.diagnostic(),
            Some("aidlc-orchestrate: intent-create requires --scope <name>.")
        );
    }

    /// 閉集合を外れた `--review` は、意味の無い上限を焼き込む前に拒む。
    #[tokio::test]
    async fn minting_with_an_unknown_review_class_is_refused_before_anything_is_written() {
        let root = minimal_workspace();
        let layout = Layout::resolve(root.path());

        let completion = create_intent(
            &layout,
            &intent_create_args(&[
                "--scope", "classic", "--label", "demo", "--review", "strict",
            ]),
        )
        .await;

        assert_eq!(completion.code(), 1);
        assert_eq!(
            completion.diagnostic(),
            Some("Unknown review class: \"strict\". Valid: adversarial, advisory, none.")
        );
        assert_eq!(Layout::resolve(root.path()).record_dir(), None);
    }

    /// 記録ディレクトリを作れなければ、何も作らずに拒む。
    #[tokio::test]
    async fn a_record_directory_that_cannot_be_created_stops_the_mint() {
        let root = minimal_workspace();
        let intents = root.path().join("aidlc/spaces/default/intents");
        std::fs::remove_dir_all(&intents).expect("既存の intents を退ける");
        std::fs::write(&intents, "not a directory\n").expect("同名のファイル");
        let layout = Layout::resolve(root.path());

        let completion = create_intent(
            &layout,
            &intent_create_args(&["--scope", "classic", "--label", "demo"]),
        )
        .await;

        assert_eq!(completion.code(), 1);
        assert!(
            completion
                .diagnostic()
                .unwrap_or_default()
                .starts_with("aidlc-orchestrate: cannot create the record directory:"),
            "{completion:?}"
        );
    }
}
