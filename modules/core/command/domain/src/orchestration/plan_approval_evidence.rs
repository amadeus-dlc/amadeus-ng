//! 現在の発行と規則に一致した計画承認の質問。
use super::{
    CodeGenerationAuthority, EmbeddedTestingContract, PlanApprovalDocuments, PlanApprovalError,
    PlanQuestions, TestingPosture,
};
use core_infrastructure::{
    ecmascript::{trim, trim_end},
    hash::sha256_hex,
};
/// 文書の整合を確認した証拠。真正な応答の受領は別の状態遷移である。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanApprovalEvidence {
    authority: CodeGenerationAuthority,
    fingerprint: String,
    questions_file: String,
    questions_sha256: String,
    prompt_sha256: String,
}
impl PlanApprovalEvidence {
    /// 発行時に検証した指紋と正規の質問パスを再構成する。
    /// # Errors
    /// 指紋や質問パスの表記が不正な場合。
    pub fn new(
        authority: CodeGenerationAuthority,
        fingerprint: String,
        questions_file: String,
        questions_sha256: String,
        prompt_sha256: String,
    ) -> Result<Self, PlanApprovalError> {
        let hexadecimal = |value: &str| {
            value.len() == 64
                && value
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        };
        let fingerprint_valid = fingerprint.strip_prefix("sha256:").is_some_and(hexadecimal);
        let path_valid = !questions_file.is_empty()
            && !questions_file.contains(['\\', '\0', '\r', '\n'])
            && questions_file
                .split('/')
                .all(|part| !matches!(part, "" | "." | ".."));
        if !fingerprint_valid
            || !hexadecimal(&questions_sha256)
            || !hexadecimal(&prompt_sha256)
            || !path_valid
        {
            return Err(PlanApprovalError::new(
                "invalid recorded Plan Approval evidence",
            ));
        }
        Ok(Self {
            authority,
            fingerprint,
            questions_file,
            questions_sha256,
            prompt_sha256,
        })
    }

    /// 発行・現在のソース・文書・規則・回答欄を照合する。
    /// # Errors
    /// どれかが現在の承認対象と一致しない場合。
    pub fn verify(
        authority: CodeGenerationAuthority,
        documents: &PlanApprovalDocuments,
        posture: &TestingPosture,
        current_source: Option<&str>,
        expected_answer: &str,
    ) -> Result<Self, PlanApprovalError> {
        Self::verify_for_file(
            authority,
            documents,
            Some(posture),
            current_source,
            documents.questions_file(),
            expected_answer,
        )
    }
    /// 実際に指定されたファイルも正規の承認対象へ照合する。
    /// # Errors
    /// 対象・発行・ソース・文書・規則・回答欄が一致しない場合。
    pub fn verify_for_file(
        authority: CodeGenerationAuthority,
        documents: &PlanApprovalDocuments,
        posture: Option<&TestingPosture>,
        current_source: Option<&str>,
        supplied_questions_file: &str,
        expected_answer: &str,
    ) -> Result<Self, PlanApprovalError> {
        if authority.source_floor() != "unbindable"
            && current_source != Some(authority.source_floor())
        {
            return Err(PlanApprovalError::new(
                "Plan Approval requires workspace source to match the Code Generation directive's pre-planning source floor",
            ));
        }
        if supplied_questions_file != documents.questions_file() {
            return Err(PlanApprovalError::new(format!(
                "Plan Approval questions file must be the active target's canonical file: {}",
                documents.questions_file()
            )));
        }
        if trim(documents.plan()).is_empty() || trim(documents.instructions()).is_empty() {
            return Err(PlanApprovalError::new(
                "Plan Approval requires non-empty plan and unit-test instructions",
            ));
        }
        let embedded = EmbeddedTestingContract::parse(documents.plan())
            .filter(|contract| posture.is_some_and(|posture| contract.is_current(posture)))
            .ok_or_else(|| {
                PlanApprovalError::new("Plan Approval requires the current Testing Contract")
            })?;
        let fingerprint = authority.approval_fingerprint(documents, embedded.hash());
        let questions = PlanQuestions::parse(documents.questions());
        if questions.fingerprint() != Some(fingerprint.as_str()) {
            return Err(PlanApprovalError::new(
                "Plan Approval fingerprint does not match the active intent, target, directive epoch, plan, instructions, and Testing Contract",
            ));
        }
        if questions.answer() != Some(expected_answer) {
            return Err(PlanApprovalError::new(format!(
                "Plan Approval questions file must contain exactly [Answer]: {}",
                if expected_answer.is_empty() {
                    "(blank)"
                } else {
                    expected_answer
                }
            )));
        }
        let questions_sha256 = sha256_hex(documents.questions().as_bytes());
        let prompt_sha256 = sha256_hex(prompt_body(documents.questions()).as_bytes());
        Self::new(
            authority,
            fingerprint,
            documents.questions_file().to_string(),
            questions_sha256,
            prompt_sha256,
        )
    }
    /// 回答欄の変化を除き、同じ提示を指すか。
    #[must_use]
    pub fn same_prompt(&self, other: &Self) -> bool {
        self.authority == other.authority
            && self.fingerprint == other.fingerprint
            && self.questions_file == other.questions_file
            && self.prompt_sha256 == other.prompt_sha256
    }
    /// 検証した発行と対象。
    #[must_use]
    pub const fn authority(&self) -> &CodeGenerationAuthority {
        &self.authority
    }
    /// 計画・指示・契約の指紋。
    #[must_use]
    pub fn fingerprint(&self) -> &str {
        &self.fingerprint
    }
    /// 正規の質問ファイル。
    #[must_use]
    pub fn questions_file(&self) -> &str {
        &self.questions_file
    }
    /// 回答を含む質問原文の指紋。
    #[must_use]
    pub fn questions_sha256(&self) -> &str {
        &self.questions_sha256
    }
    /// 回答欄の値を除いた提示内容の指紋。
    #[must_use]
    pub fn prompt_sha256(&self) -> &str {
        &self.prompt_sha256
    }
}
pub(super) fn prompt_body(questions: &str) -> String {
    let is_line_end = |character| matches!(character, '\r' | '\n' | '\u{2028}' | '\u{2029}');
    let mut body = String::new();
    for line in questions.split_inclusive(is_line_end) {
        if line.starts_with("[Answer]:") {
            body.push_str("[Answer]:");
            if let Some(last) = line.chars().last().filter(|last| is_line_end(*last)) {
                body.push(last);
            }
        } else {
            body.push_str(line);
        }
    }
    format!("{}\n", trim_end(&body))
}
