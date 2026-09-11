//! 指定IDの承認操作をリードモデルから返す。
use super::{PlanApprovalOperationDao, PlanApprovalOperationView, ReadModelReadError};
/// 集約の呼出し・判断・更新は持たない。
#[derive(Debug)]
pub struct PlanApprovalOperationUseCase<D> {
    dao: D,
}
impl<D: PlanApprovalOperationDao> PlanApprovalOperationUseCase<D> {
    /// DAOを注入する。
    #[must_use]
    pub const fn new(dao: D) -> Self {
        Self { dao }
    }
    /// 回復対象として投影された操作のIDと参照先を読む。
    /// # Errors
    /// リードモデルを読み込めない場合。
    pub fn pending(&self) -> Result<Vec<PlanApprovalOperationView>, ReadModelReadError> {
        self.dao.find_pending()
    }
    /// 呼出側が持つ操作IDで結果を返す。
    /// # Errors
    /// リードモデルを読み込めない場合。
    pub fn execute(
        &self,
        operation_id: &str,
    ) -> Result<Option<PlanApprovalOperationView>, ReadModelReadError> {
        self.dao.find(operation_id)
    }
}
