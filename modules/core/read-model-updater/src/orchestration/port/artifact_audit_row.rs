//! `ArtifactAuditRow` — 成果物監査の 1 行 (監査対象ごとの最新の書込観測)。

use chrono::{DateTime, Utc};

/// 成果物監査の 1 行。主キーは 1 列 `id` = 成果物監査の集約 ID。
///
/// 値は保存履歴から再構成した成果物監査の集約が持つ**最新の観測**の写しである
/// (監査対象 1 つにつき 1 行)。
///
/// 行は値を運ぶだけである。材料から行を組む投影は
/// [`crate::read_tables::ReadTables::project`] が持つ。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactAuditRow {
    id: String,
    target: String,
    file: String,
    tool: String,
    context: String,
    created: bool,
    occurred_at: DateTime<Utc>,
}

impl ArtifactAuditRow {
    /// 行の値を束ねる (**この型の唯一の構築経路**)。
    #[must_use]
    pub const fn new(
        id: String,
        target: String,
        file: String,
        tool: String,
        context: String,
        created: bool,
        occurred_at: DateTime<Utc>,
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

    /// 主キー — 成果物監査の集約 ID。
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// 監査の帰属先 (相対ディレクトリ)。
    #[must_use]
    pub fn target(&self) -> &str {
        &self.target
    }

    /// 最後に観測した書込のファイル。
    #[must_use]
    pub fn file(&self) -> &str {
        &self.file
    }

    /// 最後に観測した書込のツール。
    #[must_use]
    pub fn tool(&self) -> &str {
        &self.tool
    }

    /// 最後に観測した書込の文脈。
    #[must_use]
    pub fn context(&self) -> &str {
        &self.context
    }

    /// 最後に観測した書込が新規作成だったか。
    #[must_use]
    pub const fn created(&self) -> bool {
        self.created
    }

    /// 最後の観測の時刻。
    #[must_use]
    pub const fn occurred_at(&self) -> DateTime<Utc> {
        self.occurred_at
    }
}
