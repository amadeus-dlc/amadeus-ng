//! 保存されたテスト契約の読取り表現。
/// 規則解決の成功値または拒否理由。Query側では解釈し直さない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestingContractView {
    contract: Option<String>,
    rendered: Option<String>,
    error: Option<String>,
}
impl TestingContractView {
    /// リードモデルの行から組む。
    #[must_use]
    pub const fn new(
        contract: Option<String>,
        rendered: Option<String>,
        error: Option<String>,
    ) -> Self {
        Self {
            contract,
            rendered,
            error,
        }
    }
    /// 公開JSON。
    #[must_use]
    pub fn contract(&self) -> Option<&str> {
        self.contract.as_deref()
    }
    /// 公開Markdown。
    #[must_use]
    pub fn rendered(&self) -> Option<&str> {
        self.rendered.as_deref()
    }
    /// 規則の拒否理由。
    #[must_use]
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }
}
