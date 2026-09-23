//! コード生成を開始できるかの、ドメインによる判定。
use super::{
    CodeGenerationAuthority, PlanApprovalDocuments, PlanApprovalError, PlanReceipts, PlanTarget,
    TestingPosture, TestingPostureError,
};
/// 公開する判定値を保持し、開始済みでも文書と規則の照合を省かない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeGenerationApproval {
    receipt_key: Option<String>,
    ok: bool,
    unit: Option<String>,
    reason: String,
    plan_exists: bool,
    instructions_exist: bool,
    approved: bool,
    contract_valid: bool,
    fingerprint_valid: bool,
    receipt_valid: bool,
    contract_hash: Option<String>,
    approval_fingerprint: Option<String>,
    directive_epoch: Option<String>,
}
impl CodeGenerationApproval {
    /// 現在の指示・文書・規則・受領から開始可否を計算する。
    #[must_use]
    pub fn evaluate(
        authority: Result<CodeGenerationAuthority, PlanApprovalError>,
        target: &PlanTarget,
        documents: &PlanApprovalDocuments,
        posture: Result<TestingPosture, TestingPostureError>,
        receipts: &PlanReceipts,
        source: Option<&str>,
    ) -> Self {
        let mut result = Self {
            receipt_key: None,
            ok: false,
            unit: target.unit().map(str::to_string),
            reason: String::new(),
            plan_exists: false,
            instructions_exist: false,
            approved: false,
            contract_valid: false,
            fingerprint_valid: false,
            receipt_valid: false,
            contract_hash: None,
            approval_fingerprint: None,
            directive_epoch: None,
        };
        if let Err(error) = result.check(authority, documents, posture, receipts, source) {
            result.reason = error.to_string();
        }
        result
    }
    fn check(
        &mut self,
        authority: Result<CodeGenerationAuthority, PlanApprovalError>,
        documents: &PlanApprovalDocuments,
        posture: Result<TestingPosture, TestingPostureError>,
        receipts: &PlanReceipts,
        source: Option<&str>,
    ) -> Result<(), PlanApprovalError> {
        use super::{
            EmbeddedTestingContract, PlanApprovalEvidence, PlanGenerationStatus, PlanQuestions,
        };
        use core_infrastructure::{ecmascript::trim, hash::sha256_hex};
        let authority = authority?;
        self.directive_epoch = Some(authority.directive_epoch().to_string());
        let plan_exists = !trim(documents.plan()).is_empty();
        let instructions_exist = !trim(documents.instructions()).is_empty();
        let questions = PlanQuestions::parse(documents.questions());
        let embedded = if plan_exists {
            EmbeddedTestingContract::parse(documents.plan())
        } else {
            None
        };
        let current = if plan_exists {
            Some(posture.map_err(|error| PlanApprovalError::new(error.to_string()))?)
        } else {
            None
        };
        self.plan_exists = plan_exists;
        self.instructions_exist = instructions_exist;
        self.approved = questions.approved();
        self.contract_valid = embedded.as_ref().is_some_and(|contract| {
            current
                .as_ref()
                .is_some_and(|current| contract.is_current(current))
        });
        self.contract_hash = embedded
            .as_ref()
            .map(|contract| contract.hash().to_string());
        self.approval_fingerprint = embedded
            .as_ref()
            .filter(|_| plan_exists && instructions_exist && self.contract_valid)
            .map(|contract| authority.approval_fingerprint(documents, contract.hash()));
        let reason = if !self.plan_exists {
            Some("code-generation-plan.md is missing or empty")
        } else if !self.instructions_exist {
            Some("unit-test-instructions.md is missing or empty")
        } else if self.contract_hash.is_none() {
            Some("code-generation-plan.md has no valid ## Testing Contract JSON block")
        } else if !self.contract_valid {
            Some(
                "the approved Testing Contract is stale because memory, scope, test strategy, or project type changed",
            )
        } else if !self.approved {
            Some("Plan Approval is not explicitly answered Approve Plan")
        } else {
            None
        };
        if let Some(reason) = reason {
            return Err(PlanApprovalError::new(reason));
        }
        self.fingerprint_valid = self
            .approval_fingerprint
            .as_deref()
            .is_some_and(|fingerprint| questions.fingerprint() == Some(fingerprint));
        let fingerprint=self.approval_fingerprint.as_deref().filter(|_|self.fingerprint_valid).ok_or_else(||PlanApprovalError::new("the Plan Approval fingerprint does not match the active intent, target, directive epoch, plan, test instructions, and Testing Contract"))?;
        let evidence = PlanApprovalEvidence::new(
            authority,
            fingerprint.to_string(),
            documents.questions_file().to_string(),
            sha256_hex(documents.questions().as_bytes()),
            sha256_hex(
                super::plan_approval_evidence::prompt_body(documents.questions()).as_bytes(),
            ),
        )?;
        let matched_receipt = receipts.iter().find(|receipt| {
            receipt.decision().evidence() == &evidence
                && receipt.certified_source() == evidence.authority().source_floor()
                && (receipt.status() == PlanGenerationStatus::Generation
                    || source == Some(receipt.certified_source()))
        });
        self.receipt_valid = matched_receipt.is_some();
        self.receipt_key = matched_receipt.map(super::PlanApprovalReceipt::key);
        if !self.receipt_valid {
            return Err(PlanApprovalError::new(
                "no current protected Plan Approval receipt matches this prompt, session response, target, directive epoch, and source floor",
            ));
        }
        self.ok = true;
        self.reason = "approved".to_string();
        Ok(())
    }
    /// 開始判断に一致した、共有集約内の受領キー。
    #[must_use]
    pub fn receipt_key(&self) -> Option<&str> {
        self.receipt_key.as_deref()
    }
    /// 開始できるか。
    #[must_use]
    pub const fn ok(&self) -> bool {
        self.ok
    }
    /// 対象Unit。stage-levelはNone。
    #[must_use]
    pub fn unit(&self) -> Option<&str> {
        self.unit.as_deref()
    }
    /// 本家と一致する判定理由。
    #[must_use]
    pub fn reason(&self) -> &str {
        &self.reason
    }
    /// 空でない計画があるか。
    #[must_use]
    pub const fn plan_exists(&self) -> bool {
        self.plan_exists
    }
    /// 空でないテスト指示があるか。
    #[must_use]
    pub const fn instructions_exist(&self) -> bool {
        self.instructions_exist
    }
    /// 質問が明示的にApprove Planか。
    #[must_use]
    pub const fn approved(&self) -> bool {
        self.approved
    }
    /// 埋込契約が現在の規則に一致するか。
    #[must_use]
    pub const fn contract_valid(&self) -> bool {
        self.contract_valid
    }
    /// 記録された計画指紋が一致するか。
    #[must_use]
    pub const fn fingerprint_valid(&self) -> bool {
        self.fingerprint_valid
    }
    /// 現在の保護された受領が一致するか。
    #[must_use]
    pub const fn receipt_valid(&self) -> bool {
        self.receipt_valid
    }
    /// 埋込契約の自己ハッシュ。
    #[must_use]
    pub fn contract_hash(&self) -> Option<&str> {
        self.contract_hash.as_deref()
    }
    /// 現在の材料から導く計画指紋。
    #[must_use]
    pub fn approval_fingerprint(&self) -> Option<&str> {
        self.approval_fingerprint.as_deref()
    }
    /// 解決できた発行エポック。
    #[must_use]
    pub fn directive_epoch(&self) -> Option<&str> {
        self.directive_epoch.as_deref()
    }
}
