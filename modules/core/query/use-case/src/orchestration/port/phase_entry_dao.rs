//! `PhaseEntryDao` ポート — 定義側のフェーズ入口を引く DAO。

use super::read_model_read_error::ReadModelReadError;
use super::read_view::PhaseEntryView;

/// フェーズ入口の引当 (**読取専用**)。
///
/// 定義とスコープグリッドだけで決まる入口なので、state を持たない要求からも引ける。
/// 実行の実効プランで決まる入口は [`super::JumpPhaseDao`] が別に答える。
pub trait PhaseEntryDao {
    /// 定義 × scope × フェーズで入口ステージを引く。
    ///
    /// **不在は失敗ではない** — その scope でそのフェーズに実行するステージが無いのは
    /// 正常な観測なので `Ok(None)` で返す。
    ///
    /// # Errors
    ///
    /// リードモデルを引けない ([`ReadModelReadError`])。
    fn find(
        &self,
        definition_id: &str,
        scope: &str,
        phase: &str,
    ) -> Result<Option<PhaseEntryView>, ReadModelReadError>;
}
