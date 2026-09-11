//! ワークスペース全体の承認集約の識別子。
/// 保存先がワークスペースを区別し、その内側に承認集約は1つだけ存在する。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PlanApprovalRuntimeId {
    /// 全space・全intentで共有する承認の所有者。
    Workspace,
}
impl std::fmt::Display for PlanApprovalRuntimeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("workspace")
    }
}
