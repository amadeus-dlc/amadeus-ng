//! 停止要求IDの結果を引く読取専用ポート。
use super::{ContinuationResultView, ReadModelReadError};
/// 停止判断を作らず、指定行だけを返す。
pub trait ContinuationResultDao {
    /// 指定要求の投影結果。
    /// # Errors
    /// リードモデルを読めない場合。
    fn find(&self, id: &str) -> Result<Option<ContinuationResultView>, ReadModelReadError>;
    /// 公開IOを終え、コマンドの確定を待っている操作。表示結果の代用にはしない。
    /// # Errors
    /// リードモデルを読めない場合。
    fn unsettled(
        &self,
        aggregate_id: &str,
    ) -> Result<Option<ContinuationResultView>, ReadModelReadError>;
}
