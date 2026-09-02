//! `IntentStageRow` — `read_intent_stage` の 1 行 (解決済み計画のステージ 1 件)。

use core_command_domain::orchestration::{IntentId, StageEntry};

/// `read_intent_stage` の 1 行。主キーは (`intent_id`, `stage_index`)。
///
/// 値は [`StageEntry`] とその表示属性 (`StageDisplay`) の写しである。**実行時に動く値
/// (checkbox・実効プラン) はここには無い** — それは実行の表 (`read_execution_stage`) が
/// 持つ。intent の計画は誕生時に確定して以後動かないので、2 つの表は別の理由で変わる。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntentStageRow {
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
    /// 計画のステージ 1 件を 1 行へ写す (**この型の唯一の構築経路**)。
    #[must_use]
    pub fn of(intent_id: &IntentId, stage_index: usize, entry: &StageEntry) -> IntentStageRow {
        IntentStageRow {
            intent_id: intent_id.as_str().to_string(),
            stage_index,
            slug: entry.slug().as_str().to_string(),
            phase: entry.phase().as_str().to_string(),
            plan_action: entry.plan_action().as_str().to_string(),
            conditional: entry.is_conditional(),
            number: entry.display().number().as_str().to_string(),
            name: entry.display().name().to_string(),
            lead_agent: entry.display().lead_agent().to_string(),
            gated: entry.is_gated(),
        }
    }

    /// intent の識別子。
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
