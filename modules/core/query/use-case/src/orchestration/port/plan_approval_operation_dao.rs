//! 操作IDで共有承認のリードモデルを読むポート。
use super::{PlanApprovalOperationView, ReadModelReadError};
/// 状態を計算し直さず、指定IDの行を読み込む。
pub trait PlanApprovalOperationDao {
    /// 未完了として投影された操作を読む。
    /// # Errors
    /// 読取I/Oの失敗。
    fn find_pending(&self) -> Result<Vec<PlanApprovalOperationView>, ReadModelReadError>;

    /// 指定された操作を読む。
    /// # Errors
    /// 読取I/Oの失敗。
    fn find(
        &self,
        operation_id: &str,
    ) -> Result<Option<PlanApprovalOperationView>, ReadModelReadError>;
}
