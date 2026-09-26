//! `SessionAuditRow` — セッション監査の通知IDごとの 1 行。

/// 更新コマンドの戻り値を使わずに読む、保存通知の結果。
///
/// 行は値を運ぶだけである。材料から行を組む投影は
/// [`crate::read_tables::ReadTables::project`] が持つ。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionAuditRow {
    id: String,
    aggregate_id: String,
    target: String,
    kind: String,
}

impl SessionAuditRow {
    /// 行の値を束ねる (**この型の唯一の構築経路**)。
    #[must_use]
    pub const fn new(id: String, aggregate_id: String, target: String, kind: String) -> Self {
        Self {
            id,
            aggregate_id,
            target,
            kind,
        }
    }

    /// 通知ID。
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// 所有集約ID。
    #[must_use]
    pub fn aggregate_id(&self) -> &str {
        &self.aggregate_id
    }

    /// 監査帰属先。
    #[must_use]
    pub fn target(&self) -> &str {
        &self.target
    }

    /// 保存された監査種別。
    #[must_use]
    pub fn kind(&self) -> &str {
        &self.kind
    }
}
