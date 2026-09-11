//! 通知IDによるセッション監査結果取得。
use super::{ReadModelReadError, SessionAuditDao, SessionAuditView};
/// DAOが返す投影結果をそのまま渡す。
pub struct SessionAuditUseCase<D> {
    dao: D,
}
impl<D: SessionAuditDao> SessionAuditUseCase<D> {
    /// DAOを注入する。
    #[must_use]
    pub const fn new(dao: D) -> Self {
        Self { dao }
    }
    /// 通知IDの結果だけを取得する。
    /// # Errors
    /// リードモデルを読めない場合。
    pub fn execute(&self, id: &str) -> Result<Option<SessionAuditView>, ReadModelReadError> {
        self.dao.find(id)
    }
}
