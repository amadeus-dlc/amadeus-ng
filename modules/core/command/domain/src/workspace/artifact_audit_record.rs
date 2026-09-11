//! 最新の成果物保存観測。
use chrono::{DateTime, Utc};
/// 監査行へ投影する完全な保存観測。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactAuditRecord {
    file: String,
    tool: String,
    context: String,
    created: bool,
    at: DateTime<Utc>,
}
impl ArtifactAuditRecord {
    /// 全材料を受け取る完全コンストラクタ。
    #[must_use]
    pub const fn new(
        file: String,
        tool: String,
        context: String,
        created: bool,
        at: DateTime<Utc>,
    ) -> Self {
        Self {
            file,
            tool,
            context,
            created,
            at,
        }
    }
    /// ファイル。
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
    /// 発生時刻。
    #[must_use]
    pub const fn at(&self) -> DateTime<Utc> {
        self.at
    }
}
