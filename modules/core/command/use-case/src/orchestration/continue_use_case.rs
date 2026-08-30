//! `Continue` — steering 連鎖の継続 (トークン検証 → ディスク状態からの再構築 → 次部
//! または終端 run-stage — 02 §10 / §4.4)。
//!
//! **キャッシュを信用しない** (再構築原則 `:5996-6037`): 現在のディスク状態から run-stage と
//! ルール束を作り直し、トークンのピン (`gate` / `next_stage` / `unit` / `single`) を再適用し、
//! ダイジェスト束縛 (bundle / directive / route / state) を照合する。ドリフトは**すべて
//! fail-closed** — fresh `next` からのやり直しだけを指示する (I12)。ピンの再適用は
//! [`RunStageDirective::with_pins`] (ドメインの部分更新)、束縛の照合は型付きダイジェスト
//! ([`Bindings`]) の等値比較で行う。

use core_command_domain::orchestration::{Bindings, ContinueToken, Directive, StateBinding};
use core_command_domain::workflow_definition::WorkflowDefinition;

use super::next_turn_input::NextTurnInput;
use super::port::ContinueTokenCodec;
use super::port::IntentExecutionRepository;
use super::port::IntentRepository;
use super::port::WorkflowDefinitionRepository;
use super::port::{RuleBundleReadError, RuleBundleSource};

/// fail-closed の逐語文言 (02 §4.4 の完全列挙)。
mod wording {
    /// トークン欠落・デコード不能・MAC 不一致・型表違反。
    pub(super) const INVALID_TOKEN: &str = "Invalid steering continuation token: this stage's rules cannot be loaded from where they left off. Run a fresh `next` to restart delivery from part 1.";

    /// state-aware トークンの `h` 不一致。
    pub(super) const STATE_MOVED_ON: &str = "The saved position moved on: the workflow state changed while this stage's rules were being loaded. Run a fresh `next` to restart delivery from part 1.";

    /// route hash 不一致。
    pub(super) const ROUTE_CHANGED: &str = "Which stage runs next has changed: the stage route changed while its rules were being loaded. Run a fresh `next` to restart delivery from part 1.";

    /// bundle / directive ダイジェスト不一致 (stale)。
    pub(super) const STALE: &str = "This stage or its rules changed while they were being loaded, so what has arrived so far is stale. Run a fresh `next` to restart delivery from part 1.";

    /// 存在しない部の要求。
    pub(super) const PART_NOT_EXIST: &str = "This request asks for a part of the stage rules that does not exist. Run a fresh `next` to restart delivery from part 1.";

    /// ステージがグラフから消えた。
    pub(super) fn stage_gone(slug: &str) -> String {
        format!(
            "Stage \"{slug}\" no longer exists. Run a fresh `next` after recompiling the stage graph."
        )
    }
}

/// steering 連鎖の継続 (読取専用 — 書込ポートを持たない)。
#[derive(Debug)]
pub struct ContinueUseCase<E, I, D, B, C> {
    execution_repository: E,
    intent_repository: I,
    definition_repository: D,
    bundle_source: B,
    codec: C,
}

impl<E, I, D, B, C> ContinueUseCase<E, I, D, B, C>
where
    E: IntentExecutionRepository,
    I: IntentRepository,
    D: WorkflowDefinitionRepository,
    B: RuleBundleSource,
    C: ContinueTokenCodec,
{
    /// 読取専用ポート 5 本を注入する ([`super::NextUseCase`] と同じ読取束 — 綴りポートは
    /// 使わないので受けない)。
    #[must_use]
    pub const fn new(
        execution_repository: E,
        intent_repository: I,
        definition_repository: D,
        bundle_source: B,
        codec: C,
    ) -> ContinueUseCase<E, I, D, B, C> {
        ContinueUseCase {
            execution_repository,
            intent_repository,
            definition_repository,
            bundle_source,
            codec,
        }
    }

    /// encode 済みトークン 1 つを directive ちょうど 1 つに写す。
    pub async fn execute(&self, encoded: &str, input: &NextTurnInput) -> Directive {
        let Ok(token) = self.codec.verify(encoded) else {
            return Directive::Error {
                message: wording::INVALID_TOKEN.to_string(),
            };
        };
        // state 束縛 — 現在の state ダイジェストと照合する。
        let state = match self.state_binding(&token, input).await {
            Ok(state) => state,
            Err(directive) => return *directive,
        };
        let definition = match self.load_definition(&token, input).await {
            Ok(definition) => definition,
            Err(directive) => return *directive,
        };
        let Some(node) = definition
            .graph()
            .nodes()
            .iter()
            .find(|node| node.slug() == token.stage())
        else {
            return Directive::Error {
                message: wording::stage_gone(token.stage().as_str()),
            };
        };
        let scope = token.scope();
        let route = self
            .codec
            .route_digest(&definition.stage_route(scope.as_str(), node));
        if &route != token.bindings().route() {
            return Directive::Error {
                message: wording::ROUTE_CHANGED.to_string(),
            };
        }
        // ディスク状態から run-stage を再構築し、トークンのピンを再適用する。
        let rebuilt = match super::next_use_case::build_run_stage(
            node,
            &definition,
            scope.as_str(),
            input.layout(),
            token.gate(),
            token.is_single(),
        ) {
            Ok(Directive::RunStage(run_stage)) => run_stage.with_pins(&token),
            Ok(_) | Err(_) => {
                return Directive::Error {
                    message: wording::STALE.to_string(),
                };
            }
        };
        let plan = match self.bundle_source.load(node.phase()) {
            Ok(plan) => plan,
            Err(
                RuleBundleReadError::Unreadable { .. } | RuleBundleReadError::Unsplittable { .. },
            ) => {
                return Directive::Error {
                    message: wording::STALE.to_string(),
                };
            }
        };
        let bindings = Bindings::new(
            self.codec.bundle_digest(&plan),
            self.codec.directive_digest(&rebuilt),
            route,
            state,
        );
        if bindings.bundle() != token.bindings().bundle()
            || bindings.directive() != token.bindings().directive()
        {
            return Directive::Error {
                message: wording::STALE.to_string(),
            };
        }
        let delivered = token.next_part_index();
        if plan.is_delivered_through(delivered) {
            return Directive::RunStage(rebuilt.with_rules_in_context(plan.delivered_paths()));
        }
        match plan.part_after(delivered) {
            Some(part) => {
                super::next_use_case::emit_part(&self.codec, &part, &rebuilt, scope, &bindings)
            }
            None => Directive::Error {
                message: wording::PART_NOT_EXIST.to_string(),
            },
        }
    }

    /// state 束縛の照合。state-aware なら現行 state のダイジェストを計算して比較する。
    async fn state_binding(
        &self,
        token: &ContinueToken,
        input: &NextTurnInput,
    ) -> Result<Option<StateBinding>, Box<Directive>> {
        let Some(bound) = token.bindings().state() else {
            return Ok(None);
        };
        let Some(active) = input.active() else {
            return Err(Box::new(Directive::Error {
                message: wording::STATE_MOVED_ON.to_string(),
            }));
        };
        let execution = self
            .execution_repository
            .find_by_id(active.execution_id())
            .await
            .map_err(|_| {
                Box::new(Directive::Error {
                    message: wording::STATE_MOVED_ON.to_string(),
                })
            })?;
        // 束縛のダイジェストは集約だけで組めるが、**intent が読めること**は束縛の前提である
        // (読めなければ定義のピンも解決できない)。fail-closed を保つため、ここで確かめる。
        self.intent_repository
            .find_by_id(active.intent_id())
            .await
            .map_err(|_| {
                Box::new(Directive::Error {
                    message: wording::STATE_MOVED_ON.to_string(),
                })
            })?;
        let current = self.codec.state_binding(&execution);
        if &current != bound {
            return Err(Box::new(Directive::Error {
                message: wording::STATE_MOVED_ON.to_string(),
            }));
        }
        Ok(Some(current))
    }

    /// 定義の読取 — state ありなら intent のピン、無しならハーネスの定義 id。
    async fn load_definition(
        &self,
        token: &ContinueToken,
        input: &NextTurnInput,
    ) -> Result<WorkflowDefinition, Box<Directive>> {
        let id = if token.bindings().state().is_some()
            && let Some(active) = input.active()
            && let Ok(intent) = self.intent_repository.find_by_id(active.intent_id()).await
        {
            intent.definition_id().clone()
        } else if let Some(id) = input.definition_id() {
            id.clone()
        } else {
            return Err(Box::new(Directive::Error {
                message: wording::STALE.to_string(),
            }));
        };
        self.definition_repository.find_by_id(&id).map_err(|_| {
            Box::new(Directive::Error {
                message: wording::stage_gone(token.stage().as_str()),
            })
        })
    }
}
