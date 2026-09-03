//! `FindScopeUseCase` — scope カタログ 1 列を引く。

use crate::orchestration::{ReadModelReadError, ScopeDao, ScopeView};

/// scope カタログ 1 列を引く。
///
/// 本体は `execute(鍵) = dao.find(鍵)` だけである — 判断・導出・選択・文言組立のどれも
/// 持たない (`coding-rules/cqrs-boundaries.md` 規則 6 の 2026-09-02 追記)。どの鍵で引くかを
/// 決めるのはコントローラ、引いた行をどう描くかはプレゼンタである。
///
/// バインディングはスタティックが既定なので DAO は型パラメータで保持する
/// (`coding-rules/use-case-rules.md` §2)。実装 (`XxxDaoImpl`) には依存しない — 結線は
/// 合成ルートだけが行う (同 §1 の DIP)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FindScopeUseCase<D: ScopeDao> {
    scope_dao: D,
}

impl<D: ScopeDao> FindScopeUseCase<D> {
    /// 引当の口を注入する (**この型の唯一の構築経路**)。
    #[must_use]
    pub const fn new(scope_dao: D) -> FindScopeUseCase<D> {
        FindScopeUseCase { scope_dao }
    }

    /// 定義 × scope 名で 1 列を引く。行が返ること自体が「有効な scope」の答えである。
    ///
    /// # Errors
    ///
    /// リードモデルを引けない ([`ReadModelReadError`])。
    pub fn execute(
        &self,
        definition_id: &str,
        scope: &str,
    ) -> Result<Option<ScopeView>, ReadModelReadError> {
        self.scope_dao.find(definition_id, scope)
    }

    /// 既製 3 scope (`express` / `classic` / `feature`) をその順で引く。
    ///
    /// # Errors
    ///
    /// リードモデルを引けない ([`ReadModelReadError`])。
    pub fn execute_stock(&self, definition_id: &str) -> Result<Vec<ScopeView>, ReadModelReadError> {
        self.scope_dao.find_stock(definition_id)
    }
}
