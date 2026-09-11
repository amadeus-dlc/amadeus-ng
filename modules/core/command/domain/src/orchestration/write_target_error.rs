//! 書込み先として受け取れなかった生値の理由。
/// [`WriteTarget`] の検証で拒否した理由。
///
/// [`WriteTarget`]: super::WriteTarget
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteTargetError {
    /// 空文字、または空白だけの綴り。書込み先を名指していない。
    Empty,
}
impl std::fmt::Display for WriteTargetError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => formatter.write_str("write target names no path"),
        }
    }
}
impl std::error::Error for WriteTargetError {}
