//! 保存監査投影のID Query。
use super::{ArtifactAuditDao, ArtifactAuditView, ReadModelReadError};
/// DAOを呼ぶだけのQuery UseCase。
#[derive(Debug)]
pub struct ArtifactAuditUseCase<D> {
    dao: D,
}
impl<D: ArtifactAuditDao> ArtifactAuditUseCase<D> {
    /// DAOを注入する。
    #[must_use]
    pub const fn new(dao: D) -> Self {
        Self { dao }
    }
    /// 指定IDを読む。
    /// # Errors
    /// リードモデルを読めない場合。
    pub fn execute(&self, id: &str) -> Result<Option<ArtifactAuditView>, ReadModelReadError> {
        self.dao.find(id)
    }
}
