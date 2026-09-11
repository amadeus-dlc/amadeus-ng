//! HookHealth集約の取得と、heartbeat/dropの単一事実保存ポート。
use super::RepositoryError;
use core_command_domain::workspace::{HookHealth, HookHealthEvent, HookHealthId};
/// 状態判断はHookHealth、媒体への読み書きはアダプタが担当する。
#[allow(
    async_fn_in_trait,
    reason = "既存Repositoryポートと同じcurrent_thread契約"
)]
pub trait HookHealthRepository {
    /// 最新スナップショットと以後の観測事実から再構成する。
    /// # Errors
    /// 不在・破損・I/Oの失敗。
    async fn find_by_id(
        &self,
        id: &HookHealthId,
    ) -> Result<HookHealth, RepositoryError<HookHealthId>>;
    /// 集約が生成した1事実と適用後の状態を保存する。
    /// # Errors
    /// 競合・破損・I/Oの失敗。
    async fn store(
        &mut self,
        event: &HookHealthEvent,
        aggregate: &HookHealth,
    ) -> Result<(), RepositoryError<HookHealthId>>;
}
