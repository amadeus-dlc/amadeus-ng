//! セッション監査結果の読取ポート。
use super::{ReadModelReadError, SessionAuditView};
/// 元の通知IDを使って結果を読む。
pub trait SessionAuditDao {
    /// 指定通知が投影済みならその行を返す。
    /// # Errors
    /// リードモデルの読取失敗。
    fn find(&self, id: &str) -> Result<Option<SessionAuditView>, ReadModelReadError>;
}
