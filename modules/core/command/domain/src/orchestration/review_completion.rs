//! レビュー判定と同時に確定した原文の結合。
use super::{ReviewBinding, ReviewEvidenceError};
/// 要求時証拠と、Review節を含む完成文書指紋。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewCompletion {
    request: ReviewBinding,
    fingerprint: String,
}
impl ReviewCompletion {
    /// 検証された要求と完成文書の識別子を束ねる。
    /// # Errors
    /// 完成文書指紋の形式が不正な場合。
    pub fn new(request: ReviewBinding, fingerprint: String) -> Result<Self, ReviewEvidenceError> {
        if !fingerprint.strip_prefix("sha256:").is_some_and(|v| {
            v.len() == 64
                && v.bytes()
                    .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
        }) {
            return Err(ReviewEvidenceError::InvalidBinding);
        }
        Ok(Self {
            request,
            fingerprint,
        })
    }
    /// 元の要求証拠。
    #[must_use]
    pub const fn request(&self) -> &ReviewBinding {
        &self.request
    }
    /// Review節を含む全成果物の指紋。
    #[must_use]
    pub fn fingerprint(&self) -> &str {
        &self.fingerprint
    }
}
