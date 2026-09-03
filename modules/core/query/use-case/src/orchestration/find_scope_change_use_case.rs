//! `FindScopeChangeUseCase` — 要求 scope と state の scope の照合結果を引く。

use crate::orchestration::{ReadModelReadError, ScopeChangeDao, ScopeChangeView};

/// 要求 scope と state の scope の照合結果を引く。
///
/// 本体は `execute(鍵) = dao.find(鍵)` だけである — 判断・導出・選択・文言組立のどれも
/// 持たない (`coding-rules/cqrs-boundaries.md` 規則 6 の 2026-09-02 追記)。どの鍵で引くかを
/// 決めるのはコントローラ、引いた行をどう描くかはプレゼンタである。
///
/// バインディングはスタティックが既定なので DAO は型パラメータで保持する
/// (`coding-rules/use-case-rules.md` §2)。実装 (`XxxDaoImpl`) には依存しない — 結線は
/// 合成ルートだけが行う (同 §1 の DIP)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FindScopeChangeUseCase<D: ScopeChangeDao> {
    scope_change_dao: D,
}

impl<D: ScopeChangeDao> FindScopeChangeUseCase<D> {
    /// 引当の口を注入する (**この型の唯一の構築経路**)。
    #[must_use]
    pub const fn new(scope_change_dao: D) -> FindScopeChangeUseCase<D> {
        FindScopeChangeUseCase { scope_change_dao }
    }

    /// 実行 × 要求 scope で照合結果を引く。行が無ければ無効な scope である。
    ///
    /// # Errors
    ///
    /// リードモデルを引けない ([`ReadModelReadError`])。
    pub fn execute(
        &self,
        execution_id: &str,
        scope: &str,
    ) -> Result<Option<ScopeChangeView>, ReadModelReadError> {
        self.scope_change_dao.find(execution_id, scope)
    }
}
