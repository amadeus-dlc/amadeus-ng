//! 差し向け記録の `unit` として受け取れなかった生値の理由。
/// [`ReviewedUnit`] の検証で拒否した理由。
///
/// [`ReviewedUnit`]: super::ReviewedUnit
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewedUnitError {
    /// 空文字。Unit を名指していない。
    Empty,
}
impl std::fmt::Display for ReviewedUnitError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => formatter.write_str("reviewed unit is empty"),
        }
    }
}
impl std::error::Error for ReviewedUnitError {}
