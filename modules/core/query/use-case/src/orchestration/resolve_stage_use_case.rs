//! `ResolveStageUseCase` — slug か番号でステージ 1 ノードを引く (upstream `resolveStage`)。

use crate::orchestration::{ReadModelReadError, StageGraphDao, StageGraphEntryView};

/// slug または番号で 1 ノードを引く。
///
/// 本体は `execute(鍵) = dao.find(鍵)` だけである — 判断・導出・選択・文言組立のどれも
/// 持たない (`coding-rules/cqrs-boundaries.md` 規則 6 の 2026-09-02 追記)。
/// `phase-of` / `agent-for` / `validate-stage` はいずれもこの 1 引当の上に、出す側が列を
/// 選んで描くものである。
///
/// バインディングはスタティックが既定なので DAO は型パラメータで保持する
/// (`coding-rules/use-case-rules.md` §2)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolveStageUseCase<D: StageGraphDao> {
    stage_graph_dao: D,
}

impl<D: StageGraphDao> ResolveStageUseCase<D> {
    /// 引当の口を注入する (**この型の唯一の構築経路**)。
    #[must_use]
    pub const fn new(stage_graph_dao: D) -> ResolveStageUseCase<D> {
        ResolveStageUseCase { stage_graph_dao }
    }

    /// slug か番号で 1 ノードを引く。
    ///
    /// # Errors
    ///
    /// リードモデルを引けない ([`ReadModelReadError`])。
    pub fn execute(
        &self,
        slug_or_number: &str,
    ) -> Result<Option<StageGraphEntryView>, ReadModelReadError> {
        self.stage_graph_dao.find(slug_or_number)
    }
}
