//! `ScopeKeywordDao` ポート — キーワードから scope 名を引く DAO。

use super::read_model_read_error::ReadModelReadError;

/// キーワード引当 (**読取専用**)。
///
/// 自由記述をどう語に割るか (≤5 語で切る等) は要求の形の話なのでコントローラが決め、
/// この口は割られた 1 語で引くだけである (設計 §0-3)。
pub trait ScopeKeywordDao {
    /// 定義 × キーワードで scope 名を引く。
    ///
    /// 返すのは scope 名そのもの — 行が 1 列しか持たないので View 型を立てない。
    /// **不在は失敗ではない** (どの scope のキーワードでもない語は正常な観測)。
    ///
    /// # Errors
    ///
    /// リードモデルを引けない ([`ReadModelReadError`])。
    fn find(
        &self,
        definition_id: &str,
        keyword: &str,
    ) -> Result<Option<String>, ReadModelReadError>;
}
