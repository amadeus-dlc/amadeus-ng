//! 移動結果をイベントIDで引く読取ポート。
use super::{JumpResultView, ReadModelReadError};
/// イベントIDで保存した移動結果を読む。
pub trait JumpResultDao {
    /// 指定した保存事実の結果を読む。
    /// # Errors
    /// 読取り失敗を伝播する。
    fn find(&self, id: &str) -> Result<Option<JumpResultView>, ReadModelReadError>;
}
