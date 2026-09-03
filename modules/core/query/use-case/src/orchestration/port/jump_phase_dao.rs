//! `JumpPhaseDao` ポート — フェーズごとのジャンプ目的地を引く DAO。

use super::read_model_read_error::ReadModelReadError;
use super::read_view::JumpPhaseView;

/// フェーズ目的地の引当 (**読取専用**)。
///
/// 目的地の**受理判定**はこの口では返らない。それは別の表 (`read_next_jump`) の行であり、
/// 目的地の位置を鍵に [`super::JumpDao::find_by_target`] で引く — 1 表 1 引当なので、
/// たどるのはユースケースの仕事である (オーナー裁定 2026-09-03)。
pub trait JumpPhaseDao {
    /// 実行 × フェーズで、実効プランが決めた目的地を引く。
    ///
    /// **不在は失敗ではない** — 目的地を持たないフェーズには行が無いので `Ok(None)` で
    /// 返す。
    ///
    /// # Errors
    ///
    /// リードモデルを引けない ([`ReadModelReadError`])。
    fn find(
        &self,
        execution_id: &str,
        phase: &str,
    ) -> Result<Option<JumpPhaseView>, ReadModelReadError>;
}
