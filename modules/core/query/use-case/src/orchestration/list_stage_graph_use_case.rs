//! `ListStageGraphUseCase` — 有効な全ステージをグラフ順で引く (`stage-table` の材料)。

use crate::orchestration::{ReadModelReadError, StageGraphDao, StageGraphEntryView};

/// 有効な全ノードをグラフ順で引く。
///
/// 本体は `execute() = dao.find_all()` だけである — 判断・導出・選択・文言組立のどれも
/// 持たない (`coding-rules/cqrs-boundaries.md` 規則 6)。Markdown 表への整形は出す側の
/// 仕事である。
///
/// バインディングはスタティックが既定なので DAO は型パラメータで保持する
/// (`coding-rules/use-case-rules.md` §2)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListStageGraphUseCase<D: StageGraphDao> {
    stage_graph_dao: D,
}

impl<D: StageGraphDao> ListStageGraphUseCase<D> {
    /// 引当の口を注入する (**この型の唯一の構築経路**)。
    #[must_use]
    pub const fn new(stage_graph_dao: D) -> ListStageGraphUseCase<D> {
        ListStageGraphUseCase { stage_graph_dao }
    }

    /// 有効な全ノードをグラフ順で引く。
    ///
    /// # Errors
    ///
    /// リードモデルを引けない ([`ReadModelReadError`])。
    pub fn execute(&self) -> Result<Vec<StageGraphEntryView>, ReadModelReadError> {
        self.stage_graph_dao.find_all()
    }
}
