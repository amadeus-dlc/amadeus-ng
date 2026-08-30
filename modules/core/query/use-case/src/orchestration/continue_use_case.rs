//! `Continue` — steering 連鎖の継続 (トークン検証 → ディスク状態からの再構築 → 次部
//! または終端 run-stage — 02 §10 / §4.4)。
//!
//! `next` と同じく**読むだけの動詞**なのでクエリ側にある (`coding-rules/cqrs-boundaries.md`
//! 規則 5)。ポートを 1 本も持たず、読み終えた読取素材を値で受ける。
//!
//! **キャッシュを信用しない** (再構築原則 `:5996-6037`): 現在のディスク状態から run-stage と
//! ルール束を作り直し、トークンのピン (`gate` / `next_stage` / `unit` / `single`) を再適用し、
//! ダイジェスト束縛 (bundle / directive / route / state) を照合する。ドリフトは**すべて
//! fail-closed** — fresh `next` からのやり直しだけを指示する (I12)。ピンの再適用は
//! [`RunStageDirective::with_pins`] (不変オブジェクトの部分更新)、束縛の照合は型付き
//! ダイジェスト ([`Bindings`]) の等値比較で行う。
//!
//! [`RunStageDirective::with_pins`]: super::RunStageDirective::with_pins

use super::continue_token::ContinueToken;
use super::directive::Directive;
use super::next_turn_input::NextTurnInput;
use super::sources::{DefinitionSource, ExecutionStateSource, SteeringSource};
use super::steering_binding::{Bindings, StateBinding};
use crate::workflow_view::DefinitionView;

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

/// steering 連鎖の継続 (読取専用 — ポートを 1 本も持たない)。
///
/// 注入はゼロである ([`super::NextUseCase`] と同じ形) — 読取結果はすべて `execute` の
/// 引数で値として受ける。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContinueUseCase;

impl ContinueUseCase {
    /// 開封済みトークン 1 つを directive ちょうど 1 つに写す。
    ///
    /// 開封 (base64url 復号 + MAC 検証 + 厳密型表) は Controller (U7) の責務であり、失敗は
    /// `None` で渡される — 契約は fail-closed の逐語文言 1 本である。
    ///
    /// `steering` はルール束の読取結果。ここでの失敗はすべて `STALE` に畳む — 継続の契約は
    /// 「やり直せ」1 本であり、原因の区別を漏らさない (I12)。
    #[must_use]
    pub fn execute(
        token: Option<ContinueToken>,
        state: ExecutionStateSource<'_>,
        definition: DefinitionSource<'_>,
        steering: SteeringSource<'_>,
        input: &NextTurnInput,
    ) -> Directive {
        let Some(token) = token else {
            return Directive::Error {
                message: wording::INVALID_TOKEN.to_string(),
            };
        };
        // state 束縛 — 現在の state ダイジェストと照合する。
        let state = match Self::state_binding(&token, state) {
            Ok(state) => state,
            Err(directive) => return *directive,
        };
        let definition = match Self::definition_of(&token, definition) {
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
        let plan = match steering {
            // 読めない・分割不能のどちらも継続はできない — 区別せず fail-closed。
            SteeringSource::Loaded(rules) => match rules.plan_for(node.phase()) {
                Ok(plan) => plan,
                Err(_) => {
                    return Directive::Error {
                        message: wording::STALE.to_string(),
                    };
                }
            },
            SteeringSource::Unreadable { .. } => {
                return Directive::Error {
                    message: wording::STALE.to_string(),
                };
            }
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
    /// リードモデルが無い・読めないのはどちらも「保存された位置が動いた」扱いで fail-closed
    /// にする — 束縛を確かめられない以上、続きを届けてはならない。
    fn state_binding(
        token: &ContinueToken,
        state: ExecutionStateSource<'_>,
    ) -> Result<Option<StateBinding>, Box<Directive>> {
        let Some(bound) = token.bindings().state() else {
            return Ok(None);
        };
        let moved_on = || {
            Box::new(Directive::Error {
                message: wording::STATE_MOVED_ON.to_string(),
            })
        };
        let ExecutionStateSource::Loaded(view) = state else {
            return Err(moved_on());
        };
        let current = view.state_binding();
        if &current != bound {
            return Err(moved_on());
        }
        Ok(Some(current))
    }

    /// 定義ビューの取り出し — 読めない・特定できないは fail-closed。
    ///
    /// どちらの逐語文言になるかは失敗の種類で決まる: 定義 id が特定できないのは連鎖の材料が
    /// 欠けている (`STALE`)、読めないのはステージへ到達できない (`stage_gone`)。
    fn definition_of<'a>(
        token: &ContinueToken,
        definition: DefinitionSource<'a>,
    ) -> Result<&'a DefinitionView, Box<Directive>> {
        match definition {
            DefinitionSource::Loaded(view) => Ok(view),
            DefinitionSource::Unidentified => Err(Box::new(Directive::Error {
                message: wording::STALE.to_string(),
            })),
            DefinitionSource::Unreadable(_) => Err(Box::new(Directive::Error {
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
    use super::super::directive::{LoadSteeringDirective, RuleContent, RunStageDirective};
    use super::super::memory_rules::MemoryRules;
    use super::super::next_turn_input::WorkspaceLayout;
    use super::super::next_use_case::NextUseCase;
    use super::super::steering_plan::PartIndex;
    use super::super::test_fixtures::{definition, genesis_state, slug, state};
    use super::*;
    use crate::execution_view::{CheckboxState, ExecutionStateView};
    use crate::workflow_view::{
        DefinitionIdView, DefinitionRevisionView, ExecutionKindView, PhaseView, PlanActionView,
        ScopeGridView, ScopeMetadataView, StageGraphView, StageModeView, StageNumberView,
        StageSlugView, StageViewBuilder,
    };

    /// 在るのに読めないルールファイル。
    const UNREADABLE_RULES: SteeringSource<'static> = SteeringSource::Unreadable {
        path: "aidlc/spaces/default/memory/org.md",
        cause: "permission denied",
    };

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
        let rules = two_part_rules();
        expect_load_steering(NextUseCase::execute(
            ExecutionStateSource::Loaded(held),
            DefinitionSource::Loaded(graph),
            SteeringSource::Loaded(&rules),
            &input(),
        ))
    }

    #[test]
    fn an_invalid_continuation_token_fails_closed_verbatim() {
        let held = genesis_state(2);
        let graph = definition(2);
        let directive = ContinueUseCase::execute(
            None,
            ExecutionStateSource::Loaded(&held),
            DefinitionSource::Loaded(&graph),
            SteeringSource::Loaded(&two_part_rules()),
            &input(),
        );
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
        let directive = ContinueUseCase::execute(
            Some(part1.continue_token().clone()),
            ExecutionStateSource::Loaded(&moved),
            DefinitionSource::Loaded(&graph),
            SteeringSource::Loaded(&two_part_rules()),
            &input(),
        );
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
            ExecutionStateSource::Missing,
            ExecutionStateSource::Unreadable("State file not found"),
        ] {
            let directive = ContinueUseCase::execute(
                Some(part1.continue_token().clone()),
                absent,
                DefinitionSource::Loaded(&graph),
                SteeringSource::Loaded(&two_part_rules()),
                &input(),
            );
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
        let directive = ContinueUseCase::execute(
            Some(part1.continue_token().clone()),
            ExecutionStateSource::Loaded(&held),
            DefinitionSource::Loaded(&graph),
            SteeringSource::Loaded(&rewritten),
            &input(),
        );
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
        let directive = ContinueUseCase::execute(
            Some(part1.continue_token().clone()),
            ExecutionStateSource::Loaded(&held),
            DefinitionSource::Loaded(&recompiled),
            SteeringSource::Loaded(&two_part_rules()),
            &input(),
        );
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
        let directive = ContinueUseCase::execute(
            Some(part1.continue_token().clone()),
            ExecutionStateSource::Loaded(&held),
            DefinitionSource::Loaded(&definition(3)),
            SteeringSource::Loaded(&two_part_rules()),
            &input(),
        );
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
        let directive = ContinueUseCase::execute(
            Some(beyond.build()),
            ExecutionStateSource::Loaded(&held),
            DefinitionSource::Loaded(&graph),
            SteeringSource::Loaded(&two_part_rules()),
            &input(),
        );
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
        let directive = ContinueUseCase::execute(
            Some(part1.continue_token().clone()),
            ExecutionStateSource::Loaded(&held),
            DefinitionSource::Loaded(&graph),
            UNREADABLE_RULES,
            &input(),
        );
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
        let directive = ContinueUseCase::execute(
            Some(part1.continue_token().clone()),
            ExecutionStateSource::Loaded(&held),
            DefinitionSource::Unreadable("broken"),
            SteeringSource::Loaded(&two_part_rules()),
            &input(),
        );
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
        let directive = ContinueUseCase::execute(
            Some(part1.continue_token().clone()),
            ExecutionStateSource::Loaded(&held),
            DefinitionSource::Loaded(&graph),
            SteeringSource::Loaded(&two_part_rules()),
            &NextTurnInput::new(),
        );
        assert_eq!(
            error_message(&directive),
            "This stage or its rules changed while they were being loaded, so what has arrived so far is stale. Run a fresh `next` to restart delivery from part 1."
        );
    }

    // ---- state なしの連鎖 (`--single`) ----

    #[test]
    fn a_stateless_chain_continues_without_a_read_model() {
        // トークンは state 束縛なしなので、リードモデルが無くても継続できる。
        let rules = two_part_rules();
        let steering = SteeringSource::Loaded(&rules);
        let graph = definition(2);
        let turn = input().with_single().with_stage("stage-1");
        let part1 = expect_load_steering(NextUseCase::execute(
            ExecutionStateSource::Missing,
            DefinitionSource::Loaded(&graph),
            steering,
            &turn,
        ));
        let part2 = expect_load_steering(ContinueUseCase::execute(
            Some(part1.continue_token().clone()),
            ExecutionStateSource::Missing,
            DefinitionSource::Loaded(&graph),
            steering,
            &input(),
        ));
        let run_stage = expect_run_stage(ContinueUseCase::execute(
            Some(part2.continue_token().clone()),
            ExecutionStateSource::Missing,
            DefinitionSource::Loaded(&graph),
            steering,
            &input(),
        ));
        assert!(run_stage.is_single(), "single ピンの再適用");
        assert_eq!(run_stage.stage().as_str(), "stage-1");
    }

    #[test]
    fn a_stateless_continue_without_a_definition_id_fails_closed() {
        let rules = two_part_rules();
        let steering = SteeringSource::Loaded(&rules);
        let graph = definition(2);
        let turn = input().with_single().with_stage("stage-1");
        let part1 = expect_load_steering(NextUseCase::execute(
            ExecutionStateSource::Missing,
            DefinitionSource::Loaded(&graph),
            steering,
            &turn,
        ));
        let directive = ContinueUseCase::execute(
            Some(part1.continue_token().clone()),
            ExecutionStateSource::Missing,
            DefinitionSource::Unidentified,
            steering,
            &input(),
        );
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
