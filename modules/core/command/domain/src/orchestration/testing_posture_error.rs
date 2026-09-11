//! テスト方針の解決に失敗した理由。
/// 規則の矛盾や未知の方法論を、既定値で隠さず返す。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestingPostureError {
    message: String,
}
impl TestingPostureError {
    pub(super) fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}
impl std::fmt::Display for TestingPostureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for TestingPostureError {}
