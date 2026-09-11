//! 確認対象ファイルと検証済み内容の結合。
use super::SummaryQuestionsError;
/// ファイルの場所は入力境界で制約し、内容はドメインが検証する。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SummaryEvidence {
    questions_file: String,
    questions_sha256: String,
}
impl SummaryEvidence {
    /// 読み取った文書の検証結果を保持する。
    /// # Errors
    /// 内容ハッシュが小文字64桁のSHA-256でない場合。
    pub fn new(
        questions_file: impl Into<String>,
        questions_sha256: impl Into<String>,
    ) -> Result<Self, SummaryQuestionsError> {
        let questions_sha256 = questions_sha256.into();
        if questions_sha256.len() != 64
            || !questions_sha256
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(SummaryQuestionsError::InvalidStructure(
                "invalid SHA-256 content digest".to_string(),
            ));
        }
        Ok(Self {
            questions_file: questions_file.into(),
            questions_sha256,
        })
    }
    /// 公開監査に載せる相対パス。
    #[must_use]
    pub fn questions_file(&self) -> &str {
        &self.questions_file
    }
    /// 内容の検証済みハッシュ。
    #[must_use]
    pub fn questions_sha256(&self) -> &str {
        &self.questions_sha256
    }
}
