//! `Continue` — steering 連鎖の継続 (トークン検証 → ディスク状態からの再構築 → 次部
//! または終端 run-stage — 02 §10 / §4.4)。
//!
//! `next` と同じく**読むだけの動詞**なのでクエリ側にある (`coding-rules/cqrs-boundaries.md`
//! 規則 5)。読取素材はリードモデルを読む DAO ポート経由で取得する (オーナー裁定 2026-08-31)。
//! 読取はここでも遅延で、`token` が欠けているターンは I/O を 1 回も起こさず、state を読むのも
//! トークンが state 束縛を運んでいるときだけである。
//!
//! **キャッシュを信用しない** (再構築原則 `:5996-6037`): 現在のディスク状態から run-stage と
//! ルール束を作り直し、トークンのピン (`gate` / `next_stage` / `unit` / `single`) を再適用し、
//! ダイジェスト束縛 (bundle / directive / route / state) を照合する。ドリフトは**すべて
//! fail-closed** — fresh `next` からのやり直しだけを指示する (I12)。ピンの再適用は
//! [`RunStageDirective::with_pins`] (不変オブジェクトの部分更新)、束縛の照合は型付き
//! ダイジェスト ([`Bindings`]) の等値比較で行う。
//!
//! [`RunStageDirective::with_pins`]: super::RunStageDirective::with_pins

use super::bindings::Bindings;
use super::continue_token::ContinueToken;
use super::directive::Directive;
use super::next_turn_input::NextTurnInput;
use super::state_binding::StateBinding;
use crate::orchestration::DefinitionView;
use crate::orchestration::{
    ExecutionStateDao, MemoryRulesDao, WorkflowDefinitionDao, WorkflowDefinitionReadError,
};

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

/// steering 連鎖の継続 (読取専用 — [`super::NextUseCase`] と同じ 3 つの DAO を持つ)。
///
/// 保持するのは読取専用 DAO だけで、更新動詞は 1 つも無い
/// (`coding-rules/cqrs-boundaries.md` 規則 6 / オーナー裁定 2026-08-31)。
#[derive(Debug)]
pub struct ContinueUseCase<D: WorkflowDefinitionDao, S: ExecutionStateDao, M: MemoryRulesDao> {
    workflow_definition_dao: D,
    execution_state_dao: S,
    memory_rules_dao: M,
}

impl<D: WorkflowDefinitionDao, S: ExecutionStateDao, M: MemoryRulesDao> ContinueUseCase<D, S, M> {
    /// 3 つの読取専用 DAO を束ねる。
    #[must_use]
    pub const fn new(
        workflow_definition_dao: D,
        execution_state_dao: S,
        memory_rules_dao: M,
    ) -> ContinueUseCase<D, S, M> {
        ContinueUseCase {
            workflow_definition_dao,
            execution_state_dao,
            memory_rules_dao,
        }
    }

    /// 開封済みトークン 1 つを directive ちょうど 1 つに写す。
    ///
    /// 開封 (base64url 復号 + MAC 検証 + 厳密型表) は Controller (U7) の責務であり、失敗は
    /// `None` で渡される — 契約は fail-closed の逐語文言 1 本である。トークンは**リードモデル
    /// ではなく要求素材**なので、ポートではなく引数で受ける。
    ///
    /// ルール束の読取失敗はすべて `STALE` に畳む — 継続の契約は「やり直せ」1 本であり、
    /// 原因の区別を漏らさない (I12)。読み取るだけなので `&self` のクエリである (CQS)。
    #[must_use]
    pub fn execute(&self, token: Option<ContinueToken>, input: &NextTurnInput) -> Directive {
        let Some(token) = token else {
            return Directive::Error {
                message: wording::INVALID_TOKEN.to_string(),
            };
        };
        // state 束縛 — 現在の state ダイジェストと照合する。
        let state = match self.state_binding(&token) {
            Ok(state) => state,
            Err(directive) => return *directive,
        };
        let definition = match self.definition_of(&token) {
            Ok(definition) => definition,
            Err(directive) => return *directive,
        };
        let definition = &definition;
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
        let route = definition.stage_route(scope.as_str(), node).route_digest();
        if &route != token.bindings().route() {
            return Directive::Error {
                message: wording::ROUTE_CHANGED.to_string(),
            };
        }
        // ディスク状態から run-stage を再構築し、トークンのピンを再適用する。
        let rebuilt = match super::next_use_case::build_run_stage(
            node,
            definition,
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
        // 読めない・分割不能のどちらも継続はできない — 区別せず fail-closed。
        let Ok(plan) = self
            .memory_rules_dao
            .find()
            .map_err(|_| ())
            .and_then(|rules| rules.plan_for(node.phase()).map_err(|_| ()))
        else {
            return Directive::Error {
                message: wording::STALE.to_string(),
            };
        };
        let bindings = Bindings::new(
            plan.bundle_digest(),
            rebuilt.directive_digest(),
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
            Some(part) => super::next_use_case::emit_part(&part, &rebuilt, scope, &bindings),
            None => Directive::Error {
                message: wording::PART_NOT_EXIST.to_string(),
            },
        }
    }

    /// state 束縛の照合。state-aware なら現行リードモデルのダイジェストを計算して比較する。
    ///
    /// トークンが state 束縛を運んでいないときは**読みに行かない** — 照合すべきものが無い。
    /// 運んでいる場合、リードモデルが無い・読めないのはどちらも「保存された位置が動いた」
    /// 扱いで fail-closed にする — 束縛を確かめられない以上、続きを届けてはならない。
    fn state_binding(&self, token: &ContinueToken) -> Result<Option<StateBinding>, Box<Directive>> {
        let Some(bound) = token.bindings().state() else {
            return Ok(None);
        };
        let moved_on = || {
            Box::new(Directive::Error {
                message: wording::STATE_MOVED_ON.to_string(),
            })
        };
        let Ok(Some(view)) = self.execution_state_dao.find() else {
            return Err(moved_on());
        };
        let current = view.state_binding();
        if &current != bound {
            return Err(moved_on());
        }
        Ok(Some(current))
    }

    /// 定義ビューの読取 — 読めない・特定できないは fail-closed。
    ///
    /// どちらの逐語文言になるかは失敗の種類で決まる: 定義 id が特定できないのは連鎖の材料が
    /// 欠けている (`STALE`)、読めないのはステージへ到達できない (`stage_gone`)。
    fn definition_of(&self, token: &ContinueToken) -> Result<DefinitionView, Box<Directive>> {
        match self.workflow_definition_dao.find() {
            Ok(view) => Ok(view),
            Err(WorkflowDefinitionReadError::Unidentified) => Err(Box::new(Directive::Error {
                message: wording::STALE.to_string(),
            })),
            Err(_) => Err(Box::new(Directive::Error {
                message: wording::stage_gone(token.stage().as_str()),
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    // panic! は想定外バリアントの即時失敗という検証用途で使っており、テスト失敗のシグナル
    // として妥当なため許容する。
    #![allow(clippy::panic)]

    use std::collections::BTreeMap;

    use super::super::continue_token::ContinueTokenBuilder;
    use super::super::load_steering_directive::LoadSteeringDirective;
    use super::super::next_use_case::NextUseCase;
    use super::super::part_index::PartIndex;
    use super::super::rule_content::RuleContent;
    use super::super::run_stage_directive::RunStageDirective;
    use super::super::test_fixtures::{
        FakeDefinitionDao, FakeRulesDao, FakeStateDao, definition, genesis_state, slug, state,
    };
    use super::super::workspace_layout::WorkspaceLayout;
    use super::*;
    use crate::orchestration::ExecutionStateReadError;
    use crate::orchestration::MemoryRules;
    use crate::orchestration::{
        CheckboxState, DefinitionIdView, DefinitionRevisionView, ExecutionKindView,
        ExecutionStateView, PhaseView, PlanActionView, ScopeGridView, ScopeMetadataView,
        StageGraphView, StageModeView, StageNumberView, StageSlugView, StageViewBuilder,
    };

    /// 在るのに読めないルールファイル。
    fn unreadable_rules() -> FakeRulesDao {
        FakeRulesDao::unreadable("aidlc/spaces/default/memory/org.md", "permission denied")
    }

    /// 継続のユースケース (DAO 3 本を注入する)。
    fn continuing(
        workflow_definition_dao: FakeDefinitionDao,
        execution_state_dao: FakeStateDao,
        memory_rules_dao: FakeRulesDao,
    ) -> ContinueUseCase<FakeDefinitionDao, FakeStateDao, FakeRulesDao> {
        ContinueUseCase::new(
            workflow_definition_dao,
            execution_state_dao,
            memory_rules_dao,
        )
    }

    /// 2 部束・state ありの継続 (最も多い形)。
    fn continuing_with(
        graph: &DefinitionView,
        held: &ExecutionStateView,
    ) -> ContinueUseCase<FakeDefinitionDao, FakeStateDao, FakeRulesDao> {
        continuing(
            FakeDefinitionDao::holding(graph.clone()),
            FakeStateDao::holding(held.clone()),
            FakeRulesDao::holding(two_part_rules()),
        )
    }

    /// 2 部に分かれるルール束 (12KiB セクション × 2 → 20KiB 目標で 2 チャンク)。
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

    fn layout() -> WorkspaceLayout {
        WorkspaceLayout::new(
            "record".to_string(),
            "stages".to_string(),
            "agents".to_string(),
        )
    }

    fn input() -> NextTurnInput {
        NextTurnInput::new().with_layout(layout())
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

    fn error_message(directive: &Directive) -> &str {
        match directive {
            Directive::Error { message } => message,
            other => panic!("error を期待したが {:?}", other.kind()),
        }
    }

    /// state ありの連鎖を 1 部だけ起こし、その第 1 部を返す。
    fn first_part(held: &ExecutionStateView, graph: &DefinitionView) -> LoadSteeringDirective {
        expect_load_steering(
            NextUseCase::new(
                FakeDefinitionDao::holding(graph.clone()),
                FakeStateDao::holding(held.clone()),
                FakeRulesDao::holding(two_part_rules()),
            )
            .execute(&input()),
        )
    }

    #[test]
    fn an_invalid_continuation_token_fails_closed_verbatim() {
        let held = genesis_state(2);
        let graph = definition(2);
        let directive = continuing_with(&graph, &held).execute(None, &input());
        assert_eq!(
            error_message(&directive),
            "Invalid steering continuation token: this stage's rules cannot be loaded from where they left off. Run a fresh `next` to restart delivery from part 1."
        );
    }

    #[test]
    fn a_moved_on_state_fails_closed_verbatim() {
        let held = genesis_state(2);
        let graph = definition(2);
        let part1 = first_part(&held, &graph);
        // continue 側のリードモデルは 1 ステージ進んでいる。
        let moved = state(
            2,
            1,
            &[CheckboxState::Completed, CheckboxState::InProgress],
            &[PlanActionView::Execute; 2],
        );
        let directive =
            continuing_with(&graph, &moved).execute(Some(part1.continue_token().clone()), &input());
        assert_eq!(
            error_message(&directive),
            "The saved position moved on: the workflow state changed while this stage's rules were being loaded. Run a fresh `next` to restart delivery from part 1."
        );
    }

    #[test]
    fn a_state_aware_token_without_a_read_model_moves_on() {
        let held = genesis_state(2);
        let graph = definition(2);
        let part1 = first_part(&held, &graph);
        for absent in [
            FakeStateDao::absent(),
            FakeStateDao::failing(ExecutionStateReadError::NotReadable {
                path: "/r/aidlc-state.md".to_string(),
                cause: "permission denied".to_string(),
            }),
        ] {
            let directive = continuing(
                FakeDefinitionDao::holding(graph.clone()),
                absent,
                FakeRulesDao::holding(two_part_rules()),
            )
            .execute(Some(part1.continue_token().clone()), &input());
            assert_eq!(
                error_message(&directive),
                "The saved position moved on: the workflow state changed while this stage's rules were being loaded. Run a fresh `next` to restart delivery from part 1."
            );
        }
    }

    #[test]
    fn a_changed_bundle_fails_closed_as_stale() {
        let held = genesis_state(2);
        let graph = definition(2);
        let part1 = first_part(&held, &graph);
        // continue 側ではルールが書き換わっている。
        let rewritten = MemoryRules::new(
            vec![RuleContent::new(
                "aidlc/spaces/default/memory/org.md".to_string(),
                "# Org\nrewritten\n".to_string(),
            )],
            BTreeMap::new(),
        );
        let directive = continuing(
            FakeDefinitionDao::holding(graph),
            FakeStateDao::holding(held),
            FakeRulesDao::holding(rewritten),
        )
        .execute(Some(part1.continue_token().clone()), &input());
        assert_eq!(
            error_message(&directive),
            "This stage or its rules changed while they were being loaded, so what has arrived so far is stale. Run a fresh `next` to restart delivery from part 1."
        );
    }

    #[test]
    fn a_vanished_stage_fails_closed_verbatim() {
        let held = genesis_state(2);
        let graph = definition(2);
        let part1 = first_part(&held, &graph);
        // グラフが再コンパイルされ stage-0 が消えた。
        let recompiled = definition_with_single_node("someone-else");
        let directive = continuing_with(&recompiled, &held)
            .execute(Some(part1.continue_token().clone()), &input());
        assert_eq!(
            error_message(&directive),
            "Stage \"stage-0\" no longer exists. Run a fresh `next` after recompiling the stage graph."
        );
    }

    #[test]
    fn a_changed_route_fails_closed_verbatim() {
        let held = genesis_state(2);
        let graph = definition(2);
        let part1 = first_part(&held, &graph);
        // scope のステージメンバーシップが変わった (3 ステージ目が増えた)。
        let directive = continuing_with(&definition(3), &held)
            .execute(Some(part1.continue_token().clone()), &input());
        assert_eq!(
            error_message(&directive),
            "Which stage runs next has changed: the stage route changed while its rules were being loaded. Run a fresh `next` to restart delivery from part 1."
        );
    }

    #[test]
    fn a_part_beyond_the_plan_fails_closed_verbatim() {
        let held = genesis_state(2);
        let graph = definition(2);
        let part1 = first_part(&held, &graph);
        // 正規の連鎖からダイジェスト束縛を写し、索引だけ超過させる。
        let token = part1.continue_token();
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
        let directive = continuing_with(&graph, &held).execute(Some(beyond.build()), &input());
        assert_eq!(
            error_message(&directive),
            "This request asks for a part of the stage rules that does not exist. Run a fresh `next` to restart delivery from part 1."
        );
    }

    #[test]
    fn an_unreadable_bundle_on_continue_is_stale() {
        let held = genesis_state(2);
        let graph = definition(2);
        let part1 = first_part(&held, &graph);
        let directive = continuing(
            FakeDefinitionDao::holding(graph),
            FakeStateDao::holding(held),
            unreadable_rules(),
        )
        .execute(Some(part1.continue_token().clone()), &input());
        assert_eq!(
            error_message(&directive),
            "This stage or its rules changed while they were being loaded, so what has arrived so far is stale. Run a fresh `next` to restart delivery from part 1."
        );
    }

    #[test]
    fn an_unreadable_definition_on_continue_reports_the_stage_gone() {
        let held = genesis_state(2);
        let graph = definition(2);
        let part1 = first_part(&held, &graph);
        let directive = continuing(
            FakeDefinitionDao::failing(WorkflowDefinitionReadError::ScopeFile {
                message: "broken".to_string(),
            }),
            FakeStateDao::holding(held),
            FakeRulesDao::holding(two_part_rules()),
        )
        .execute(Some(part1.continue_token().clone()), &input());
        assert_eq!(
            error_message(&directive),
            "Stage \"stage-0\" no longer exists. Run a fresh `next` after recompiling the stage graph."
        );
    }

    #[test]
    fn a_missing_layout_on_continue_is_stale() {
        let held = genesis_state(2);
        let graph = definition(2);
        let part1 = first_part(&held, &graph);
        let directive = continuing_with(&graph, &held)
            .execute(Some(part1.continue_token().clone()), &NextTurnInput::new());
        assert_eq!(
            error_message(&directive),
            "This stage or its rules changed while they were being loaded, so what has arrived so far is stale. Run a fresh `next` to restart delivery from part 1."
        );
    }

    // ---- state なしの連鎖 (`--single`) ----

    /// state 束縛なしの連鎖 (`--single`) の第 1 部。
    fn stateless_first_part(graph: &DefinitionView) -> LoadSteeringDirective {
        expect_load_steering(
            NextUseCase::new(
                FakeDefinitionDao::holding(graph.clone()),
                FakeStateDao::absent(),
                FakeRulesDao::holding(two_part_rules()),
            )
            .execute(&input().with_single().with_stage("stage-1")),
        )
    }

    #[test]
    fn a_stateless_chain_continues_without_a_read_model() {
        // トークンは state 束縛なしなので、リードモデルが無くても継続できる。
        let graph = definition(2);
        let chain = || {
            continuing(
                FakeDefinitionDao::holding(graph.clone()),
                FakeStateDao::absent(),
                FakeRulesDao::holding(two_part_rules()),
            )
        };
        let part1 = stateless_first_part(&graph);
        let part2 =
            expect_load_steering(chain().execute(Some(part1.continue_token().clone()), &input()));
        let run_stage =
            expect_run_stage(chain().execute(Some(part2.continue_token().clone()), &input()));
        assert!(run_stage.is_single(), "single ピンの再適用");
        assert_eq!(run_stage.stage().as_str(), "stage-1");
    }

    #[test]
    fn a_stateless_continue_without_a_definition_id_fails_closed() {
        let graph = definition(2);
        let part1 = stateless_first_part(&graph);
        let directive = continuing(
            FakeDefinitionDao::failing(WorkflowDefinitionReadError::Unidentified),
            FakeStateDao::absent(),
            FakeRulesDao::holding(two_part_rules()),
        )
        .execute(Some(part1.continue_token().clone()), &input());
        assert_eq!(
            error_message(&directive),
            "This stage or its rules changed while they were being loaded, so what has arrived so far is stale. Run a fresh `next` to restart delivery from part 1."
        );
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

    #[test]
    fn the_fixture_slug_helper_names_the_stages() {
        assert_eq!(slug(1).as_str(), "stage-1");
    }
}
