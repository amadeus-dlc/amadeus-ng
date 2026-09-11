//! フック観測の構築・履歴が満たさなかった条件。
/// 観測履歴の内部契約違反。処理系のファイル診断文言とは分離する。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HookHealthError {
    /// フック名が安全な識別子ではない。
    InvalidHookName,
    /// 観測集約の識別子が正準形ではない。
    InvalidIdentity,
    /// 観測イベントの識別子が正準UUIDv7ではない。
    InvalidEventIdentity,
    /// 集約と観測対象が一致しない。
    TargetMismatch,
    /// 通番や観測回数に矛盾がある。
    InvalidHistory,
    /// 通番または失敗回数を増やせない。
    CounterExhausted,
}
impl std::fmt::Display for HookHealthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::InvalidHookName => "invalid hook name",
            Self::InvalidIdentity => "invalid hook health identity",
            Self::InvalidEventIdentity => "invalid hook health event identity",
            Self::TargetMismatch => "hook health target mismatch",
            Self::InvalidHistory => "invalid hook health history",
            Self::CounterExhausted => "hook health counter exhausted",
        })
    }
}
impl std::error::Error for HookHealthError {}
