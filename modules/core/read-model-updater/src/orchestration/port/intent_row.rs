//! `IntentRow` — `read_intent` の 1 行 (intent 1 件の静的な材料)。

/// `read_intent` の 1 行。主キーは 1 列 `id` = intent の識別子 (集約そのものの表なので
/// 代理キーを作らない)。`definition_id` は `read_definition.id` を指す FK である。
///
/// 値はすべて [`Intent`] のアクセサの写しである。走査結果は 2 つの綴りを持つ
/// (`project_type` は状態ファイル面の `Greenfield` / `Brownfield`、`project_kind` は
/// `stage-graph.json` 面の小文字) ので、**両方を列にする** — どちらか一方に寄せると、
/// 読取側がもう一方の綴りを組み直すことになる。
///
/// 行は値を運ぶだけである。材料から行を組む投影は
/// [`crate::read_tables::ReadTables::project`] が持つ。
///
/// [`Intent`]: core_command_domain::orchestration::Intent
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntentRow {
    id: String,
    execute_count: usize,
    first_stage: Option<String>,
    first_phase: Option<String>,
    definition_id: String,
    definition_revision: String,
    scope: String,
    request: String,
    depth: Option<String>,
    test_strategy: Option<String>,
    review: Option<String>,
    created_at: String,
    project_type: String,
    project_kind: String,
    languages: String,
    frameworks: String,
    build_system: String,
}

impl IntentRow {
    /// 行の値を束ねる (**この型の唯一の構築経路**)。
    #[must_use]
    #[allow(
        clippy::too_many_arguments,
        reason = "表の 1 行の全列を唯一の構築口へ渡す — 列と引数の対応を一覧で読めることを優先する"
    )]
    pub const fn new(
        id: String,
        execute_count: usize,
        first_stage: Option<String>,
        first_phase: Option<String>,
        definition_id: String,
        definition_revision: String,
        scope: String,
        request: String,
        depth: Option<String>,
        test_strategy: Option<String>,
        review: Option<String>,
        created_at: String,
        project_type: String,
        project_kind: String,
        languages: String,
        frameworks: String,
        build_system: String,
    ) -> Self {
        Self {
            id,
            execute_count,
            first_stage,
            first_phase,
            definition_id,
            definition_revision,
            scope,
            request,
            depth,
            test_strategy,
            review,
            created_at,
            project_type,
            project_kind,
            languages,
            frameworks,
            build_system,
        }
    }

    /// 解決済みの実行ステージ件数。
    #[must_use]
    pub const fn execute_count(&self) -> usize {
        self.execute_count
    }

    /// 初期化後の最初のステージ。
    #[must_use]
    pub fn first_stage(&self) -> Option<&str> {
        self.first_stage.as_deref()
    }

    /// 初期化後のフェーズ。
    #[must_use]
    pub fn first_phase(&self) -> Option<&str> {
        self.first_phase.as_deref()
    }

    /// 主キー — intent の識別子 (UUIDv7)。
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// 依拠した定義の系譜 ID。
    #[must_use]
    pub fn definition_id(&self) -> &str {
        &self.definition_id
    }

    /// 依拠した定義の内容版。
    #[must_use]
    pub fn definition_revision(&self) -> &str {
        &self.definition_revision
    }

    /// 選ばれたスコープ名。
    #[must_use]
    pub fn scope(&self) -> &str {
        &self.scope
    }

    /// 人が書いた依頼文 (逐語)。
    #[must_use]
    pub fn request(&self) -> &str {
        &self.request
    }

    /// 詳細度の上書き (無ければ NULL)。
    #[must_use]
    pub fn depth(&self) -> Option<&str> {
        self.depth.as_deref()
    }

    /// テスト戦略の上書き (無ければ NULL)。
    #[must_use]
    pub fn test_strategy(&self) -> Option<&str> {
        self.test_strategy.as_deref()
    }

    /// レビュー階級の上書き (無ければ NULL)。
    #[must_use]
    pub fn review(&self) -> Option<&str> {
        self.review.as_deref()
    }

    /// 誕生時刻 (RFC3339 / 秒精度 / `Z`)。
    #[must_use]
    pub fn created_at(&self) -> &str {
        &self.created_at
    }

    /// 状態ファイル面の種別綴り (`Greenfield` / `Brownfield`)。
    #[must_use]
    pub fn project_type(&self) -> &str {
        &self.project_type
    }

    /// `stage-graph.json` 面の種別綴り (小文字)。
    #[must_use]
    pub fn project_kind(&self) -> &str {
        &self.project_kind
    }

    /// 検出した言語 (未検出は `Unknown`)。
    #[must_use]
    pub fn languages(&self) -> &str {
        &self.languages
    }

    /// 検出したフレームワーク (未検出は `Unknown`)。
    #[must_use]
    pub fn frameworks(&self) -> &str {
        &self.frameworks
    }

    /// 検出したビルドシステム (未検出は `Unknown`)。
    #[must_use]
    pub fn build_system(&self) -> &str {
        &self.build_system
    }
}
