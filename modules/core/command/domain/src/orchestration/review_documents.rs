//! レビュー対象の安定した原文集合と、その時点のソース観測。
use super::{
    ReviewArtifact, ReviewBinding, ReviewCompletion, ReviewDraft, ReviewEvidenceError,
    ReviewRequestIdentity, ReviewVerdict,
};
use core_infrastructure::{
    canon_json::{JsonValue, SerializationProfile, serialize},
    hash::sha256_hex,
};
/// ファイルの観測を保持する。受理判断と指紋計算の所有者。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewDocuments {
    artifacts: Vec<ReviewArtifact>,
    require_artifacts: bool,
    source: Option<String>,
    nonce: String,
    stable: bool,
    drafts: Vec<ReviewDraft>,
}
impl ReviewDocuments {
    /// 入力境界が読み取った全材料を束ねる。
    ///
    /// `drafts` は判定の iteration に当たる単独レビュー（試行ごとの下書き）である。依頼では
    /// 読まないので空でよい。
    #[must_use]
    pub const fn new(
        artifacts: Vec<ReviewArtifact>,
        require_artifacts: bool,
        source: Option<String>,
        nonce: String,
        stable: bool,
        drafts: Vec<ReviewDraft>,
    ) -> Self {
        Self {
            artifacts,
            require_artifacts,
            source,
            nonce,
            stable,
            drafts,
        }
    }
    fn target(&self) -> Result<&ReviewArtifact, ReviewEvidenceError> {
        if !self.stable {
            return Err(ReviewEvidenceError::ArtifactsUnavailable);
        }
        let target = self
            .artifacts
            .iter()
            .filter(|a| a.is_appendix_target())
            .min_by(|a, b| a.path().cmp(b.path()))
            .ok_or(ReviewEvidenceError::ArtifactsUnavailable)?;
        if !target.is_regular() || target.body().is_none() {
            return Err(ReviewEvidenceError::ArtifactsUnavailable);
        }
        Ok(target)
    }
    fn fingerprint(&self, binding: Option<(&str, usize)>) -> Result<String, ReviewEvidenceError> {
        if !self.stable {
            return Err(ReviewEvidenceError::ArtifactsUnavailable);
        }
        let mut artifacts: Vec<_> = self.artifacts.iter().collect();
        artifacts.sort_by(|a, b| a.path().cmp(b.path()));
        let mut manifest = Vec::new();
        for artifact in artifacts {
            let digest = if !artifact.is_regular() {
                if self.require_artifacts && artifact.is_required() {
                    return Err(ReviewEvidenceError::ArtifactsUnavailable);
                }
                "not-file".to_string()
            } else if let Some(body) = artifact.body() {
                let body = match binding {
                    Some((path, offset)) if path == artifact.path() => body
                        .get(..offset)
                        .ok_or(ReviewEvidenceError::ArtifactsUnavailable)?,
                    _ => body,
                };
                format!("sha256:{}", sha256_hex(body))
            } else {
                if self.require_artifacts && artifact.is_required() {
                    return Err(ReviewEvidenceError::ArtifactsUnavailable);
                }
                "missing".to_string()
            };
            manifest.push(JsonValue::Array(vec![
                JsonValue::String(artifact.path().to_string()),
                JsonValue::String(digest),
            ]));
        }
        let encoded = serialize(
            &JsonValue::Array(manifest),
            SerializationProfile::ContractCompact,
        );
        Ok(format!("sha256:{}", sha256_hex(encoded.as_bytes())))
    }
    /// 初回要求の原文と既存Review節を固定する。
    ///
    /// `opening` は同じ試行の最初の依頼の識別である（試行 ID を引き継ぐ）。
    /// # Errors
    /// 不安定・欠落成果物、不正な追記先またはnonce。
    pub fn bind(
        &self,
        opening: Option<&ReviewRequestIdentity>,
    ) -> Result<ReviewBinding, ReviewEvidenceError> {
        let target = self.target()?;
        let body = target
            .body()
            .ok_or(ReviewEvidenceError::ArtifactsUnavailable)?;
        let offset =
            super::review_appendix::ReviewAppendix::existing_offset(body).unwrap_or(body.len());
        let appendix = super::review_appendix::ReviewAppendix::new(
            body.get(offset..)
                .ok_or(ReviewEvidenceError::ArtifactsUnavailable)?
                .to_vec(),
        );
        ReviewBinding::new(
            self.fingerprint(Some((target.path(), offset)))?,
            target.path().to_string(),
            offset,
            appendix.digest(),
            appendix.evidence().len(),
            if appendix.evidence().is_empty() {
                None
            } else {
                Some(format!("review:{}", self.nonce))
            },
            self.source.clone(),
            ReviewRequestIdentity::generate(&self.nonce, opening)?,
        )
    }
    /// 要求時点と同じ原文を再び観測したか。
    /// # Errors
    /// 成果物・追記先またはソースが要求と異なる場合。
    pub fn verify_request(&self, request: &ReviewBinding) -> Result<(), ReviewEvidenceError> {
        let target = self.target()?;
        if target.path() != request.appendix_artifact()
            || self.fingerprint(Some((target.path(), request.appendix_offset())))?
                != request.fingerprint()
        {
            return Err(ReviewEvidenceError::ArtifactsChanged);
        }
        if self.source.as_deref() != request.source() {
            return Err(ReviewEvidenceError::SourceChanged);
        }
        Ok(())
    }
    /// レビュー（単独の下書き、または成果物への追記）を検証し、完成後の指紋を確定する。
    ///
    /// upstream 2.8.2 の `handleReview` と同じ順で決める: 依頼の試行に当たる下書きがあれば
    /// それがレビューであり、成果物は依頼時の原文のままでなければならない（追記も併せて
    /// あれば拒否）。下書きが無ければ、非推奨の入力経路である成果物への `## Review` 追記を
    /// 読む。どちらも無いのは、再試行済みの依頼を NOT-READY で閉じる場合だけである。
    /// # Errors
    /// 要求内容不一致、古いReview節、レビューの欠落・二重・不正。
    pub fn certify(
        &self,
        request: &ReviewBinding,
        reviewer: &str,
        iteration: u32,
        verdict: ReviewVerdict,
        retried: bool,
    ) -> Result<ReviewCompletion, ReviewEvidenceError> {
        self.verify_request(request)?;
        let body = self
            .target()?
            .body()
            .ok_or(ReviewEvidenceError::ArtifactsUnavailable)?;
        let appendix = super::review_appendix::ReviewAppendix::new(
            body.get(request.appendix_offset()..)
                .ok_or(ReviewEvidenceError::ArtifactsUnavailable)?
                .to_vec(),
        );
        let appended = appendix.digest() != request.prior_digest();
        let draft = self
            .drafts
            .iter()
            .find(|draft| draft.attempt() == request.identity().attempt());
        if let Some(draft) = draft {
            if appended {
                return Err(ReviewEvidenceError::ReviewWrittenTwice);
            }
            super::review_appendix::ReviewAppendix::new(draft.body().to_vec())
                .validate(reviewer, iteration, verdict, None, true)?;
            return ReviewCompletion::new(request.clone(), self.fingerprint(None)?);
        }
        if request.prior_length() > 0
            && appendix
                .evidence()
                .get(..request.prior_length())
                .is_some_and(|prefix| {
                    format!("sha256:{}", sha256_hex(prefix)) == request.prior_digest()
                })
        {
            return Err(ReviewEvidenceError::StaleAppendix);
        }
        if appendix.is_empty() {
            if retried && verdict == ReviewVerdict::NotReady {
                return ReviewCompletion::new(request.clone(), self.fingerprint(None)?);
            }
            return Err(ReviewEvidenceError::ReviewMissing(
                request.identity().attempt().to_string(),
            ));
        }
        appendix.validate(reviewer, iteration, verdict, request.challenge(), false)?;
        ReviewCompletion::new(request.clone(), self.fingerprint(None)?)
    }
    /// 受領後の全成果物とソースが一致するか。
    #[must_use]
    pub fn covers(&self, completion: &ReviewCompletion) -> bool {
        self.source.as_deref() == completion.request().source()
            && self
                .fingerprint(None)
                .is_ok_and(|hash| hash == completion.fingerprint())
    }
}
