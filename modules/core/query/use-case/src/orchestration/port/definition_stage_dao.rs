//! `DefinitionStageDao` ポート — グラフのステージ 1 行を引く DAO。

use super::read_model_read_error::ReadModelReadError;
use super::read_view::DefinitionStageView;

/// グラフのステージ 1 行の引当 (**読取専用**)。
///
/// **行が引けないこと自体が「そのステージがコンパイル済みグラフに無い」の答え**である —
/// 配布ファイルを読みに行く経路はクエリ側に無い (`coding-rules/cqrs-boundaries.md` 規則 7)。
pub trait DefinitionStageDao {
    /// 定義識別子と slug でステージ 1 行を引く。
    ///
    /// **不在は失敗ではない** — グラフに無い slug は正常な観測なので `Ok(None)` で返す。
    ///
    /// # Errors
    ///
    /// リードモデルを引けない ([`ReadModelReadError`])。
    fn find(
        &self,
        definition_id: &str,
        stage_slug: &str,
    ) -> Result<Option<DefinitionStageView>, ReadModelReadError>;
}
