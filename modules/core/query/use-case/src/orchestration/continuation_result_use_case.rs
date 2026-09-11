//! 停止要求IDの結果取得。
use super::{ContinuationResultDao, ContinuationResultView, ReadModelReadError};
/// DAOの行をそのまま返す読取ユースケース。
pub struct ContinuationResultUseCase<D> {
    dao: D,
}
impl<D: ContinuationResultDao> ContinuationResultUseCase<D> {
    /// DAOを注入する。
    #[must_use]
    pub const fn new(dao: D) -> Self {
        Self { dao }
    }
    /// 要求IDで投影結果を読む。
    /// # Errors
    /// リードモデルを読めない場合。
    pub fn execute(&self, id: &str) -> Result<Option<ContinuationResultView>, ReadModelReadError> {
        self.dao.find(id)
    }
    /// 公開済みIOの確定処理を再開するための操作IDと観測結果。
    /// # Errors
    /// リードモデルを読めない場合。
    pub fn unsettled(
        &self,
        aggregate_id: &str,
    ) -> Result<Option<ContinuationResultView>, ReadModelReadError> {
        self.dao.unsettled(aggregate_id)
    }
}
