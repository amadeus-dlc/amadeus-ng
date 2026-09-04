//! `FindDefinitionStageUseCase` — グラフのステージ 1 行を引く。

use crate::orchestration::{DefinitionStageDao, DefinitionStageView, ReadModelReadError};

/// グラフのステージ 1 行を引く。
///
/// 本体は `execute(鍵) = dao.find(鍵)` だけである — 判断・導出・選択・文言組立のどれも
/// 持たない (`coding-rules/cqrs-boundaries.md` 規則 6 の 2026-09-02 追記)。
/// 「practices-discovery がグラフに在るか」の判断は**行が引けたかどうか**であり、
/// それを読むのは合成ルートである。
///
/// バインディングはスタティックが既定なので DAO は型パラメータで保持する
/// (`coding-rules/use-case-rules.md` §2)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FindDefinitionStageUseCase<D: DefinitionStageDao> {
    definition_stage_dao: D,
}

impl<D: DefinitionStageDao> FindDefinitionStageUseCase<D> {
    /// 引当の口を注入する (**この型の唯一の構築経路**)。
    #[must_use]
    pub const fn new(definition_stage_dao: D) -> FindDefinitionStageUseCase<D> {
        FindDefinitionStageUseCase {
            definition_stage_dao,
        }
    }

    /// 定義識別子と slug でステージ 1 行を引く。
    ///
    /// # Errors
    ///
    /// リードモデルを引けない ([`ReadModelReadError`])。
    pub fn execute(
        &self,
        definition_id: &str,
        stage_slug: &str,
    ) -> Result<Option<DefinitionStageView>, ReadModelReadError> {
        self.definition_stage_dao.find(definition_id, stage_slug)
    }
}
