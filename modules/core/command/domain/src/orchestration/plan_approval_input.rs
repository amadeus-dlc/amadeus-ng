//! 計画承認の入力境界が取得した参照情報。
use super::{PlanApprovalDocuments, PlanTarget, TestingSections};
/// ファイルI/Oと集約の判断を分離する入力値。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanApprovalInput {
    supplied_questions_file: Option<String>,
    documents: PlanApprovalDocuments,
    sections: TestingSections,
    target: PlanTarget,
    state_sha256: Option<String>,
    source_sha256: Option<String>,
}
impl PlanApprovalInput {
    /// 入力境界がプロジェクト相対へ正規化した指定ファイル。
    #[must_use]
    pub fn with_supplied_questions_file(mut self, file: String) -> Self {
        self.supplied_questions_file = Some(file);
        self
    }
    /// 呼出側が実際に指定した質問ファイル。
    #[must_use]
    pub fn supplied_questions_file(&self) -> Option<&str> {
        self.supplied_questions_file.as_deref()
    }

    /// 同じ呼出しで取得した材料を束ねる。
    #[must_use]
    pub const fn new(
        documents: PlanApprovalDocuments,
        sections: TestingSections,
        target: PlanTarget,
        state_sha256: Option<String>,
        source_sha256: Option<String>,
    ) -> Self {
        Self {
            supplied_questions_file: None,
            documents,
            sections,
            target,
            state_sha256,
            source_sha256,
        }
    }
    /// 承認対象の文書。
    #[must_use]
    pub const fn documents(&self) -> &PlanApprovalDocuments {
        &self.documents
    }
    /// 現在の規則。
    #[must_use]
    pub const fn sections(&self) -> &TestingSections {
        &self.sections
    }
    /// 正規化済み対象。
    #[must_use]
    pub const fn target(&self) -> &PlanTarget {
        &self.target
    }
    /// 実際に読んだ状態本文の照合子。
    #[must_use]
    pub fn state_sha256(&self) -> Option<&str> {
        self.state_sha256.as_deref()
    }
    /// 現在のソース照合子。取得不能はNone。
    #[must_use]
    pub fn source_sha256(&self) -> Option<&str> {
        self.source_sha256.as_deref()
    }
}
