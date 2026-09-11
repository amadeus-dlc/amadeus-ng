//! 開始許可の公開からソース再照合までの状態。
/// Pendingの間は実装を許可せず、確定か失効を待つ。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanGenerationState {
    /// 開始許可を公開し、ソースの再照合を待っている。
    Pending,
    /// ソース照合を終え、実装開始を確定した。
    Active,
    /// 公開中のソース変更で開始許可を失効させた。
    Revoked,
}
