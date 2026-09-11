//! ワークスペース全体の承認操作の投影行。
/// 操作IDが主キー。状態の判断をQueryへ持ち込まない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanApprovalOperationRow {
    kind: String,
    id: String,
    status: String,
    space: Option<String>,
    execution_id: Option<String>,
}
impl PlanApprovalOperationRow {
    /// 投影核が確定した値を束ねる。
    #[must_use]
    pub const fn new(
        id: String,
        status: String,
        space: Option<String>,
        execution_id: Option<String>,
        kind: String,
    ) -> Self {
        Self {
            kind,
            id,
            status,
            space,
            execution_id,
        }
    }
    /// 回復を担当する操作の種類。
    #[must_use]
    pub fn kind(&self) -> &str {
        &self.kind
    }
    /// 元の操作の識別子。
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }
    /// 投影時点の操作状態。
    #[must_use]
    pub fn status(&self) -> &str {
        &self.status
    }
    /// 回復先のspace。
    #[must_use]
    pub fn space(&self) -> Option<&str> {
        self.space.as_deref()
    }
    /// 回復先の実行。
    #[must_use]
    pub fn execution_id(&self) -> Option<&str> {
        self.execution_id.as_deref()
    }
}
