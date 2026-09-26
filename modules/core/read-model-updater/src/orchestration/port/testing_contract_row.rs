//! `TestingContractRow` — `read_testing_contract` の 1 行 (依頼ごとのテスト契約)。

/// `read_testing_contract` の 1 行。成功した契約か、現在の規則を受理できない理由のどちらかを
/// 保持する。主キーは依頼 ID (依頼前の既定行は `bare-space`)。
///
/// `source_digest` と `as_of` は断面全体の性質なので行型には持たせず、DAO
/// ([`super::TestingContractDao::replace`]) が全行へ同じ値を書く。行は値を運ぶだけである。
/// 規則と依頼から行を組む投影は [`crate::read_tables::TestingTables::project`] が持つ。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestingContractRow {
    id: String,
    contract: Option<String>,
    rendered: Option<String>,
    error: Option<String>,
}

impl TestingContractRow {
    /// 行の値を束ねる (**この型の唯一の構築経路**)。
    #[must_use]
    pub const fn new(
        id: String,
        contract: Option<String>,
        rendered: Option<String>,
        error: Option<String>,
    ) -> Self {
        Self {
            id,
            contract,
            rendered,
            error,
        }
    }

    /// 依頼ID。依頼前の既定行はbare-space。
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// 解決済み契約の公開JSON。
    #[must_use]
    pub fn contract(&self) -> Option<&str> {
        self.contract.as_deref()
    }

    /// 計画へ載せる公開Markdown。
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
