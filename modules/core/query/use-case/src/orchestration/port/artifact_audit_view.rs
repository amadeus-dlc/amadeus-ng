//! ArtifactAuditの投影行を返すQuery DTO。
/// ドメインに依存しない保存監査の最新行。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactAuditView {
    id: String,
    target: String,
    file: String,
    tool: String,
    context: String,
    created: bool,
    occurred_at: String,
}
impl ArtifactAuditView {
    /// 完全な投影行を組む。
    #[must_use]
    pub const fn new(
        id: String,
        target: String,
        file: String,
        tool: String,
        context: String,
        created: bool,
        occurred_at: String,
    ) -> Self {
        Self {
            id,
            target,
            file,
            tool,
            context,
            created,
            occurred_at,
        }
    }
    /// 識別子。
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }
    /// 観測先。
    #[must_use]
    pub fn target(&self) -> &str {
        &self.target
    }
    /// 対象ファイル。
    #[must_use]
    pub fn file(&self) -> &str {
        &self.file
    }
    /// ツール。
    #[must_use]
    pub fn tool(&self) -> &str {
        &self.tool
    }
    /// 文脈。
    #[must_use]
    pub fn context(&self) -> &str {
        &self.context
    }
    /// 新規作成か。
    #[must_use]
    pub const fn created(&self) -> bool {
        self.created
    }
    /// 保存時刻。
    #[must_use]
    pub fn occurred_at(&self) -> &str {
        &self.occurred_at
    }
}
