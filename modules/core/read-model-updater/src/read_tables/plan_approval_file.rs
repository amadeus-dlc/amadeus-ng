//! ワークスペース全体の承認状態から描く、本家互換の機械ローカルファイル。
use core_command_domain::orchestration::PlanChallenge;
use core_infrastructure::canon_json::{
    JsonValue, Number, ObjectMembers, SerializationProfile, serialize,
};
/// 完全に再生成できる投影ファイル。監査や利用者文書は含まない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanApprovalFile {
    name: String,
    content: String,
}
impl PlanApprovalFile {
    const fn new(name: String, content: String) -> Self {
        Self { name, content }
    }
    pub(super) fn challenge(challenge: &PlanChallenge) -> Self {
        let evidence = challenge.evidence();
        let authority = evidence.authority();
        let mut fields = ObjectMembers::new();
        fields.insert("version", JsonValue::Number(Number::PosInt(1)));
        for (name, value) in [
            ("targetId", authority.target_id()),
            ("intentId", authority.intent_id()),
            ("directiveEpoch", authority.directive_epoch()),
            ("runFloor", authority.run_floor()),
            ("fingerprint", evidence.fingerprint()),
            ("questionsFile", evidence.questions_file()),
            ("promptSha256", evidence.prompt_sha256()),
            ("sourceFloor", authority.source_floor()),
        ] {
            fields.insert(name, JsonValue::String(value.to_string()));
        }
        fields.insert(
            "markerRevision",
            JsonValue::Number(Number::PosInt(authority.marker_revision())),
        );
        fields.insert(
            "session",
            JsonValue::String(challenge.session().raw().to_string()),
        );
        fields.insert("challengeId", JsonValue::String(challenge.id().to_string()));
        fields.insert(
            "options",
            JsonValue::Array(
                challenge
                    .options()
                    .iter()
                    .map(|option| JsonValue::String(option.clone()))
                    .collect(),
            ),
        );
        fields.insert(
            "requireExactOptionLabels",
            JsonValue::Bool(challenge.require_exact()),
        );
        fields.insert("hashedOptionLabels", JsonValue::Bool(false));
        Self::new(
            format!("challenge-{}.json", challenge.session().key()),
            serialize(
                &JsonValue::Object(fields),
                SerializationProfile::ContractPretty,
            ),
        )
    }
    pub(super) fn receipt(
        receipt: &core_command_domain::orchestration::PlanApprovalReceipt,
    ) -> Self {
        let evidence = receipt.decision().evidence();
        let authority = evidence.authority();
        let mut fields = ObjectMembers::new();
        fields.insert("version", JsonValue::Number(Number::PosInt(1)));
        for (name, value) in [
            ("targetId", authority.target_id()),
            ("intentId", authority.intent_id()),
            ("directiveEpoch", authority.directive_epoch()),
            ("runFloor", authority.run_floor()),
            ("fingerprint", evidence.fingerprint()),
            ("questionsFile", evidence.questions_file()),
            ("promptSha256", evidence.prompt_sha256()),
            ("sourceFloor", authority.source_floor()),
        ] {
            fields.insert(name, JsonValue::String(value.to_string()));
        }
        fields.insert(
            "markerRevision",
            JsonValue::Number(Number::PosInt(authority.marker_revision())),
        );
        for (name, value) in [
            ("session", receipt.decision().session().raw()),
            ("challengeId", receipt.challenge_id()),
            ("choice", "Approve Plan"),
            ("questionsSha256", evidence.questions_sha256()),
            ("certifiedSourceSha256", receipt.certified_source()),
            ("status", receipt.status().as_str()),
        ] {
            fields.insert(name, JsonValue::String(value.to_string()));
        }
        Self::new(
            format!("receipt-{}.json", receipt.key()),
            serialize(
                &JsonValue::Object(fields),
                SerializationProfile::ContractPretty,
            ),
        )
    }
    pub(super) fn response(
        challenge: &PlanChallenge,
        response: &core_command_domain::orchestration::PlanHumanResponse,
    ) -> Self {
        let mut fields = ObjectMembers::new();
        fields.insert("version", JsonValue::Number(Number::PosInt(1)));
        fields.insert(
            "session",
            JsonValue::String(challenge.session().raw().to_string()),
        );
        fields.insert("challengeId", JsonValue::String(challenge.id().to_string()));
        fields.insert(
            "choice",
            JsonValue::String(response.choice().as_str().to_string()),
        );
        fields.insert(
            "responseSha256",
            JsonValue::String(response.response_sha256().to_string()),
        );
        Self::new(
            format!("response-{}.json", challenge.session().key()),
            serialize(
                &JsonValue::Object(fields),
                SerializationProfile::ContractPretty,
            ),
        )
    }
    /// Runtimeディレクトリ内のファイル名。
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
    /// イベントから描いた本文。
    #[must_use]
    pub fn content(&self) -> &str {
        &self.content
    }
}
