//! `RunStageView` — `read_run_stage` 1 行の写し (run-stage を組む材料一式)。

/// `read_run_stage` の 1 行 (自然キー `definition_id` × `scope` × `stage_slug` は UNIQUE 索引)。
///
/// # `id` と FK 列を運ぶ
///
/// [`RunStageView::id`] は他の行 (`read_next_answer.run_stage_id`) が指す主キーであり、
/// [`RunStageView::steering_plan_id`] はこの行から配信計画へたどる FK である。View が
/// これらを運ぶのは、**関連をたどるのがユースケースの仕事**だからである
/// (オーナー裁定 2026-09-03 — `coding-rules/cqrs-boundaries.md` 規則 6)。
///
/// # 行の写しであって判断ではない
///
/// フィールドは列そのままの基本データ型で、判断メソッドを持たない。ゲートを開けるか・
/// どのパスを前置するか・どの文言を出すかはこの型の外 (プレゼンタ) の仕事である
/// (設計 §0-5)。
///
/// # パスは相対のまま運ぶ
///
/// `*_rel` の各列は基準ごとの相対パスである (行が絶対パスを持つと、ワークスペースを移した
/// だけで全行が書き替わる)。絶対化はプレゼンタが Layout の対応する dir を前置して行う。
/// JSON 配列の列 (`support_agents` / `consumes_rel` / `produces_rel` /
/// `inline_context_paths_rel` / `sensors_applicable` / `protocol_modules`) も 1 行 JSON の
/// **文字列のまま**運ぶ — 配列へ開くのは描く側であり、途中で形を変えると行との突合せが
/// 効かなくなる。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunStageView {
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

impl RunStageView {
    /// 23 列をそのまま束ねる (**この型の唯一の構築経路**)。
    ///
    /// ビルダーを立てないのは、この型が**行の写し**であり任意フィールドが無いからである
    /// (`StageView` がビルダーを持つのは 28 フィールドの多くが省略可能で既定値を要する
    /// ため)。全列が毎行あるので、ビルダーは「まだ埋めていない」状態を増やすだけで
    /// 何も守らない。取り違えは列名で読む写像 (アダプタ) と全列を突き合わせる契約テストが
    /// 捉える。
    #[expect(
        clippy::too_many_arguments,
        reason = "行の写しの完全コンストラクタ — 全列が必須なので引数がそのまま列になる"
    )]
    #[must_use]
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
    ) -> RunStageView {
        RunStageView {
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
            inline_context_paths_rel,
            stage_file_rel,
            memory_path_rel,
            consumes_rel,
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

    /// 主キー — 他の行の FK (`read_next_answer.run_stage_id`) が指す値。
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// 定義の識別子。
    #[must_use]
    pub fn definition_id(&self) -> &str {
        &self.definition_id
    }

    /// スコープ名。
    #[must_use]
    pub fn scope(&self) -> &str {
        &self.scope
    }

    /// ステージ slug。
    #[must_use]
    pub fn stage_slug(&self) -> &str {
        &self.stage_slug
    }

    /// フェーズの綴り。
    #[must_use]
    pub fn phase(&self) -> &str {
        &self.phase
    }

    /// このステージのフェーズに配る steering 計画を指す FK。
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

    /// 実行様式の綴り。
    #[must_use]
    pub fn mode(&self) -> &str {
        &self.mode
    }

    /// 定義が既定とするゲートの有無 (要求と token のピンが上書きしうる)。
    #[must_use]
    pub const fn gate_default(&self) -> bool {
        self.gate_default
    }

    /// ハーネス根からの相対で並ぶ inline context パスの 1 行 JSON 配列。
    #[must_use]
    pub fn inline_context_paths_rel(&self) -> &str {
        &self.inline_context_paths_rel
    }

    /// ステージ本体の置き場からの相対パス。
    #[must_use]
    pub fn stage_file_rel(&self) -> &str {
        &self.stage_file_rel
    }

    /// record からの相対で指す観察日誌のパス。
    #[must_use]
    pub fn memory_path_rel(&self) -> &str {
        &self.memory_path_rel
    }

    /// record からの相対で並ぶ入力成果物の 1 行 JSON 配列。
    #[must_use]
    pub fn consumes_rel(&self) -> &str {
        &self.consumes_rel
    }

    /// record からの相対で並ぶ出力成果物の 1 行 JSON 配列。
    #[must_use]
    pub fn produces_rel(&self) -> &str {
        &self.produces_rel
    }

    /// このステージで発火するセンサの 1 行 JSON 配列。
    #[must_use]
    pub fn sensors_applicable(&self) -> &str {
        &self.sensors_applicable
    }

    /// レビュアのエージェント名 (レビュー無しなら `None`)。
    #[must_use]
    pub fn reviewer(&self) -> Option<&str> {
        self.reviewer.as_deref()
    }

    /// レビュー往復の上限。
    #[must_use]
    pub const fn reviewer_max_iterations(&self) -> Option<u32> {
        self.reviewer_max_iterations
    }

    /// レビューの厳しさの綴り。
    #[must_use]
    pub fn review_class(&self) -> Option<&str> {
        self.review_class.as_deref()
    }

    /// ステージ手順が読むプロトコルモジュールの 1 行 JSON 配列。
    #[must_use]
    pub fn protocol_modules(&self) -> &str {
        &self.protocol_modules
    }

    /// 次ステージの表示名 (末尾なら `None`)。
    #[must_use]
    pub fn next_stage_name(&self) -> Option<&str> {
        self.next_stage_name.as_deref()
    }

    /// 経路の束縛ダイジェスト (`continue` の照合キー)。
    #[must_use]
    pub fn route_digest(&self) -> &str {
        &self.route_digest
    }

    /// directive の束縛ダイジェスト (`continue` の照合キー)。
    #[must_use]
    pub fn directive_digest(&self) -> &str {
        &self.directive_digest
    }
}
