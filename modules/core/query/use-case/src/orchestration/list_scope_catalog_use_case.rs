//! `ListScopeCatalogUseCase` — `scope-table` の全行を組む (upstream `renderScopeTable` の材料)。

use crate::orchestration::{
    ReadModelReadError, ScopeCatalogRowView, ScopeGridDao, ScopeMetadataDao,
};

/// `scope-table` の全行を **scope 名の綴り順**で組む。
///
/// 行は 2 つのリードモデル — scope 定義メタ (`depth` / `testStrategy`) と scope グリッド
/// (EXECUTE/総数) — を **scope 名の突合**で組んだものである。組むのは規則 6 (2026-09-03) が
/// クエリ側ユースケースに認めた「FK をたどって表ごとに引き、View を組む」に当たる
/// (`coding-rules/cqrs-boundaries.md`)。行の並びと員数は scope 定義メタが決める (upstream
/// `validScopes()` = `Object.keys(loadScopeMapping()).sort()`) — グリッドに列が無い scope は
/// 0/0 で載る。Markdown 表への整形は出す側の仕事である。
///
/// バインディングはスタティックが既定なので DAO は型パラメータで保持する
/// (`coding-rules/use-case-rules.md` §2)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListScopeCatalogUseCase<M: ScopeMetadataDao, G: ScopeGridDao> {
    scope_metadata_dao: M,
    scope_grid_dao: G,
}

impl<M: ScopeMetadataDao, G: ScopeGridDao> ListScopeCatalogUseCase<M, G> {
    /// 2 つの引当の口を注入する (**この型の唯一の構築経路**)。
    #[must_use]
    pub const fn new(scope_metadata_dao: M, scope_grid_dao: G) -> ListScopeCatalogUseCase<M, G> {
        ListScopeCatalogUseCase {
            scope_metadata_dao,
            scope_grid_dao,
        }
    }

    /// `scope-table` の全行を組んで返す。
    ///
    /// # Errors
    ///
    /// リードモデルを引けない ([`ReadModelReadError`])。
    pub fn execute(&self) -> Result<Vec<ScopeCatalogRowView>, ReadModelReadError> {
        let mut rows = Vec::new();
        for meta in self.scope_metadata_dao.find_all()? {
            let (execute, total) = self
                .scope_grid_dao
                .find(meta.scope())?
                .map_or((0, 0), |actions| {
                    (actions.execute_count(), actions.total_count())
                });
            rows.push(ScopeCatalogRowView::new(
                meta.scope().to_string(),
                meta.depth().to_string(),
                meta.test_strategy().map(str::to_string),
                execute,
                total,
            ));
        }
        Ok(rows)
    }
}
