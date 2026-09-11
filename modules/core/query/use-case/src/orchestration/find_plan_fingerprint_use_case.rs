//! 現在の計画指紋を指定対象で読む。
use super::{PlanFingerprintDao, PlanFingerprintView, ReadModelReadError};
/// 規則や文書を解釈し直さず、DAOの行を返す。
#[derive(Debug)]
pub struct FindPlanFingerprintUseCase<D: PlanFingerprintDao> {
    dao: D,
}
impl<D: PlanFingerprintDao> FindPlanFingerprintUseCase<D> {
    /// 読取りポートを注入する。
    #[must_use]
    pub const fn new(dao: D) -> Self {
        Self { dao }
    }
    /// 対象の行を取得する。
    /// # Errors
    /// DAOの読取りが失敗した場合。
    pub fn execute(
        &self,
        execution_id: &str,
        target_id: &str,
    ) -> Result<Option<PlanFingerprintView>, ReadModelReadError> {
        self.dao.find(execution_id, target_id)
    }
}
