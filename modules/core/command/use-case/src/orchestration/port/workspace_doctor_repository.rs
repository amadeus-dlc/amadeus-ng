//! `WorkspaceDoctor` 集約の取得と、1 回の診断事実の保存ポート。
use super::RepositoryError;
use core_command_domain::workspace::{WorkspaceDoctor, WorkspaceDoctorEvent, WorkspaceDoctorId};
/// 判断は `WorkspaceDoctor`、媒体への読み書きはアダプタが担当する。
#[allow(
    async_fn_in_trait,
    reason = "既存Repositoryポートと同じcurrent_thread契約"
)]
pub trait WorkspaceDoctorRepository {
    /// 最新スナップショットと以後の診断事実から再構成する。
    /// # Errors
    /// 不在・破損・I/Oの失敗。
    async fn find_by_id(
        &self,
        id: &WorkspaceDoctorId,
    ) -> Result<WorkspaceDoctor, RepositoryError<WorkspaceDoctorId>>;
    /// 集約が生成した1事実と適用後の状態を保存する。
    /// # Errors
    /// 競合・破損・I/Oの失敗。
    async fn store(
        &mut self,
        event: &WorkspaceDoctorEvent,
        aggregate: &WorkspaceDoctor,
    ) -> Result<(), RepositoryError<WorkspaceDoctorId>>;
}
