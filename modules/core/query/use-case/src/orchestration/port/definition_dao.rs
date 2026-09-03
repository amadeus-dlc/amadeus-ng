//! `DefinitionDao` ポート — 定義 1 行の要約を引く DAO。

use super::read_model_read_error::ReadModelReadError;
use super::read_view::DefinitionSummaryView;

/// 定義要約の引当 (**読取専用**)。
///
/// **行が引けないこと自体が「定義が未取込」の答え**である — 配布ファイルを読みに行く経路は
/// クエリ側に無い (`coding-rules/cqrs-boundaries.md` 規則 7)。
pub trait DefinitionDao {
    /// 定義識別子で要約を引く。
    ///
    /// **不在は失敗ではない** — まだ取り込まれていない定義は正常な観測なので `Ok(None)` で
    /// 返す。
    ///
    /// # Errors
    ///
    /// リードモデルを引けない ([`ReadModelReadError`])。
    fn find(
        &self,
        definition_id: &str,
    ) -> Result<Option<DefinitionSummaryView>, ReadModelReadError>;
}
