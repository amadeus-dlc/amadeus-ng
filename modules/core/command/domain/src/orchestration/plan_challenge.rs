//! 実際に提示する計画承認の選択肢と対象。
use super::{PlanApprovalEvidence, PlanSession};
use core_infrastructure::canon_json::{JsonValue, Number, ObjectMembers, hash_canonical};
/// 提示を識別する値。人間応答を受領した事実とは別である。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanChallenge {
    id: String,
    evidence: PlanApprovalEvidence,
    session: PlanSession,
    options: [String; 2],
    require_exact: bool,
}
impl PlanChallenge {
    /// 質問の選択肢を検証し、内容・セッションへ結び付いた提示を作る。
    /// # Errors
    /// 質問の選択肢が正しい2択になっていない場合。
    pub fn from_prompt(
        evidence: super::PlanApprovalEvidence,
        session: super::PlanSession,
        prompt: &super::DecisionPrompt,
        require_exact: bool,
    ) -> Result<Self, super::PlanApprovalError> {
        let options = super::PlanOfferedOptions::parse(prompt.options().unwrap_or(""))?;
        Ok(Self::issue(
            evidence,
            session,
            options.values().clone(),
            require_exact,
        ))
    }

    /// 検証済み文書と提示する2択を結び付ける。
    #[must_use]
    pub fn issue(
        evidence: PlanApprovalEvidence,
        session: PlanSession,
        options: [String; 2],
        require_exact: bool,
    ) -> Self {
        let mut identity = ObjectMembers::new();
        for (key, value) in [
            ("targetId", evidence.authority().target_id()),
            ("intentId", evidence.authority().intent_id()),
            ("directiveEpoch", evidence.authority().directive_epoch()),
            ("runFloor", evidence.authority().run_floor()),
            ("fingerprint", evidence.fingerprint()),
            ("questionsFile", evidence.questions_file()),
            ("promptSha256", evidence.prompt_sha256()),
            ("sourceFloor", evidence.authority().source_floor()),
            ("session", session.raw()),
        ] {
            identity.insert(key, JsonValue::String(value.to_string()));
        }
        identity.insert(
            "markerRevision",
            JsonValue::Number(Number::PosInt(evidence.authority().marker_revision())),
        );
        identity.insert(
            "options",
            JsonValue::Array(
                options
                    .iter()
                    .map(|value| JsonValue::String(value.clone()))
                    .collect(),
            ),
        );
        identity.insert("requireExactOptionLabels", JsonValue::Bool(require_exact));
        identity.insert("hashedOptionLabels", JsonValue::Bool(false));
        identity.insert("legacyDirectiveOffer", JsonValue::Bool(false));
        let id = hash_canonical(&JsonValue::Object(identity)).rendered();
        Self {
            id,
            evidence,
            session,
            options,
            require_exact,
        }
    }
    /// 実際の人間応答を、その提示の2択へ写す。
    #[must_use]
    pub fn offered_choice(&self, response: &str) -> Option<super::PlanChoice> {
        use super::PlanChoice;
        let response = core_infrastructure::ecmascript::trim(response);
        let comparison = response.to_lowercase();
        if let Some(index) = self
            .options
            .iter()
            .position(|option| option.to_lowercase() == comparison)
        {
            return Some(if index == 0 {
                PlanChoice::ApprovePlan
            } else {
                PlanChoice::RequestChanges
            });
        }
        if self.require_exact {
            return None;
        }
        match comparison.as_str() {
            "1" | "approve plan" => Some(PlanChoice::ApprovePlan),
            "2" | "request changes" => Some(PlanChoice::RequestChanges),
            _ => None,
        }
    }

    /// 提示した文書と対象の証拠。
    #[must_use]
    pub const fn evidence(&self) -> &PlanApprovalEvidence {
        &self.evidence
    }
    /// 提示したセッション。
    #[must_use]
    pub const fn session(&self) -> &PlanSession {
        &self.session
    }
    /// 順序付きの2択。
    #[must_use]
    pub const fn options(&self) -> &[String; 2] {
        &self.options
    }
    /// 完全一致したラベルだけを受け取るか。
    #[must_use]
    pub const fn require_exact(&self) -> bool {
        self.require_exact
    }

    /// 提示の識別子。
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }
}
