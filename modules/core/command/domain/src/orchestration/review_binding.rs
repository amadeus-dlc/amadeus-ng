//! レビュー要求を原文と追記境界へ結び付ける値。
use super::{ReviewEvidenceError, ReviewRequestIdentity};
/// 要求時点の証拠。判定や再試行で新しい原文へ置き換えない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewBinding {
    fingerprint: String,
    appendix_artifact: String,
    appendix_offset: usize,
    prior_digest: String,
    prior_length: usize,
    challenge: Option<String>,
    source: Option<String>,
    identity: ReviewRequestIdentity,
}
impl ReviewBinding {
    /// 全結合材料を検査して構築する。
    /// # Errors
    /// 指紋または追記証拠の形式が不正な場合。
    #[expect(
        clippy::too_many_arguments,
        reason = "要求時の証拠は互いに独立した材料であり、束ねる上位の語がドメインに無い"
    )]
    pub fn new(
        fingerprint: String,
        appendix_artifact: String,
        appendix_offset: usize,
        prior_digest: String,
        prior_length: usize,
        challenge: Option<String>,
        source: Option<String>,
        identity: ReviewRequestIdentity,
    ) -> Result<Self, ReviewEvidenceError> {
        let digest = |s: &str| {
            s.strip_prefix("sha256:").is_some_and(|v| {
                v.len() == 64
                    && v.bytes()
                        .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
            })
        };
        if !digest(&fingerprint)
            || appendix_artifact.is_empty()
            || (prior_length == 0 && (prior_digest != "none" || challenge.is_some()))
            || (prior_length > 0
                && (!digest(&prior_digest)
                    || !challenge.as_deref().is_some_and(|s| {
                        s.strip_prefix("review:").is_some_and(|v| {
                            v.len() == 32
                                && v.bytes()
                                    .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
                        })
                    })))
        {
            return Err(ReviewEvidenceError::InvalidBinding);
        }
        Ok(Self {
            fingerprint,
            appendix_artifact,
            appendix_offset,
            prior_digest,
            prior_length,
            challenge,
            source,
            identity,
        })
    }
    /// 要求本文の指紋。
    #[must_use]
    pub fn fingerprint(&self) -> &str {
        &self.fingerprint
    }
    /// 追記対象の論理パス。
    #[must_use]
    pub fn appendix_artifact(&self) -> &str {
        &self.appendix_artifact
    }
    /// 追記可能な先頭バイト位置。
    #[must_use]
    pub const fn appendix_offset(&self) -> usize {
        self.appendix_offset
    }
    /// 既存Review節の指紋またはnone。
    #[must_use]
    pub fn prior_digest(&self) -> &str {
        &self.prior_digest
    }
    /// 既存Review証跡のバイト長。
    #[must_use]
    pub const fn prior_length(&self) -> usize {
        self.prior_length
    }
    /// 既存Review節を置き換える要求のchallenge。
    #[must_use]
    pub fn challenge(&self) -> Option<&str> {
        self.challenge.as_deref()
    }
    /// ソースを書き換える段階で観測した指紋。
    #[must_use]
    pub fn source(&self) -> Option<&str> {
        self.source.as_deref()
    }
    /// 依頼の識別（Request Id と試行 ID）。
    #[must_use]
    pub const fn identity(&self) -> &ReviewRequestIdentity {
        &self.identity
    }
}
