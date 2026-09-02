//! `RunStageRow` — `read_run_stage` の 1 行 (run-stage を組む材料一式)。

use core_command_domain::orchestration::StageKey;
use core_command_domain::workflow_definition::{
    PhaseId, ReviewClass, StageMode, StageNode, StageRoute, WorkflowDefinitionId,
};

use super::digest;
use super::json_column;

/// `read_run_stage` の 1 行。主キーは (`definition_id`, `scope`, `stage_slug`)。
///
/// # 定義 × scope で決まる (実行には依らない)
///
/// run-stage directive の材料のうち、実行の状態で変わるのは pins (`gate` の上書き・`unit`・
/// `single`) だけである。それらは要求と token が運ぶので行には載らず、行が持つのは
/// **定義とスコープグリッドだけで決まる部分**である。したがって同じ定義・同じ scope なら、
/// どの実行から引いても同じ行が返る。
///
/// # パスは相対である
///
/// 絶対パスにすると、ワークスペースを移しただけで全行が書き替わる。行は**基準ごとの相対**で
/// 持ち、プレゼンタが Layout の対応する dir を前置する。
///
/// | 列 | 基準 | 綴り |
/// | --- | --- | --- |
/// | `stage_file_rel` | ステージ本体の置き場 | `{phase}/{slug}.md` |
/// | `memory_path_rel` | record | `{phase}/{slug}/memory.md` |
/// | `consumes_rel` | record | `{artifact}` |
/// | `produces_rel` | record | `{phase}/{slug}/{artifact}` |
/// | `inline_context_paths_rel` | ハーネス根 | `agents/{agent}.md` |
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunStageRow {
    definition_id: String,
    scope: String,
    stage_slug: String,
    phase: String,
    lead_agent: String,
    support_agents: String,
    mode: String,
    gate_default: bool,
    inline_context_paths_rel: String,
    stage_file_rel: String,
    memory_path_rel: String,
    consumes_rel: String,
    produces_rel: String,
    sensors_applicable: String,
    reviewer: Option<String>,
    reviewer_max_iterations: Option<u32>,
    review_class: Option<String>,
    protocol_modules: String,
    next_stage_name: Option<String>,
    route_digest: String,
    directive_digest: String,
}

impl RunStageRow {
    /// 定義のノード 1 件を、あるスコープの run-stage 材料 1 行へ写す
    /// (**この型の唯一の構築経路**)。
    ///
    /// `route` は集約のクエリ [`WorkflowDefinition::stage_route`] の答え、`next_stage_name` は
    /// 呼出側が文書順の列から拾った表示名である。どちらもこの型は組み直さない。
    ///
    /// [`WorkflowDefinition::stage_route`]: core_command_domain::workflow_definition::WorkflowDefinition::stage_route
    #[must_use]
    pub fn of(
        definition_id: &WorkflowDefinitionId,
        scope: &str,
        node: &StageNode,
        route: &StageRoute,
        next_stage_name: Option<&str>,
    ) -> RunStageRow {
        let phase_dir = node.phase().as_str();
        let slug = node.slug().as_str();
        let stage_file_rel = format!("{phase_dir}/{slug}.md");
        let memory_path_rel = format!("{phase_dir}/{slug}/memory.md");
        // reviewer / review_class / reviewer_max_iterations は**対で載る**。定義が
        // reviewer だけを名乗って階級を欠くとき、クエリ側の組み立ては 3 つとも付けない —
        // 階級の無いレビューは回せないので、片方だけ載せると行が嘘をつく。
        let review = node.reviewer().zip(node.review_class());
        RunStageRow {
            definition_id: definition_id.as_str().to_string(),
            scope: scope.to_string(),
            stage_slug: slug.to_string(),
            phase: phase_dir.to_string(),
            lead_agent: node.lead_agent().to_string(),
            support_agents: json_column::strings(node.support_agents()),
            mode: node.mode().as_str().to_string(),
            gate_default: StageKey::new(node.slug().clone(), node.phase()).is_gated(),
            inline_context_paths_rel: json_column::strings(&inline_context_paths(node)),
            consumes_rel: json_column::strings(
                &node
                    .consumes()
                    .iter()
                    .map(|consume| consume.artifact().to_string())
                    .collect::<Vec<_>>(),
            ),
            produces_rel: json_column::strings(
                &node
                    .produces()
                    .iter()
                    .map(|artifact| format!("{phase_dir}/{slug}/{artifact}"))
                    .collect::<Vec<_>>(),
            ),
            sensors_applicable: json_column::sensors_applicable(node.sensors_applicable()),
            reviewer: review.map(|(reviewer, _)| reviewer.to_string()),
            reviewer_max_iterations: review.map(|_| {
                node.reviewer_max_iterations()
                    .unwrap_or(DEFAULT_REVIEW_ITERATIONS)
            }),
            review_class: review.map(|(_, class)| ReviewClass::as_str(class).to_string()),
            protocol_modules: json_column::strings(&protocol_modules(node)),
            next_stage_name: next_stage_name.map(str::to_string),
            route_digest: digest::route(route.stage(), route.stages_in_scope()),
            directive_digest: digest::directive(
                node.slug(),
                &stage_file_rel,
                &memory_path_rel,
                next_stage_name,
            ),
            stage_file_rel,
            memory_path_rel,
        }
    }

    /// 定義の系譜 ID。
    #[must_use]
    pub fn definition_id(&self) -> &str {
        &self.definition_id
    }

    /// スコープ名。
    #[must_use]
    pub fn scope(&self) -> &str {
        &self.scope
    }

    /// ステージの slug。
    #[must_use]
    pub fn stage_slug(&self) -> &str {
        &self.stage_slug
    }

    /// フェーズの綴り (`PhaseId::as_str` — 相対パスのディレクトリ名でもある)。
    #[must_use]
    pub fn phase(&self) -> &str {
        &self.phase
    }

    /// 主担当エージェント。
    #[must_use]
    pub fn lead_agent(&self) -> &str {
        &self.lead_agent
    }

    /// 支援エージェントの 1 行 JSON 配列。
    #[must_use]
    pub fn support_agents(&self) -> &str {
        &self.support_agents
    }

    /// 実行様式の綴り (`StageMode::as_str`)。
    #[must_use]
    pub fn mode(&self) -> &str {
        &self.mode
    }

    /// 定義側の既定ゲート ([`StageKey::is_gated`] の答え — 要求の上書きは含まない)。
    #[must_use]
    pub const fn gate_default(&self) -> bool {
        self.gate_default
    }

    /// ハーネス根からの相対で並べたエージェントペルソナの 1 行 JSON 配列。
    #[must_use]
    pub fn inline_context_paths_rel(&self) -> &str {
        &self.inline_context_paths_rel
    }

    /// ステージ本体の置き場からの相対パス (`{phase}/{slug}.md`)。
    #[must_use]
    pub fn stage_file_rel(&self) -> &str {
        &self.stage_file_rel
    }

    /// record からの相対パス (`{phase}/{slug}/memory.md`)。
    #[must_use]
    pub fn memory_path_rel(&self) -> &str {
        &self.memory_path_rel
    }

    /// record からの相対で並べた上流成果物の 1 行 JSON 配列。
    #[must_use]
    pub fn consumes_rel(&self) -> &str {
        &self.consumes_rel
    }

    /// record からの相対で並べた産出成果物の 1 行 JSON 配列。
    #[must_use]
    pub fn produces_rel(&self) -> &str {
        &self.produces_rel
    }

    /// 適用センサー宣言の 1 行 JSON 配列。
    #[must_use]
    pub fn sensors_applicable(&self) -> &str {
        &self.sensors_applicable
    }

    /// レビュアーのエージェント名 (階級と対で載る。片方だけなら NULL)。
    #[must_use]
    pub fn reviewer(&self) -> Option<&str> {
        self.reviewer.as_deref()
    }

    /// レビュー往復の上限 (宣言が無ければ既定 1。reviewer が無ければ NULL)。
    #[must_use]
    pub const fn reviewer_max_iterations(&self) -> Option<u32> {
        self.reviewer_max_iterations
    }

    /// レビュー階級の綴り (reviewer と対で載る)。
    #[must_use]
    pub fn review_class(&self) -> Option<&str> {
        self.review_class.as_deref()
    }

    /// 追加で読み込むプロトコルモジュールの 1 行 JSON 配列。
    #[must_use]
    pub fn protocol_modules(&self) -> &str {
        &self.protocol_modules
    }

    /// 文書順で自ノードの後にある最初の in-scope EXECUTE ステージの**表示名**。
    #[must_use]
    pub fn next_stage_name(&self) -> Option<&str> {
        self.next_stage_name.as_deref()
    }

    /// route 束縛のダイジェスト (対象ステージ + scope の顔ぶれ)。
    #[must_use]
    pub fn route_digest(&self) -> &str {
        &self.route_digest
    }

    /// directive 束縛のダイジェスト (環境由来の 4 キー — pins を含まない)。
    #[must_use]
    pub fn directive_digest(&self) -> &str {
        &self.directive_digest
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
