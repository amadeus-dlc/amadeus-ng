//! `ScopeGridDao` ポート — scope グリッド (EXECUTE/SKIP 割当) を引く DAO。

use super::read_model_read_error::ReadModelReadError;
use super::read_view::ScopeActionsView;

/// scope グリッドの引当 (**読取専用**)。
///
/// `scope-grid.json` は compile コンテキストの投影であり、クエリ側はこれを**読むだけ**である
/// (`coding-rules/cqrs-boundaries.md` 規則 6/7)。媒体はポート面に現れない。
pub trait ScopeGridDao {
    /// scope 名で 1 scope 分の割当を引く。
    ///
    /// **不在は失敗ではない** — グリッドに無い scope は正常な観測なので `Ok(None)` で返す。
    ///
    /// # Errors
    ///
    /// リードモデルを引けない ([`ReadModelReadError`])。
    fn find(&self, scope: &str) -> Result<Option<ScopeActionsView>, ReadModelReadError>;
}
