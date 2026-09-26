//! `ExecutionStageRow` — `read_execution_stage` の 1 行 (実行 × ステージの実行時状態)。

/// `read_execution_stage` の 1 行。主キーは 1 列 `id` (自然キー
/// (`execution_id`, `stage_index`) から導いた代理キー)。`execution_id` は
/// `read_execution.id` を指す FK である。
///
/// 値はすべて集約のステージ単位クエリの答えである — `checkbox` / `effective_plan` /
/// `approved` / `revision_count` / `gated`。**実効プランは静的グリッドではない**
/// (recompose のオーバレイが勝つ) ので、`read_intent_stage.plan_action` とは別の列である。
///
/// 行は値を運ぶだけである。材料から行を組む投影は
/// [`crate::read_tables::ReadTables::project`] が持つ。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionStageRow {
    id: String,
    execution_id: String,
    stage_index: usize,
    slug: String,
    phase: String,
    checkbox: Option<String>,
    effective_plan: Option<String>,
    approved: Option<bool>,
    revision_count: Option<u32>,
    gated: Option<bool>,
}

impl ExecutionStageRow {
    /// 行の値を束ねる (**この型の唯一の構築経路**)。
    #[must_use]
    #[allow(
        clippy::too_many_arguments,
        reason = "表の 1 行の全列を唯一の構築口へ渡す — 列と引数の対応を一覧で読めることを優先する"
    )]
    pub const fn new(
        id: String,
        execution_id: String,
        stage_index: usize,
        slug: String,
        phase: String,
        checkbox: Option<String>,
        effective_plan: Option<String>,
        approved: Option<bool>,
        revision_count: Option<u32>,
        gated: Option<bool>,
    ) -> Self {
        Self {
            id,
            execution_id,
            stage_index,
            slug,
            phase,
            checkbox,
            effective_plan,
            approved,
            revision_count,
            gated,
        }
    }

    /// 主キー — 自然キー (`execution_id`, `stage_index`) から導いた代理キー。
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// `read_execution.id` を指す FK。
    #[must_use]
    pub fn execution_id(&self) -> &str {
        &self.execution_id
    }

    /// 文書順の位置 (0 始まり)。
    #[must_use]
    pub const fn stage_index(&self) -> usize {
        self.stage_index
    }

    /// ステージの slug。
    #[must_use]
    pub fn slug(&self) -> &str {
        &self.slug
    }

    /// フェーズの綴り。
    #[must_use]
    pub fn phase(&self) -> &str {
        &self.phase
    }

    /// 観測 checkbox の綴り (`pending` … `skipped`)。
    #[must_use]
    pub fn checkbox(&self) -> Option<&str> {
        self.checkbox.as_deref()
    }

    /// 実効プラン (recompose のオーバレイ反映後)。
    #[must_use]
    pub fn effective_plan(&self) -> Option<&str> {
        self.effective_plan.as_deref()
    }

    /// ゲートが承認済みか。
    #[must_use]
    pub const fn approved(&self) -> Option<bool> {
        self.approved
    }

    /// 差し戻し回数。
    #[must_use]
    pub const fn revision_count(&self) -> Option<u32> {
        self.revision_count
    }

    /// このステージが承認ゲートを要するか。
    #[must_use]
    pub const fn gated(&self) -> Option<bool> {
        self.gated
    }
}
