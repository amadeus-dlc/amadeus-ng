//! `ScopeMetadataDao` ポート — scope 定義 frontmatter を引く DAO。

use super::read_model_read_error::ReadModelReadError;
use super::read_view::ScopeMetadataView;

/// scope 定義メタの引当 (**読取専用**)。
///
/// `scopes/aidlc-*.md` は compile コンテキストが読む入力の投影であり、クエリ側はこれを
/// **読むだけ**である (`coding-rules/cqrs-boundaries.md` 規則 6/7)。媒体 (Markdown frontmatter)
/// はポート面に現れない。
pub trait ScopeMetadataDao {
    /// 有効な全 scope のメタを **scope 名の綴り順**でまとめて引く (upstream `validScopes`)。
    ///
    /// 並びを綴り順に固定するのは、行に順序の列が無いためである (upstream も
    /// `Object.keys(...).sort()` で並べる)。
    ///
    /// # Errors
    ///
    /// リードモデルを引けない ([`ReadModelReadError`])。
    fn find_all(&self) -> Result<Vec<ScopeMetadataView>, ReadModelReadError>;
}
