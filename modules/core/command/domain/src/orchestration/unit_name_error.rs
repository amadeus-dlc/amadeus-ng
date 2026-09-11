//! Unit 名として受け取れなかった生値の理由。
/// [`UnitName`] の検証で拒否した理由。
///
/// [`UnitName`]: super::UnitName
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnitNameError {
    /// 空文字。Unit を名指していない。
    Empty,
    /// 経路区切りを含む。1 階層の Unit ディレクトリ名ではない。
    Separated,
}
impl std::fmt::Display for UnitNameError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => formatter.write_str("unit name is empty"),
            Self::Separated => formatter.write_str("unit name contains a path separator"),
        }
    }
}
impl std::error::Error for UnitNameError {}
