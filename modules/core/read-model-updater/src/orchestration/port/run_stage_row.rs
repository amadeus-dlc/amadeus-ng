//! `RunStageRow` — `read_run_stage` の 1 行 (run-stage を組む材料一式)。

/// `read_run_stage` の 1 行。主キーは 1 列 `id` (自然キー
/// (`definition_id`, `scope`, `stage_slug`) から導いた代理キー)。`definition_id` は
/// `read_definition.id` を、`steering_plan_id` は `read_steering_plan.id` を指す FK である
/// (束は phase の関数なので、指す先はこのステージのフェーズの計画である)。
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
/// | `consumes_rel` | record | `{artifact}` (種別を問わず全宣言) |
/// | `consumes_brownfield_rel` | record | `{artifact}` (Brownfield で残る宣言) |
/// | `consumes_greenfield_rel` | record | `{artifact}` (Greenfield で残る宣言) |
/// | `produces_rel` | record | `{phase}/{slug}/{artifact}` |
/// | `inline_context_paths_rel` | ハーネス根 | `agents/{agent}.md` |
///
/// 行は値を運ぶだけである。材料から行を組む投影は
/// [`crate::read_tables::ReadTables::project`] が持つ。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunStageRow {
    id: String,
    definition_id: String,
    scope: String,
    stage_slug: String,
    phase: String,
    steering_plan_id: String,
    lead_agent: String,
    support_agents: String,
    mode: String,
    gate_default: bool,
    in_scope: bool,
    inline_context_paths_rel: String,
    stage_file_rel: String,
    memory_path_rel: String,
    consumes_rel: String,
    consumes_brownfield_rel: String,
    consumes_greenfield_rel: String,
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
    /// 行の値を束ねる (**この型の唯一の構築経路**)。
    #[must_use]
    #[allow(
        clippy::too_many_arguments,
        reason = "表の 1 行の全列を唯一の構築口へ渡す — 列と引数の対応を一覧で読めることを優先する"
    )]
    pub const fn new(
        id: String,
        definition_id: String,
        scope: String,
        stage_slug: String,
        phase: String,
        steering_plan_id: String,
        lead_agent: String,
        support_agents: String,
        mode: String,
        gate_default: bool,
        in_scope: bool,
        inline_context_paths_rel: String,
        stage_file_rel: String,
        memory_path_rel: String,
        consumes_rel: String,
        consumes_brownfield_rel: String,
        consumes_greenfield_rel: String,
        produces_rel: String,
        sensors_applicable: String,
        reviewer: Option<String>,
        reviewer_max_iterations: Option<u32>,
        review_class: Option<String>,
        protocol_modules: String,
        next_stage_name: Option<String>,
        route_digest: String,
        directive_digest: String,
    ) -> Self {
        Self {
            id,
            definition_id,
            scope,
            stage_slug,
            phase,
            steering_plan_id,
            lead_agent,
            support_agents,
            mode,
            gate_default,
            in_scope,
            inline_context_paths_rel,
            stage_file_rel,
            memory_path_rel,
            consumes_rel,
            consumes_brownfield_rel,
            consumes_greenfield_rel,
            produces_rel,
            sensors_applicable,
            reviewer,
            reviewer_max_iterations,
            review_class,
            protocol_modules,
            next_stage_name,
            route_digest,
            directive_digest,
        }
    }

    /// 主キー — 自然キー (`definition_id`, `scope`, `stage_slug`) から導いた代理キー。
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// `read_definition.id` を指す FK。
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

    /// `read_steering_plan.id` を指す FK (このステージのフェーズの配信計画)。
    #[must_use]
    pub fn steering_plan_id(&self) -> &str {
        &self.steering_plan_id
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
    ///
    /// [`StageKey::is_gated`]: core_command_domain::orchestration::StageKey::is_gated
    #[must_use]
    pub const fn gate_default(&self) -> bool {
        self.gate_default
    }

    /// このスコープの EXECUTE サブグラフの一員か (**静的グリッド**の値)。
    ///
    /// `--single` の「そのステージはこの scope では読み飛ばされる」ガードが要る材料である
    /// (upstream `emitSingleRunStage` の `subgraphForScope` 検査 — ピン `:4461-4468`)。
    /// recompose のオーバレイは実行の持ち物なのでここには載らない — この行は
    /// **定義 × scope だけで決まる**。
    #[must_use]
    pub const fn in_scope(&self) -> bool {
        self.in_scope
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

    /// record からの相対で並べた上流成果物の 1 行 JSON 配列 (種別を問わず全宣言)。
    #[must_use]
    pub fn consumes_rel(&self) -> &str {
        &self.consumes_rel
    }

    /// Brownfield の作業で残る上流成果物の 1 行 JSON 配列 (`conditional_on: greenfield`
    /// を落としたもの)。
    #[must_use]
    pub fn consumes_brownfield_rel(&self) -> &str {
        &self.consumes_brownfield_rel
    }

    /// Greenfield の作業で残る上流成果物の 1 行 JSON 配列 (`conditional_on: brownfield`
    /// を落としたもの)。
    #[must_use]
    pub fn consumes_greenfield_rel(&self) -> &str {
        &self.consumes_greenfield_rel
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
