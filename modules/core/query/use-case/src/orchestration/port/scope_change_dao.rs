//! `ScopeChangeDao` ポート — 要求 scope と state の scope の照合結果を引く DAO。

use super::read_model_read_error::ReadModelReadError;
use super::read_view::ScopeChangeView;

/// scope 照合の引当 (**読取専用**)。
pub trait ScopeChangeDao {
    /// 実行 × 要求 scope で照合結果を引く。
    ///
    /// **行が返らなければ無効な scope** である (有効な scope にしか行が無い)。返れば
    /// `kind` が「state の scope と違う」か「同じ」かを言う — 比較はクエリ側に無い。
    ///
    /// # Errors
    ///
    /// リードモデルを引けない ([`ReadModelReadError`])。
    fn find(
        &self,
        execution_id: &str,
        scope: &str,
    ) -> Result<Option<ScopeChangeView>, ReadModelReadError>;
}
