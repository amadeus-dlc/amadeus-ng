//! 指示発行時に観測した対象と状態。
use super::PublishedDirective;
/// ファイルI/Oを行う入力境界から渡す観測値。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectivePublication {
    approval_operation_id: Option<super::PlanApprovalOperationId>,
    project_sha256: String,
    state_sha256: String,
    directive: PublishedDirective,
    source_floor: Option<String>,
}
impl DirectivePublication {
    /// 共有承認の失効準備と、この発行を同じ操作IDで結ぶ。
    #[must_use]
    pub fn with_approval_operation(mut self, id: Option<super::PlanApprovalOperationId>) -> Self {
        self.approval_operation_id = id;
        self
    }
    /// 発行と共有承認の失効を対応付ける操作。
    #[must_use]
    pub const fn approval_operation_id(&self) -> Option<&super::PlanApprovalOperationId> {
        self.approval_operation_id.as_ref()
    }

    /// 計算済みのハッシュと発行内容を束ねる。
    #[must_use]
    pub const fn new(
        project_sha256: String,
        state_sha256: String,
        directive: PublishedDirective,
    ) -> Self {
        Self {
            approval_operation_id: None,
            project_sha256,
            state_sha256,
            directive,
            source_floor: None,
        }
    }
    /// 計画承認が束縛するソースの観測値を付ける。
    #[must_use]
    pub fn with_source_floor(mut self, source_floor: Option<String>) -> Self {
        self.source_floor = source_floor;
        self
    }
    /// 発行時に観測したソースの照合子。
    #[must_use]
    pub fn source_floor(&self) -> Option<&str> {
        self.source_floor.as_deref()
    }
    /// 対象プロジェクトの照合子。
    #[must_use]
    pub fn project_sha256(&self) -> &str {
        &self.project_sha256
    }
    /// 発行時の状態本文の照合子。
    #[must_use]
    pub fn state_sha256(&self) -> &str {
        &self.state_sha256
    }
    /// 発行する指示。
    #[must_use]
    pub const fn directive(&self) -> &PublishedDirective {
        &self.directive
    }
}
