//! レビュー依頼 1 件の識別 — Request Id と、その依頼が属する試行の ID。
use super::ReviewEvidenceError;
use core_infrastructure::hash::sha256_hex;

/// 依頼 1 件の Request Id と試行 ID の組。
///
/// upstream 2.8.2 は依頼のたびに `review:<32 桁の 16 進>` を発行して監査行と記録に残し
/// （`aidlc-log.ts` の `mintReviewRequestId`）、レビュアーの下書きと記録を**試行ごとの
/// ディレクトリ** `.aidlc-reviews/<stage>/stage/<attempt>/` へ置く
/// （`aidlc-lib.ts` の `reviewDraftRelativePath` / `reviewRecordRelativePath`）。
///
/// 試行 ID は upstream では試行の床（監査行の位置）の指紋である。この build では試行の床が
/// 集約の状態（[`super::ReviewAttempt`] の区間）なので、**試行の最初の依頼の Request Id**
/// の指紋を試行 ID とする。同じ試行の 2 回目以降の依頼は最初の依頼から試行 ID を引き継ぐ。
/// 値の綴り（16 桁の 16 進）は upstream と同じである。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewRequestIdentity {
    request_id: String,
    attempt: String,
}

impl ReviewRequestIdentity {
    /// 保存された組を検査して構築する（基本コンストラクタ）。
    ///
    /// # Errors
    ///
    /// Request Id が `review:<32 桁の 16 進>`、試行 ID が 16 桁の 16 進でない場合。
    pub fn new(request_id: String, attempt: String) -> Result<Self, ReviewEvidenceError> {
        let hex = |s: &str, len: usize| {
            s.len() == len
                && s.bytes()
                    .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
        };
        let valid_request = request_id
            .strip_prefix("review:")
            .is_some_and(|value| hex(value, 32));
        if !valid_request || !hex(&attempt, 16) {
            return Err(ReviewEvidenceError::InvalidBinding);
        }
        Ok(Self {
            request_id,
            attempt,
        })
    }

    /// 入力境界が採った nonce から新しい依頼の識別を算出する。
    ///
    /// `opening` は同じ試行の最初の依頼の識別である。無ければこの依頼が試行を開くので、
    /// 自分の Request Id から試行 ID を導く。
    ///
    /// # Errors
    ///
    /// nonce が 32 桁の 16 進でない場合。
    pub fn generate(
        nonce: &str,
        opening: Option<&ReviewRequestIdentity>,
    ) -> Result<Self, ReviewEvidenceError> {
        let request_id = format!("review:{nonce}");
        let attempt = opening.map_or_else(
            || sha256_hex(request_id.as_bytes()).chars().take(16).collect(),
            |opening| opening.attempt.clone(),
        );
        Self::new(request_id, attempt)
    }

    /// 依頼の Request Id（`review:<32 桁の 16 進>`）。
    #[must_use]
    pub fn request_id(&self) -> &str {
        &self.request_id
    }

    /// 依頼が属する試行の ID（16 桁の 16 進）。
    #[must_use]
    pub fn attempt(&self) -> &str {
        &self.attempt
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_opening_request_derives_the_attempt_from_its_own_request_id() {
        let opening = ReviewRequestIdentity::generate(&"a".repeat(32), None).unwrap();
        assert_eq!(opening.request_id(), format!("review:{}", "a".repeat(32)));
        assert_eq!(opening.attempt().len(), 16);

        let second = ReviewRequestIdentity::generate(&"b".repeat(32), Some(&opening)).unwrap();
        assert_eq!(second.attempt(), opening.attempt());
        assert_ne!(second.request_id(), opening.request_id());
    }

    #[test]
    fn a_malformed_pair_is_refused() {
        assert!(ReviewRequestIdentity::new("review:xyz".into(), "0".repeat(16)).is_err());
        assert!(
            ReviewRequestIdentity::new(format!("review:{}", "a".repeat(32)), "zz".into()).is_err()
        );
        assert!(ReviewRequestIdentity::generate("not-hex", None).is_err());
    }
}
