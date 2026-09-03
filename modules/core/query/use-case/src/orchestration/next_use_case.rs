//! `Next` — `next` 1 回の 21 分岐ラダー (フロー制御のみ — FR3.1 / FR3.3)。
//!
//! **読むだけの動詞なのでクエリ側にある** (`coding-rules/cqrs-boundaries.md` 規則 5 —
//! 「ただ集約や集約以外のデータを読むための責務はコマンド側では許容されない」)。Repository は
//! 1 本も持たず、集約の再構成もしない — 実行状態・定義・ルール束はいずれも**リードモデルを
//! 読む DAO ポート**から取得する (オーナー裁定 2026-08-31)。状態判断は実行状態ビュー
//! ([`ExecutionStateView::next_decision`]) が持ち、本ユースケースは観測 ([`NextTurnInput`]) と
//! 読み取った素材を畳んで directive ちょうど 1 つに写す。
//!
//! **読取はラダーの位置で遅延して行う。** 前置ガード (パース失敗・`--review` 併用・Kiro
//! ラッチ・読み取り専用ユーティリティ・名詞トークン・`--stage`/`--phase` 併用) はどの
//! リードモデルも読まずに答えが決まるので、そこへ到達したターンは I/O を 1 回も起こさない。
//! ルール束はさらに遅く、run-stage を届ける段 ([`NextUseCase::deliver`]) でだけ読む。
//!
//! ラダーの分岐順・逐語文言の正本は契約マップ
//! `docs/specs/research/orchestration-next-ladder.md` §1。コマンドの概念と綴りは
//! [`EngineCommand`] が持ち、差し替え点は [`EngineCommand::cli_spelling`] 1 点である
//! (逸脱台帳 #1)。scope 解決ラダー・キーワード推論は定義ビューの判断ポリシーである
//! ([`DefinitionView::resolve_scope`])。
//!
//! [`ExecutionStateView::next_decision`]: crate::orchestration::ExecutionStateView::next_decision

use super::ask_directive::AskDirective;
use super::ask_kind::AskKind;
use super::bindings::Bindings;
use super::directive::Directive;
use super::engine_command::EngineCommand;
use super::gate_field::GateField;
use super::load_steering_directive::LoadSteeringDirective;
use super::next_decision::NextDecision;
use super::next_request::NextRequest;
use super::next_turn_input::NextTurnInput;
use super::run_stage_directive::RunStageDirective;
use super::run_stage_directive::RunStageDirectiveBuilder;
use super::scope_resolution_error::ScopeResolutionError;
use super::stage_name::StageName;
use super::state_binding::StateBinding;
use super::steering_part::SteeringPart;
use super::workspace_layout::WorkspaceLayout;
use crate::orchestration::{
    DefinitionView, ExecutionStateView, PhaseView, PlanActionView, ScopeSlugView, StageIndex,
    StageModeView, StageSlugView, StageView,
};
use crate::orchestration::{
    ExecutionStateDao, ExecutionStateReadError, MemoryRulesDao, WorkflowDefinitionDao,
    WorkflowDefinitionReadError,
};

/// 逐語文言 — ラダーが放出する公開契約の文字列 (出典: 契約マップ §1。コマンド参照は写像形)。
///
/// リードモデル読取の失敗もここで文言になる。ポートが運ぶのは材料だけなので、**文言を持つのは
/// 出す側**である (`coding-rules/error-handling.md`)。定義 3 入力の逐語文言 (12 §4 / §6) は
/// 2026-08-31 のポート化でアダプタからここへ移した — 材料を受け取って描く主体が
/// ユースケースになったためである。
mod wording {

    use super::{ExecutionStateReadError, ScopeCost, WorkflowDefinitionReadError};
    use crate::orchestration::CheckboxState;

    /// `--review` の併用ガード (前置)。
    pub(super) const REVIEW_COMBINATION: &str = "Cannot combine --review with read-only, workspace, compose, single-stage, jump, or resume modes. Apply /aidlc --review <class> first, then run the other command.";

    /// 分岐 2。
    pub(super) const STAGE_AND_PHASE: &str =
        "Cannot use --stage and --phase together. Use one or the other.";

    /// 分岐 4c の併用ガード。
    pub(super) const COMPOSE_WITH_JUMP: &str = "Cannot combine compose with --stage/--phase. Compose re-shapes the plan; jump moves the cursor. Run them separately.";

    /// 分岐 7 の init ジャンプガード (`INIT_JUMP_ERROR`)。
    pub(super) const INIT_JUMP: &str = "Cannot jump to initialization stages. The Initialization phase runs automatically when you start a workflow (describe what to build, e.g. /aidlc \"build the auth service\").";

    /// 分岐 9b。
    pub(super) const NO_STATE: &str = "No workflow state found (no active intent). Start one by describing what to build (/aidlc \"build the auth service\") or by naming a scope (/aidlc --scope <scope>).";

    /// 定義 id が特定できない (読取対象を名指しできなかった)。
    const NO_DEFINITION_ID: &str = "No workflow definition id was provided.";

    /// 実行状態リードモデルの読取失敗 → 文言。
    ///
    /// upstream に対応する逐語文言は無い (旧 state バージョンガードに相当する位置であり、
    /// 本家は復号器の例外をそのまま出す)。したがって材料を素直に並べた診断文言とし、
    /// 「どのファイルが、なぜ」だけを述べる。
    pub(super) fn state_read_error(error: &ExecutionStateReadError) -> String {
        match error {
            ExecutionStateReadError::NotReadable { path, cause }
            | ExecutionStateReadError::Malformed { path, cause } => format!("{path}: {cause}"),
        }
    }

    /// 定義リードモデルの読取失敗 → 逐語文言 (12 §4 / §6)。
    ///
    /// 読取対象を名指しできなかった [`WorkflowDefinitionReadError::Unidentified`] だけが
    /// 別系統の文言 (`NO_DEFINITION_ID`) になる — 「読めなかった」ではなく「読む先が
    /// 決まらなかった」という別の観測だからである。
    pub(super) fn definition_read_error(error: &WorkflowDefinitionReadError) -> String {
        match error {
            WorkflowDefinitionReadError::Unidentified => NO_DEFINITION_ID.to_string(),
            WorkflowDefinitionReadError::NotReadable {
                path,
                cause,
                env_override,
            } => stage_graph_not_readable(path, cause, *env_override),
            WorkflowDefinitionReadError::InvalidJson { path, cause } => {
                stage_graph_invalid_json(path, cause)
            }
            WorkflowDefinitionReadError::ScopeFile { message }
            | WorkflowDefinitionReadError::Malformed { message } => message.clone(),
            WorkflowDefinitionReadError::HarnessIdentity { path, cause } => {
                harness_identity(path, cause)
            }
        }
    }

    /// `stage-graph.json` が読めないときの逐語文言 (12 §4 #1)。
    ///
    /// `env_override` が真のときだけ hint 節が「`AIDLC_STAGE_GRAPH` を unset して既定に戻せ」
    /// 形へ切り替わる — この分岐自体が観測可能な契約。
    ///
    /// ピン留めソース採取で逐語確認済み (`aidlc-lib.ts:8565-8570` @3c3146cf、`loadStageGraphAll`
    /// `:8558-8585` の内側 — `docs/specs/research/golden-3c3146cf-lib.md` §8.2)。hint 分岐の
    /// 条件は `process.env.AIDLC_STAGE_GRAPH` の **truthy 判定**なので、空文字列の env では
    /// 既定 hint に落ちる。
    fn stage_graph_not_readable(path: &str, cause: &str, env_override: bool) -> String {
        if env_override {
            format!(
                "Stage graph not readable at {path}: {cause}. AIDLC_STAGE_GRAPH points to {path}; unset it to use the default."
            )
        } else {
            format!(
                "Stage graph not readable at {path}: {cause}. Reinstall the framework or re-run setup to restore the data file."
            )
        }
    }

    /// `stage-graph.json` が不正 JSON のときの逐語文言 (12 §4 #2 — 読めない場合とは別文言)。
    ///
    /// ピン留めソース採取で逐語確認済み (`aidlc-lib.ts:8579-8581` @3c3146cf —
    /// `docs/specs/research/golden-3c3146cf-lib.md` §8.2)。
    fn stage_graph_invalid_json(path: &str, cause: &str) -> String {
        format!("Stage graph at {path} is not valid JSON: {cause}")
    }

    /// ハーネス identity (`harness.json`) を読めない・`name` が無い (ADR-008)。upstream に
    /// 対応する逐語文言は無い (診断文言)。
    fn harness_identity(path: &str, cause: &str) -> String {
        format!("Harness identity not readable at {path}: {cause}")
    }

    /// 分岐 2.5。
    pub(super) fn parked(stage: &str) -> String {
        format!("Workflow parked at \"{stage}\". Resume with /aidlc --resume.")
    }

    /// 分岐 2.6。
    pub(super) fn unpark_then_resume(spelled: &str) -> String {
        format!(
            "This workflow is parked. Run `{spelled}` to clear the park marker, then re-run `next --resume` to continue."
        )
    }

    /// 分岐 3b / 解決不能。
    pub(super) fn unknown_scope(scope: &str, valid: &[&str]) -> String {
        format!(
            "Unknown scope \"{scope}\". Valid scopes: {}.",
            valid.join(", ")
        )
    }

    /// 分岐 4。
    pub(super) fn invalid_env_scope(value: &str, valid: &[&str]) -> String {
        format!(
            "Invalid AWS_AIDLC_DEFAULT_SCOPE \"{value}\". Valid scopes: {}.",
            valid.join(", ")
        )
    }

    /// 分岐 4a — 記述が空 (upstream `aidlc-orchestrate.ts:2980-2982` 逐語)。
    pub(super) const NEW_INTENT_BLANK: &str =
        "`next --new-intent` requires a nonblank new-work description after the confirmed scope.";

    /// `--label` の畳み方を conductor へ伝える一文 (upstream `:889-890` 逐語)。
    ///
    /// 先頭の空白は upstream のまま — 直前の文へ連結される位置にある。
    const LABEL_HINT: &str = " Replace `--label` with a 2-3 word kebab essence of the description (e.g. \"simple calc\"), which becomes the readable folder name for this piece of work.";

    /// 誕生 print の本文 (upstream `createPrintDirective` `:900-910` 逐語)。
    ///
    /// `new_intent` が真なら「別 intent なのでこのセッションを畳め」の 4 文が続き、偽なら
    /// 「そのまま `next` を再実行せよ」の継続形になる。分岐するのは尾部だけで、コマンドの
    /// 名指し・コスト節・ラベル助言は共通である。
    pub(super) fn birth_print(
        spelled: &str,
        cost: &str,
        has_description: bool,
        new_intent: bool,
    ) -> String {
        let label_hint = if has_description { LABEL_HINT } else { "" };
        if new_intent {
            format!(
                "Run `{spelled}` to start the new intent{cost}.{label_hint} Then STOP, do NOT re-run `next` in this session. \
This is a NEW, unrelated intent, and the current session still carries the previous intent's context. \
Tell the user to start a fresh session using this harness's reset or restart flow, then invoke its AI-DLC entry skill to begin the new intent with a clean slate. \
Nothing is lost: the intent is saved on disk and resumes on the next `next`."
            )
        } else {
            format!(
                "Run `{spelled}` to start the workflow{cost}, then re-run `next` to continue.{label_hint}"
            )
        }
    }

    /// コスト節 (upstream `costClause` `:669-676` 逐語)。括弧は呼出側が付ける。
    pub(super) fn cost_clause(cost: &ScopeCost) -> String {
        let per_unit = match cost.per_unit_stages() {
            0 => String::new(),
            1 => ", 1 stage repeats per unit of work in Construction".to_string(),
            count => format!(", {count} stages repeat per unit of work in Construction"),
        };
        format!(
            "{} of {} stages, {} approval gates{per_unit}",
            cost.execute(),
            cost.total(),
            cost.gates()
        )
    }

    /// 未知ステージ (upstream `:4441` / `:4605` / `:5281` 逐語 — 一覧への案内まで含む)。
    pub(super) fn unknown_stage(stage: &str) -> String {
        format!("Unknown stage \"{stage}\". Run /aidlc --help for the full list.")
    }

    /// 未知フェーズ (upstream `:4578` 逐語)。
    ///
    /// フェーズ語彙の並びは upstream `PHASES` の宣言順である (`aidlc-lib.ts:130-136`、
    /// 辞書順ではない)。列挙を所有すべきなのは [`PhaseView`] だが、その定義ファイルは
    /// b29 の書込スコープ外なので文言側に置いた — 移設は後続 Bolt。
    ///
    /// [`PhaseView`]: crate::orchestration::PhaseView
    pub(super) fn unknown_phase(phase: &str) -> String {
        format!(
            "Unknown phase \"{phase}\". Valid phases: initialization, ideation, inception, construction, operation."
        )
    }

    /// 分岐 5 — scope 変更の名指し (upstream `:3056` 逐語)。
    pub(super) fn scope_change(spelled: &str) -> String {
        format!("Run `{spelled}` to change scope, then print its output verbatim and stop.")
    }

    /// 分岐 5 — 設定変更の名指し (upstream `:3072` 逐語)。
    pub(super) fn config_change(spelled: &str) -> String {
        format!(
            "Run `{spelled}` to update the configuration, then print its output verbatim and stop."
        )
    }

    /// 分岐 8 — キーワードが当たった scope の確認 (upstream `:3150-3152` 逐語)。
    ///
    /// コスト節の区切りは誕生 print の括弧ではなく ` - ` である (upstream `:3148`)。
    pub(super) fn scope_confirm(scope: &str, intent: &str, cost: &str) -> String {
        format!(
            "This looks like \"{scope}\" work, so I'd run the \"{scope}\" plan for: \"{intent}\"{cost}. \
Say go ahead, name a different plan, or say \"compose\" and I'll tailor one to this task."
        )
    }

    /// 分岐 8 — どの既製 scope も当たらないときの compose 提案 (upstream `:3168-3171` 逐語)。
    pub(super) fn compose_offer(intent: &str, examples: &str) -> String {
        format!(
            "None of the ready-made plans is an obvious fit for: \"{intent}\". \
I can work out a plan tailored to this task (recommended: reply \"compose\"), \
or you can pick one directly (e.g. {examples}; see /aidlc --help for the full list)."
        )
    }

    /// 分岐 1 (upstream `:2717` 逐語)。
    pub(super) fn read_only(spelled: &str) -> String {
        format!(
            "Run `{spelled}`, print its output verbatim, then stop. \
This is a read-only utility, NOT workflow work: do NOT run `next` and do NOT advance, resume, or run any workflow stage."
        )
    }

    /// 分岐 1b/1c/1d — 名詞トークンの終端コマンド (upstream `:2764` 逐語)。
    ///
    /// 読み取り専用ユーティリティとは 1 語だけ違う (`read-only` ではなく `terminal`)。
    pub(super) fn terminal_utility(spelled: &str) -> String {
        format!(
            "Run `{spelled}`, print its output verbatim, then stop. \
This is a terminal utility, NOT workflow work: do NOT run `next` and do NOT advance, resume, or run any workflow stage."
        )
    }

    /// 分岐 6。
    pub(super) fn resume_menu(stage: &str) -> String {
        format!(
            "An existing workflow was found (currently at \"{stage}\"). How would you like to proceed? Resume from last checkpoint, redo the current stage, jump to a stage, or start fresh."
        )
    }

    /// 分岐 10 手順 3 (回復可能な plan/cursor 不整合。upstream `:3294-3297` 逐語)。
    pub(super) fn recover_skip(stage: &str, spelled: &str) -> String {
        format!(
            "Stage \"{stage}\" is SKIP in the approved workflow plan but is still the active cursor. \
Do not run this stage. Run `{spelled}` to recover the stale pointer, then re-run `next` to continue."
        )
    }

    /// 分岐 10 手順 3 (回復経路のない plan/cursor 不整合)。
    pub(super) fn inconsistent_skip(stage: &str, checkbox: CheckboxState) -> String {
        format!(
            "Stage \"{stage}\" is SKIP in the approved workflow plan but its active cursor state is \"{}\". Refusing to emit run-stage; repair the inconsistent state before continuing.",
            checkbox_word(checkbox)
        )
    }

    /// 完了した intent の `done` reason に必ず続く新規作業のヒント (upstream `:855-859` 逐語)。
    ///
    /// 先頭の空白は upstream のまま — reason 本文へ連結される位置にある。scope-runner の
    /// 転送ループがここで行き止まりにならないための出口案内であり、落とすと「新しい仕事は
    /// どう始めるのか」が読み手に届かない。
    const NEW_WORK_HINT: &str = " If this input is genuinely NEW, unrelated work (not a follow-up to the completed intent), don't stop here: offer to start a second intent, and on the human's yes run `next --new-intent --scope <scope> \"<text>\"` (see the SKILL's new-work offer, never auto-birth).";

    /// 分岐 10 手順 5 (upstream `:3332-3348` — reason + `NEW_WORK_HINT`)。
    pub(super) fn workflow_complete(stage: &str, scope: &str) -> String {
        format!(
            "Workflow complete — no in-scope stage remains after {stage} (scope: {scope}).{NEW_WORK_HINT}"
        )
    }

    /// steering 読取失敗 (blocking — run-stage の代わりに error)。
    pub(super) fn rule_unreadable(path: &str, cause: &str) -> String {
        format!(
            "Cannot load required stage rule \"{path}\" ({cause}). The stage has not started. Restore the file or fix its permissions/UTF-8 encoding, then run `next` again."
        )
    }

    /// 分割不能セクション (blocking)。
    pub(super) const UNSPLITTABLE_SECTION: &str = "A rule section could not be split below the directive transport limit. Shorten the affected heading section, then run a fresh `next`.";

    /// checkbox 状態の upstream 語。
    pub(super) const fn checkbox_word(state: CheckboxState) -> &'static str {
        // amadeus-lint: allow(checkbox-vocabulary) — 判断ではなくワイヤ逐語 (upstream の状態語) への綴り写像
        match state {
            CheckboxState::Pending => "pending",
            CheckboxState::InProgress => "in-progress",
            CheckboxState::AwaitingApproval => "awaiting-approval",
            CheckboxState::Revising => "revising",
            CheckboxState::Completed => "completed",
            CheckboxState::Skipped => "skipped",
        }
    }
}

/// `next` の 21 分岐ラダー (読取専用 — リードモデルを読む DAO ポートだけを持つ)。
///
/// 保持するのは 3 つの読取専用 DAO である。いずれも動詞は `find` 1 本で、更新動詞が
/// 存在しないことが「リードモデルは更新できない」の型保証になる
/// (`coding-rules/cqrs-boundaries.md` 規則 6 / オーナー裁定 2026-08-31)。バインディングは
/// スタティックが既定なので型パラメータで受ける (`coding-rules/use-case-rules.md` §2)。
#[derive(Debug)]
pub struct NextUseCase<D: WorkflowDefinitionDao, S: ExecutionStateDao, M: MemoryRulesDao> {
    workflow_definition_dao: D,
    execution_state_dao: S,
    memory_rules_dao: M,
}

impl<D: WorkflowDefinitionDao, S: ExecutionStateDao, M: MemoryRulesDao> NextUseCase<D, S, M> {
    /// 3 つの読取専用 DAO を束ねる。
    #[must_use]
    pub const fn new(
        workflow_definition_dao: D,
        execution_state_dao: S,
        memory_rules_dao: M,
    ) -> NextUseCase<D, S, M> {
        NextUseCase {
            workflow_definition_dao,
            execution_state_dao,
            memory_rules_dao,
        }
    }

    /// 観測 1 回を directive ちょうど 1 つに写す。失敗も `Directive::Error` で返す —
    /// エンジンの契約は「stdout に directive ちょうど 1 つ」である (§3.2)。
    ///
    /// 読み取るだけなので `&self` のクエリである (CQS)。リードモデルはラダーの位置で遅延して
    /// 読む — 前置ガードで答えが決まるターンは I/O を 1 回も起こさない。
    #[must_use]
    pub fn execute(&self, input: &NextTurnInput) -> Directive {
        // ---- 前置ガード ----
        if let Some(message) = input.parse_error() {
            return Directive::Error {
                message: message.to_string(),
            };
        }
        if input.review().is_some()
            && (input.read_only().is_some()
                || input.noun_token().is_some()
                || input.is_compose()
                || input.is_single()
                || input.stage().is_some()
                || input.phase().is_some()
                || input.is_resume())
        {
            return Directive::Error {
                message: wording::REVIEW_COMBINATION.to_string(),
            };
        }
        // ---- 分岐 0: Kiro roll-forward ラッチ (advisory, fail-open) ----
        if input.is_kiro_latch_bare_next() {
            return Directive::Done { reason: None };
        }
        // ---- 分岐 1: 読み取り専用ユーティリティ ----
        if let Some(verb) = input.read_only() {
            return Directive::Print {
                message: wording::read_only(&EngineCommand::ReadOnlyUtility(verb).cli_spelling()),
            };
        }
        // ---- 分岐 1b/1c/1d: 名詞トークン (先頭トークン意味論のみ) ----
        if let Some(token) = input.noun_token() {
            return Directive::Print {
                message: wording::terminal_utility(
                    &EngineCommand::NounTokens(token.tokens().to_vec()).cli_spelling(),
                ),
            };
        }
        // ---- 分岐 2: --stage と --phase の併用 ----
        if input.stage().is_some() && input.phase().is_some() {
            return Directive::Error {
                message: wording::STAGE_AND_PHASE.to_string(),
            };
        }
        // ---- state の読取 (読取失敗はカーソルを使う前に逐語で止める) ----
        // 不在は失敗ではない (誕生分岐の群へ) — 「無い」と「読めない」で行き先が違う。
        let held = match self.execution_state_dao.find() {
            Ok(held) => held,
            Err(error) => {
                return Directive::Error {
                    message: wording::state_read_error(&error),
                };
            }
        };
        let state = held.as_ref();
        // ---- 定義の読取 ----
        let definition = match self.workflow_definition_dao.find() {
            Ok(view) => view,
            Err(error) => {
                return Directive::Error {
                    message: wording::definition_read_error(&error),
                };
            }
        };
        let definition = &definition;
        // ---- 分岐 2.5 / 2.6: park (判断はビュー — reentry フラグは NextRequest に畳む) ----
        let request = NextRequest::new(
            input.is_resume(),
            input.stage().is_some()
                || input.phase().is_some()
                || input.review().is_some()
                || input.new_intent().is_some(),
            input.freeform().is_some(),
        );
        let decision = state.map(|view| view.next_decision(&request));
        if let (Some(NextDecision::Parked { stage }), Some(view)) = (decision, state) {
            let slug = stage_slug(view, stage);
            return Directive::Parked {
                stage: slug.clone(),
                message: wording::parked(slug.as_str()),
            };
        }
        if decision == Some(NextDecision::UnparkThenResume) {
            return Directive::Print {
                message: wording::unpark_then_resume(&EngineCommand::Unpark.cli_spelling()),
            };
        }
        // ---- 分岐 3b / 4 / 解決不能: scope 解決ラダー ----
        let state_scope = state.map(|view| view.scope().as_str().to_string());
        let resolved = match definition.resolve_scope(
            state_scope.as_deref(),
            input.scope(),
            input.freeform(),
            input.env_default_scope(),
        ) {
            Ok(resolved) => resolved,
            Err(error) => {
                let valid = definition.valid_scopes();
                let message = match error {
                    ScopeResolutionError::UnknownExplicit { scope }
                    | ScopeResolutionError::Unresolvable { scope } => {
                        wording::unknown_scope(&scope, &valid)
                    }
                    ScopeResolutionError::UnknownEnv { value } => {
                        wording::invalid_env_scope(&value, &valid)
                    }
                };
                return Directive::Error { message };
            }
        };
        // ---- 分岐 4c: compose ----
        if input.is_compose() {
            if input.stage().is_some() || input.phase().is_some() {
                return Directive::Error {
                    message: wording::COMPOSE_WITH_JUMP.to_string(),
                };
            }
            return Directive::Print {
                message: format!(
                    "Dispatch the composer: run `{}`.",
                    EngineCommand::DispatchComposer.cli_spelling()
                ),
            };
        }
        // ---- 分岐 4a: --new-intent ----
        //
        // scope は **明示 `--scope` が勝ち、無ければ解決済み scope へ落ちる**
        // (upstream `flags.scope ?? scope` — `:2991`)。明示があればそれを使うのは、確認された
        // のが「新しい仕事の scope」であって進行中 intent の state scope ではないためである。
        if let Some(description) = input.new_intent() {
            let description = description.trim();
            if description.is_empty() {
                return Directive::Error {
                    message: wording::NEW_INTENT_BLANK.to_string(),
                };
            }
            let scope = input.scope().unwrap_or_else(|| resolved.name().as_str());
            return mint_intent_print(definition, scope, Some(description), input, true);
        }
        // ---- 分岐 4b: --single (scope-change / jump より前) ----
        if input.is_single() {
            return self.emit_single(input, definition, resolved.name());
        }
        // ---- 分岐 5: state あり + 有効で異なる設定 ----
        //
        // 設定修飾 (`--depth` / `--test-strategy` / `--review`) は **1 本の命令へまとめて**
        // 載せる (upstream `:3051-3054` / `:3067-3070`)。フィールドごとに命令を分けると、
        // 人間が 2 つ指定したときに片方が黙って落ちる。
        if let Some(view) = state {
            let depth = input.depth().map(str::to_string);
            let test_strategy = input.test_strategy().map(str::to_string);
            let review = input.review().map(str::to_string);
            if let Some(scope) = input.scope()
                && scope != view.scope().as_str()
            {
                let Ok(scope) = ScopeSlugView::parse(scope) else {
                    // ラダーが既に membership 検証済み — 文法違反はここへ届かない (防御的)。
                    return Directive::Error {
                        message: wording::unknown_scope(scope, &definition.valid_scopes()),
                    };
                };
                return Directive::Print {
                    message: wording::scope_change(
                        &EngineCommand::ChangeScope {
                            scope,
                            depth,
                            test_strategy,
                            review,
                        }
                        .cli_spelling(),
                    ),
                };
            }
            if depth.is_some() || test_strategy.is_some() || review.is_some() {
                return Directive::Print {
                    message: wording::config_change(
                        &EngineCommand::ChangeConfig {
                            depth,
                            test_strategy,
                            review,
                        }
                        .cli_spelling(),
                    ),
                };
            }
        }
        // ---- 分岐 6: state ありでの --resume ----
        if decision == Some(NextDecision::ResumeMenu) {
            let stage = state.map_or_else(String::new, |view| {
                stage_slug(view, view.cursor()).as_str().to_string()
            });
            return Directive::Ask(AskDirective::new(
                AskKind::ResumeMenu,
                wording::resume_menu(&stage),
            ));
        }
        // ---- 分岐 7: --stage / --phase (jump) ----
        if input.stage().is_some() || input.phase().is_some() {
            return self.emit_jump(input, state, definition, resolved.name());
        }
        // ---- state なしの群: 7b / 8 / 9a / 9b ----
        let (Some(view), Some(decision)) = (state, decision) else {
            return emit_birth_group(input, definition);
        };
        // ---- 分岐 9c: 稼働中の自由記述 ----
        if decision == NextDecision::NewWorkRouting {
            let description = input.freeform().unwrap_or_default().to_string();
            let proposed = definition
                .resolve_scope(None, None, Some(&description), None)
                .map_or_else(
                    |_| resolved.name().clone(),
                    |inferred| inferred.name().clone(),
                );
            return Directive::Ask(
                AskDirective::new(
                    AskKind::NewWorkRouting,
                    "Does this continue the active work, start separate new work, or re-shape the plan?".to_string(),
                )
                .with_new_work(proposed.as_str(), description),
            );
        }
        // ---- 分岐 10: ハッピーパス (判断はビューの next_decision) ----
        self.emit_happy_path(input, view, definition, resolved.name(), decision)
    }

    /// run-stage を steering 連鎖経由で届ける — ルール束が空なら bare run-stage、あれば
    /// 第 1 部の `load-steering` + continue_token (02 §10)。
    ///
    /// ルール束を読むのはここだけである。run-stage を届ける段に来て初めて要るものなので、
    /// ラダーが手前で止まったターンでは memory 層に触れない。
    fn deliver(
        &self,
        definition: &DefinitionView,
        scope: &ScopeSlugView,
        node: &StageView,
        run_stage: &RunStageDirective,
        state: Option<StateBinding>,
    ) -> Directive {
        let rules = match self.memory_rules_dao.find() {
            Ok(rules) => rules,
            Err(error) => {
                return Directive::Error {
                    message: wording::rule_unreadable(error.path(), error.cause()),
                };
            }
        };
        // 分割不能は読取時ではなくパック時に判明する。
        let Ok(plan) = rules.plan_for(node.phase()) else {
            return Directive::Error {
                message: wording::UNSPLITTABLE_SECTION.to_string(),
            };
        };
        let Some(first) = plan.first_part() else {
            // 空計画 — bare run-stage (台帳は空)。
            return Directive::RunStage(run_stage.with_rules_in_context(plan.delivered_paths()));
        };
        let bindings = Bindings::new(
            plan.bundle_digest(),
            run_stage.directive_digest(),
            definition.stage_route(scope.as_str(), node).route_digest(),
            state,
        );
        emit_part(&first, run_stage, scope, &bindings)
    }

    /// 分岐 4b — 単一ステージ隔離モード。
    fn emit_single(
        &self,
        input: &NextTurnInput,
        definition: &DefinitionView,
        scope: &ScopeSlugView,
    ) -> Directive {
        let Some(stage) = input.stage() else {
            return Directive::Error {
                message: "--single requires --stage <slug>.".to_string(),
            };
        };
        match find_node(definition, stage) {
            Some(node) => {
                let gate = default_gate(node);
                match build_run_stage(node, definition, scope.as_str(), input.layout(), gate, true)
                {
                    Ok(Directive::RunStage(run_stage)) => {
                        self.deliver(definition, scope, node, &run_stage, None)
                    }
                    Ok(directive) => directive,
                    Err(message) => Directive::Error { message },
                }
            }
            None => Directive::Error {
                message: wording::unknown_stage(stage),
            },
        }
    }

    /// 分岐 7 — jump。state ありなら純読み取り解決の名指し、無しなら直接グラフ検索。
    fn emit_jump(
        &self,
        input: &NextTurnInput,
        state: Option<&ExecutionStateView>,
        definition: &DefinitionView,
        scope: &ScopeSlugView,
    ) -> Directive {
        let target = match (input.stage(), input.phase()) {
            (Some(stage), _) => match find_node(definition, stage) {
                Some(node) => node,
                None => {
                    return Directive::Error {
                        message: wording::unknown_stage(stage),
                    };
                }
            },
            (None, Some(phase)) => {
                let Ok(phase) = PhaseView::parse(&phase.to_lowercase()) else {
                    return Directive::Error {
                        message: wording::unknown_phase(phase),
                    };
                };
                match definition.first_in_scope_stage_of_phase(phase, scope.as_str()) {
                    Some(node) => node,
                    None => {
                        return Directive::Error {
                            message: format!(
                                "No in-scope stage found for phase \"{}\".",
                                phase.as_str()
                            ),
                        };
                    }
                }
            }
            // 分岐 7 は --stage / --phase のいずれかが前提 (防御的)。
            (None, None) => {
                return Directive::Error {
                    message: wording::STAGE_AND_PHASE.to_string(),
                };
            }
        };
        // init ジャンプガード (`INIT_JUMP_ERROR`)。
        if target.phase() == PhaseView::Initialization {
            return Directive::Error {
                message: wording::INIT_JUMP.to_string(),
            };
        }
        if state.is_some() {
            return Directive::Print {
                message: format!(
                    "Run `{}`.",
                    EngineCommand::ResolveJump {
                        stage: target.slug().clone(),
                    }
                    .cli_spelling()
                ),
            };
        }
        let gate = default_gate(target);
        match build_run_stage(
            target,
            definition,
            scope.as_str(),
            input.layout(),
            gate,
            false,
        ) {
            Ok(Directive::RunStage(run_stage)) => {
                self.deliver(definition, scope, target, &run_stage, None)
            }
            Ok(directive) => directive,
            Err(message) => Directive::Error { message },
        }
    }

    /// 分岐 10 — ハッピーパス。判断 ([`NextDecision`]) を directive に写すだけ。
    fn emit_happy_path(
        &self,
        input: &NextTurnInput,
        state: &ExecutionStateView,
        definition: &DefinitionView,
        scope: &ScopeSlugView,
        decision: NextDecision,
    ) -> Directive {
        match decision {
            NextDecision::RunStage { stage, gate } => {
                let slug = stage_slug(state, stage).clone();
                // ゲート判定はビュー (BR1.3) が正 — 定義側の既定は使わない。
                let gate = if gate {
                    GateField::Gated
                } else {
                    GateField::Ungated
                };
                match find_node(definition, slug.as_str()) {
                    Some(node) => {
                        match build_run_stage(
                            node,
                            definition,
                            scope.as_str(),
                            input.layout(),
                            gate,
                            false,
                        ) {
                            Ok(Directive::RunStage(run_stage)) => self.deliver(
                                definition,
                                scope,
                                node,
                                &run_stage,
                                Some(state.state_binding()),
                            ),
                            Ok(directive) => directive,
                            Err(message) => Directive::Error { message },
                        }
                    }
                    None => Directive::Error {
                        message: wording::unknown_stage(slug.as_str()),
                    },
                }
            }
            NextDecision::Done => done_with_reason(state, scope.as_str()),
            NextDecision::RecoverSkipInconsistency { stage, .. } => {
                let slug = stage_slug(state, stage).clone();
                let spelled = EngineCommand::ReportSkipped {
                    stage: slug.clone(),
                }
                .cli_spelling();
                Directive::Print {
                    message: wording::recover_skip(slug.as_str(), &spelled),
                }
            }
            NextDecision::InconsistentSkip { stage, checkbox } => {
                let slug = stage_slug(state, stage);
                Directive::Error {
                    message: wording::inconsistent_skip(slug.as_str(), checkbox),
                }
            }
            // 先行分岐で消費済みの決定 — ここへ来たらラダーのプログラミング誤り (防御的)。
            NextDecision::Parked { .. }
            | NextDecision::UnparkThenResume
            | NextDecision::ResumeMenu
            | NextDecision::NewWorkRouting => Directive::Error {
                message: "internal: a routing decision reached the happy path".to_string(),
            },
        }
    }
}

/// state なしの群 (7b / 8 / 9a / 9b)。
fn emit_birth_group(input: &NextTurnInput, definition: &DefinitionView) -> Directive {
    // 分岐 9a: 明示 --scope (membership はラダーが検証済み)。自由記述が併記されていれば
    // 生まれる intent の説明として運ぶ (upstream `:3196` — `flags.intent`)。
    if let Some(scope) = input.scope() {
        return mint_intent_print(definition, scope, input.freeform(), input, false);
    }
    if let Some(text) = input.freeform() {
        // 分岐 7b: 位置引数が scope 名そのもの。
        if definition.is_valid_scope(text.trim()) {
            if input.records_exist_without_cursor() {
                return Directive::Ask(AskDirective::new(
                    AskKind::IntentPick,
                    "Existing intent records were found without an active cursor. Which intent should become active?".to_string(),
                ));
            }
            // 位置引数が scope 名そのものなので、記述として残る語は無い (upstream は
            // scope 語を剥がした残りを `flags.intent` に持つが、その剥がしは未実装 —
            // b29 の対象外)。
            return mint_intent_print(definition, text.trim(), None, input, false);
        }
        // 分岐 8: キーワードヒット → scope 確認 / 非ヒット → compose 提案。
        if let Some(scope) = definition.infer_scope_from_text(text) {
            let cost = scope_cost(definition, scope.as_str()).map_or_else(String::new, |cost| {
                format!(" - {}", wording::cost_clause(&cost))
            });
            return Directive::Ask(AskDirective::new(
                AskKind::ScopeConfirm,
                wording::scope_confirm(scope.as_str(), text, &cost),
            ));
        }
        return Directive::Ask(AskDirective::new(
            AskKind::ComposeOffer,
            wording::compose_offer(text, &compose_examples(definition)),
        ));
    }
    // 分岐 9b: 何も名指しされていない。
    Directive::Error {
        message: wording::NO_STATE.to_string(),
    }
}

/// intent 鋳造の名指し (分岐 4a / 7b / 9a) — upstream `createPrintDirective` (`:878-918`)。
///
/// scope は membership 検証済みの綴りを型に上げる。自由記述・任意フラグ・コスト節は
/// upstream と同じ材料から組み、尾部だけ `new_intent` で分岐する。
fn mint_intent_print(
    definition: &DefinitionView,
    scope: &str,
    description: Option<&str>,
    input: &NextTurnInput,
    new_intent: bool,
) -> Directive {
    let Ok(parsed) = ScopeSlugView::parse(scope) else {
        // membership 検証済みなので文法違反はここへ届かない (防御的)。
        return Directive::Error {
            message: wording::unknown_scope(scope, &definition.valid_scopes()),
        };
    };
    let description = description.filter(|text| !text.is_empty());
    let command = EngineCommand::MintIntent {
        scope: parsed,
        description: description.map(str::to_string),
        depth: input.depth().map(str::to_string),
        test_strategy: input.test_strategy().map(str::to_string),
        review: input.review().map(str::to_string),
    };
    let cost = match scope_cost(definition, scope) {
        Some(cost) => format!(" ({})", wording::cost_clause(&cost)),
        // グリッド列が無い scope (フィクスチャ木) — upstream も括弧ごと落とす。
        None => String::new(),
    };
    Directive::Print {
        message: wording::birth_print(
            &command.cli_spelling(),
            &cost,
            description.is_some(),
            new_intent,
        ),
    }
}

/// compose 提案が並べる目安 (upstream `:3158-3166`)。
///
/// 3 つの既製 scope がすべて解決できたときだけ実数を出し、1 つでも欠ければ有効 scope の
/// 先頭 3 件へ落ちる (upstream の `fallbackExamples`)。並び順は定義ビューの辞書順である。
fn compose_examples(definition: &DefinitionView) -> String {
    if let (Some(express), Some(classic), Some(feature)) = (
        scope_cost(definition, "express"),
        scope_cost(definition, "classic"),
        scope_cost(definition, "feature"),
    ) {
        return format!(
            "express = {} of {} stages, classic = {}, feature = all {}",
            express.execute(),
            express.total(),
            classic.execute(),
            feature.execute()
        );
    }
    let names: Vec<&str> = definition.valid_scopes().into_iter().take(3).collect();
    if names.is_empty() {
        "an explicit scope".to_string()
    } else {
        names.join(", ")
    }
}

/// スコープ 1 列のコスト材料 (upstream `ScopeCostSummary` `aidlc-lib.ts:9820-9827`)。
///
/// **本来の置き場は [`DefinitionView`]** — グリッド列とグラフを所有する型が導出も持つべきで
/// ある (`coding-rules/domain-services.md`)。b29 の書込スコープが `next_use_case.rs` /
/// `engine_command.rs` に限定されているため、ここに私有で置いて据え置いた。移設は後続 Bolt。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ScopeCost {
    total: usize,
    execute: usize,
    gates: usize,
    per_unit_stages: usize,
}

impl ScopeCost {
    /// グリッド列に載るステージ数 (EXECUTE + SKIP)。
    const fn total(&self) -> usize {
        self.total
    }

    /// EXECUTE 数。
    const fn execute(&self) -> usize {
        self.execute
    }

    /// 承認ゲート数 = initialization 以外の EXECUTE ステージ数。
    const fn gates(&self) -> usize {
        self.gates
    }

    /// Unit of Work ごとに反復する EXECUTE ステージ数。
    const fn per_unit_stages(&self) -> usize {
        self.per_unit_stages
    }
}

/// 反復軸の観測値 (upstream `PER_UNIT_FOR_EACH`)。
const PER_UNIT_FOR_EACH: &str = "unit-of-work";

/// `for_each` の誤記に備えた既知の per-unit ステージ (upstream `KNOWN_PER_UNIT_STAGES`)。
const KNOWN_PER_UNIT_STAGES: [&str; 5] = [
    "nfr-requirements",
    "nfr-design",
    "functional-design",
    "infrastructure-design",
    "code-generation",
];

/// スコープ 1 列のコストを数える (upstream `gridCostSummary` `aidlc-lib.ts:9836-9854`)。
///
/// グリッドに在ってグラフに無い slug は total / execute には数え、gates / per-unit には
/// 数えない (upstream と同じ防御)。
fn scope_cost(definition: &DefinitionView, scope: &str) -> Option<ScopeCost> {
    let column = definition.grid().column(scope)?;
    let mut cost = ScopeCost {
        total: column.len(),
        execute: 0,
        gates: 0,
        per_unit_stages: 0,
    };
    for (slug, action) in column {
        if *action != PlanActionView::Execute {
            continue;
        }
        cost.execute += 1;
        let Some(node) = find_node(definition, slug.as_str()) else {
            continue;
        };
        if node.phase() != PhaseView::Initialization {
            cost.gates += 1;
        }
        if node.for_each() == Some(PER_UNIT_FOR_EACH)
            || KNOWN_PER_UNIT_STAGES.contains(&slug.as_str())
        {
            cost.per_unit_stages += 1;
        }
    }
    Some(cost)
}

/// 完了 reason つきの `done` (分岐 10 手順 5) — ビューの Done は最終ステージ通過後に出るため、
/// reason は呼出側 (本関数) が現在ステージと scope から組む。
fn done_with_reason(state: &ExecutionStateView, scope: &str) -> Directive {
    let slug = stage_slug(state, state.cursor());
    Directive::Done {
        reason: Some(wording::workflow_complete(slug.as_str(), scope)),
    }
}

/// 索引 → slug。索引はビューの不変条件で範囲内だが、添字 panic は使わない — 範囲外は
/// 先頭へ畳む (防御的。ここへ来る索引は `next_decision` が発行する)。
#[allow(
    clippy::indexing_slicing,
    reason = "ExecutionStateView の不変条件 (空の Stage Progress は構成不能) により先頭要素は必ず存在する"
)]
fn stage_slug(state: &ExecutionStateView, stage: StageIndex) -> &StageSlugView {
    state
        .slug(stage)
        .unwrap_or_else(|| state.stages()[0].slug())
}

/// 定義側の既定ゲート (初期化のみ非ゲート — BR1.3 の静的既定)。
const fn default_gate(node: &StageView) -> GateField {
    if matches!(node.phase(), PhaseView::Initialization) {
        GateField::Ungated
    } else {
        GateField::Gated
    }
}

/// slug からグラフノードを引く。
fn find_node<'a>(definition: &'a DefinitionView, slug: &str) -> Option<&'a StageView> {
    definition
        .graph()
        .nodes()
        .iter()
        .find(|node| node.slug().as_str() == slug)
}

/// `run-stage` の組み立て (定義ビューのノード + 配置 VO)。
pub(crate) fn build_run_stage(
    node: &StageView,
    definition: &DefinitionView,
    scope: &str,
    layout: Option<&WorkspaceLayout>,
    gate: GateField,
    single: bool,
) -> Result<Directive, String> {
    let Some(layout) = layout else {
        return Err("No workspace layout was provided for run-stage assembly.".to_string());
    };
    let phase_dir = node.phase().as_str().to_lowercase();
    let record = layout.record_dir();
    let inline_context_paths = match node.mode() {
        StageModeView::Inline => {
            let mut paths = vec![format!("{}/{}.md", layout.agent_dir(), node.lead_agent())];
            paths.extend(
                node.support_agents()
                    .iter()
                    .map(|agent| format!("{}/{agent}.md", layout.agent_dir())),
            );
            paths
        }
        StageModeView::Mob => vec![format!("{}/{}.md", layout.agent_dir(), node.lead_agent())],
        StageModeView::Subagent | StageModeView::Pipeline | StageModeView::AgentTeam => Vec::new(),
    };
    let mut protocol_modules = Vec::new();
    if node.reviewer().is_some() {
        protocol_modules.push("reviewer".to_string());
    }
    if node.mode() != StageModeView::Inline || !node.support_agents().is_empty() {
        protocol_modules.push("ensemble".to_string());
    }
    if node.phase() == PhaseView::Construction {
        protocol_modules.push("construction".to_string());
    }
    let next_stage = next_in_scope_name(definition, scope, node);
    let mut builder = RunStageDirectiveBuilder::new(
        node.slug().clone(),
        node.phase(),
        node.lead_agent(),
        node.mode(),
        gate,
        format!(
            "{}/{phase_dir}/{}.md",
            layout.stage_library_dir(),
            node.slug().as_str()
        ),
        format!("{record}/{phase_dir}/{}/memory.md", node.slug().as_str()),
    )
    .with_support_agents(node.support_agents().to_vec())
    .with_inline_context_paths(inline_context_paths)
    .with_consumes(
        node.consumes()
            .iter()
            .map(|consume| format!("{record}/{}", consume.artifact()))
            .collect(),
    )
    .with_produces(
        node.produces()
            .iter()
            .map(|artifact| format!("{record}/{phase_dir}/{}/{artifact}", node.slug().as_str()))
            .collect(),
    )
    .with_sensors(node.sensors().to_vec())
    .with_protocol_modules(protocol_modules);
    if let Some(name) = next_stage {
        builder = builder.with_next_stage(name);
    }
    if let (Some(reviewer), Some(class)) = (node.reviewer(), node.review_class()) {
        builder =
            builder.with_reviewer(reviewer, class, node.reviewer_max_iterations().unwrap_or(1));
    }
    if single {
        builder = builder.with_single();
    }
    Ok(Directive::RunStage(builder.build()))
}

/// 連鎖 1 部の発出 — 計画上の部と、次を指すトークン (中身 — 封緘は U7 Presenter) を組む。
///
/// 部は [`SteeringPart`] (計画のクエリのみが構築 — 範囲外は表現不能)、束縛は [`Bindings`]
/// で受けるので、範囲外 part の内部エラーと裸ダイジェスト 4 本・センチネル文字列は存在しない。
pub(crate) fn emit_part(
    part: &SteeringPart<'_>,
    run_stage: &RunStageDirective,
    scope: &ScopeSlugView,
    bindings: &Bindings,
) -> Directive {
    let mut builder = super::continue_token::ContinueTokenBuilder::new(
        run_stage.stage().clone(),
        scope.clone(),
        part.index(),
        bindings.clone(),
        run_stage.gate(),
    );
    if let Some(next_stage) = run_stage.next_stage()
        && let Ok(name) = StageName::parse(next_stage)
    {
        builder = builder.with_next_stage(name);
    }
    if let Some(unit) = run_stage.unit() {
        builder = builder.with_unit(unit.clone());
    }
    if run_stage.is_single() {
        builder = builder.with_single();
    }
    Directive::LoadSteering(LoadSteeringDirective::new(
        run_stage.stage().clone(),
        bindings.bundle().clone(),
        part.index(),
        part.of(),
        part.chunk().to_vec(),
        builder.build(),
    ))
}

/// 現ノードの後で最初の in-scope EXECUTE ステージの表示名。
fn next_in_scope_name(
    definition: &DefinitionView,
    scope: &str,
    node: &StageView,
) -> Option<String> {
    let stages = definition.stages_in_scope(scope);
    let position = stages
        .iter()
        .position(|(slug, _, _)| slug.as_str() == node.slug().as_str())?;
    stages
        .iter()
        .skip(position + 1)
        .find(|(_, _, action)| *action == Some(PlanActionView::Execute))
        .and_then(|(slug, _, _)| find_node(definition, slug.as_str()))
        .map(|next| next.name().to_string())
}

#[cfg(test)]
mod tests {
    // panic! は想定外バリアントの即時失敗という検証用途で使っており、テスト失敗のシグナル
    // として妥当なため許容する。
    #![allow(clippy::panic)]

    use std::collections::BTreeMap;

    use super::super::bundle_digest::BundleDigest;
    use super::super::continue_token::ContinueTokenBuilder;
    use super::super::continue_use_case::ContinueUseCase;
    use super::super::directive_digest::DirectiveDigest;
    use super::super::noun_family::NounFamily;
    use super::super::noun_token::NounToken;
    use super::super::part_index::PartIndex;
    use super::super::route_digest::RouteDigest;
    use super::super::rule_content::RuleContent;
    use super::super::scope_source::ScopeSource;
    use super::super::test_fixtures::{FakeDefinitionDao, FakeRulesDao, FakeStateDao};
    use super::super::unit_kind::UnitKind;
    use super::super::unit_name::UnitName;
    use super::super::unit_ref::UnitRef;
    use super::super::{ReadOnlyVerb, test_fixtures};
    use super::*;
    use crate::orchestration::MemoryRules;
    use crate::orchestration::{
        CheckboxState, DefinitionIdView, DefinitionRevisionView, ExecutionKindView,
        ExecutionStatus, ReviewClassView, ScopeGridView, ScopeMetadataView, StageGraphView,
        StageNumberView, StageProgressView, StageViewBuilder,
    };

    use test_fixtures::{definition, genesis_state, parked_state, slug, state};

    /// 在るのに読めないルールファイル (blocking)。
    fn unreadable_rules() -> FakeRulesDao {
        FakeRulesDao::unreadable("aidlc/spaces/default/memory/org.md", "permission denied")
    }

    fn layout() -> WorkspaceLayout {
        WorkspaceLayout::new(
            "record".to_string(),
            "stages".to_string(),
            "agents".to_string(),
        )
    }

    /// 入力の共通形 (state の有無は注入する DAO で決まる)。
    fn input() -> NextTurnInput {
        NextTurnInput::new().with_layout(layout())
    }

    /// 3 つの DAO を束ねたユースケース。
    fn use_case(
        workflow_definition_dao: FakeDefinitionDao,
        execution_state_dao: FakeStateDao,
        memory_rules_dao: FakeRulesDao,
    ) -> NextUseCase<FakeDefinitionDao, FakeStateDao, FakeRulesDao> {
        NextUseCase::new(
            workflow_definition_dao,
            execution_state_dao,
            memory_rules_dao,
        )
    }

    /// state ありで走らせる (ルール束なし)。
    fn run_with(
        held: &ExecutionStateView,
        definition_view: &DefinitionView,
        turn: &NextTurnInput,
    ) -> Directive {
        run_with_rules(held, definition_view, FakeRulesDao::empty(), turn)
    }

    /// state ありで、ルール束の読取結果を指定して走らせる。
    fn run_with_rules(
        held: &ExecutionStateView,
        definition_view: &DefinitionView,
        memory_rules_dao: FakeRulesDao,
        turn: &NextTurnInput,
    ) -> Directive {
        use_case(
            FakeDefinitionDao::holding(definition_view.clone()),
            FakeStateDao::holding(held.clone()),
            memory_rules_dao,
        )
        .execute(turn)
    }

    /// state なしで走らせる (ルール束なし)。
    fn run_without(definition_view: &DefinitionView, turn: &NextTurnInput) -> Directive {
        use_case(
            FakeDefinitionDao::holding(definition_view.clone()),
            FakeStateDao::absent(),
            FakeRulesDao::empty(),
        )
        .execute(turn)
    }

    fn expect_load_steering(directive: Directive) -> LoadSteeringDirective {
        match directive {
            Directive::LoadSteering(part) => part,
            other => panic!("load-steering を期待したが {:?}", other.kind()),
        }
    }

    fn expect_run_stage(directive: Directive) -> RunStageDirective {
        match directive {
            Directive::RunStage(run_stage) => run_stage,
            other => panic!("run-stage を期待したが {:?}", other.kind()),
        }
    }

    fn expect_ask(directive: Directive) -> AskDirective {
        match directive {
            Directive::Ask(ask) => ask,
            other => panic!("ask を期待したが {:?}", other.kind()),
        }
    }

    fn error_message(directive: &Directive) -> &str {
        match directive {
            Directive::Error { message } => message,
            other => panic!("error を期待したが {:?}", other.kind()),
        }
    }

    fn print_message(directive: &Directive) -> &str {
        match directive {
            Directive::Print { message } => message,
            other => panic!("print を期待したが {:?}", other.kind()),
        }
    }

    /// 索引 0 完了・索引 1 着手中の状態 (ゲート付きステージのカーソル)。
    fn after_first_stage(stage_count: usize) -> ExecutionStateView {
        let markers: Vec<CheckboxState> = (0..stage_count)
            .map(|index| match index {
                0 => CheckboxState::Completed,
                1 => CheckboxState::InProgress,
                _ => CheckboxState::Pending,
            })
            .collect();
        state(
            stage_count,
            1,
            &markers,
            &vec![PlanActionView::Execute; stage_count],
        )
    }

    // ---- 前置ガード ----

    #[test]
    fn a_parse_error_is_relayed_verbatim() {
        let directive = run_without(
            &definition(2),
            &input().with_parse_error("--review requires <adversarial|advisory|none>."),
        );
        assert_eq!(
            error_message(&directive),
            "--review requires <adversarial|advisory|none>."
        );
    }

    #[test]
    fn review_combined_with_another_mode_is_refused() {
        let directive = run_without(
            &definition(2),
            &input().with_review("advisory").with_resume(),
        );
        assert_eq!(
            error_message(&directive),
            "Cannot combine --review with read-only, workspace, compose, single-stage, jump, or resume modes. Apply /aidlc --review <class> first, then run the other command."
        );
    }

    // ---- 分岐 0 / 1 / 1b ----

    #[test]
    fn branch_0_the_kiro_latch_ends_the_bare_next() {
        let directive = run_without(&definition(2), &input().with_kiro_latch_bare_next());
        assert_eq!(directive, Directive::Done { reason: None });
    }

    #[test]
    fn branch_1_a_read_only_flag_names_the_utility() {
        let directive = run_without(
            &definition(2),
            &input().with_read_only(ReadOnlyVerb::Status),
        );
        assert_eq!(
            print_message(&directive),
            "Run `aidlc-utility status`, print its output verbatim, then stop. \
This is a read-only utility, NOT workflow work: do NOT run `next` and do NOT advance, resume, or run any workflow stage."
        );
    }

    #[test]
    fn branch_1b_a_noun_token_passes_through_verbatim() {
        let token = NounToken::new(
            NounFamily::Workspace,
            vec!["intent".to_string(), "list".to_string()],
        );
        let directive = run_without(&definition(2), &input().with_noun_token(token));
        // 名詞は「終端ユーティリティ」— 読み取り専用フラグとは 1 語だけ違う (upstream `:2764`)。
        assert_eq!(
            print_message(&directive),
            "Run `aidlc-utility intent list`, print its output verbatim, then stop. \
This is a terminal utility, NOT workflow work: do NOT run `next` and do NOT advance, resume, or run any workflow stage."
        );
    }

    #[test]
    fn branch_1b_plugin_and_knowledge_tokens_also_pass_through() {
        for family in [NounFamily::Plugin, NounFamily::Knowledge] {
            let token = NounToken::new(family, vec!["list".to_string()]);
            let directive = run_without(&definition(2), &input().with_noun_token(token));
            assert!(print_message(&directive).contains("aidlc-utility list"));
        }
    }

    #[test]
    fn every_read_only_verb_names_its_subcommand() {
        for (verb, sub) in [
            (ReadOnlyVerb::Status, "status"),
            (ReadOnlyVerb::Help, "help"),
            (ReadOnlyVerb::Doctor, "doctor"),
            (ReadOnlyVerb::Version, "version"),
        ] {
            let directive = run_without(&definition(2), &input().with_read_only(verb));
            assert!(print_message(&directive).contains(&format!("aidlc-utility {sub}")));
        }
    }

    // ---- 分岐 2 / state 読取ガード ----

    #[test]
    fn branch_2_stage_and_phase_together_are_refused() {
        let directive = run_without(
            &definition(2),
            &input().with_stage("stage-1").with_phase("Inception"),
        );
        assert_eq!(
            error_message(&directive),
            "Cannot use --stage and --phase together. Use one or the other."
        );
    }

    #[test]
    fn a_broken_state_read_stops_before_the_cursor_is_used() {
        // 旧 state バージョンガードの相当 — 読取失敗は復元前に材料つきで止める。
        let directive = use_case(
            FakeDefinitionDao::holding(definition(2)),
            FakeStateDao::failing(ExecutionStateReadError::NotReadable {
                path: "/tmp/aidlc-state.md".to_string(),
                cause: "permission denied".to_string(),
            }),
            FakeRulesDao::empty(),
        )
        .execute(&input());
        assert_eq!(
            error_message(&directive),
            "/tmp/aidlc-state.md: permission denied"
        );
    }

    #[test]
    fn a_malformed_state_read_stops_with_the_decoding_refusal() {
        let directive = use_case(
            FakeDefinitionDao::holding(definition(2)),
            FakeStateDao::failing(ExecutionStateReadError::Malformed {
                path: "/tmp/aidlc-state.md".to_string(),
                cause: "missing field \"Status\"".to_string(),
            }),
            FakeRulesDao::empty(),
        )
        .execute(&input());
        assert_eq!(
            error_message(&directive),
            "/tmp/aidlc-state.md: missing field \"Status\""
        );
    }

    /// 定義の読取失敗は 12 §4 の逐語文言になる (文言は出す側 = 本ユースケースが組む)。
    #[test]
    fn an_unreadable_definition_is_refused_with_the_upstream_wording() {
        let directive = use_case(
            FakeDefinitionDao::failing(WorkflowDefinitionReadError::NotReadable {
                path: "/d/stage-graph.json".to_string(),
                cause: "ENOENT".to_string(),
                env_override: false,
            }),
            FakeStateDao::absent(),
            FakeRulesDao::empty(),
        )
        .execute(&input());
        assert_eq!(
            error_message(&directive),
            "Stage graph not readable at /d/stage-graph.json: ENOENT. Reinstall the framework or re-run setup to restore the data file."
        );
    }

    #[test]
    fn an_env_overridden_graph_path_switches_the_hint_clause() {
        let directive = use_case(
            FakeDefinitionDao::failing(WorkflowDefinitionReadError::NotReadable {
                path: "/x.json".to_string(),
                cause: "ENOENT".to_string(),
                env_override: true,
            }),
            FakeStateDao::absent(),
            FakeRulesDao::empty(),
        )
        .execute(&input());
        assert_eq!(
            error_message(&directive),
            "Stage graph not readable at /x.json: ENOENT. AIDLC_STAGE_GRAPH points to /x.json; unset it to use the default."
        );
    }

    #[test]
    fn every_definition_read_failure_has_its_wording() {
        for (error, expected) in [
            (
                WorkflowDefinitionReadError::InvalidJson {
                    path: "/d/stage-graph.json".to_string(),
                    cause: "eof".to_string(),
                },
                "Stage graph at /d/stage-graph.json is not valid JSON: eof",
            ),
            (
                WorkflowDefinitionReadError::HarnessIdentity {
                    path: "/d/harness.json".to_string(),
                    cause: "ENOENT".to_string(),
                },
                "Harness identity not readable at /d/harness.json: ENOENT",
            ),
            (
                WorkflowDefinitionReadError::ScopeFile {
                    message: "Scope file missing frontmatter: /s/aidlc-x.md".to_string(),
                },
                "Scope file missing frontmatter: /s/aidlc-x.md",
            ),
            (
                WorkflowDefinitionReadError::Malformed {
                    message: "unknown phase \"daydreaming\"".to_string(),
                },
                "unknown phase \"daydreaming\"",
            ),
        ] {
            let directive = use_case(
                FakeDefinitionDao::failing(error),
                FakeStateDao::absent(),
                FakeRulesDao::empty(),
            )
            .execute(&input());
            assert_eq!(error_message(&directive), expected);
        }
    }

    #[test]
    fn a_missing_definition_id_without_state_is_refused() {
        let directive = use_case(
            FakeDefinitionDao::failing(WorkflowDefinitionReadError::Unidentified),
            FakeStateDao::absent(),
            FakeRulesDao::empty(),
        )
        .execute(&input());
        assert_eq!(
            error_message(&directive),
            "No workflow definition id was provided."
        );
    }

    /// 前置ガードで答えが決まるターンは、どのリードモデルにも触れない。
    #[test]
    fn a_pre_guard_turn_reads_no_read_model() {
        // 3 つの DAO をすべて失敗させても、前置ガードの逐語文言がそのまま出る。
        let directive = use_case(
            FakeDefinitionDao::failing(WorkflowDefinitionReadError::Unidentified),
            FakeStateDao::failing(ExecutionStateReadError::NotReadable {
                path: "/tmp/aidlc-state.md".to_string(),
                cause: "permission denied".to_string(),
            }),
            unreadable_rules(),
        )
        .execute(&input().with_read_only(ReadOnlyVerb::Status));
        assert!(print_message(&directive).contains("aidlc-utility status"));
    }

    // ---- 分岐 2.5 / 2.6 (park) ----

    #[test]
    fn branch_2_5_a_parked_workflow_stops_with_the_parked_directive() {
        let parked = parked_state(
            2,
            0,
            Some(0),
            &[CheckboxState::InProgress, CheckboxState::Pending],
            &[PlanActionView::Execute; 2],
        );
        let directive = run_with(&parked, &definition(2), &input());
        assert_eq!(
            directive,
            Directive::Parked {
                stage: slug(0),
                message: "Workflow parked at \"stage-0\". Resume with /aidlc --resume.".to_string()
            }
        );
    }

    #[test]
    fn branch_2_6_resume_on_a_parked_workflow_names_unpark() {
        let parked = parked_state(
            2,
            0,
            Some(0),
            &[CheckboxState::InProgress, CheckboxState::Pending],
            &[PlanActionView::Execute; 2],
        );
        let directive = run_with(&parked, &definition(2), &input().with_resume());
        assert_eq!(
            print_message(&directive),
            "This workflow is parked. Run `aidlc-state unpark` to clear the park marker, then re-run `next --resume` to continue."
        );
    }

    // ---- 分岐 3b / 4 (scope 検証) ----

    #[test]
    fn branch_3b_an_invalid_explicit_scope_is_refused_even_when_state_wins() {
        let directive = run_with(
            &genesis_state(2),
            &definition(2),
            &input().with_scope("warp-drive"),
        );
        assert_eq!(
            error_message(&directive),
            "Unknown scope \"warp-drive\". Valid scopes: bugfix, classic."
        );
    }

    #[test]
    fn branch_4_an_invalid_env_default_scope_is_refused_verbatim() {
        let directive = run_without(
            &definition(2),
            &input().with_env_default_scope("warp-drive"),
        );
        assert_eq!(
            error_message(&directive),
            "Invalid AWS_AIDLC_DEFAULT_SCOPE \"warp-drive\". Valid scopes: bugfix, classic."
        );
    }

    #[test]
    fn a_state_scope_missing_from_the_definition_is_unresolvable() {
        // リードモデルは classic を握っているが、定義に classic が無い。
        let held = definition_with_scopes(&["bugfix"]);
        let directive = run_with(&genesis_state(2), &held, &input());
        assert_eq!(
            error_message(&directive),
            "Unknown scope \"classic\". Valid scopes: bugfix."
        );
    }

    // ---- 分岐 4c / 4a / 4b ----

    #[test]
    fn branch_4c_compose_with_a_jump_flag_is_refused() {
        let directive = run_without(
            &definition(2),
            &input().with_compose().with_stage("stage-1"),
        );
        assert_eq!(
            error_message(&directive),
            "Cannot combine compose with --stage/--phase. Compose re-shapes the plan; jump moves the cursor. Run them separately."
        );
    }

    #[test]
    fn branch_4c_compose_names_the_composer_dispatch() {
        let directive = run_without(&definition(2), &input().with_compose());
        assert!(print_message(&directive).contains("aidlc-composer detect"));
    }

    /// 配置が無ければ run-stage は組めない — 逐語で止め、run-stage は出さない。
    ///
    /// 分岐 10（ハッピーパス）・分岐 4b（`--single`）・分岐 7（jump）の 3 経路がいずれも
    /// 同じ組み立てを通るので、3 つとも同じ拒否になる。
    #[test]
    fn a_run_stage_cannot_be_assembled_without_a_workspace_layout() {
        let bare = || NextTurnInput::new();
        let expected = "No workspace layout was provided for run-stage assembly.";

        // 分岐 10 — state ありのハッピーパス。
        assert_eq!(
            error_message(&run_with(&genesis_state(2), &definition(2), &bare())),
            expected
        );
        // 分岐 4b — `--single`。
        assert_eq!(
            error_message(&run_without(
                &definition(2),
                &bare().with_single().with_stage("stage-1")
            )),
            expected
        );
        // 分岐 7 — jump（state 無しでも組み立てまで進む）。
        assert_eq!(
            error_message(&run_without(&definition(2), &bare().with_stage("stage-1"))),
            expected
        );
    }

    /// コスト算術を見るための定義 — 3 つの既製 scope・SKIP セル・per-unit ステージ 2 種・
    /// グリッドにだけ在る slug・グリッド列を持たない scope をすべて含む。
    fn cost_definition() -> DefinitionView {
        use crate::orchestration::{
            DefinitionIdView, DefinitionRevisionView, ExecutionKindView, ScopeGridView,
            ScopeMetadataView, StageGraphView, StageNumberView, StageViewBuilder,
        };
        use std::collections::BTreeMap;

        let node = |slug: &str, number: &str, phase: PhaseView, per_unit: bool| {
            let builder = StageViewBuilder::new(
                StageSlugView::parse(slug).expect("固定の slug"),
                StageNumberView::parse(number).expect("固定の番号"),
                format!("Stage {slug}"),
                phase,
                ExecutionKindView::Always,
                StageModeView::Inline,
            )
            .with_lead_agent("orchestrator".to_string());
            if per_unit {
                builder.with_for_each("unit-of-work".to_string()).build()
            } else {
                builder.build()
            }
        };
        let nodes = vec![
            node("state-init", "0.1", PhaseView::Initialization, false),
            node("domain-design", "1.1", PhaseView::Inception, false),
            // KNOWN_PER_UNIT_STAGES による判定（`for_each` は付いていない）。
            node("nfr-design", "3.2", PhaseView::Construction, false),
            // `for_each` による判定。
            node("code-generation", "3.5", PhaseView::Construction, true),
        ];
        let cell = |slug: &str, action: PlanActionView| {
            (StageSlugView::parse(slug).expect("固定の slug"), action)
        };
        let grid = ScopeGridView::new(
            [
                (
                    "express".to_string(),
                    [
                        cell("state-init", PlanActionView::Execute),
                        cell("domain-design", PlanActionView::Skip),
                        cell("nfr-design", PlanActionView::Skip),
                        cell("code-generation", PlanActionView::Execute),
                    ]
                    .into_iter()
                    .collect::<BTreeMap<_, _>>(),
                ),
                (
                    "classic".to_string(),
                    [
                        cell("state-init", PlanActionView::Execute),
                        cell("domain-design", PlanActionView::Execute),
                        cell("nfr-design", PlanActionView::Execute),
                        cell("code-generation", PlanActionView::Execute),
                        // グリッドにだけ在る slug — total / execute には数え、
                        // gate と per-unit には数えない（upstream と同じ防御）。
                        cell("ghost-stage", PlanActionView::Execute),
                    ]
                    .into_iter()
                    .collect::<BTreeMap<_, _>>(),
                ),
                (
                    "feature".to_string(),
                    [
                        cell("state-init", PlanActionView::Execute),
                        cell("domain-design", PlanActionView::Execute),
                    ]
                    .into_iter()
                    .collect::<BTreeMap<_, _>>(),
                ),
            ]
            .into_iter()
            .collect(),
        );
        let scopes: BTreeMap<String, ScopeMetadataView> =
            ["express", "classic", "feature", "gridless"]
                .into_iter()
                .map(|name| {
                    (
                        name.to_string(),
                        ScopeMetadataView::new(name).expect("固定の scope 名"),
                    )
                })
                .collect();
        DefinitionView::new(
            DefinitionIdView::parse("claude").expect("固定の定義 id"),
            DefinitionRevisionView::parse(&format!("sha256:{}", "0".repeat(64)))
                .expect("固定の内容版"),
            StageGraphView::new(nodes).expect("固定のグラフ"),
            grid,
            scopes,
        )
    }

    /// コスト節はグリッドを数える — EXECUTE / 総数 / ゲート（initialization を除く）/
    /// per-unit の反復。反復 1 本は単数形。
    #[test]
    fn the_cost_clause_counts_gates_and_per_unit_repeats() {
        let directive = run_without(&cost_definition(), &input().with_scope("express"));
        let message = print_message(&directive);
        assert!(
            message.contains(
                " (2 of 4 stages, 1 approval gates, 1 stage repeats per unit of work in Construction)"
            ),
            "{message}"
        );
    }

    /// 反復が 2 本以上なら複数形。グラフに無い slug は総数には入るがゲートには入らない。
    #[test]
    fn the_cost_clause_pluralises_repeats_and_ignores_slugs_outside_the_graph() {
        let directive = run_without(&cost_definition(), &input().with_scope("classic"));
        let message = print_message(&directive);
        assert!(
            message.contains(
                " (5 of 5 stages, 3 approval gates, 2 stages repeat per unit of work in Construction)"
            ),
            "{message}"
        );
    }

    /// グリッド列を持たない scope では括弧ごと落ちる（upstream も同じ）。
    #[test]
    fn a_scope_without_a_grid_column_drops_the_cost_parenthetical() {
        let directive = run_without(&cost_definition(), &input().with_scope("gridless"));
        assert_eq!(
            print_message(&directive),
            "Run `aidlc-utility intent-create --scope gridless` to start the workflow, then re-run `next` to continue."
        );
    }

    /// compose 提案は 3 つの既製 scope の実数を並べる（3 つとも解決できたとき）。
    #[test]
    fn branch_8_the_compose_offer_lists_the_three_stock_scopes() {
        let directive = run_without(
            &cost_definition(),
            &input().with_freeform("something entirely novel"),
        );
        let ask = expect_ask(directive);
        assert_eq!(ask.ask_kind(), AskKind::ComposeOffer);
        let question = ask.question();
        assert!(
            question.contains("(e.g. express = 2 of 4 stages, classic = 5, feature = all 2;"),
            "{question}"
        );
    }

    #[test]
    fn branch_4a_a_blank_new_intent_description_is_refused_verbatim() {
        let directive = run_without(
            &definition(2),
            &input().with_new_intent("   ").with_scope("bugfix"),
        );
        assert_eq!(
            error_message(&directive),
            "`next --new-intent` requires a nonblank new-work description after the confirmed scope."
        );
    }

    /// 明示 `--scope` があればそれが勝ち、命令行は upstream の完全形になる。
    #[test]
    fn branch_4a_new_intent_names_the_full_intent_create_command() {
        let directive = run_without(
            &definition(2),
            &input()
                .with_new_intent("fix the crash")
                .with_scope("bugfix"),
        );
        let message = print_message(&directive);
        assert!(
            message.starts_with(
                "Run `aidlc-utility intent-create --scope bugfix --arguments='fix the crash' --label \"<2-3 word kebab essence>\"` to start the new intent (2 of 2 stages, 1 approval gates)."
            ),
            "{message}"
        );
    }

    /// 任意フラグは命令行へ素通しされる (upstream `:892-894`)。
    #[test]
    fn branch_4a_new_intent_threads_the_optional_flags() {
        let directive = run_without(
            &definition(2),
            &input()
                .with_new_intent("fix the crash")
                .with_scope("bugfix")
                .with_depth("standard")
                .with_test_strategy("minimal"),
        );
        let message = print_message(&directive);
        assert!(
            message.contains(
                "--label \"<2-3 word kebab essence>\" --depth standard --test-strategy minimal`"
            ),
            "{message}"
        );
    }

    /// 散文 3 点 (コスト節・ラベル助言・STOP 4 文) が upstream 逐語で載る。
    #[test]
    fn branch_4a_new_intent_carries_the_upstream_prose() {
        let directive = run_without(
            &definition(2),
            &input()
                .with_new_intent("fix the crash")
                .with_scope("bugfix"),
        );
        let message = print_message(&directive);
        assert!(
            message.contains(" (2 of 2 stages, 1 approval gates)."),
            "{message}"
        );
        assert!(
            message.contains(
                " Replace `--label` with a 2-3 word kebab essence of the description (e.g. \"simple calc\"), which becomes the readable folder name for this piece of work."
            ),
            "{message}"
        );
        assert!(
            message.ends_with(
                "Then STOP, do NOT re-run `next` in this session. \
This is a NEW, unrelated intent, and the current session still carries the previous intent's context. \
Tell the user to start a fresh session using this harness's reset or restart flow, then invoke its AI-DLC entry skill to begin the new intent with a clean slate. \
Nothing is lost: the intent is saved on disk and resumes on the next `next`."
            ),
            "{message}"
        );
    }

    /// 稼働中の intent があっても、確認された **新しい仕事の scope** が勝つ。
    ///
    /// ラダーは state scope を最優先にするので、そちらを使うと新 intent が進行中の仕事の
    /// scope で生まれてしまう (upstream `:2985-2990` がまさにこれを避けている)。
    #[test]
    fn branch_4a_the_explicit_scope_outranks_the_active_intents_state_scope() {
        // 実行状態の scope は classic。新しい仕事は bugfix として確認された。
        let directive = run_with(
            &genesis_state(2),
            &definition(2),
            &input()
                .with_new_intent("fix the crash")
                .with_scope("bugfix"),
        );
        let message = print_message(&directive);
        assert!(
            message.starts_with("Run `aidlc-utility intent-create --scope bugfix"),
            "{message}"
        );
    }

    /// 明示が無いときは解決済み scope (稼働中なら state の scope) が使われる。
    #[test]
    fn branch_4a_without_an_explicit_scope_the_active_state_scope_is_inherited() {
        let directive = run_with(
            &genesis_state(2),
            &definition(2),
            &input().with_new_intent("fix the crash"),
        );
        let message = print_message(&directive);
        assert!(
            message.starts_with("Run `aidlc-utility intent-create --scope classic"),
            "{message}"
        );
    }

    /// 明示 `--scope` が無ければ解決済み scope へ落ちる (upstream `flags.scope ?? scope`)。
    /// 拒否ではない — ここで拒むのは upstream に無い造語だった。
    #[test]
    fn branch_4a_new_intent_without_an_explicit_scope_falls_back_to_the_resolved_scope() {
        let directive = run_without(&definition(2), &input().with_new_intent("fix the crash"));
        let message = print_message(&directive);
        assert!(
            message.starts_with(
                "Run `aidlc-utility intent-create --scope classic --arguments='fix the crash'"
            ),
            "{message}"
        );
    }

    #[test]
    fn branch_4b_single_requires_a_stage() {
        let directive = run_without(&definition(2), &input().with_single());
        assert_eq!(
            error_message(&directive),
            "--single requires --stage <slug>."
        );
    }

    #[test]
    fn branch_4b_single_emits_an_isolated_run_stage() {
        let directive = run_without(&definition(2), &input().with_single().with_stage("stage-1"));
        let run_stage = expect_run_stage(directive);
        assert!(run_stage.is_single());
        assert_eq!(run_stage.stage().as_str(), "stage-1");
    }

    #[test]
    fn branch_4b_single_with_an_unknown_stage_is_refused() {
        let directive = run_without(
            &definition(2),
            &input().with_single().with_stage("no-such-stage"),
        );
        assert_eq!(
            error_message(&directive),
            "Unknown stage \"no-such-stage\". Run /aidlc --help for the full list."
        );
    }

    #[test]
    fn branch_4b_single_on_an_initialization_stage_is_ungated() {
        let directive = run_without(&definition(2), &input().with_single().with_stage("stage-0"));
        assert_eq!(expect_run_stage(directive).gate(), GateField::Ungated);
    }

    // ---- 分岐 5 / 6 ----

    #[test]
    fn branch_5_a_differing_valid_scope_names_scope_change() {
        let directive = run_with(
            &genesis_state(2),
            &definition(2),
            &input().with_scope("bugfix"),
        );
        assert_eq!(
            print_message(&directive),
            "Run `aidlc-utility scope-change --scope bugfix` to change scope, then print its output verbatim and stop."
        );
    }

    /// scope 変更に併記された設定修飾は同じ 1 本へ載る (upstream `:3051-3054`)。
    #[test]
    fn branch_5_a_scope_change_carries_the_modifiers_typed_alongside_it() {
        let directive = run_with(
            &genesis_state(2),
            &definition(2),
            &input()
                .with_scope("bugfix")
                .with_depth("minimal")
                .with_review("none"),
        );
        assert_eq!(
            print_message(&directive),
            "Run `aidlc-utility scope-change --scope bugfix --depth minimal --review none` to change scope, then print its output verbatim and stop."
        );
    }

    #[test]
    fn branch_5_a_depth_override_names_config_change() {
        let directive = run_with(
            &genesis_state(2),
            &definition(2),
            &input().with_depth("minimal"),
        );
        assert_eq!(
            print_message(&directive),
            "Run `aidlc-utility config-change --depth minimal` to update the configuration, then print its output verbatim and stop."
        );
    }

    /// 修飾を 2 つ与えたら 2 つとも同じ命令に載る（片方が黙って落ちない）。
    #[test]
    fn branch_5_two_modifiers_ride_one_config_change() {
        let directive = run_with(
            &genesis_state(2),
            &definition(2),
            &input().with_depth("minimal").with_test_strategy("standard"),
        );
        assert_eq!(
            print_message(&directive),
            "Run `aidlc-utility config-change --depth minimal --test-strategy standard` to update the configuration, then print its output verbatim and stop."
        );
    }

    #[test]
    fn branch_5_a_test_strategy_override_names_config_change() {
        let directive = run_with(
            &genesis_state(2),
            &definition(2),
            &input().with_test_strategy("minimal"),
        );
        assert!(
            print_message(&directive)
                .contains("aidlc-utility config-change --test-strategy minimal")
        );
    }

    #[test]
    fn branch_5_a_review_override_alone_names_config_change() {
        let directive = run_with(
            &genesis_state(2),
            &definition(2),
            &input().with_review("adversarial"),
        );
        assert!(
            print_message(&directive).contains("aidlc-utility config-change --review adversarial")
        );
    }

    #[test]
    fn branch_6_resume_with_state_asks_the_resume_menu() {
        let directive = run_with(&genesis_state(2), &definition(2), &input().with_resume());
        let ask = expect_ask(directive);
        assert_eq!(ask.ask_kind(), AskKind::ResumeMenu);
        assert_eq!(
            ask.question(),
            "An existing workflow was found (currently at \"stage-0\"). How would you like to proceed? Resume from last checkpoint, redo the current stage, jump to a stage, or start fresh."
        );
    }

    // ---- 分岐 7 (jump) ----

    #[test]
    fn branch_7_a_jump_to_an_initialization_stage_is_refused_verbatim() {
        let directive = run_with(
            &genesis_state(2),
            &definition(2),
            &input().with_stage("stage-0"),
        );
        assert_eq!(
            error_message(&directive),
            "Cannot jump to initialization stages. The Initialization phase runs automatically when you start a workflow (describe what to build, e.g. /aidlc \"build the auth service\")."
        );
    }

    #[test]
    fn branch_7_a_jump_with_state_names_the_pure_resolve() {
        let directive = run_with(
            &genesis_state(3),
            &definition(3),
            &input().with_stage("stage-2"),
        );
        assert!(print_message(&directive).contains("aidlc-jump resolve --stage stage-2"));
    }

    #[test]
    fn branch_7_a_jump_without_state_searches_the_graph_directly() {
        let directive = run_without(&definition(3), &input().with_stage("stage-2"));
        let run_stage = expect_run_stage(directive);
        assert_eq!(run_stage.stage().as_str(), "stage-2");
        assert!(!run_stage.is_single());
    }

    #[test]
    fn branch_7_a_jump_to_an_unknown_stage_is_refused() {
        let directive = run_with(
            &genesis_state(2),
            &definition(2),
            &input().with_stage("no-such-stage"),
        );
        assert_eq!(
            error_message(&directive),
            "Unknown stage \"no-such-stage\". Run /aidlc --help for the full list."
        );
    }

    #[test]
    fn branch_7_a_phase_jump_without_state_searches_the_graph() {
        let directive = run_without(&definition(3), &input().with_phase("inception"));
        assert_eq!(
            expect_run_stage(directive).stage().as_str(),
            "stage-1",
            "フェーズ先頭の in-scope ステージ"
        );
    }

    #[test]
    fn branch_7_an_unknown_phase_is_refused() {
        let directive = run_without(&definition(2), &input().with_phase("Daydreaming"));
        assert_eq!(
            error_message(&directive),
            "Unknown phase \"Daydreaming\". Valid phases: initialization, ideation, inception, construction, operation."
        );
    }

    #[test]
    fn a_phase_with_no_in_scope_stage_is_refused() {
        let directive = run_without(&definition(2), &input().with_phase("operation"));
        assert_eq!(
            error_message(&directive),
            "No in-scope stage found for phase \"operation\"."
        );
    }

    #[test]
    fn branch_7_a_jump_without_layout_stops_run_stage_assembly() {
        let directive = run_without(&definition(2), &NextTurnInput::new().with_stage("stage-1"));
        assert_eq!(
            error_message(&directive),
            "No workspace layout was provided for run-stage assembly."
        );
    }

    #[test]
    fn a_missing_layout_stops_run_stage_assembly() {
        let directive = run_without(
            &definition(2),
            &NextTurnInput::new().with_single().with_stage("stage-1"),
        );
        assert_eq!(
            error_message(&directive),
            "No workspace layout was provided for run-stage assembly."
        );
    }

    #[test]
    fn a_missing_layout_on_the_happy_path_stops_run_stage_assembly() {
        let directive = run_with(&genesis_state(2), &definition(2), &NextTurnInput::new());
        assert_eq!(
            error_message(&directive),
            "No workspace layout was provided for run-stage assembly."
        );
    }

    // ---- state なしの群 (7b / 8 / 9a / 9b) ----

    #[test]
    fn branch_7b_a_positional_scope_names_the_birth() {
        let directive = run_without(&definition(2), &input().with_freeform("bugfix"));
        assert!(print_message(&directive).contains("aidlc-utility intent-create --scope bugfix"));
    }

    #[test]
    fn branch_7b_records_without_a_cursor_ask_the_intent_pick() {
        let directive = run_without(
            &definition(2),
            &input()
                .with_freeform("bugfix")
                .with_records_without_cursor(),
        );
        assert_eq!(expect_ask(directive).ask_kind(), AskKind::IntentPick);
    }

    #[test]
    fn branch_8_a_keyword_hit_asks_the_scope_confirmation() {
        let directive = run_without(&definition(2), &input().with_freeform("fix the login"));
        let ask = expect_ask(directive);
        assert_eq!(ask.ask_kind(), AskKind::ScopeConfirm);
        // 逐語 — コスト節の区切りは ` - ` (誕生 print の括弧とは違う)。
        assert_eq!(
            ask.question(),
            "This looks like \"bugfix\" work, so I'd run the \"bugfix\" plan for: \"fix the login\" - 2 of 2 stages, 1 approval gates. \
Say go ahead, name a different plan, or say \"compose\" and I'll tailor one to this task."
        );
    }

    #[test]
    fn branch_8_a_keyword_in_a_long_description_is_suppressed() {
        // 5 語超のテキストは推論を抑止する — キーワードが偶然含まれる記述のガード。
        let directive = run_without(
            &definition(2),
            &input().with_freeform("please fix the login page for our production customers"),
        );
        let ask = expect_ask(directive);
        assert_eq!(ask.ask_kind(), AskKind::ComposeOffer);
        // 逐語。フィクスチャに express / feature が無いので目安は有効 scope 先頭 3 件へ落ちる。
        assert_eq!(
            ask.question(),
            "None of the ready-made plans is an obvious fit for: \"please fix the login page for our production customers\". \
I can work out a plan tailored to this task (recommended: reply \"compose\"), \
or you can pick one directly (e.g. bugfix, classic; see /aidlc --help for the full list)."
        );
    }

    #[test]
    fn branch_9a_an_explicit_scope_names_the_birth() {
        let directive = run_without(&definition(2), &input().with_scope("classic"));
        // 記述が無いので `--arguments` / `--label` もラベル助言も出ない (upstream `:881`)。
        assert_eq!(
            print_message(&directive),
            "Run `aidlc-utility intent-create --scope classic` to start the workflow (2 of 2 stages, 1 approval gates), then re-run `next` to continue."
        );
    }

    /// 明示 scope に自由記述が併記されると、記述が `--arguments` へ乗りラベル助言が続く。
    #[test]
    fn branch_9a_a_described_birth_threads_the_arguments_and_label_hint() {
        let directive = run_without(
            &definition(2),
            &input()
                .with_scope("classic")
                .with_freeform("build the auth service"),
        );
        assert_eq!(
            print_message(&directive),
            "Run `aidlc-utility intent-create --scope classic --arguments='build the auth service' --label \"<2-3 word kebab essence>\"` to start the workflow (2 of 2 stages, 1 approval gates), then re-run `next` to continue. \
Replace `--label` with a 2-3 word kebab essence of the description (e.g. \"simple calc\"), which becomes the readable folder name for this piece of work."
        );
    }

    #[test]
    fn branch_9b_nothing_named_without_state_is_refused_verbatim() {
        let directive = run_without(&definition(2), &input());
        assert_eq!(
            error_message(&directive),
            "No workflow state found (no active intent). Start one by describing what to build (/aidlc \"build the auth service\") or by naming a scope (/aidlc --scope <scope>)."
        );
    }

    // ---- 分岐 9c ----

    #[test]
    fn branch_9c_freeform_prose_on_a_running_workflow_asks_the_routing() {
        let directive = run_with(
            &genesis_state(2),
            &definition(2),
            &input().with_freeform("fix the crash"),
        );
        let ask = expect_ask(directive);
        assert_eq!(ask.ask_kind(), AskKind::NewWorkRouting);
        assert_eq!(ask.new_work_description(), Some("fix the crash"));
        assert_eq!(ask.proposed_scope(), Some("bugfix"));
    }

    #[test]
    fn branch_9c_an_uninferable_description_falls_back_to_the_resolved_scope() {
        // 5 語超の記述はキーワード推論が抑止される — 提案 scope は稼働中の解決値に畳む。
        let directive = run_with(
            &genesis_state(2),
            &definition(2),
            &input().with_freeform("please fix the login crash we saw yesterday in production"),
        );
        assert_eq!(expect_ask(directive).proposed_scope(), Some("classic"));
    }

    // ---- 分岐 10 (ハッピーパス) ----

    #[test]
    fn branch_10_the_happy_path_emits_a_run_stage_for_the_cursor() {
        let directive = run_with(&genesis_state(2), &definition(2), &input());
        let run_stage = expect_run_stage(directive);
        assert_eq!(run_stage.stage().as_str(), "stage-0");
        assert_eq!(
            run_stage.gate(),
            GateField::Ungated,
            "initialization は非ゲート"
        );
        assert_eq!(run_stage.stage_file(), "stages/initialization/stage-0.md");
        assert_eq!(
            run_stage.memory_path(),
            "record/initialization/stage-0/memory.md"
        );
        assert_eq!(
            run_stage.inline_context_paths(),
            ["agents/orchestrator.md"],
            "inline はリード (+支援) のペルソナを読む"
        );
        assert_eq!(run_stage.next_stage(), Some("Stage 1"));
    }

    #[test]
    fn branch_10_a_gated_stage_carries_the_gate() {
        let directive = run_with(&after_first_stage(2), &definition(2), &input());
        let run_stage = expect_run_stage(directive);
        assert_eq!(run_stage.stage().as_str(), "stage-1");
        assert_eq!(run_stage.gate(), GateField::Gated);
    }

    #[test]
    fn branch_10_a_finished_workflow_is_done_with_the_verbatim_reason() {
        let done = test_fixtures::completed_state(1);
        let directive = run_with(&done, &definition(1), &input());
        assert_eq!(
            directive,
            Directive::Done {
                reason: Some(
                    "Workflow complete — no in-scope stage remains after stage-0 (scope: classic). \
If this input is genuinely NEW, unrelated work (not a follow-up to the completed intent), don't stop here: offer to start a second intent, and on the human's yes run `next --new-intent --scope <scope> \"<text>\"` (see the SKILL's new-work offer, never auto-birth)."
                        .to_string()
                )
            }
        );
    }

    #[test]
    fn branch_10_a_recoverable_skip_inconsistency_names_the_repair() {
        // カーソルが実効 SKIP かつ着手済み。
        let held = state(
            2,
            1,
            &[CheckboxState::Completed, CheckboxState::InProgress],
            &[PlanActionView::Execute, PlanActionView::Skip],
        );
        let directive = run_with(&held, &definition(2), &input());
        assert_eq!(
            print_message(&directive),
            "Stage \"stage-1\" is SKIP in the approved workflow plan but is still the active cursor. \
Do not run this stage. Run `aidlc-orchestrate report --stage stage-1 --result skipped --reason 'stage is SKIP in the approved workflow plan'` \
to recover the stale pointer, then re-run `next` to continue."
        );
    }

    #[test]
    fn branch_10_an_unrecoverable_skip_inconsistency_is_refused_verbatim() {
        // カーソルが実効 SKIP かつ復旧経路の外 (awaiting-approval)。
        let held = state(
            2,
            1,
            &[CheckboxState::Completed, CheckboxState::AwaitingApproval],
            &[PlanActionView::Execute, PlanActionView::Skip],
        );
        let directive = run_with(&held, &definition(2), &input());
        assert_eq!(
            error_message(&directive),
            "Stage \"stage-1\" is SKIP in the approved workflow plan but its active cursor state is \"awaiting-approval\". Refusing to emit run-stage; repair the inconsistent state before continuing."
        );
    }

    #[test]
    fn a_routing_decision_that_reaches_the_happy_path_is_a_defensive_error() {
        let held = genesis_state(2);
        let directive = use_case(
            FakeDefinitionDao::holding(definition(2)),
            FakeStateDao::holding(held.clone()),
            FakeRulesDao::empty(),
        )
        .emit_happy_path(
            &input(),
            &held,
            &definition(2),
            &ScopeSlugView::parse("classic").unwrap(),
            NextDecision::ResumeMenu,
        );
        assert!(error_message(&directive).starts_with("internal:"));
    }

    #[test]
    fn every_checkbox_state_has_its_upstream_word() {
        use CheckboxState as S;
        for (state, word) in [
            (S::Pending, "pending"),
            (S::InProgress, "in-progress"),
            (S::AwaitingApproval, "awaiting-approval"),
            (S::Revising, "revising"),
            (S::Completed, "completed"),
            (S::Skipped, "skipped"),
        ] {
            assert_eq!(wording::checkbox_word(state), word);
        }
    }

    #[test]
    fn a_cursor_slug_missing_from_the_graph_is_refused_on_the_happy_path() {
        // 定義のグラフに cursor の slug が無い (定義とリードモデルの食い違い方の一種)。
        let held = definition_with_single_node("someone-else");
        let directive = run_with(&genesis_state(1), &held, &input());
        assert_eq!(
            error_message(&directive),
            "Unknown stage \"stage-0\". Run /aidlc --help for the full list."
        );
    }

    // ---- run-stage 組み立ての面 (レビュアー・モード別 inline paths) ----

    #[test]
    fn a_reviewer_bearing_stage_carries_the_reviewer_and_protocol_hint() {
        let held = definition_with(|index| {
            let mut builder = StageViewBuilder::new(
                slug(index),
                StageNumberView::parse(&format!("{index}.1")).unwrap(),
                format!("Stage {index}"),
                test_fixtures::phase_of(index),
                ExecutionKindView::Always,
                if index == 1 {
                    StageModeView::Mob
                } else {
                    StageModeView::Inline
                },
            )
            .with_lead_agent("orchestrator".to_string())
            .with_scopes(vec!["classic".to_string()]);
            if index == 1 {
                builder = builder
                    .with_reviewer("aidlc-product-lead-agent".to_string())
                    .with_review_class(ReviewClassView::Advisory);
            }
            builder.build()
        });
        let directive = run_with(&after_first_stage(2), &held, &input());
        let run_stage = expect_run_stage(directive);
        assert_eq!(run_stage.reviewer(), Some("aidlc-product-lead-agent"));
        assert_eq!(run_stage.review_class(), Some(ReviewClassView::Advisory));
        assert_eq!(run_stage.reviewer_max_iterations(), Some(1), "既定は 1");
        assert!(
            run_stage
                .protocol_modules()
                .contains(&"reviewer".to_string())
        );
        assert!(
            run_stage
                .protocol_modules()
                .contains(&"ensemble".to_string())
        );
        assert_eq!(
            run_stage.inline_context_paths(),
            ["agents/orchestrator.md"],
            "mob はリードのペルソナだけを読む"
        );
    }

    #[test]
    fn a_dispatched_stage_reads_no_inline_context() {
        // subagent / pipeline は完全委任 — inline_context_paths は空。construction フェーズは
        // protocol_modules に construction が載る。
        let held = definition_with(|index| {
            StageViewBuilder::new(
                slug(index),
                StageNumberView::parse(&format!("{index}.1")).unwrap(),
                format!("Stage {index}"),
                if index == 0 {
                    PhaseView::Initialization
                } else {
                    PhaseView::Construction
                },
                ExecutionKindView::Always,
                if index == 0 {
                    StageModeView::Inline
                } else {
                    StageModeView::Subagent
                },
            )
            .with_lead_agent(if index == 0 {
                "orchestrator".to_string()
            } else {
                "aidlc-developer-agent".to_string()
            })
            .with_scopes(vec!["classic".to_string()])
            .build()
        });
        let directive = run_with(&after_first_stage(2), &held, &input());
        let run_stage = expect_run_stage(directive);
        assert!(run_stage.inline_context_paths().is_empty());
        assert!(
            run_stage
                .protocol_modules()
                .contains(&"ensemble".to_string())
        );
        assert!(
            run_stage
                .protocol_modules()
                .contains(&"construction".to_string())
        );
    }

    // ---- scope 解決ラダー (ユースケース越しの通し確認) ----

    #[test]
    fn the_scope_ladder_walks_state_over_explicit_over_env_over_default() {
        let held = definition(2);
        let resolved = held
            .resolve_scope(Some("classic"), None, None, Some("bugfix"))
            .unwrap();
        assert_eq!(
            (resolved.name().as_str(), resolved.source()),
            ("classic", ScopeSource::State)
        );
        let resolved = held
            .resolve_scope(None, Some("bugfix"), None, None)
            .unwrap();
        assert_eq!(
            (resolved.name().as_str(), resolved.source()),
            ("bugfix", ScopeSource::Explicit)
        );
        let resolved = held
            .resolve_scope(None, None, Some("fix the login"), None)
            .unwrap();
        assert_eq!(
            (resolved.name().as_str(), resolved.source()),
            ("bugfix", ScopeSource::Inferred)
        );
        let resolved = held
            .resolve_scope(None, None, None, Some("bugfix"))
            .unwrap();
        assert_eq!(
            (resolved.name().as_str(), resolved.source()),
            ("bugfix", ScopeSource::Env)
        );
        let resolved = held.resolve_scope(None, None, None, None).unwrap();
        assert_eq!(
            (resolved.name().as_str(), resolved.source()),
            ("classic", ScopeSource::Default)
        );
    }

    #[test]
    fn an_empty_keyword_never_matches() {
        let scopes: BTreeMap<String, ScopeMetadataView> = [(
            "bugfix".to_string(),
            ScopeMetadataView::new("bugfix")
                .unwrap()
                .with_keywords(vec![String::new()]),
        )]
        .into_iter()
        .collect();
        let held = DefinitionView::new(
            DefinitionIdView::parse("claude").unwrap(),
            DefinitionRevisionView::parse(&format!("sha256:{}", "0".repeat(64))).unwrap(),
            StageGraphView::new(vec![
                StageViewBuilder::new(
                    slug(0),
                    StageNumberView::parse("0.1").unwrap(),
                    "Stage 0".to_string(),
                    PhaseView::Initialization,
                    ExecutionKindView::Always,
                    StageModeView::Inline,
                )
                .with_lead_agent("orchestrator".to_string())
                .with_scopes(vec!["bugfix".to_string()])
                .build(),
            ])
            .unwrap(),
            ScopeGridView::new(
                [(
                    "bugfix".to_string(),
                    [(slug(0), PlanActionView::Execute)].into_iter().collect(),
                )]
                .into_iter()
                .collect(),
            ),
            scopes,
        );
        assert_eq!(held.infer_scope_from_text("anything at all"), None);
    }

    // ---- B16: steering 連鎖 ----

    /// 2 部に分かれるルール束 (12KiB セクション × 2 → 20KiB 目標で 2 チャンク —
    /// 分割・パックは `MemoryRules::plan_for` → `SteeringPlan::pack` が行う)。
    fn two_part_rules() -> MemoryRules {
        let big = "x".repeat(12 * 1024);
        MemoryRules::new(
            vec![RuleContent::new(
                "aidlc/spaces/default/memory/org.md".to_string(),
                format!("# Org\n{big}\n# Team\n{big}\n"),
            )],
            BTreeMap::new(),
        )
    }

    #[test]
    fn a_rule_bundle_is_delivered_in_parts_then_the_run_stage_arrives() {
        let held = genesis_state(2);
        let graph = definition(2);
        let chain = || {
            ContinueUseCase::new(
                FakeDefinitionDao::holding(graph.clone()),
                FakeStateDao::holding(held.clone()),
                FakeRulesDao::holding(two_part_rules()),
            )
        };
        // 第 1 部。
        let part1 = expect_load_steering(run_with_rules(
            &held,
            &graph,
            FakeRulesDao::holding(two_part_rules()),
            &input(),
        ));
        assert_eq!(part1.part().as_u32(), 1);
        assert_eq!(part1.parts().as_u32(), 2);
        assert_eq!(part1.stage().as_str(), "stage-0");
        assert!(!part1.rules_content().is_empty());
        assert_eq!(
            part1.rules_content().first().map(RuleContent::path),
            Some("aidlc/spaces/default/memory/org.md")
        );
        // 第 2 部。
        let part2 =
            expect_load_steering(chain().execute(Some(part1.continue_token().clone()), &input()));
        assert_eq!(part2.part().as_u32(), 2);
        assert_eq!(part2.parts().as_u32(), 2);
        // 終端 — run-stage がルール台帳つきで届く。
        let run_stage =
            expect_run_stage(chain().execute(Some(part2.continue_token().clone()), &input()));
        assert_eq!(run_stage.stage().as_str(), "stage-0");
        assert_eq!(
            run_stage.rules_in_context(),
            ["aidlc/spaces/default/memory/org.md"],
            "配信済みルールのパス台帳"
        );
        assert_eq!(run_stage.gate(), GateField::Ungated, "ピンの再適用");
    }

    #[test]
    fn an_unreadable_rule_file_blocks_the_stage_verbatim() {
        let directive = run_with_rules(
            &genesis_state(2),
            &definition(2),
            unreadable_rules(),
            &input(),
        );
        assert_eq!(
            error_message(&directive),
            "Cannot load required stage rule \"aidlc/spaces/default/memory/org.md\" (permission denied). The stage has not started. Restore the file or fix its permissions/UTF-8 encoding, then run `next` again."
        );
    }

    #[test]
    fn an_unsplittable_bundle_blocks_the_stage_verbatim() {
        // パス長が輸送目標を食い尽くすと 1 コードポイントも収まらない (防御的分岐)。
        let rules = MemoryRules::new(
            vec![RuleContent::new(
                "x".repeat(20 * 1024),
                "# Org\nrule text\n".to_string(),
            )],
            BTreeMap::new(),
        );
        let directive = run_with_rules(
            &genesis_state(2),
            &definition(2),
            FakeRulesDao::holding(rules),
            &input(),
        );
        assert_eq!(
            error_message(&directive),
            "A rule section could not be split below the directive transport limit. Shorten the affected heading section, then run a fresh `next`."
        );
    }

    #[test]
    fn the_pins_survive_the_rebuild() {
        let run_stage = RunStageDirectiveBuilder::new(
            slug(1),
            PhaseView::Inception,
            "aidlc-product-agent",
            StageModeView::Inline,
            GateField::Gated,
            "stage.md",
            "memory.md",
        )
        .with_support_agents(vec!["aidlc-design-agent".to_string()])
        .with_reviewer("aidlc-product-lead-agent", ReviewClassView::Advisory, 2)
        .with_narration("note")
        .build();
        let token = ContinueTokenBuilder::new(
            slug(1),
            ScopeSlugView::parse("classic").unwrap(),
            PartIndex::FIRST,
            Bindings::new(
                BundleDigest::new("b"),
                DirectiveDigest::new("d"),
                RouteDigest::new("r"),
                Some(StateBinding::new("h")),
            ),
            GateField::Ungated,
        )
        .with_unit(UnitRef::new(
            UnitName::parse("u6").unwrap(),
            UnitKind::Library,
        ))
        .with_next_stage(StageName::parse("Stage 2").unwrap())
        .with_single()
        .build();
        let rebuilt = run_stage.with_pins(&token);
        assert_eq!(rebuilt.gate(), GateField::Ungated, "gate ピン");
        assert_eq!(rebuilt.next_stage(), Some("Stage 2"), "next_stage ピン");
        assert_eq!(
            rebuilt.unit().map(|unit| unit.name().as_str()),
            Some("u6"),
            "unit ピン"
        );
        assert!(rebuilt.is_single(), "single ピン");
        assert_eq!(rebuilt.reviewer(), Some("aidlc-product-lead-agent"));
        assert_eq!(rebuilt.reviewer_max_iterations(), Some(2));
        assert_eq!(rebuilt.narration(), Some("note"));
        assert_eq!(rebuilt.support_agents(), ["aidlc-design-agent"]);
    }

    #[test]
    fn the_run_stage_directive_reports_its_required_faces() {
        let directive = RunStageDirectiveBuilder::new(
            slug(1),
            PhaseView::Inception,
            "aidlc-product-agent",
            StageModeView::Inline,
            GateField::Gated,
            "stages/inception/stage-1.md",
            "record/inception/stage-1/memory.md",
        )
        .build();
        assert_eq!(directive.stage().as_str(), "stage-1");
        assert_eq!(directive.phase(), PhaseView::Inception);
        assert_eq!(directive.lead_agent(), "aidlc-product-agent");
        assert!(directive.support_agents().is_empty());
        assert_eq!(directive.mode(), StageModeView::Inline);
        assert_eq!(directive.gate(), GateField::Gated);
        assert_eq!(directive.stage_file(), "stages/inception/stage-1.md");
        assert_eq!(
            directive.memory_path(),
            "record/inception/stage-1/memory.md"
        );
        assert!(directive.consumes().is_empty());
        assert!(directive.produces().is_empty());
        assert!(directive.sensors_applicable().is_empty());
        assert_eq!(directive.next_stage(), None);
        assert_eq!(directive.narration(), None);
        assert!(!directive.is_single());
    }

    // ---- テスト用の定義ビルダ ----

    /// 2 ノードの定義を、ノード組み立てだけ差し替えて組む (scope は `classic` のみ)。
    fn definition_with(node: impl Fn(usize) -> StageView) -> DefinitionView {
        let nodes: Vec<StageView> = (0..2).map(node).collect();
        DefinitionView::new(
            DefinitionIdView::parse("claude").unwrap(),
            DefinitionRevisionView::parse(&format!("sha256:{}", "0".repeat(64))).unwrap(),
            StageGraphView::new(nodes).unwrap(),
            ScopeGridView::new(
                [(
                    "classic".to_string(),
                    (0..2)
                        .map(|i| (slug(i), PlanActionView::Execute))
                        .collect::<BTreeMap<_, _>>(),
                )]
                .into_iter()
                .collect(),
            ),
            [(
                "classic".to_string(),
                ScopeMetadataView::new("classic").unwrap(),
            )]
            .into_iter()
            .collect(),
        )
    }

    /// 指定した scope 名だけを有効に持つ 2 ノードの定義。
    fn definition_with_scopes(names: &[&str]) -> DefinitionView {
        let nodes: Vec<StageView> = (0..2)
            .map(|index| {
                StageViewBuilder::new(
                    slug(index),
                    StageNumberView::parse(&format!("{index}.1")).unwrap(),
                    format!("Stage {index}"),
                    test_fixtures::phase_of(index),
                    ExecutionKindView::Always,
                    StageModeView::Inline,
                )
                .with_lead_agent("orchestrator".to_string())
                .with_scopes(names.iter().map(|s| (*s).to_string()).collect())
                .build()
            })
            .collect();
        DefinitionView::new(
            DefinitionIdView::parse("claude").unwrap(),
            DefinitionRevisionView::parse(&format!("sha256:{}", "0".repeat(64))).unwrap(),
            StageGraphView::new(nodes).unwrap(),
            ScopeGridView::new(
                names
                    .iter()
                    .map(|name| {
                        (
                            (*name).to_string(),
                            (0..2)
                                .map(|i| (slug(i), PlanActionView::Execute))
                                .collect::<BTreeMap<_, _>>(),
                        )
                    })
                    .collect(),
            ),
            names
                .iter()
                .map(|name| ((*name).to_string(), ScopeMetadataView::new(name).unwrap()))
                .collect(),
        )
    }

    /// 1 ノードだけの定義 (slug を差し替えられる)。
    fn definition_with_single_node(node_slug: &str) -> DefinitionView {
        let node = StageViewBuilder::new(
            StageSlugView::parse(node_slug).unwrap(),
            StageNumberView::parse("0.1").unwrap(),
            "Other".to_string(),
            PhaseView::Initialization,
            ExecutionKindView::Always,
            StageModeView::Inline,
        )
        .with_lead_agent("orchestrator".to_string())
        .with_scopes(vec!["classic".to_string()])
        .build();
        DefinitionView::new(
            DefinitionIdView::parse("claude").unwrap(),
            DefinitionRevisionView::parse(&format!("sha256:{}", "0".repeat(64))).unwrap(),
            StageGraphView::new(vec![node]).unwrap(),
            ScopeGridView::new(
                [(
                    "classic".to_string(),
                    [(
                        StageSlugView::parse(node_slug).unwrap(),
                        PlanActionView::Execute,
                    )]
                    .into_iter()
                    .collect(),
                )]
                .into_iter()
                .collect(),
            ),
            [(
                "classic".to_string(),
                ScopeMetadataView::new("classic").unwrap(),
            )]
            .into_iter()
            .collect(),
        )
    }

    /// 行の型を直接組むテスト (ビューの構築経路そのものの確認)。
    #[test]
    fn a_stage_progress_row_is_the_unit_of_the_read_model() {
        let row = StageProgressView::new(
            slug(0),
            PhaseView::Initialization,
            CheckboxState::InProgress,
            PlanActionView::Execute,
        );
        let held = ExecutionStateView::new(
            ScopeSlugView::parse("classic").unwrap(),
            ExecutionStatus::Running,
            "stage-0",
            None,
            "t",
            vec![row],
        )
        .unwrap();
        assert_eq!(held.stage_count(), 1);
    }
}
