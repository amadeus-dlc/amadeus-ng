//! 保存された実行境界の整合違反。
/// 最新の境界と、その種類の発生回数が対応していない。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RunFloorError;
impl std::fmt::Display for RunFloorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("run boundary and occurrence counts do not agree")
    }
}
impl std::error::Error for RunFloorError {}
