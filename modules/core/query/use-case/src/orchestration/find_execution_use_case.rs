//! `FindExecutionUseCase` — 実行 1 本の現在地を引く。

use crate::orchestration::{ExecutionDao, ExecutionView, ReadModelReadError};

/// 実行 1 本の現在地を引く。
///
/// 本体は `execute(鍵) = dao.find(鍵)` だけである — 判断・導出・選択・文言組立のどれも
/// 持たない (`coding-rules/cqrs-boundaries.md` 規則 6 の 2026-09-02 追記)。どの鍵で引くかを
/// 決めるのはコントローラ、引いた行をどう描くかはプレゼンタである。
///
/// バインディングはスタティックが既定なので DAO は型パラメータで保持する
/// (`coding-rules/use-case-rules.md` §2)。実装 (`XxxDaoImpl`) には依存しない — 結線は
/// 合成ルートだけが行う (同 §1 の DIP)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FindExecutionUseCase<D: ExecutionDao> {
    dao: D,
}

impl<D: ExecutionDao> FindExecutionUseCase<D> {
    /// 引当の口を注入する (**この型の唯一の構築経路**)。
    #[must_use]
    pub const fn new(dao: D) -> FindExecutionUseCase<D> {
        FindExecutionUseCase { dao }
    }

    /// 実行識別子で現在地を引く。
    ///
    /// # Errors
    ///
    /// リードモデルを引けない ([`ReadModelReadError`])。
    pub fn execute(&self, execution_id: &str) -> Result<Option<ExecutionView>, ReadModelReadError> {
        self.dao.find(execution_id)
    }

    /// 状態の束縛ダイジェストで現在地を引く (`continue` の state 照合)。
    ///
    /// 束縛を照合するかどうかは token がそれを運ぶかで決まる — つまり**要求の形**の分岐な
    /// ので、この口を呼ぶかどうかはコントローラが決める (`coding-rules/cqrs-boundaries.md`
    /// 規則 6 — 要求フラグの分岐は構文的ルーティング)。
    ///
    /// # Errors
    ///
    /// リードモデルを引けない ([`ReadModelReadError`])。
    pub fn execute_by_state_binding(
        &self,
        state_binding: &str,
    ) -> Result<Option<ExecutionView>, ReadModelReadError> {
        self.dao.find_by_state_binding(state_binding)
    }
}
