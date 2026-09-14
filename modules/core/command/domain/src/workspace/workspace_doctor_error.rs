//! 自己診断集約の構築・履歴が満たさなかった条件。

/// 診断履歴の内部契約違反。観測の失敗 (`ObservationFailure`) とは別で、こちらは
/// 集約の不変条件である。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceDoctorError {
    /// 集約の識別子が正準形ではない。
    InvalidIdentity,
    /// 診断イベントの識別子が正準 UUIDv7 ではない。
    InvalidEventIdentity,
    /// 検査 ID の綴りが C7 の表に無い。
    InvalidCheckId,
    /// 集約と診断対象が一致しない。
    TargetMismatch,
    /// 通番に矛盾がある。
    InvalidHistory,
    /// 通番を増やせない。
    CounterExhausted,
}

impl std::fmt::Display for WorkspaceDoctorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::InvalidIdentity => "invalid workspace doctor identity",
            Self::InvalidEventIdentity => "invalid workspace doctor event identity",
            Self::InvalidCheckId => "invalid doctor check id",
            Self::TargetMismatch => "workspace doctor target mismatch",
            Self::InvalidHistory => "invalid workspace doctor history",
            Self::CounterExhausted => "workspace doctor counter exhausted",
        })
    }
}

impl std::error::Error for WorkspaceDoctorError {}
