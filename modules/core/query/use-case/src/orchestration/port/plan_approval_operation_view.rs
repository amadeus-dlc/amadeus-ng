//! 共有承認操作の、Queryが返す独立したビュー。
/// ドメイン型や業務判断を持たず、投影済みの値だけを運ぶ。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanApprovalOperationView {
    kind: String,
    id: String,
    status: String,
    space: Option<String>,
    execution_id: Option<String>,
    as_of: u64,
}
impl PlanApprovalOperationView {
    /// 読取り行から組む。
    #[must_use]
    pub const fn new(
        id: String,
        status: String,
        space: Option<String>,
        execution_id: Option<String>,
        as_of: u64,
        kind: String,
    ) -> Self {
        Self {
            kind,
            id,
            status,
            space,
            execution_id,
            as_of,
        }
    }
    /// 投影された操作の種類。
    #[must_use]
    pub fn kind(&self) -> &str {
        &self.kind
    }
    /// 指定された操作の識別子。
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }
    /// 投影済みの操作状態。
    #[must_use]
    pub fn status(&self) -> &str {
        &self.status
    }
    /// 未完了操作が参照するspace。
    #[must_use]
    pub fn space(&self) -> Option<&str> {
        self.space.as_deref()
    }
    /// 未完了操作が参照する実行。
    #[must_use]
    pub fn execution_id(&self) -> Option<&str> {
        self.execution_id.as_deref()
    }
    /// 投影が読み込んだ最後の位置。
    #[must_use]
    pub const fn as_of(&self) -> u64 {
        self.as_of
    }
}
