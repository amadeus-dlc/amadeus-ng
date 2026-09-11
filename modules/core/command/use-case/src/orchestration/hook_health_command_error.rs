//! フック稼働観測の更新失敗。
use super::RepositoryError;
use core_command_domain::workspace::{HookHealthError, HookHealthId};
/// 観測の拒否と保存失敗を区別する。
#[derive(Debug)]
pub enum HookHealthCommandError {
    /// ドメインの観測条件違反。
    Domain(HookHealthError),
    /// 保存・再構成の失敗。
    Repository(RepositoryError<HookHealthId>),
}
impl std::fmt::Display for HookHealthCommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Domain(error) => error.fmt(f),
            Self::Repository(error) => error.fmt(f),
        }
    }
}
impl std::error::Error for HookHealthCommandError {}
