//! `Next` — `next` 1 回の 21 分岐ラダー (フロー制御のみ — FR3.1 / FR3.3)。
//!
//! 状態判断はドメイン ([`IntentExecution::next_decision`]) が持ち、本ユースケースは
//! 観測 ([`NextTurnInput`]) と読取専用ポートを畳んで directive ちょうど 1 つに写す。
//! **書込ポートは注入しない** — I8 (`next` は読み取り専用) は「find 系のみの読取専用
//! ポート注入」で型強制する (use-case-rules §2b — 旧 I8 機構は失効、2026-08-30 正典)。
//!
//! ラダーの分岐順・逐語文言の正本は契約マップ
//! `docs/specs/research/orchestration-next-ladder.md` §1。コマンドの**概念**はドメインの
//! [`EngineCommand`]、**綴り**は注入ポート [`CommandSpelling`] のアダプタ実装が持つ
//! (逸脱台帳 #1 の写像点)。scope 解決ラダー・キーワード推論はドメイン
//! (`scope_resolution`) の判断ポリシーである。
//!
//! [`IntentExecution::next_decision`]: core_command_domain::orchestration::IntentExecution::next_decision

use core_command_domain::orchestration::{
    AskDirective, AskKind, Bindings, ConfigField, ContinueTokenBuilder, Directive, EngineCommand,
    GateField, Intent, IntentExecution, LoadSteeringDirective, NextDecision, NextRequest,
    RunStageDirective, RunStageDirectiveBuilder, ScopeResolutionError, StageIndex, StageName,
    StateBinding, SteeringPart, resolve_scope,
};
use core_command_domain::workflow_definition::{
    PhaseId, PlanAction, ScopeSlug, StageMode, StageNode, WorkflowDefinition,
};

use super::next_turn_input::{NextTurnInput, WorkspaceLayout};
use super::port::CommandSpelling;
use super::port::ContinueTokenCodec;
use super::port::IntentExecutionRepository;
use super::port::IntentRepository;
use super::port::WorkflowDefinitionRepository;
use super::port::{RuleBundleReadError, RuleBundleSource};

/// 逐語文言 — ラダーが放出する公開契約の文字列 (出典: 契約マップ §1。コマンド参照は写像形)。
mod wording {

    use core_command_domain::workspace::CheckboxState;

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

    /// 分岐 1。
    pub(super) fn read_only(spelled: &str) -> String {
        format!(
            "Run `{spelled}`. This is a read-only utility, NOT workflow work: do NOT run `next` for it."
        )
    }

    /// 分岐 6。
    pub(super) fn resume_menu(stage: &str) -> String {
        format!(
            "An existing workflow was found (currently at \"{stage}\"). How would you like to proceed? Resume from last checkpoint, redo the current stage, jump to a stage, or start fresh."
        )
    }

    /// 分岐 10 手順 3 (回復可能な plan/cursor 不整合)。
    pub(super) fn recover_skip(stage: &str) -> String {
        format!(
            "Run `aidlc-orchestrate report --stage {stage} --result skipped --reason \"stage is SKIP in the approved workflow plan\"`, then re-run `next`."
        )
    }

    /// 分岐 10 手順 3 (回復経路のない plan/cursor 不整合)。
    pub(super) fn inconsistent_skip(stage: &str, checkbox: CheckboxState) -> String {
        format!(
            "Stage \"{stage}\" is SKIP in the approved workflow plan but its active cursor state is \"{}\". Refusing to emit run-stage; repair the inconsistent state before continuing.",
            checkbox_word(checkbox)
        )
    }

    /// 分岐 10 手順 5。
    pub(super) fn workflow_complete(stage: &str, scope: &str) -> String {
        format!("Workflow complete — no in-scope stage remains after {stage} (scope: {scope}).")
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

/// `next` の 21 分岐ラダー (読取専用 — 書込ポートを持たない)。
#[derive(Debug)]
pub struct NextUseCase<E, I, D, B, C, S> {
    execution_repository: E,
    intent_repository: I,
    definition_repository: D,
    bundle_source: B,
    codec: C,
    spelling: S,
}

/// 稼働中ワークフローの読取済みコンテキスト。
///
/// 楽観 version は集約が運んでいるので、ここに別枠で持たない (オーナー裁定 2026-08-30)。
struct LoadedWorkflow {
    intent: Intent,
    execution: IntentExecution,
}

impl<E, I, D, B, C, S> NextUseCase<E, I, D, B, C, S>
where
    E: IntentExecutionRepository,
    I: IntentRepository,
    D: WorkflowDefinitionRepository,
    B: RuleBundleSource,
    C: ContinueTokenCodec,
    S: CommandSpelling,
{
    /// 読取専用ポート 6 本を注入する (find / load / mint / spell 系のみ — 書込動詞は
    /// 呼ばない。codec の鍵鋳造はマシンローカルで I8 の例外 1)。
    #[must_use]
    pub const fn new(
        execution_repository: E,
        intent_repository: I,
        definition_repository: D,
        bundle_source: B,
        codec: C,
        spelling: S,
    ) -> NextUseCase<E, I, D, B, C, S> {
        NextUseCase {
            execution_repository,
            intent_repository,
            definition_repository,
            bundle_source,
            codec,
            spelling,
        }
    }

    /// 観測 1 回を directive ちょうど 1 つに写す。失敗も `Directive::Error` で返す —
    /// エンジンの契約は「stdout に directive ちょうど 1 つ」である (§3.2)。
    pub async fn execute(&self, input: &NextTurnInput) -> Directive {
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
                message: wording::read_only(
                    &self.spelling.spell(&EngineCommand::ReadOnlyUtility(verb)),
                ),
            };
        }
        // ---- 分岐 1b/1c/1d: 名詞トークン (先頭トークン意味論のみ) ----
        if let Some(token) = input.noun_token() {
            return Directive::Print {
                message: format!(
                    "Run `{}`.",
                    self.spelling
                        .spell(&EngineCommand::NounTokens(token.tokens().to_vec()))
                ),
            };
        }
        // ---- 分岐 2: --stage と --phase の併用 ----
        if input.stage().is_some() && input.phase().is_some() {
            return Directive::Error {
                message: wording::STAGE_AND_PHASE.to_string(),
            };
        }
        // ---- state の読取 (state バージョンガードの相当: 読取失敗は復元前に逐語で止める) ----
        let context = match self.load(input).await {
            Ok(context) => context,
            Err(message) => return Directive::Error { message },
        };
        // ---- 定義の読取 (state あり: intent がピンした定義。無し: ハーネスの定義) ----
        let definition = match self.load_definition(input, context.as_ref()) {
            Ok(definition) => definition,
            Err(message) => return Directive::Error { message },
        };
        // ---- 分岐 2.5 / 2.6: park (判断はドメイン — reentry フラグは NextRequest に畳む) ----
        let request = NextRequest::new(
            input.is_resume(),
            input.stage().is_some()
                || input.phase().is_some()
                || input.review().is_some()
                || input.new_intent().is_some(),
            input.freeform().is_some(),
        );
        let decision = context.as_ref().map(|context| {
            context
                .execution
                .next_decision(&context.intent, &definition, &request)
        });
        if let Some(Ok(NextDecision::Parked { stage })) = &decision
            && let Some(context) = context.as_ref()
        {
            let slug = stage_entry_slug(context, *stage);
            return Directive::Parked {
                stage: slug.clone(),
                message: wording::parked(slug.as_str()),
            };
        }
        if let Some(Ok(NextDecision::UnparkThenResume)) = &decision {
            return Directive::Print {
                message: wording::unpark_then_resume(&self.spelling.spell(&EngineCommand::Unpark)),
            };
        }
        // ---- 分岐 3b / 4 / 解決不能: scope 解決ラダー ----
        let state_scope = context.as_ref().map(|c| c.intent.scope().to_string());
        let resolved = match resolve_scope(
            &definition,
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
                    self.spelling.spell(&EngineCommand::DispatchComposer)
                ),
            };
        }
        // ---- 分岐 4a: --new-intent (scope は明示 --scope のみ — ラダーを使わない) ----
        if let Some(description) = input.new_intent() {
            if description.trim().is_empty() {
                return Directive::Error {
                    message:
                        "The --new-intent description must not be blank. Describe the new work."
                            .to_string(),
                };
            }
            let Some(scope) = input.scope() else {
                return Directive::Error {
                    message: "--new-intent requires an explicit --scope <name>.".to_string(),
                };
            };
            let Ok(scope) = ScopeSlug::parse(scope) else {
                // ラダーが既に membership 検証済み — 文法違反はここへ届かない (防御的)。
                return Directive::Error {
                    message: wording::unknown_scope(scope, &definition.valid_scopes()),
                };
            };
            return Directive::Print {
                message: format!(
                    "Run `{}`, then hand off to a fresh session.",
                    self.spelling.spell(&EngineCommand::MintIntent { scope })
                ),
            };
        }
        // ---- 分岐 4b: --single (scope-change / jump より前) ----
        if input.is_single() {
            return self.emit_single(input, &definition, resolved.name());
        }
        // ---- 分岐 5: state あり + 有効で異なる設定 ----
        if let Some(context) = context.as_ref() {
            if let Some(scope) = input.scope()
                && scope != context.intent.scope()
            {
                let Ok(scope) = ScopeSlug::parse(scope) else {
                    // ラダーが既に membership 検証済み — 文法違反はここへ届かない (防御的)。
                    return Directive::Error {
                        message: wording::unknown_scope(scope, &definition.valid_scopes()),
                    };
                };
                return Directive::Print {
                    message: format!(
                        "Run `{}`.",
                        self.spelling.spell(&EngineCommand::ChangeScope { scope })
                    ),
                };
            }
            if let Some(depth) = input.depth() {
                return Directive::Print {
                    message: format!(
                        "Run `{}`.",
                        self.spelling.spell(&EngineCommand::ChangeConfig {
                            field: ConfigField::Depth,
                            value: depth.to_string(),
                        })
                    ),
                };
            }
            if let Some(level) = input.test_strategy() {
                return Directive::Print {
                    message: format!(
                        "Run `{}`.",
                        self.spelling.spell(&EngineCommand::ChangeConfig {
                            field: ConfigField::TestStrategy,
                            value: level.to_string(),
                        })
                    ),
                };
            }
            if let Some(class) = input.review() {
                return Directive::Print {
                    message: format!(
                        "Run `{}`.",
                        self.spelling.spell(&EngineCommand::ChangeConfig {
                            field: ConfigField::Review,
                            value: class.to_string(),
                        })
                    ),
                };
            }
        }
        // ---- 分岐 6: state ありでの --resume ----
        if let Some(Ok(NextDecision::ResumeMenu)) = &decision {
            let stage = context
                .as_ref()
                .map_or_else(String::new, |c| current_stage_slug(c).as_str().to_string());
            return Directive::Ask(AskDirective::new(
                AskKind::ResumeMenu,
                wording::resume_menu(&stage),
            ));
        }
        // ---- 分岐 7: --stage / --phase (jump) ----
        if input.stage().is_some() || input.phase().is_some() {
            return self.emit_jump(input, context.as_ref(), &definition, resolved.name());
        }
        // ---- state なしの群: 7b / 8 / 9a / 9b ----
        if context.is_none() {
            return emit_birth_group(input, &definition, &self.spelling);
        }
        // ---- 分岐 9c: 稼働中の自由記述 ----
        if let Some(Ok(NextDecision::NewWorkRouting)) = &decision {
            let description = input.freeform().unwrap_or_default().to_string();
            let proposed = resolve_scope(&definition, None, None, Some(&description), None)
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
        // ---- 分岐 10: ハッピーパス (判断はドメインの next_decision) ----
        let (Some(context_value), Some(decision)) = (context, decision) else {
            // state なしはここへ到達しない (上の群で返している) — 防御的。
            return Directive::Error {
                message: wording::NO_STATE.to_string(),
            };
        };
        let decision = match decision {
            Ok(decision) => decision,
            Err(error) => {
                return Directive::Error {
                    message: error.to_string(),
                };
            }
        };
        self.emit_happy_path(
            input,
            &context_value,
            &definition,
            resolved.name(),
            decision,
        )
    }

    /// run-stage を steering 連鎖経由で届ける — ルール束が空なら bare run-stage、あれば
    /// 第 1 部の `load-steering` + continue_token (02 §10)。
    fn deliver(
        &self,
        definition: &WorkflowDefinition,
        scope: &ScopeSlug,
        node: &StageNode,
        run_stage: &RunStageDirective,
        state: Option<StateBinding>,
    ) -> Directive {
        let plan = match self.bundle_source.load(node.phase()) {
            Ok(plan) => plan,
            Err(RuleBundleReadError::Unreadable { path, cause }) => {
                return Directive::Error {
                    message: wording::rule_unreadable(&path, &cause),
                };
            }
            Err(RuleBundleReadError::Unsplittable { .. }) => {
                return Directive::Error {
                    message: wording::UNSPLITTABLE_SECTION.to_string(),
                };
            }
        };
        let Some(first) = plan.first_part() else {
            // 空計画 — bare run-stage (台帳は空)。
            return Directive::RunStage(run_stage.with_rules_in_context(plan.delivered_paths()));
        };
        let bindings = Bindings::new(
            self.codec.bundle_digest(&plan),
            self.codec.directive_digest(run_stage),
            self.codec
                .route_digest(&definition.stage_route(scope.as_str(), node)),
            state,
        );
        emit_part(&self.codec, &first, run_stage, scope, &bindings)
    }

    /// state 束縛のダイジェスト (state ありのときだけ)。
    fn state_binding(&self, context: Option<&LoadedWorkflow>) -> Option<StateBinding> {
        context.map(|context| self.codec.state_binding(&context.execution))
    }

    /// active-intent カーソルが指す集約群を読む。読取失敗は逐語メッセージで返す (材料は
    /// `RepositoryError` の Display — state バージョンガードの相当)。
    async fn load(&self, input: &NextTurnInput) -> Result<Option<LoadedWorkflow>, String> {
        let Some(active) = input.active() else {
            return Ok(None);
        };
        let execution = self
            .execution_repository
            .find_by_id(active.execution_id())
            .await
            .map_err(|error| error.to_string())?;
        let intent = self
            .intent_repository
            .find_by_id(active.intent_id())
            .await
            .map_err(|error| error.to_string())?;
        Ok(Some(LoadedWorkflow { intent, execution }))
    }

    /// 定義を読む — state ありなら intent がピンした定義 id、無しならハーネスの定義 id。
    fn load_definition(
        &self,
        input: &NextTurnInput,
        context: Option<&LoadedWorkflow>,
    ) -> Result<WorkflowDefinition, String> {
        let id = match context {
            Some(context) => context.intent.definition_id().clone(),
            None => input
                .definition_id()
                .cloned()
                .ok_or_else(|| "No workflow definition id was provided.".to_string())?,
        };
        self.definition_repository
            .find_by_id(&id)
            .map_err(|error| format!("{error:?}"))
    }

    /// 分岐 4b — 単一ステージ隔離モード。
    fn emit_single(
        &self,
        input: &NextTurnInput,
        definition: &WorkflowDefinition,
        scope: &ScopeSlug,
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
                message: format!("Unknown stage \"{stage}\"."),
            },
        }
    }

    /// 分岐 7 — jump。state ありなら純読み取り解決の名指し、無しなら直接グラフ検索。
    fn emit_jump(
        &self,
        input: &NextTurnInput,
        context: Option<&LoadedWorkflow>,
        definition: &WorkflowDefinition,
        scope: &ScopeSlug,
    ) -> Directive {
        let target = match (input.stage(), input.phase()) {
            (Some(stage), _) => match find_node(definition, stage) {
                Some(node) => node,
                None => {
                    return Directive::Error {
                        message: format!("Unknown stage \"{stage}\"."),
                    };
                }
            },
            (None, Some(phase)) => {
                let Ok(phase) = PhaseId::parse(phase) else {
                    return Directive::Error {
                        message: format!("Unknown phase \"{phase}\"."),
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
        if target.phase() == PhaseId::Initialization {
            return Directive::Error {
                message: wording::INIT_JUMP.to_string(),
            };
        }
        if context.is_some() {
            return Directive::Print {
                message: format!(
                    "Run `{}`.",
                    self.spelling.spell(&EngineCommand::ResolveJump {
                        stage: target.slug().clone(),
                    })
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
}

/// state なしの群 (7b / 8 / 9a / 9b)。
fn emit_birth_group<S: CommandSpelling>(
    input: &NextTurnInput,
    definition: &WorkflowDefinition,
    spelling: &S,
) -> Directive {
    // 分岐 9a: 明示 --scope (membership はラダーが検証済み)。
    if let Some(scope) = input.scope() {
        return mint_intent_print(definition, spelling, scope);
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
            return mint_intent_print(definition, spelling, text.trim());
        }
        // 分岐 8: キーワードヒット → scope 確認 / 非ヒット → compose 提案。
        if let Some(scope) =
            core_command_domain::orchestration::infer_scope_from_text(definition, text)
        {
            return Directive::Ask(AskDirective::new(
                AskKind::ScopeConfirm,
                format!(
                    "This looks like \"{scope}\" work. Start a {scope} workflow for it? Larger scopes run more stages and cost more."
                ),
            ));
        }
        return Directive::Ask(AskDirective::new(
            AskKind::ComposeOffer,
            "No stock scope matched. Compose a tailored plan for this task?".to_string(),
        ));
    }
    // 分岐 9b: 何も名指しされていない。
    Directive::Error {
        message: wording::NO_STATE.to_string(),
    }
}

/// intent 鋳造の名指し (分岐 7b / 9a) — scope は membership 検証済みの綴りを型に上げる。
fn mint_intent_print<S: CommandSpelling>(
    definition: &WorkflowDefinition,
    spelling: &S,
    scope: &str,
) -> Directive {
    match ScopeSlug::parse(scope) {
        Ok(scope) => Directive::Print {
            message: format!(
                "Run `{}`.",
                spelling.spell(&EngineCommand::MintIntent { scope })
            ),
        },
        // membership 検証済みなので文法違反はここへ届かない (防御的)。
        Err(_) => Directive::Error {
            message: wording::unknown_scope(scope, &definition.valid_scopes()),
        },
    }
}

/// 分岐 10 — ハッピーパス。判断 (`NextDecision`) を directive に写すだけ。
impl<E, I, D, B, C, S> NextUseCase<E, I, D, B, C, S>
where
    E: IntentExecutionRepository,
    I: IntentRepository,
    D: WorkflowDefinitionRepository,
    B: RuleBundleSource,
    C: ContinueTokenCodec,
    S: CommandSpelling,
{
    fn emit_happy_path(
        &self,
        input: &NextTurnInput,
        context: &LoadedWorkflow,
        definition: &WorkflowDefinition,
        scope: &ScopeSlug,
        decision: NextDecision,
    ) -> Directive {
        match decision {
            NextDecision::RunStage { stage, gate } => {
                let slug = stage_entry_slug(context, stage).clone();
                // ゲート判定はドメイン (BR1.3) が正 — 定義側の既定は使わない。
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
                            Ok(Directive::RunStage(run_stage)) => {
                                let binding = self.state_binding(Some(context));
                                self.deliver(definition, scope, node, &run_stage, binding)
                            }
                            Ok(directive) => directive,
                            Err(message) => Directive::Error { message },
                        }
                    }
                    None => Directive::Error {
                        message: format!("Unknown stage \"{}\".", slug.as_str()),
                    },
                }
            }
            NextDecision::Done => done_with_reason(context, scope.as_str()),
            NextDecision::RecoverSkipInconsistency { stage, .. } => {
                let slug = stage_entry_slug(context, stage);
                Directive::Print {
                    message: wording::recover_skip(slug.as_str()),
                }
            }
            NextDecision::InconsistentSkip { stage, checkbox } => {
                let slug = stage_entry_slug(context, stage);
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

/// 完了 reason つきの `done` (分岐 10 手順 5) — 集約の Done は最終ステージ通過後に出るため、
/// reason は呼出側 (本関数) が現在ステージと scope から組む。
fn done_with_reason(context: &LoadedWorkflow, scope: &str) -> Directive {
    let slug = current_stage_slug(context);
    Directive::Done {
        reason: Some(wording::workflow_complete(slug.as_str(), scope)),
    }
}

/// 現在カーソルのステージ slug。
fn current_stage_slug(
    context: &LoadedWorkflow,
) -> &core_command_domain::workflow_definition::StageSlug {
    stage_entry_slug(context, context.execution.cursor())
}

/// 索引 → slug (計画は intent の持ち物)。索引は集約不変条件で範囲内だが、添字 panic は
/// 使わない — 範囲外は先頭へ畳む (防御的。ここへ来る索引は `next_decision` が発行する)。
#[allow(
    clippy::indexing_slicing,
    reason = "Intent の不変条件 (空計画は構成不能) により先頭要素は必ず存在する"
)]
fn stage_entry_slug(
    context: &LoadedWorkflow,
    stage: StageIndex,
) -> &core_command_domain::workflow_definition::StageSlug {
    let stages = context.intent.stages();
    stages
        .get(stage.to_usize())
        .unwrap_or_else(|| &stages[0])
        .slug()
}

/// 定義側の既定ゲート (初期化のみ非ゲート — BR1.3 の静的既定)。
const fn default_gate(node: &StageNode) -> GateField {
    if matches!(node.phase(), PhaseId::Initialization) {
        GateField::Ungated
    } else {
        GateField::Gated
    }
}

/// slug からグラフノードを引く。
fn find_node<'a>(definition: &'a WorkflowDefinition, slug: &str) -> Option<&'a StageNode> {
    definition
        .graph()
        .nodes()
        .iter()
        .find(|node| node.slug().as_str() == slug)
}

/// `run-stage` の組み立て (StageNode + 配置 VO)。steering 由来フィールドは B16。
pub(crate) fn build_run_stage(
    node: &StageNode,
    definition: &WorkflowDefinition,
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
        StageMode::Inline => {
            let mut paths = vec![format!("{}/{}.md", layout.agent_dir(), node.lead_agent())];
            paths.extend(
                node.support_agents()
                    .iter()
                    .map(|agent| format!("{}/{agent}.md", layout.agent_dir())),
            );
            paths
        }
        StageMode::Mob => vec![format!("{}/{}.md", layout.agent_dir(), node.lead_agent())],
        StageMode::Subagent | StageMode::Pipeline | StageMode::AgentTeam => Vec::new(),
    };
    let mut protocol_modules = Vec::new();
    if node.reviewer().is_some() {
        protocol_modules.push("reviewer".to_string());
    }
    if node.mode() != StageMode::Inline || !node.support_agents().is_empty() {
        protocol_modules.push("ensemble".to_string());
    }
    if node.phase() == PhaseId::Construction {
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

/// 連鎖 1 部の発出 — 計画上の部と、次を指すトークンを封緘する。
///
/// 部は [`SteeringPart`] (計画のクエリのみが構築 — 範囲外は表現不能)、束縛は [`Bindings`]
/// で受けるので、旧 `emit_part` の「範囲外 part の内部エラー」と裸ダイジェスト 4 本・
/// センチネル文字列は存在しない。
pub(crate) fn emit_part<C: ContinueTokenCodec>(
    codec: &C,
    part: &SteeringPart<'_>,
    run_stage: &RunStageDirective,
    scope: &ScopeSlug,
    bindings: &Bindings,
) -> Directive {
    let mut builder = ContinueTokenBuilder::new(
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
    let token = builder.build();
    Directive::LoadSteering(LoadSteeringDirective::new(
        run_stage.stage().clone(),
        bindings.bundle().clone(),
        part,
        codec.mint(&token),
    ))
}

/// 現ノードの後で最初の in-scope EXECUTE ステージの表示名。
fn next_in_scope_name(
    definition: &WorkflowDefinition,
    scope: &str,
    node: &StageNode,
) -> Option<String> {
    let stages = definition.stages_in_scope(scope);
    let position = stages
        .iter()
        .position(|(slug, _, _)| slug.as_str() == node.slug().as_str())?;
    stages
        .iter()
        .skip(position + 1)
        .find(|(_, _, action)| *action == Some(PlanAction::Execute))
        .and_then(|(slug, _, _)| find_node(definition, slug.as_str()))
        .map(|next| next.name().to_string())
}

#[cfg(test)]
mod tests {
    // panic! は想定外バリアントの即時失敗という検証用途で使っており、テスト失敗のシグナル
    // として妥当なため許容する (集約のテストモジュールと同じ作法)。
    #![allow(clippy::panic)]

    use core_command_domain::workflow_definition::{
        ExecutionKind, ScopeGrid, ScopeMetadata, StageGraph, StageMode, StageNodeBuilder,
        StageNumber, StageSlug, WorkflowDefinition, WorkflowDefinitionId,
    };
    use std::collections::BTreeMap;

    use super::super::continue_use_case::ContinueUseCase;
    use super::super::next_turn_input::{
        ActiveWorkflow, NextTurnInput, NounFamily, NounToken, WorkspaceLayout,
    };
    use super::super::port::{CommandSpelling, ContinueTokenCodec, InvalidContinueToken};
    use super::super::port::{GraphReadError, WorkflowDefinitionRepository};
    use super::super::port::{RuleBundleReadError, RuleBundleSource};
    use super::super::test_support::{
        InMemoryIntentExecutionRepository, InMemoryIntentRepository, at, execution_id, genesis,
        intent as intent_id, slug,
    };
    use super::*;
    use core_command_domain::orchestration::{
        BundleDigest, DirectiveDigest, PartIndex, ReadOnlyVerb, RouteDigest, StateBinding,
        SteeringPlan, UnitKind, UnitName, UnitRef,
    };
    use core_command_domain::orchestration::{ContinueToken, ContinueTokenBuilder, RuleContent};
    use core_command_domain::workflow_definition::ScopeSlug;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use std::rc::Rc;

    /// ルール束なしのダブル (bare run-stage 経路)。
    #[derive(Debug, Default)]
    struct NoRules;

    impl RuleBundleSource for NoRules {
        fn load(&self, _phase: PhaseId) -> Result<SteeringPlan, RuleBundleReadError> {
            Ok(SteeringPlan::new(Vec::new()))
        }
    }

    /// 固定の配信計画を返すダブル (分割はアダプタの知識なので、テストは計画を直接組む)。
    #[derive(Debug, Clone, Default)]
    struct StaticRules {
        chunks: Vec<Vec<RuleContent>>,
    }

    impl RuleBundleSource for StaticRules {
        fn load(&self, _phase: PhaseId) -> Result<SteeringPlan, RuleBundleReadError> {
            Ok(SteeringPlan::new(self.chunks.clone()))
        }
    }

    /// 読取に失敗するダブル。
    #[derive(Debug, Default)]
    struct BrokenRules;

    impl RuleBundleSource for BrokenRules {
        fn load(&self, _phase: PhaseId) -> Result<SteeringPlan, RuleBundleReadError> {
            Err(RuleBundleReadError::Unreadable {
                path: "aidlc/spaces/default/memory/org.md".to_string(),
                cause: "permission denied".to_string(),
            })
        }
    }

    /// 決定論ハッシュ (テストダブル用 — 等値性だけが契約)。
    fn hashed(material: &str) -> String {
        let mut hasher = DefaultHasher::new();
        material.hash(&mut hasher);
        format!("{:016x}", hasher.finish())
    }

    /// 封緘の共有ストアを持つ codec ダブル (clone は同じストアを指す)。
    #[derive(Debug, Clone, Default)]
    struct FakeCodec {
        minted: Rc<RefCell<HashMap<String, ContinueToken>>>,
    }

    impl ContinueTokenCodec for FakeCodec {
        fn mint(&self, token: &ContinueToken) -> String {
            let mut minted = self.minted.borrow_mut();
            let key = format!("token-{}", minted.len() + 1);
            minted.insert(key.clone(), token.clone());
            key
        }

        fn verify(&self, encoded: &str) -> Result<ContinueToken, InvalidContinueToken> {
            self.minted
                .borrow()
                .get(encoded)
                .cloned()
                .ok_or(InvalidContinueToken)
        }

        fn bundle_digest(&self, plan: &SteeringPlan) -> BundleDigest {
            let material: String = plan
                .chunks()
                .iter()
                .flatten()
                .map(|piece| format!("{}\n{}\n", piece.path(), piece.text()))
                .collect();
            BundleDigest::new(hashed(&material))
        }

        fn directive_digest(&self, run_stage: &RunStageDirective) -> DirectiveDigest {
            let gate = match run_stage.gate() {
                GateField::Gated => "gated",
                GateField::Ungated => "ungated",
                GateField::Unresolved => "unresolved",
            };
            let unit = run_stage
                .unit()
                .map(|unit| format!("{}/{}", unit.name().as_str(), unit.kind().as_str()))
                .unwrap_or_default();
            DirectiveDigest::new(hashed(&format!(
                "{}\n{gate}\n{}\n{}\n{}\n{unit}\n{}",
                run_stage.stage().as_str(),
                run_stage.stage_file(),
                run_stage.memory_path(),
                run_stage.next_stage().unwrap_or_default(),
                run_stage.is_single(),
            )))
        }

        fn route_digest(
            &self,
            route: &core_command_domain::workflow_definition::StageRoute,
        ) -> RouteDigest {
            let stages: Vec<&str> = route
                .stages_in_scope()
                .iter()
                .map(StageSlug::as_str)
                .collect();
            RouteDigest::new(hashed(&format!(
                "{}\n{}",
                route.stage().as_str(),
                stages.join("\n")
            )))
        }

        fn state_binding(&self, execution: &IntentExecution) -> StateBinding {
            StateBinding::new(hashed(&format!(
                "{}\n{}\n{}",
                execution.intent_id().as_str(),
                execution.seq_nr(),
                execution.version()
            )))
        }
    }

    /// マルチコール綴りのダブル (アダプタ実装と同じ綴り — 逐語アサーションの前提)。
    #[derive(Debug, Clone, Copy, Default)]
    struct FakeSpelling;

    impl CommandSpelling for FakeSpelling {
        fn spell(&self, command: &core_command_domain::orchestration::EngineCommand) -> String {
            use core_command_domain::orchestration::EngineCommand as Cmd;
            match command {
                Cmd::ReadOnlyUtility(verb) => {
                    let sub = match verb {
                        ReadOnlyVerb::Status => "status",
                        ReadOnlyVerb::Help => "help",
                        ReadOnlyVerb::Doctor => "doctor",
                        ReadOnlyVerb::Version => "version",
                    };
                    format!("aidlc-utility {sub}")
                }
                Cmd::NounTokens(tokens) => format!("aidlc-utility {}", tokens.join(" ")),
                Cmd::Unpark => "aidlc-state unpark".to_string(),
                Cmd::ResolveJump { stage } => {
                    format!("aidlc-jump resolve --stage {}", stage.as_str())
                }
                Cmd::MintIntent { scope } => format!(
                    "aidlc-utility intent-create --scope {} --label \"<2-3 word kebab essence>\"",
                    scope.as_str()
                ),
                Cmd::ChangeScope { scope } => {
                    format!("aidlc-utility scope-change --scope {}", scope.as_str())
                }
                Cmd::ChangeConfig { field, value } => {
                    use core_command_domain::orchestration::ConfigField;
                    let flag = match field {
                        ConfigField::Depth => "depth",
                        ConfigField::TestStrategy => "test-strategy",
                        ConfigField::Review => "review",
                    };
                    format!("aidlc-utility config-change --{flag} {value}")
                }
                Cmd::DispatchComposer => "aidlc-composer detect".to_string(),
            }
        }
    }

    /// 定義フィクスチャを保持する読取専用ダブル。
    #[derive(Debug)]
    struct InMemoryWorkflowDefinitionRepository {
        held: WorkflowDefinition,
    }

    impl WorkflowDefinitionRepository for InMemoryWorkflowDefinitionRepository {
        fn find_by_id(
            &self,
            _id: &WorkflowDefinitionId,
        ) -> Result<WorkflowDefinition, GraphReadError> {
            Ok(self.held.clone())
        }
    }

    /// genesis の合成計画 (索引 0 = initialization、以降 = inception) に一致する定義。
    /// scope は `classic` (推論キーワードなし) と `bugfix` (キーワード `fix`)。
    fn definition(stage_count: usize) -> WorkflowDefinition {
        let nodes = (0..stage_count)
            .map(|index| {
                let phase = if index == 0 {
                    PhaseId::Initialization
                } else {
                    PhaseId::Inception
                };
                StageNodeBuilder::new(
                    slug(index),
                    StageNumber::parse(&format!("{index}.1")).unwrap(),
                    format!("Stage {index}"),
                    phase,
                    ExecutionKind::Always,
                    StageMode::Inline,
                )
                .lead_agent("orchestrator".to_string())
                .scopes(vec!["classic".to_string(), "bugfix".to_string()])
                .build()
            })
            .collect::<Vec<_>>();
        let entries = |_: ()| {
            (0..stage_count)
                .map(|index| (slug(index), PlanAction::Execute))
                .collect::<BTreeMap<_, _>>()
        };
        let grid = ScopeGrid::new(
            [
                ("classic".to_string(), entries(())),
                ("bugfix".to_string(), entries(())),
            ]
            .into_iter()
            .collect(),
        );
        let scopes: BTreeMap<String, ScopeMetadata> = [
            (
                "classic".to_string(),
                ScopeMetadata::new("classic").unwrap(),
            ),
            (
                "bugfix".to_string(),
                ScopeMetadata::new("bugfix")
                    .unwrap()
                    .with_keywords(vec!["fix".to_string()]),
            ),
        ]
        .into_iter()
        .collect();
        WorkflowDefinition::from_artifacts(
            WorkflowDefinitionId::parse("claude").unwrap(),
            core_command_domain::workflow_definition::DefinitionRevision::parse(&format!(
                "sha256:{}",
                "0".repeat(64)
            ))
            .unwrap(),
            StageGraph::new(nodes).unwrap(),
            grid,
            scopes,
        )
    }

    fn layout() -> WorkspaceLayout {
        WorkspaceLayout::new(
            "record".to_string(),
            "stages".to_string(),
            "agents".to_string(),
        )
    }

    fn active() -> ActiveWorkflow {
        ActiveWorkflow::new(intent_id(), execution_id())
    }

    /// state ありの入力の共通形。
    fn active_input() -> NextTurnInput {
        NextTurnInput::new()
            .with_active(active())
            .with_layout(layout())
    }

    /// state なしの入力の共通形。
    fn stateless_input() -> NextTurnInput {
        NextTurnInput::new()
            .with_layout(layout())
            .with_definition_id(WorkflowDefinitionId::parse("claude").unwrap())
    }

    /// ユースケースを組む (state あり)。
    fn with_workflow(
        stage_count: usize,
    ) -> NextUseCase<
        InMemoryIntentExecutionRepository,
        InMemoryIntentRepository,
        InMemoryWorkflowDefinitionRepository,
        NoRules,
        FakeCodec,
        FakeSpelling,
    > {
        let (intent, execution, _) = genesis(stage_count);
        NextUseCase::new(
            InMemoryIntentExecutionRepository::holding(execution, 1),
            InMemoryIntentRepository::holding(intent),
            InMemoryWorkflowDefinitionRepository {
                held: definition(stage_count),
            },
            NoRules,
            FakeCodec::default(),
            FakeSpelling,
        )
    }

    /// ユースケースを組む (集約を差し替えて)。
    fn with_execution(
        stage_count: usize,
        execution: IntentExecution,
    ) -> NextUseCase<
        InMemoryIntentExecutionRepository,
        InMemoryIntentRepository,
        InMemoryWorkflowDefinitionRepository,
        NoRules,
        FakeCodec,
        FakeSpelling,
    > {
        let (intent, _, _) = genesis(stage_count);
        NextUseCase::new(
            InMemoryIntentExecutionRepository::holding(execution, 1),
            InMemoryIntentRepository::holding(intent),
            InMemoryWorkflowDefinitionRepository {
                held: definition(stage_count),
            },
            NoRules,
            FakeCodec::default(),
            FakeSpelling,
        )
    }

    /// ユースケースを組む (state なし)。
    fn without_workflow(
        stage_count: usize,
    ) -> NextUseCase<
        InMemoryIntentExecutionRepository,
        InMemoryIntentRepository,
        InMemoryWorkflowDefinitionRepository,
        NoRules,
        FakeCodec,
        FakeSpelling,
    > {
        NextUseCase::new(
            InMemoryIntentExecutionRepository::empty(),
            InMemoryIntentRepository::empty(),
            InMemoryWorkflowDefinitionRepository {
                held: definition(stage_count),
            },
            NoRules,
            FakeCodec::default(),
            FakeSpelling,
        )
    }

    fn expect_load_steering(
        directive: Directive,
    ) -> core_command_domain::orchestration::LoadSteeringDirective {
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

    // ---- 前置ガード ----

    #[tokio::test]
    async fn a_parse_error_is_relayed_verbatim() {
        let directive = without_workflow(2)
            .execute(
                &stateless_input()
                    .with_parse_error("--review requires <adversarial|advisory|none>."),
            )
            .await;
        assert_eq!(
            error_message(&directive),
            "--review requires <adversarial|advisory|none>."
        );
    }

    #[tokio::test]
    async fn review_combined_with_another_mode_is_refused() {
        let directive = without_workflow(2)
            .execute(&stateless_input().with_review("advisory").with_resume())
            .await;
        assert_eq!(
            error_message(&directive),
            "Cannot combine --review with read-only, workspace, compose, single-stage, jump, or resume modes. Apply /aidlc --review <class> first, then run the other command."
        );
    }

    // ---- 分岐 0 / 1 / 1b ----

    #[tokio::test]
    async fn branch_0_the_kiro_latch_ends_the_bare_next() {
        let directive = without_workflow(2)
            .execute(&stateless_input().with_kiro_latch_bare_next())
            .await;
        assert_eq!(directive, Directive::Done { reason: None });
    }

    #[tokio::test]
    async fn branch_1_a_read_only_flag_names_the_utility() {
        let directive = without_workflow(2)
            .execute(&stateless_input().with_read_only(ReadOnlyVerb::Status))
            .await;
        let message = print_message(&directive);
        assert!(message.contains("aidlc-utility status"), "{message}");
        assert!(
            message.contains("This is a read-only utility, NOT workflow work: do NOT run `next`"),
            "{message}"
        );
    }

    #[tokio::test]
    async fn branch_1b_a_noun_token_passes_through_verbatim() {
        let token = NounToken::new(
            NounFamily::Workspace,
            vec!["intent".to_string(), "list".to_string()],
        );
        let directive = without_workflow(2)
            .execute(&stateless_input().with_noun_token(token))
            .await;
        assert!(print_message(&directive).contains("aidlc-utility intent list"));
    }

    // ---- 分岐 2 / state 読取ガード ----

    #[tokio::test]
    async fn branch_2_stage_and_phase_together_are_refused() {
        let directive = without_workflow(2)
            .execute(
                &stateless_input()
                    .with_stage("stage-1")
                    .with_phase("Inception"),
            )
            .await;
        assert_eq!(
            error_message(&directive),
            "Cannot use --stage and --phase together. Use one or the other."
        );
    }

    #[tokio::test]
    async fn a_broken_state_read_stops_before_the_cursor_is_used() {
        // state バージョンガードの相当 — 読取失敗 (ここでは NotFound) は復元前に逐語で止める。
        let use_case = without_workflow(2);
        let directive = use_case.execute(&active_input()).await;
        assert!(error_message(&directive).starts_with("not found:"));
    }

    // ---- 分岐 2.5 / 2.6 (park) ----

    #[tokio::test]
    async fn branch_2_5_a_parked_workflow_stops_with_the_parked_directive() {
        let (intent, mut execution, _) = genesis(2);
        execution.park(&intent, at()).unwrap();
        let directive = with_execution(2, execution).execute(&active_input()).await;
        assert_eq!(
            directive,
            Directive::Parked {
                stage: slug(0),
                message: "Workflow parked at \"stage-0\". Resume with /aidlc --resume.".to_string()
            }
        );
    }

    #[tokio::test]
    async fn branch_2_6_resume_on_a_parked_workflow_names_unpark() {
        let (intent, mut execution, _) = genesis(2);
        execution.park(&intent, at()).unwrap();
        let directive = with_execution(2, execution)
            .execute(&active_input().with_resume())
            .await;
        assert_eq!(
            print_message(&directive),
            "This workflow is parked. Run `aidlc-state unpark` to clear the park marker, then re-run `next --resume` to continue."
        );
    }

    // ---- 分岐 3b / 4 (scope 検証) ----

    #[tokio::test]
    async fn branch_3b_an_invalid_explicit_scope_is_refused_even_when_state_wins() {
        let directive = with_workflow(2)
            .execute(&active_input().with_scope("warp-drive"))
            .await;
        assert_eq!(
            error_message(&directive),
            "Unknown scope \"warp-drive\". Valid scopes: bugfix, classic."
        );
    }

    #[tokio::test]
    async fn branch_4_an_invalid_env_default_scope_is_refused_verbatim() {
        let directive = without_workflow(2)
            .execute(&stateless_input().with_env_default_scope("warp-drive"))
            .await;
        assert_eq!(
            error_message(&directive),
            "Invalid AWS_AIDLC_DEFAULT_SCOPE \"warp-drive\". Valid scopes: bugfix, classic."
        );
    }

    // ---- 分岐 4c / 4a / 4b ----

    #[tokio::test]
    async fn branch_4c_compose_with_a_jump_flag_is_refused() {
        let directive = without_workflow(2)
            .execute(&stateless_input().with_compose().with_stage("stage-1"))
            .await;
        assert_eq!(
            error_message(&directive),
            "Cannot combine compose with --stage/--phase. Compose re-shapes the plan; jump moves the cursor. Run them separately."
        );
    }

    #[tokio::test]
    async fn branch_4c_compose_names_the_composer_dispatch() {
        let directive = without_workflow(2)
            .execute(&stateless_input().with_compose())
            .await;
        assert!(print_message(&directive).contains("aidlc-composer detect"));
    }

    #[tokio::test]
    async fn branch_4a_a_blank_new_intent_description_is_refused() {
        let directive = without_workflow(2)
            .execute(
                &stateless_input()
                    .with_new_intent("   ")
                    .with_scope("bugfix"),
            )
            .await;
        assert!(error_message(&directive).contains("must not be blank"));
    }

    #[tokio::test]
    async fn branch_4a_new_intent_names_intent_create_with_the_explicit_scope_only() {
        let directive = without_workflow(2)
            .execute(
                &stateless_input()
                    .with_new_intent("fix the crash")
                    .with_scope("bugfix"),
            )
            .await;
        let message = print_message(&directive);
        assert!(
            message.contains(
                "aidlc-utility intent-create --scope bugfix --label \"<2-3 word kebab essence>\""
            ),
            "{message}"
        );
    }

    #[tokio::test]
    async fn branch_4b_single_requires_a_stage() {
        let directive = without_workflow(2)
            .execute(&stateless_input().with_single())
            .await;
        assert_eq!(
            error_message(&directive),
            "--single requires --stage <slug>."
        );
    }

    #[tokio::test]
    async fn branch_4b_single_emits_an_isolated_run_stage() {
        let directive = without_workflow(2)
            .execute(&stateless_input().with_single().with_stage("stage-1"))
            .await;
        let run_stage = expect_run_stage(directive);
        assert!(run_stage.is_single());
        assert_eq!(run_stage.stage().as_str(), "stage-1");
    }

    // ---- 分岐 5 / 6 ----

    #[tokio::test]
    async fn branch_5_a_differing_valid_scope_names_scope_change() {
        let directive = with_workflow(2)
            .execute(&active_input().with_scope("bugfix"))
            .await;
        assert!(print_message(&directive).contains("aidlc-utility scope-change --scope bugfix"));
    }

    #[tokio::test]
    async fn branch_5_a_depth_override_names_config_change() {
        let directive = with_workflow(2)
            .execute(&active_input().with_depth("minimal"))
            .await;
        assert!(print_message(&directive).contains("aidlc-utility config-change --depth minimal"));
    }

    #[tokio::test]
    async fn branch_6_resume_with_state_asks_the_resume_menu() {
        let directive = with_workflow(2)
            .execute(&active_input().with_resume())
            .await;
        let ask = expect_ask(directive);
        assert_eq!(ask.ask_kind(), AskKind::ResumeMenu);
        assert_eq!(
            ask.question(),
            "An existing workflow was found (currently at \"stage-0\"). How would you like to proceed? Resume from last checkpoint, redo the current stage, jump to a stage, or start fresh."
        );
    }

    // ---- 分岐 7 (jump) ----

    #[tokio::test]
    async fn branch_7_a_jump_to_an_initialization_stage_is_refused_verbatim() {
        let directive = with_workflow(2)
            .execute(&active_input().with_stage("stage-0"))
            .await;
        assert_eq!(
            error_message(&directive),
            "Cannot jump to initialization stages. The Initialization phase runs automatically when you start a workflow (describe what to build, e.g. /aidlc \"build the auth service\")."
        );
    }

    #[tokio::test]
    async fn branch_7_a_jump_with_state_names_the_pure_resolve() {
        let directive = with_workflow(3)
            .execute(&active_input().with_stage("stage-2"))
            .await;
        assert!(print_message(&directive).contains("aidlc-jump resolve --stage stage-2"));
    }

    #[tokio::test]
    async fn branch_7_a_jump_without_state_searches_the_graph_directly() {
        let directive = without_workflow(3)
            .execute(&stateless_input().with_stage("stage-2"))
            .await;
        let run_stage = expect_run_stage(directive);
        assert_eq!(run_stage.stage().as_str(), "stage-2");
        assert!(!run_stage.is_single());
    }

    // ---- state なしの群 (7b / 8 / 9a / 9b) ----

    #[tokio::test]
    async fn branch_7b_a_positional_scope_names_the_birth() {
        let directive = without_workflow(2)
            .execute(&stateless_input().with_freeform("bugfix"))
            .await;
        assert!(print_message(&directive).contains("aidlc-utility intent-create --scope bugfix"));
    }

    #[tokio::test]
    async fn branch_7b_records_without_a_cursor_ask_the_intent_pick() {
        let directive = without_workflow(2)
            .execute(
                &stateless_input()
                    .with_freeform("bugfix")
                    .with_records_without_cursor(),
            )
            .await;
        let ask = expect_ask(directive);
        assert_eq!(ask.ask_kind(), AskKind::IntentPick);
    }

    #[tokio::test]
    async fn branch_8_a_keyword_hit_asks_the_scope_confirmation() {
        let directive = without_workflow(2)
            .execute(&stateless_input().with_freeform("fix the login"))
            .await;
        let ask = expect_ask(directive);
        assert_eq!(ask.ask_kind(), AskKind::ScopeConfirm);
        assert!(ask.question().contains("bugfix"));
    }

    #[tokio::test]
    async fn branch_8_a_keyword_in_a_long_description_is_suppressed() {
        // 5 語超のテキストは推論を抑止する — キーワードが偶然含まれる記述のガード。
        let directive = without_workflow(2)
            .execute(
                &stateless_input()
                    .with_freeform("please fix the login page for our production customers"),
            )
            .await;
        let ask = expect_ask(directive);
        assert_eq!(ask.ask_kind(), AskKind::ComposeOffer);
    }

    #[tokio::test]
    async fn branch_9a_an_explicit_scope_names_the_birth() {
        let directive = without_workflow(2)
            .execute(&stateless_input().with_scope("classic"))
            .await;
        assert!(print_message(&directive).contains("aidlc-utility intent-create --scope classic"));
    }

    #[tokio::test]
    async fn branch_9b_nothing_named_without_state_is_refused_verbatim() {
        let directive = without_workflow(2).execute(&stateless_input()).await;
        assert_eq!(
            error_message(&directive),
            "No workflow state found (no active intent). Start one by describing what to build (/aidlc \"build the auth service\") or by naming a scope (/aidlc --scope <scope>)."
        );
    }

    // ---- 分岐 9c ----

    #[tokio::test]
    async fn branch_9c_freeform_prose_on_a_running_workflow_asks_the_routing() {
        let directive = with_workflow(2)
            .execute(&active_input().with_freeform("fix the crash"))
            .await;
        let ask = expect_ask(directive);
        assert_eq!(ask.ask_kind(), AskKind::NewWorkRouting);
        assert_eq!(ask.new_work_description(), Some("fix the crash"));
        assert_eq!(ask.proposed_scope(), Some("bugfix"));
    }

    // ---- 分岐 10 (ハッピーパス) ----

    #[tokio::test]
    async fn branch_10_the_happy_path_emits_a_run_stage_for_the_cursor() {
        let directive = with_workflow(2).execute(&active_input()).await;
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

    #[tokio::test]
    async fn branch_10_a_gated_stage_carries_the_gate() {
        let (intent, mut execution, _) = genesis(2);
        execution.complete_stage(&intent, at()).unwrap();
        let directive = with_execution(2, execution).execute(&active_input()).await;
        let run_stage = expect_run_stage(directive);
        assert_eq!(run_stage.stage().as_str(), "stage-1");
        assert_eq!(run_stage.gate(), GateField::Gated);
    }

    #[tokio::test]
    async fn branch_10_a_finished_workflow_is_done_with_the_verbatim_reason() {
        let (intent, mut execution, _) = genesis(1);
        execution.complete_stage(&intent, at()).unwrap();
        let directive = with_execution(1, execution).execute(&active_input()).await;
        assert_eq!(
            directive,
            Directive::Done {
                reason: Some(
                    "Workflow complete — no in-scope stage remains after stage-0 (scope: classic)."
                        .to_string()
                )
            }
        );
    }

    // ---- 第 2 波: 分岐 10 の不整合腕 (直接呼出 — 到達には壊れた歴史の再生が要るため) ----

    #[test]
    fn branch_10_a_recoverable_skip_inconsistency_names_the_repair() {
        let (intent, execution, _) = genesis(2);
        let context = LoadedWorkflow { intent, execution };
        let directive = with_workflow(2).emit_happy_path(
            &active_input(),
            &context,
            &definition(2),
            &ScopeSlug::parse("classic").unwrap(),
            NextDecision::RecoverSkipInconsistency {
                stage: context.execution.stage_index(1).unwrap(),
                checkbox: core_command_domain::workspace::CheckboxState::InProgress,
            },
        );
        assert_eq!(
            print_message(&directive),
            "Run `aidlc-orchestrate report --stage stage-1 --result skipped --reason \"stage is SKIP in the approved workflow plan\"`, then re-run `next`."
        );
    }

    #[test]
    fn branch_10_an_unrecoverable_skip_inconsistency_is_refused_verbatim() {
        let (intent, execution, _) = genesis(2);
        let context = LoadedWorkflow { intent, execution };
        let directive = with_workflow(2).emit_happy_path(
            &active_input(),
            &context,
            &definition(2),
            &ScopeSlug::parse("classic").unwrap(),
            NextDecision::InconsistentSkip {
                stage: context.execution.stage_index(1).unwrap(),
                checkbox: core_command_domain::workspace::CheckboxState::AwaitingApproval,
            },
        );
        assert_eq!(
            error_message(&directive),
            "Stage \"stage-1\" is SKIP in the approved workflow plan but its active cursor state is \"awaiting-approval\". Refusing to emit run-stage; repair the inconsistent state before continuing."
        );
    }

    #[test]
    fn a_routing_decision_that_reaches_the_happy_path_is_a_defensive_error() {
        let (intent, execution, _) = genesis(2);
        let context = LoadedWorkflow { intent, execution };
        let directive = with_workflow(2).emit_happy_path(
            &active_input(),
            &context,
            &definition(2),
            &ScopeSlug::parse("classic").unwrap(),
            NextDecision::ResumeMenu,
        );
        assert!(error_message(&directive).starts_with("internal:"));
    }

    #[test]
    fn every_checkbox_state_has_its_upstream_word() {
        use core_command_domain::workspace::CheckboxState as S;
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

    // ---- 第 2 波: scope 解決ラダーの残腕 ----

    #[test]
    fn the_scope_ladder_walks_state_over_explicit_over_env_over_default() {
        use core_command_domain::orchestration::{ScopeSource, resolve_scope};
        let definition = definition(2);
        // state が勝つ。
        let resolved =
            resolve_scope(&definition, Some("classic"), None, None, Some("bugfix")).unwrap();
        assert_eq!(
            (resolved.name().as_str(), resolved.source()),
            ("classic", ScopeSource::State)
        );
        // state 由来値が定義に無ければ解決不能。
        assert_eq!(
            resolve_scope(&definition, Some("warp-drive"), None, None, None).unwrap_err(),
            ScopeResolutionError::Unresolvable {
                scope: "warp-drive".to_string()
            }
        );
        // 明示 --scope (state なし)。
        let resolved = resolve_scope(&definition, None, Some("bugfix"), None, None).unwrap();
        assert_eq!(
            (resolved.name().as_str(), resolved.source()),
            ("bugfix", ScopeSource::Explicit)
        );
        // 位置引数の推論。
        let resolved = resolve_scope(&definition, None, None, Some("fix the login"), None).unwrap();
        assert_eq!(
            (resolved.name().as_str(), resolved.source()),
            ("bugfix", ScopeSource::Inferred)
        );
        // 有効な env。
        let resolved = resolve_scope(&definition, None, None, None, Some("bugfix")).unwrap();
        assert_eq!(
            (resolved.name().as_str(), resolved.source()),
            ("bugfix", ScopeSource::Env)
        );
        // 何も無ければ classic。
        let resolved = resolve_scope(&definition, None, None, None, None).unwrap();
        assert_eq!(
            (resolved.name().as_str(), resolved.source()),
            ("classic", ScopeSource::Default)
        );
    }

    // ---- 第 2 波: 未踏のフラグ分岐 ----

    #[tokio::test]
    async fn branch_1b_plugin_and_knowledge_tokens_also_pass_through() {
        for family in [NounFamily::Plugin, NounFamily::Knowledge] {
            let token = NounToken::new(family, vec!["list".to_string()]);
            let directive = without_workflow(2)
                .execute(&stateless_input().with_noun_token(token))
                .await;
            assert!(print_message(&directive).contains("aidlc-utility list"));
        }
    }

    #[tokio::test]
    async fn branch_5_a_test_strategy_override_names_config_change() {
        let directive = with_workflow(2)
            .execute(&active_input().with_test_strategy("minimal"))
            .await;
        assert!(
            print_message(&directive)
                .contains("aidlc-utility config-change --test-strategy minimal")
        );
    }

    #[tokio::test]
    async fn branch_5_a_review_override_alone_names_config_change() {
        let directive = with_workflow(2)
            .execute(&active_input().with_review("adversarial"))
            .await;
        assert!(
            print_message(&directive).contains("aidlc-utility config-change --review adversarial")
        );
    }

    #[tokio::test]
    async fn branch_4a_new_intent_without_an_explicit_scope_is_refused() {
        let directive = without_workflow(2)
            .execute(&stateless_input().with_new_intent("fix the crash"))
            .await;
        assert_eq!(
            error_message(&directive),
            "--new-intent requires an explicit --scope <name>."
        );
    }

    #[tokio::test]
    async fn branch_4b_single_with_an_unknown_stage_is_refused() {
        let directive = without_workflow(2)
            .execute(&stateless_input().with_single().with_stage("no-such-stage"))
            .await;
        assert_eq!(
            error_message(&directive),
            "Unknown stage \"no-such-stage\"."
        );
    }

    #[tokio::test]
    async fn branch_7_a_jump_to_an_unknown_stage_is_refused() {
        let directive = with_workflow(2)
            .execute(&active_input().with_stage("no-such-stage"))
            .await;
        assert_eq!(
            error_message(&directive),
            "Unknown stage \"no-such-stage\"."
        );
    }

    #[tokio::test]
    async fn branch_7_a_phase_jump_without_state_searches_the_graph() {
        let directive = without_workflow(3)
            .execute(&stateless_input().with_phase("inception"))
            .await;
        let run_stage = expect_run_stage(directive);
        assert_eq!(
            run_stage.stage().as_str(),
            "stage-1",
            "フェーズ先頭の in-scope ステージ"
        );
    }

    #[tokio::test]
    async fn branch_7_an_unknown_phase_is_refused() {
        let directive = without_workflow(2)
            .execute(&stateless_input().with_phase("Daydreaming"))
            .await;
        assert_eq!(error_message(&directive), "Unknown phase \"Daydreaming\".");
    }

    #[tokio::test]
    async fn a_missing_layout_stops_run_stage_assembly() {
        let input = NextTurnInput::new()
            .with_definition_id(WorkflowDefinitionId::parse("claude").unwrap())
            .with_single()
            .with_stage("stage-1");
        let directive = without_workflow(2).execute(&input).await;
        assert_eq!(
            error_message(&directive),
            "No workspace layout was provided for run-stage assembly."
        );
    }

    #[tokio::test]
    async fn a_missing_definition_id_without_state_is_refused() {
        let input = NextTurnInput::new().with_layout(layout());
        let directive = without_workflow(2).execute(&input).await;
        assert_eq!(
            error_message(&directive),
            "No workflow definition id was provided."
        );
    }

    #[tokio::test]
    async fn a_missing_intent_row_stops_before_the_ladder_continues() {
        // 実行は在るのに intent が引けない — 読取ガードは 2 本目のポートでも働く。
        let (_, execution, _) = genesis(2);
        let use_case = NextUseCase::new(
            InMemoryIntentExecutionRepository::holding(execution, 1),
            InMemoryIntentRepository::empty(),
            InMemoryWorkflowDefinitionRepository {
                held: definition(2),
            },
            NoRules,
            FakeCodec::default(),
            FakeSpelling,
        );
        let directive = use_case.execute(&active_input()).await;
        assert!(error_message(&directive).starts_with("not found:"));
    }

    // ---- 第 2 波: run-stage 組み立ての面 (レビュアー・モード別 inline paths) ----

    #[tokio::test]
    async fn a_reviewer_bearing_stage_carries_the_reviewer_and_protocol_hint() {
        let mut nodes = Vec::new();
        for index in 0..2 {
            let phase = if index == 0 {
                PhaseId::Initialization
            } else {
                PhaseId::Inception
            };
            let mut builder = StageNodeBuilder::new(
                slug(index),
                StageNumber::parse(&format!("{index}.1")).unwrap(),
                format!("Stage {index}"),
                phase,
                ExecutionKind::Always,
                if index == 1 {
                    StageMode::Mob
                } else {
                    StageMode::Inline
                },
            )
            .lead_agent("orchestrator".to_string())
            .scopes(vec!["classic".to_string()]);
            if index == 1 {
                builder = builder
                    .reviewer("aidlc-product-lead-agent".to_string())
                    .review_class(core_command_domain::workflow_definition::ReviewClass::Advisory);
            }
            nodes.push(builder.build());
        }
        let grid = ScopeGrid::new(
            [(
                "classic".to_string(),
                (0..2).map(|i| (slug(i), PlanAction::Execute)).collect(),
            )]
            .into_iter()
            .collect(),
        );
        let scopes: BTreeMap<String, ScopeMetadata> = [(
            "classic".to_string(),
            ScopeMetadata::new("classic").unwrap(),
        )]
        .into_iter()
        .collect();
        let definition = WorkflowDefinition::from_artifacts(
            WorkflowDefinitionId::parse("claude").unwrap(),
            core_command_domain::workflow_definition::DefinitionRevision::parse(&format!(
                "sha256:{}",
                "0".repeat(64)
            ))
            .unwrap(),
            StageGraph::new(nodes).unwrap(),
            grid,
            scopes,
        );
        let (intent, mut execution, _) = genesis(2);
        execution.complete_stage(&intent, at()).unwrap();
        let use_case = NextUseCase::new(
            InMemoryIntentExecutionRepository::holding(execution, 1),
            InMemoryIntentRepository::holding(intent),
            InMemoryWorkflowDefinitionRepository { held: definition },
            NoRules,
            FakeCodec::default(),
            FakeSpelling,
        );
        let directive = use_case.execute(&active_input()).await;
        let run_stage = expect_run_stage(directive);
        assert_eq!(run_stage.reviewer(), Some("aidlc-product-lead-agent"));
        assert_eq!(
            run_stage.review_class(),
            Some(core_command_domain::workflow_definition::ReviewClass::Advisory)
        );
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

    // ---- 第 2 波: VO の観測面 ----

    #[test]
    fn the_turn_input_reports_every_observation_back() {
        let input = NextTurnInput::new()
            .with_parse_error("boom")
            .with_review("advisory")
            .with_read_only(ReadOnlyVerb::Doctor)
            .with_stage("stage-1")
            .with_phase("Inception")
            .with_scope("classic")
            .with_depth("minimal")
            .with_test_strategy("standard")
            .with_freeform("fix it")
            .with_resume()
            .with_single()
            .with_compose()
            .with_new_intent("desc")
            .with_env_default_scope("classic")
            .with_kiro_latch_bare_next()
            .with_records_without_cursor()
            .with_active(active())
            .with_layout(layout())
            .with_definition_id(WorkflowDefinitionId::parse("claude").unwrap());
        assert_eq!(input.parse_error(), Some("boom"));
        assert_eq!(input.review(), Some("advisory"));
        assert_eq!(input.read_only(), Some(ReadOnlyVerb::Doctor));
        assert_eq!(input.stage(), Some("stage-1"));
        assert_eq!(input.phase(), Some("Inception"));
        assert_eq!(input.scope(), Some("classic"));
        assert_eq!(input.depth(), Some("minimal"));
        assert_eq!(input.test_strategy(), Some("standard"));
        assert_eq!(input.freeform(), Some("fix it"));
        assert!(input.is_resume());
        assert!(input.is_single());
        assert!(input.is_compose());
        assert_eq!(input.new_intent(), Some("desc"));
        assert_eq!(input.env_default_scope(), Some("classic"));
        assert!(input.is_kiro_latch_bare_next());
        assert!(input.records_exist_without_cursor());
        assert_eq!(input.active().unwrap().intent_id(), &intent_id());
        assert_eq!(input.active().unwrap().execution_id(), &execution_id());
        assert_eq!(input.layout().unwrap().record_dir(), "record");
        assert_eq!(input.layout().unwrap().stage_library_dir(), "stages");
        assert_eq!(input.layout().unwrap().agent_dir(), "agents");
        assert_eq!(input.definition_id().unwrap().to_string(), "claude");
        let token = NounToken::new(NounFamily::Workspace, vec!["intent".to_string()]);
        assert_eq!(token.family(), NounFamily::Workspace);
        assert_eq!(token.tokens(), ["intent"]);
    }

    // ---- 第 3 波: 残りの腕 ----

    #[tokio::test]
    async fn a_state_scope_missing_from_the_definition_is_unresolvable() {
        // intent は classic を握っているが、定義に classic が無い。
        let mut scopes: BTreeMap<String, ScopeMetadata> = BTreeMap::new();
        scopes.insert("bugfix".to_string(), ScopeMetadata::new("bugfix").unwrap());
        let grid = ScopeGrid::new(
            [(
                "bugfix".to_string(),
                (0..2).map(|i| (slug(i), PlanAction::Execute)).collect(),
            )]
            .into_iter()
            .collect(),
        );
        let nodes = (0..2)
            .map(|index| {
                StageNodeBuilder::new(
                    slug(index),
                    StageNumber::parse(&format!("{index}.1")).unwrap(),
                    format!("Stage {index}"),
                    if index == 0 {
                        PhaseId::Initialization
                    } else {
                        PhaseId::Inception
                    },
                    ExecutionKind::Always,
                    StageMode::Inline,
                )
                .lead_agent("orchestrator".to_string())
                .scopes(vec!["bugfix".to_string()])
                .build()
            })
            .collect::<Vec<_>>();
        let held = WorkflowDefinition::from_artifacts(
            WorkflowDefinitionId::parse("claude").unwrap(),
            core_command_domain::workflow_definition::DefinitionRevision::parse(&format!(
                "sha256:{}",
                "0".repeat(64)
            ))
            .unwrap(),
            StageGraph::new(nodes).unwrap(),
            grid,
            scopes,
        );
        let (intent, execution, _) = genesis(2);
        let use_case = NextUseCase::new(
            InMemoryIntentExecutionRepository::holding(execution, 1),
            InMemoryIntentRepository::holding(intent),
            InMemoryWorkflowDefinitionRepository { held },
            NoRules,
            FakeCodec::default(),
            FakeSpelling,
        );
        let directive = use_case.execute(&active_input()).await;
        assert_eq!(
            error_message(&directive),
            "Unknown scope \"classic\". Valid scopes: bugfix."
        );
    }

    #[tokio::test]
    async fn a_definition_identity_mismatch_is_relayed_from_the_domain() {
        // 定義 id が intent のピンと食い違う — next_decision の拒否を逐語で中継する。
        let held = {
            let nodes = (0..2)
                .map(|index| {
                    StageNodeBuilder::new(
                        slug(index),
                        StageNumber::parse(&format!("{index}.1")).unwrap(),
                        format!("Stage {index}"),
                        if index == 0 {
                            PhaseId::Initialization
                        } else {
                            PhaseId::Inception
                        },
                        ExecutionKind::Always,
                        StageMode::Inline,
                    )
                    .lead_agent("orchestrator".to_string())
                    .scopes(vec!["classic".to_string()])
                    .build()
                })
                .collect::<Vec<_>>();
            WorkflowDefinition::from_artifacts(
                WorkflowDefinitionId::parse("kiro").unwrap(),
                core_command_domain::workflow_definition::DefinitionRevision::parse(&format!(
                    "sha256:{}",
                    "0".repeat(64)
                ))
                .unwrap(),
                StageGraph::new(nodes).unwrap(),
                ScopeGrid::new(
                    [(
                        "classic".to_string(),
                        (0..2).map(|i| (slug(i), PlanAction::Execute)).collect(),
                    )]
                    .into_iter()
                    .collect(),
                ),
                [(
                    "classic".to_string(),
                    ScopeMetadata::new("classic").unwrap(),
                )]
                .into_iter()
                .collect(),
            )
        };
        let (intent, execution, _) = genesis(2);
        let use_case = NextUseCase::new(
            InMemoryIntentExecutionRepository::holding(execution, 1),
            InMemoryIntentRepository::holding(intent),
            InMemoryWorkflowDefinitionRepository { held },
            NoRules,
            FakeCodec::default(),
            FakeSpelling,
        );
        let directive = use_case.execute(&active_input()).await;
        assert!(
            error_message(&directive).contains("definition"),
            "{directive:?}"
        );
    }

    #[tokio::test]
    async fn branch_4b_single_on_an_initialization_stage_is_ungated() {
        let directive = without_workflow(2)
            .execute(&stateless_input().with_single().with_stage("stage-0"))
            .await;
        let run_stage = expect_run_stage(directive);
        assert_eq!(run_stage.gate(), GateField::Ungated);
    }

    #[tokio::test]
    async fn branch_7_a_jump_without_layout_stops_run_stage_assembly() {
        let input = NextTurnInput::new()
            .with_definition_id(WorkflowDefinitionId::parse("claude").unwrap())
            .with_stage("stage-1");
        let directive = without_workflow(2).execute(&input).await;
        assert_eq!(
            error_message(&directive),
            "No workspace layout was provided for run-stage assembly."
        );
    }

    #[tokio::test]
    async fn a_dispatched_stage_reads_no_inline_context() {
        // subagent / pipeline は完全委任 — inline_context_paths は空。construction フェーズは
        // protocol_modules に construction が载る。
        let nodes = vec![
            StageNodeBuilder::new(
                slug(0),
                StageNumber::parse("0.1").unwrap(),
                "Stage 0".to_string(),
                PhaseId::Initialization,
                ExecutionKind::Always,
                StageMode::Inline,
            )
            .lead_agent("orchestrator".to_string())
            .scopes(vec!["classic".to_string()])
            .build(),
            StageNodeBuilder::new(
                slug(1),
                StageNumber::parse("1.1").unwrap(),
                "Stage 1".to_string(),
                PhaseId::Construction,
                ExecutionKind::Always,
                StageMode::Subagent,
            )
            .lead_agent("aidlc-developer-agent".to_string())
            .scopes(vec!["classic".to_string()])
            .build(),
        ];
        let held = WorkflowDefinition::from_artifacts(
            WorkflowDefinitionId::parse("claude").unwrap(),
            core_command_domain::workflow_definition::DefinitionRevision::parse(&format!(
                "sha256:{}",
                "0".repeat(64)
            ))
            .unwrap(),
            StageGraph::new(nodes).unwrap(),
            ScopeGrid::new(
                [(
                    "classic".to_string(),
                    (0..2).map(|i| (slug(i), PlanAction::Execute)).collect(),
                )]
                .into_iter()
                .collect(),
            ),
            [(
                "classic".to_string(),
                ScopeMetadata::new("classic").unwrap(),
            )]
            .into_iter()
            .collect(),
        );
        let (intent, mut execution, _) = genesis(2);
        execution.complete_stage(&intent, at()).unwrap();
        let use_case = NextUseCase::new(
            InMemoryIntentExecutionRepository::holding(execution, 1),
            InMemoryIntentRepository::holding(intent),
            InMemoryWorkflowDefinitionRepository { held },
            NoRules,
            FakeCodec::default(),
            FakeSpelling,
        );
        let directive = use_case.execute(&active_input()).await;
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

    #[tokio::test]
    async fn every_read_only_verb_names_its_subcommand() {
        for (verb, sub) in [
            (ReadOnlyVerb::Status, "status"),
            (ReadOnlyVerb::Help, "help"),
            (ReadOnlyVerb::Doctor, "doctor"),
            (ReadOnlyVerb::Version, "version"),
        ] {
            let directive = without_workflow(2)
                .execute(&stateless_input().with_read_only(verb))
                .await;
            assert!(print_message(&directive).contains(&format!("aidlc-utility {sub}")));
        }
    }

    #[test]
    fn an_empty_keyword_never_matches() {
        use core_command_domain::orchestration::infer_scope_from_text;
        let mut scopes: BTreeMap<String, ScopeMetadata> = BTreeMap::new();
        scopes.insert(
            "bugfix".to_string(),
            ScopeMetadata::new("bugfix")
                .unwrap()
                .with_keywords(vec![String::new()]),
        );
        let held = WorkflowDefinition::from_artifacts(
            WorkflowDefinitionId::parse("claude").unwrap(),
            core_command_domain::workflow_definition::DefinitionRevision::parse(&format!(
                "sha256:{}",
                "0".repeat(64)
            ))
            .unwrap(),
            StageGraph::new(vec![
                StageNodeBuilder::new(
                    slug(0),
                    StageNumber::parse("0.1").unwrap(),
                    "Stage 0".to_string(),
                    PhaseId::Initialization,
                    ExecutionKind::Always,
                    StageMode::Inline,
                )
                .lead_agent("orchestrator".to_string())
                .scopes(vec!["bugfix".to_string()])
                .build(),
            ])
            .unwrap(),
            ScopeGrid::new(
                [(
                    "bugfix".to_string(),
                    [(slug(0), PlanAction::Execute)].into_iter().collect(),
                )]
                .into_iter()
                .collect(),
            ),
            scopes,
        );
        assert_eq!(infer_scope_from_text(&held, "anything at all"), None);
    }

    #[test]
    fn the_run_stage_directive_reports_its_required_faces() {
        let directive = RunStageDirectiveBuilder::new(
            slug(1),
            PhaseId::Inception,
            "aidlc-product-agent",
            StageMode::Inline,
            GateField::Gated,
            "stages/inception/stage-1.md",
            "record/inception/stage-1/memory.md",
        )
        .build();
        assert_eq!(directive.stage().as_str(), "stage-1");
        assert_eq!(directive.phase(), PhaseId::Inception);
        assert_eq!(directive.lead_agent(), "aidlc-product-agent");
        assert!(directive.support_agents().is_empty());
        assert_eq!(directive.mode(), StageMode::Inline);
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

    // ---- 第 4 波: 余白 (相対ゲート) ----

    #[tokio::test]
    async fn a_phase_with_no_in_scope_stage_is_refused() {
        // 定義に operation フェーズのステージが無い。
        let directive = without_workflow(2)
            .execute(&stateless_input().with_phase("operation"))
            .await;
        assert_eq!(
            error_message(&directive),
            "No in-scope stage found for phase \"operation\"."
        );
    }

    #[tokio::test]
    async fn a_cursor_slug_missing_from_the_graph_is_refused_on_the_happy_path() {
        // 定義のグラフに cursor の slug が無い (定義とピンの食い違い方の一種)。
        let nodes = vec![
            StageNodeBuilder::new(
                StageSlug::parse("someone-else").unwrap(),
                StageNumber::parse("0.1").unwrap(),
                "Other".to_string(),
                PhaseId::Initialization,
                ExecutionKind::Always,
                StageMode::Inline,
            )
            .lead_agent("orchestrator".to_string())
            .scopes(vec!["classic".to_string()])
            .build(),
        ];
        let held = WorkflowDefinition::from_artifacts(
            WorkflowDefinitionId::parse("claude").unwrap(),
            core_command_domain::workflow_definition::DefinitionRevision::parse(&format!(
                "sha256:{}",
                "0".repeat(64)
            ))
            .unwrap(),
            StageGraph::new(nodes).unwrap(),
            ScopeGrid::new(
                [(
                    "classic".to_string(),
                    [(
                        StageSlug::parse("someone-else").unwrap(),
                        PlanAction::Execute,
                    )]
                    .into_iter()
                    .collect(),
                )]
                .into_iter()
                .collect(),
            ),
            [(
                "classic".to_string(),
                ScopeMetadata::new("classic").unwrap(),
            )]
            .into_iter()
            .collect(),
        );
        let (intent, execution, _) = genesis(1);
        let use_case = NextUseCase::new(
            InMemoryIntentExecutionRepository::holding(execution, 1),
            InMemoryIntentRepository::holding(intent),
            InMemoryWorkflowDefinitionRepository { held },
            NoRules,
            FakeCodec::default(),
            FakeSpelling,
        );
        let directive = use_case.execute(&active_input()).await;
        assert_eq!(error_message(&directive), "Unknown stage \"stage-0\".");
    }

    // ---- B16: steering 連鎖 ----

    /// 2 部の配信計画 (同一ファイル由来の 2 チャンク)。
    fn two_part_rules() -> StaticRules {
        let big = "x".repeat(12 * 1024);
        StaticRules {
            chunks: vec![
                vec![RuleContent::new(
                    "aidlc/spaces/default/memory/org.md".to_string(),
                    format!("# Org\n{big}\n"),
                )],
                vec![RuleContent::new(
                    "aidlc/spaces/default/memory/org.md".to_string(),
                    format!("# Team\n{big}\n"),
                )],
            ],
        }
    }

    type ChainedNext = NextUseCase<
        InMemoryIntentExecutionRepository,
        InMemoryIntentRepository,
        InMemoryWorkflowDefinitionRepository,
        StaticRules,
        FakeCodec,
        FakeSpelling,
    >;
    type ChainedContinue = ContinueUseCase<
        InMemoryIntentExecutionRepository,
        InMemoryIntentRepository,
        InMemoryWorkflowDefinitionRepository,
        StaticRules,
        FakeCodec,
    >;

    fn chained_use_cases(rules: StaticRules) -> (ChainedNext, ChainedContinue) {
        let codec = FakeCodec::default();
        let (intent, execution, _) = genesis(2);
        let next = NextUseCase::new(
            InMemoryIntentExecutionRepository::holding(execution.clone(), 1),
            InMemoryIntentRepository::holding(intent.clone()),
            InMemoryWorkflowDefinitionRepository {
                held: definition(2),
            },
            rules.clone(),
            codec.clone(),
            FakeSpelling,
        );
        let continuation = ContinueUseCase::new(
            InMemoryIntentExecutionRepository::holding(execution, 1),
            InMemoryIntentRepository::holding(intent),
            InMemoryWorkflowDefinitionRepository {
                held: definition(2),
            },
            rules,
            codec,
        );
        (next, continuation)
    }

    #[tokio::test]
    async fn a_rule_bundle_is_delivered_in_parts_then_the_run_stage_arrives() {
        let (next, continuation) = chained_use_cases(two_part_rules());
        // 第 1 部。
        let directive = next.execute(&active_input()).await;
        let part1 = expect_load_steering(directive);
        assert_eq!(part1.part().as_u32(), 1);
        assert_eq!(part1.parts().as_u32(), 2);
        assert_eq!(part1.stage().as_str(), "stage-0");
        assert!(!part1.rules_content().is_empty());
        assert_eq!(
            part1.rules_content().first().map(RuleContent::path),
            Some("aidlc/spaces/default/memory/org.md")
        );
        // 第 2 部。
        let directive = continuation
            .execute(part1.continue_token(), &active_input())
            .await;
        let part2 = expect_load_steering(directive);
        assert_eq!(part2.part().as_u32(), 2);
        assert_eq!(part2.parts().as_u32(), 2);
        // 終端 — run-stage がルール台帳つきで届く。
        let directive = continuation
            .execute(part2.continue_token(), &active_input())
            .await;
        let run_stage = expect_run_stage(directive);
        assert_eq!(run_stage.stage().as_str(), "stage-0");
        assert_eq!(
            run_stage.rules_in_context(),
            ["aidlc/spaces/default/memory/org.md"],
            "配信済みルールのパス台帳"
        );
        assert_eq!(run_stage.gate(), GateField::Ungated, "ピンの再適用");
    }

    #[tokio::test]
    async fn a_continuation_whose_intent_cannot_be_read_fails_closed() {
        // state 束縛の前提は intent が読めること — 読めなければ STATE_MOVED_ON で止める
        // (fail-closed。定義のピンも解決できないため)。
        let codec = FakeCodec::default();
        let (intent, execution, _) = genesis(2);
        let next = NextUseCase::new(
            InMemoryIntentExecutionRepository::holding(execution.clone(), 1),
            InMemoryIntentRepository::holding(intent),
            InMemoryWorkflowDefinitionRepository {
                held: definition(2),
            },
            two_part_rules(),
            codec.clone(),
            FakeSpelling,
        );
        let continuation = ContinueUseCase::new(
            InMemoryIntentExecutionRepository::holding(execution, 1),
            InMemoryIntentRepository::empty(),
            InMemoryWorkflowDefinitionRepository {
                held: definition(2),
            },
            two_part_rules(),
            codec,
        );
        let directive = next.execute(&active_input()).await;
        let part1 = expect_load_steering(directive);
        let directive = continuation
            .execute(part1.continue_token(), &active_input())
            .await;
        assert_eq!(
            directive,
            Directive::Error {
                message: "The saved position moved on: the workflow state changed while this \
                          stage's rules were being loaded. Run a fresh `next` to restart \
                          delivery from part 1."
                    .to_string()
            }
        );
    }

    #[tokio::test]
    async fn an_invalid_continuation_token_fails_closed_verbatim() {
        let (_, continuation) = chained_use_cases(two_part_rules());
        let directive = continuation.execute("garbage", &active_input()).await;
        assert_eq!(
            error_message(&directive),
            "Invalid steering continuation token: this stage's rules cannot be loaded from where they left off. Run a fresh `next` to restart delivery from part 1."
        );
    }

    #[tokio::test]
    async fn a_moved_on_state_fails_closed_verbatim() {
        let (next, _) = chained_use_cases(two_part_rules());
        let directive = next.execute(&active_input()).await;
        let part1 = expect_load_steering(directive);
        // continue 側のストアでは実行が 1 コマンド進んでいる (通番が動いた)。
        let codec = FakeCodec::default();
        let _ = codec; // 検証には next と同じ codec が要る — chained の続きで別状態を組む。
        let (intent, mut execution, _) = genesis(2);
        execution.complete_stage(&intent, at()).unwrap();
        let continuation = ContinueUseCase::new(
            InMemoryIntentExecutionRepository::holding(execution, 2),
            InMemoryIntentRepository::holding(intent),
            InMemoryWorkflowDefinitionRepository {
                held: definition(2),
            },
            two_part_rules(),
            next_codec(&next),
        );
        let directive = continuation
            .execute(part1.continue_token(), &active_input())
            .await;
        assert_eq!(
            error_message(&directive),
            "The saved position moved on: the workflow state changed while this stage's rules were being loaded. Run a fresh `next` to restart delivery from part 1."
        );
    }

    #[tokio::test]
    async fn a_changed_bundle_fails_closed_as_stale() {
        let (next, _) = chained_use_cases(two_part_rules());
        let directive = next.execute(&active_input()).await;
        let part1 = expect_load_steering(directive);
        // continue 側ではルールが書き換わっている。
        let (intent, execution, _) = genesis(2);
        let continuation = ContinueUseCase::new(
            InMemoryIntentExecutionRepository::holding(execution, 1),
            InMemoryIntentRepository::holding(intent),
            InMemoryWorkflowDefinitionRepository {
                held: definition(2),
            },
            StaticRules {
                chunks: vec![vec![RuleContent::new(
                    "aidlc/spaces/default/memory/org.md".to_string(),
                    "# Org\nrewritten\n".to_string(),
                )]],
            },
            next_codec(&next),
        );
        let directive = continuation
            .execute(part1.continue_token(), &active_input())
            .await;
        assert_eq!(
            error_message(&directive),
            "This stage or its rules changed while they were being loaded, so what has arrived so far is stale. Run a fresh `next` to restart delivery from part 1."
        );
    }

    #[tokio::test]
    async fn a_vanished_stage_fails_closed_verbatim() {
        let (next, _) = chained_use_cases(two_part_rules());
        let directive = next.execute(&active_input()).await;
        let part1 = expect_load_steering(directive);
        // グラフが再コンパイルされ stage-0 が消えた。
        let held = {
            let nodes = vec![
                StageNodeBuilder::new(
                    StageSlug::parse("someone-else").unwrap(),
                    StageNumber::parse("0.1").unwrap(),
                    "Other".to_string(),
                    PhaseId::Initialization,
                    ExecutionKind::Always,
                    StageMode::Inline,
                )
                .lead_agent("orchestrator".to_string())
                .scopes(vec!["classic".to_string()])
                .build(),
            ];
            WorkflowDefinition::from_artifacts(
                WorkflowDefinitionId::parse("claude").unwrap(),
                core_command_domain::workflow_definition::DefinitionRevision::parse(&format!(
                    "sha256:{}",
                    "0".repeat(64)
                ))
                .unwrap(),
                StageGraph::new(nodes).unwrap(),
                ScopeGrid::new(
                    [(
                        "classic".to_string(),
                        [(
                            StageSlug::parse("someone-else").unwrap(),
                            PlanAction::Execute,
                        )]
                        .into_iter()
                        .collect(),
                    )]
                    .into_iter()
                    .collect(),
                ),
                [(
                    "classic".to_string(),
                    ScopeMetadata::new("classic").unwrap(),
                )]
                .into_iter()
                .collect(),
            )
        };
        let (intent, execution, _) = genesis(2);
        let continuation = ContinueUseCase::new(
            InMemoryIntentExecutionRepository::holding(execution, 1),
            InMemoryIntentRepository::holding(intent),
            InMemoryWorkflowDefinitionRepository { held },
            two_part_rules(),
            next_codec(&next),
        );
        let directive = continuation
            .execute(part1.continue_token(), &active_input())
            .await;
        assert_eq!(
            error_message(&directive),
            "Stage \"stage-0\" no longer exists. Run a fresh `next` after recompiling the stage graph."
        );
    }

    #[tokio::test]
    async fn a_changed_route_fails_closed_verbatim() {
        let (next, _) = chained_use_cases(two_part_rules());
        let directive = next.execute(&active_input()).await;
        let part1 = expect_load_steering(directive);
        // scope のステージメンバーシップが変わった (3 ステージ目が増えた)。
        let (intent, execution, _) = genesis(2);
        let continuation = ContinueUseCase::new(
            InMemoryIntentExecutionRepository::holding(execution, 1),
            InMemoryIntentRepository::holding(intent),
            InMemoryWorkflowDefinitionRepository {
                held: definition(3),
            },
            two_part_rules(),
            next_codec(&next),
        );
        let directive = continuation
            .execute(part1.continue_token(), &active_input())
            .await;
        assert_eq!(
            error_message(&directive),
            "Which stage runs next has changed: the stage route changed while its rules were being loaded. Run a fresh `next` to restart delivery from part 1."
        );
    }

    #[tokio::test]
    async fn a_part_beyond_the_plan_fails_closed_verbatim() {
        let (next, continuation) = chained_use_cases(two_part_rules());
        // まず正規の連鎖を 1 回起こし、ダイジェスト束縛を写した上で索引だけ超過させる。
        let directive = next.execute(&active_input()).await;
        let part1 = expect_load_steering(directive);
        let codec = next_codec(&next);
        let token = codec.verify(part1.continue_token()).unwrap();
        let mut beyond = ContinueTokenBuilder::new(
            token.stage().clone(),
            token.scope().clone(),
            PartIndex::from_raw(9).unwrap(),
            token.bindings().clone(),
            token.gate(),
        );
        if let Some(next_stage) = token.next_stage() {
            beyond = beyond.with_next_stage(next_stage.clone());
        }
        let beyond = beyond.build();
        let encoded = codec.mint(&beyond);
        let directive = continuation.execute(&encoded, &active_input()).await;
        assert_eq!(
            error_message(&directive),
            "This request asks for a part of the stage rules that does not exist. Run a fresh `next` to restart delivery from part 1."
        );
    }

    #[tokio::test]
    async fn an_unreadable_rule_file_blocks_the_stage_verbatim() {
        let (intent, execution, _) = genesis(2);
        let use_case = NextUseCase::new(
            InMemoryIntentExecutionRepository::holding(execution, 1),
            InMemoryIntentRepository::holding(intent),
            InMemoryWorkflowDefinitionRepository {
                held: definition(2),
            },
            BrokenRules,
            FakeCodec::default(),
            FakeSpelling,
        );
        let directive = use_case.execute(&active_input()).await;
        assert_eq!(
            error_message(&directive),
            "Cannot load required stage rule \"aidlc/spaces/default/memory/org.md\" (permission denied). The stage has not started. Restore the file or fix its permissions/UTF-8 encoding, then run `next` again."
        );
    }

    /// next が抱える codec の複製 (共有ストア) を取り出す。
    fn next_codec(next: &ChainedNext) -> FakeCodec {
        next.codec.clone()
    }

    // ---- B16: continue の残腕と steering_chain ----

    #[tokio::test]
    async fn a_state_aware_token_without_an_active_workflow_moves_on() {
        let (next, continuation) = chained_use_cases(two_part_rules());
        let directive = next.execute(&active_input()).await;
        let part1 = expect_load_steering(directive);
        // continue 側の入力に active が無い。
        let input = NextTurnInput::new()
            .with_layout(layout())
            .with_definition_id(WorkflowDefinitionId::parse("claude").unwrap());
        let directive = continuation.execute(part1.continue_token(), &input).await;
        assert_eq!(
            error_message(&directive),
            "The saved position moved on: the workflow state changed while this stage's rules were being loaded. Run a fresh `next` to restart delivery from part 1."
        );
    }

    #[tokio::test]
    async fn a_stateless_chain_continues_without_a_workflow() {
        // state なしの --single 連鎖 — トークンは state 束縛なしで、active なしでも継続できる。
        let codec = FakeCodec::default();
        let rules = two_part_rules();
        let next = NextUseCase::new(
            InMemoryIntentExecutionRepository::empty(),
            InMemoryIntentRepository::empty(),
            InMemoryWorkflowDefinitionRepository {
                held: definition(2),
            },
            rules.clone(),
            codec.clone(),
            FakeSpelling,
        );
        let continuation = ContinueUseCase::new(
            InMemoryIntentExecutionRepository::empty(),
            InMemoryIntentRepository::empty(),
            InMemoryWorkflowDefinitionRepository {
                held: definition(2),
            },
            rules,
            codec,
        );
        let input = stateless_input().with_single().with_stage("stage-1");
        let directive = next.execute(&input).await;
        let part1 = expect_load_steering(directive);
        let directive = continuation
            .execute(part1.continue_token(), &stateless_input())
            .await;
        let part2 = expect_load_steering(directive);
        let directive = continuation
            .execute(part2.continue_token(), &stateless_input())
            .await;
        let run_stage = expect_run_stage(directive);
        assert!(run_stage.is_single(), "single ピンの再適用");
        assert_eq!(run_stage.stage().as_str(), "stage-1");
    }

    #[tokio::test]
    async fn a_stateless_continue_without_a_definition_id_fails_closed() {
        let codec = FakeCodec::default();
        let rules = two_part_rules();
        let next = NextUseCase::new(
            InMemoryIntentExecutionRepository::empty(),
            InMemoryIntentRepository::empty(),
            InMemoryWorkflowDefinitionRepository {
                held: definition(2),
            },
            rules.clone(),
            codec.clone(),
            FakeSpelling,
        );
        let continuation = ContinueUseCase::new(
            InMemoryIntentExecutionRepository::empty(),
            InMemoryIntentRepository::empty(),
            InMemoryWorkflowDefinitionRepository {
                held: definition(2),
            },
            rules,
            codec,
        );
        let input = stateless_input().with_single().with_stage("stage-1");
        let part1 = expect_load_steering(next.execute(&input).await);
        let bare = NextTurnInput::new().with_layout(layout());
        let directive = continuation.execute(part1.continue_token(), &bare).await;
        assert_eq!(
            error_message(&directive),
            "This stage or its rules changed while they were being loaded, so what has arrived so far is stale. Run a fresh `next` to restart delivery from part 1."
        );
    }

    #[tokio::test]
    async fn an_unreadable_bundle_on_continue_is_stale() {
        let (next, _) = chained_use_cases(two_part_rules());
        let part1 = expect_load_steering(next.execute(&active_input()).await);
        let (intent, execution, _) = genesis(2);
        let continuation = ContinueUseCase::new(
            InMemoryIntentExecutionRepository::holding(execution, 1),
            InMemoryIntentRepository::holding(intent),
            InMemoryWorkflowDefinitionRepository {
                held: definition(2),
            },
            BrokenRules,
            next_codec(&next),
        );
        let directive = continuation
            .execute(part1.continue_token(), &active_input())
            .await;
        assert_eq!(
            error_message(&directive),
            "This stage or its rules changed while they were being loaded, so what has arrived so far is stale. Run a fresh `next` to restart delivery from part 1."
        );
    }

    #[test]
    fn the_pins_survive_the_rebuild() {
        use core_command_domain::orchestration::{Bindings, StageName};
        let run_stage = core_command_domain::orchestration::RunStageDirectiveBuilder::new(
            slug(1),
            PhaseId::Inception,
            "aidlc-product-agent",
            StageMode::Inline,
            GateField::Gated,
            "stage.md",
            "memory.md",
        )
        .with_support_agents(vec!["aidlc-design-agent".to_string()])
        .with_reviewer(
            "aidlc-product-lead-agent",
            core_command_domain::workflow_definition::ReviewClass::Advisory,
            2,
        )
        .with_narration("note")
        .build();
        let token = ContinueTokenBuilder::new(
            slug(1),
            ScopeSlug::parse("classic").unwrap(),
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

    /// 定義が読めない読取専用ダブル。
    #[derive(Debug)]
    struct BrokenDefinitions;

    impl WorkflowDefinitionRepository for BrokenDefinitions {
        fn find_by_id(
            &self,
            _id: &WorkflowDefinitionId,
        ) -> Result<WorkflowDefinition, GraphReadError> {
            Err(GraphReadError::ScopeFile {
                message: "broken".to_string(),
            })
        }
    }

    #[tokio::test]
    async fn a_state_aware_token_with_a_vanished_store_moves_on() {
        let (next, _) = chained_use_cases(two_part_rules());
        let part1 = expect_load_steering(next.execute(&active_input()).await);
        // continue 側のストアが空 (実行が消えた)。
        let continuation = ContinueUseCase::new(
            InMemoryIntentExecutionRepository::empty(),
            InMemoryIntentRepository::empty(),
            InMemoryWorkflowDefinitionRepository {
                held: definition(2),
            },
            two_part_rules(),
            next_codec(&next),
        );
        let directive = continuation
            .execute(part1.continue_token(), &active_input())
            .await;
        assert_eq!(
            error_message(&directive),
            "The saved position moved on: the workflow state changed while this stage's rules were being loaded. Run a fresh `next` to restart delivery from part 1."
        );
    }

    #[tokio::test]
    async fn an_unreadable_definition_on_continue_reports_the_stage_gone() {
        let (next, _) = chained_use_cases(two_part_rules());
        let part1 = expect_load_steering(next.execute(&active_input()).await);
        let (intent, execution, _) = genesis(2);
        let continuation = ContinueUseCase::new(
            InMemoryIntentExecutionRepository::holding(execution, 1),
            InMemoryIntentRepository::holding(intent),
            BrokenDefinitions,
            two_part_rules(),
            next_codec(&next),
        );
        let directive = continuation
            .execute(part1.continue_token(), &active_input())
            .await;
        assert_eq!(
            error_message(&directive),
            "Stage \"stage-0\" no longer exists. Run a fresh `next` after recompiling the stage graph."
        );
    }

    #[tokio::test]
    async fn branch_9c_an_uninferable_description_falls_back_to_the_resolved_scope() {
        // 5 語超の記述はキーワード推論が抑止される — 提案 scope は稼働中の解決値に畳む。
        let directive = with_workflow(2)
            .execute(
                &active_input()
                    .with_freeform("please fix the login crash we saw yesterday in production"),
            )
            .await;
        let ask = expect_ask(directive);
        assert_eq!(ask.proposed_scope(), Some("classic"));
    }

    #[tokio::test]
    async fn a_missing_layout_on_the_happy_path_stops_run_stage_assembly() {
        let input = NextTurnInput::new().with_active(active());
        let directive = with_workflow(2).execute(&input).await;
        assert_eq!(
            error_message(&directive),
            "No workspace layout was provided for run-stage assembly."
        );
    }

    #[tokio::test]
    async fn an_unsplittable_bundle_blocks_the_stage_verbatim() {
        /// 分割不能を返すダブル。
        #[derive(Debug, Default)]
        struct UnsplittableRules;
        impl RuleBundleSource for UnsplittableRules {
            fn load(&self, _phase: PhaseId) -> Result<SteeringPlan, RuleBundleReadError> {
                Err(RuleBundleReadError::Unsplittable {
                    path: "aidlc/spaces/default/memory/org.md".to_string(),
                })
            }
        }
        let (intent, execution, _) = genesis(2);
        let use_case = NextUseCase::new(
            InMemoryIntentExecutionRepository::holding(execution, 1),
            InMemoryIntentRepository::holding(intent),
            InMemoryWorkflowDefinitionRepository {
                held: definition(2),
            },
            UnsplittableRules,
            FakeCodec::default(),
            FakeSpelling,
        );
        let directive = use_case.execute(&active_input()).await;
        assert_eq!(
            error_message(&directive),
            "A rule section could not be split below the directive transport limit. Shorten the affected heading section, then run a fresh `next`."
        );
    }
}
