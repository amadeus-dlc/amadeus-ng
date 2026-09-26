//! `IntentStageRow` — `read_intent_stage` の 1 行 (解決済み計画のステージ 1 件)。

/// `read_intent_stage` の 1 行。主キーは 1 列 `id` (自然キー
/// (`intent_id`, `stage_index`) から導いた代理キー)。`intent_id` は `read_intent.id` を
/// 指す FK である。
///
/// 値は [`StageEntry`] とその表示属性 (`StageDisplay`) の写しである。**実行時に動く値
/// (checkbox・実効プラン) はここには無い** — それは実行の表 (`read_execution_stage`) が
/// 持つ。intent の計画は誕生時に確定して以後動かないので、2 つの表は別の理由で変わる。
///
/// 行は値を運ぶだけである。材料から行を組む投影は
/// [`crate::read_tables::ReadTables::project`] が持つ。
///
/// [`StageEntry`]: core_command_domain::orchestration::StageEntry
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntentStageRow {
    id: String,
    intent_id: String,
    stage_index: usize,
    slug: String,
    phase: String,
    plan_action: String,
    conditional: bool,
    number: String,
    name: String,
    lead_agent: String,
    gated: bool,
}

impl IntentStageRow {
    /// 行の値を束ねる (**この型の唯一の構築経路**)。
    #[must_use]
    #[allow(
        clippy::too_many_arguments,
        reason = "表の 1 行の全列を唯一の構築口へ渡す — 列と引数の対応を一覧で読めることを優先する"
    )]
    pub const fn new(
        id: String,
        intent_id: String,
        stage_index: usize,
        slug: String,
        phase: String,
        plan_action: String,
        conditional: bool,
        number: String,
        name: String,
        lead_agent: String,
        gated: bool,
    ) -> Self {
        Self {
            id,
            intent_id,
            stage_index,
            slug,
            phase,
            plan_action,
            conditional,
            number,
            name,
            lead_agent,
            gated,
        }
    }

    /// 主キー — 自然キー (`intent_id`, `stage_index`) から導いた代理キー。
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// `read_intent.id` を指す FK。
    #[must_use]
    pub fn intent_id(&self) -> &str {
        &self.intent_id
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

    /// 計画時の静的な計画 (`EXECUTE` / `SKIP`)。
    #[must_use]
    pub fn plan_action(&self) -> &str {
        &self.plan_action
    }

    /// 条件付き実行のステージか。
    #[must_use]
    pub const fn conditional(&self) -> bool {
        self.conditional
    }

    /// ステージ番号。
    #[must_use]
    pub fn number(&self) -> &str {
        &self.number
    }

    /// 表示名。
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 主担当エージェント。
    #[must_use]
    pub fn lead_agent(&self) -> &str {
        &self.lead_agent
    }

    /// 承認ゲート付きか。
    #[must_use]
    pub const fn gated(&self) -> bool {
        self.gated
    }
}
