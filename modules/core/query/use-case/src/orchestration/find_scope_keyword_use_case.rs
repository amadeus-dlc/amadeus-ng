//! `FindScopeKeywordUseCase` — キーワードから scope 名を引く。

use crate::orchestration::{ReadModelReadError, ScopeKeywordDao};

/// キーワードから scope 名を引く。
///
/// 本体は `execute(鍵) = dao.find(鍵)` だけである — 判断・導出・選択・文言組立のどれも
/// 持たない (`coding-rules/cqrs-boundaries.md` 規則 6 の 2026-09-02 追記)。どの鍵で引くかを
/// 決めるのはコントローラ、引いた行をどう描くかはプレゼンタである。
///
/// バインディングはスタティックが既定なので DAO は型パラメータで保持する
/// (`coding-rules/use-case-rules.md` §2)。実装 (`XxxDaoImpl`) には依存しない — 結線は
/// 合成ルートだけが行う (同 §1 の DIP)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FindScopeKeywordUseCase<D: ScopeKeywordDao> {
    dao: D,
}

impl<D: ScopeKeywordDao> FindScopeKeywordUseCase<D> {
    /// 引当の口を注入する (**この型の唯一の構築経路**)。
    #[must_use]
    pub const fn new(dao: D) -> FindScopeKeywordUseCase<D> {
        FindScopeKeywordUseCase { dao }
    }

    /// 定義 × キーワードで scope 名を引く。語の割り方はコントローラが決める。
    ///
    /// # Errors
    ///
    /// リードモデルを引けない ([`ReadModelReadError`])。
    pub fn execute(
        &self,
        definition_id: &str,
        keyword: &str,
    ) -> Result<Option<String>, ReadModelReadError> {
        self.dao.find(definition_id, keyword)
    }
}
