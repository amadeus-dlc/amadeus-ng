//! `read_run_stage` の行を組む投影 — 定義のノード 1 件を、あるスコープの run-stage 材料
//! [`RunStageRow`] へ写す。

use core_command_domain::orchestration::StageKey;
use core_command_domain::workflow_definition::{
    BrownfieldGreenfield, PhaseId, ReviewClass, StageMode, StageNode, StageRoute,
    WorkflowDefinitionId,
};

use super::digest;
use super::json_column;
use super::row_id;
use crate::orchestration::RunStageRow;

/// 定義のノード 1 件を、あるスコープの run-stage 材料 1 行へ写す。
///
/// `route` は集約のクエリ [`WorkflowDefinition::stage_route`] の答え、`next_stage_name` は
/// 呼出側が文書順の列から拾った表示名である。どちらもここでは組み直さない。
///
/// [`WorkflowDefinition::stage_route`]: core_command_domain::workflow_definition::WorkflowDefinition::stage_route
pub(super) fn row(
    definition_id: &WorkflowDefinitionId,
    scope: &str,
    node: &StageNode,
    route: &StageRoute,
    next_stage_name: Option<&str>,
    in_scope: bool,
) -> RunStageRow {
    let phase_dir = node.phase().as_str();
    let slug = node.slug().as_str();
    let stage_file_rel = format!("{phase_dir}/{slug}.md");
    let memory_path_rel = format!("{phase_dir}/{slug}/memory.md");
    // reviewer / review_class / reviewer_max_iterations は**対で載る**。定義が
    // reviewer だけを名乗って階級を欠くとき、クエリ側の組み立ては 3 つとも付けない —
    // 階級の無いレビューは回せないので、片方だけ載せると行が嘘をつく。
    let review = node.reviewer().zip(node.review_class());
    let route_digest = digest::route(route.stage(), route.stages_in_scope());
    let directive_digest = digest::directive(
        node.slug(),
        &stage_file_rel,
        &memory_path_rel,
        next_stage_name,
    );
    RunStageRow::new(
        row_id::run_stage(definition_id.as_str(), scope, slug),
        definition_id.as_str().to_string(),
        scope.to_string(),
        slug.to_string(),
        phase_dir.to_string(),
        row_id::steering_plan(phase_dir),
        node.lead_agent().to_string(),
        json_column::strings(node.support_agents()),
        node.mode().as_str().to_string(),
        StageKey::new(node.slug().clone(), node.phase()).is_gated(),
        in_scope,
        json_column::strings(&inline_context_paths(node)),
        stage_file_rel,
        memory_path_rel,
        consumes_for(node, None),
        consumes_for(node, Some(BrownfieldGreenfield::Brownfield)),
        consumes_for(node, Some(BrownfieldGreenfield::Greenfield)),
        json_column::strings(
            &node
                .produces()
                .iter()
                .map(|artifact| format!("{phase_dir}/{slug}/{}", artifact_filename(artifact)))
                .collect::<Vec<_>>(),
        ),
        // run-stageの公開面は参照のIDだけを運ぶ。定義表の完全な参照とは別の列である。
        json_column::strings(
            &node
                .sensors_applicable()
                .iter()
                .map(|sensor| sensor.id().to_string())
                .collect::<Vec<_>>(),
        ),
        review.map(|(reviewer, _)| reviewer.to_string()),
        review.map(|_| {
            node.reviewer_max_iterations()
                .unwrap_or(DEFAULT_REVIEW_ITERATIONS)
        }),
        review.map(|(_, class)| ReviewClass::as_str(class).to_string()),
        json_column::strings(&protocol_modules(node)),
        next_stage_name.map(str::to_string),
        route_digest,
        directive_digest,
    )
}

/// 本家の成果物語彙からファイル名への写像。
fn artifact_filename(name: &str) -> String {
    if name.ends_with(".md") || name.ends_with(".json") {
        return name.to_string();
    }
    match name {
        "build-test-results" | "load-test-results" => "test-results.md".to_string(),
        "traceability" => "traceability.json".to_string(),
        _ => format!("{name}.md"),
    }
}

/// 定義が回数を宣言しないときのレビュー往復上限。
const DEFAULT_REVIEW_ITERATIONS: u32 = 1;

/// 会話にそのまま載せるエージェントペルソナ (ハーネス根からの相対)。
///
/// 様式ごとに誰の声が会話へ入るかが決まる — Inline は lead と support の全員、Mob は
/// 統合役の lead だけ、残り (Subagent / Pipeline / AgentTeam) は別プロセスへ渡すので
/// 会話には載らない。
fn inline_context_paths(node: &StageNode) -> Vec<String> {
    let persona = |agent: &str| format!("agents/{agent}.md");
    match node.mode() {
        StageMode::Inline => {
            let mut paths = vec![persona(node.lead_agent())];
            paths.extend(node.support_agents().iter().map(|agent| persona(agent)));
            paths
        }
        StageMode::Mob => vec![persona(node.lead_agent())],
        StageMode::Subagent | StageMode::Pipeline | StageMode::AgentTeam => Vec::new(),
    }
}

/// 追加で読み込むプロトコルモジュール (宣言の写像 — 順序も宣言どおり)。
fn protocol_modules(node: &StageNode) -> Vec<String> {
    let mut modules = Vec::new();
    if node.reviewer().is_some() {
        modules.push("reviewer".to_string());
    }
    if node.mode() != StageMode::Inline || !node.support_agents().is_empty() {
        modules.push("ensemble".to_string());
    }
    if node.phase() == PhaseId::Construction {
        modules.push("construction".to_string());
    }
    modules
}

/// ある種別の作業で残る `consumes[]` の語彙名を 1 行 JSON 配列にする。
///
/// 本家 `resolveConsumes` (`aidlc-orchestrate.ts:2519-2534` @a277af21) は
/// `conditional_on` が作業の種別と食い違う宣言を落とし、種別が不明 (`None`) なら全宣言を
/// 残す。行は 3 通りすべてを持ち、どれを読むかは描く側が作業の種別で選ぶ。
fn consumes_for(node: &StageNode, kind: Option<BrownfieldGreenfield>) -> String {
    json_column::strings(
        &node
            .consumes()
            .iter()
            .filter(|consume| match (consume.conditional_on(), kind) {
                (Some(condition), Some(kind)) => condition == kind,
                _ => true,
            })
            .map(|consume| consume.artifact().to_string())
            .collect::<Vec<_>>(),
    )
}
