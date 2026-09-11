//! Pipelineが引き渡したファイルの検証済み観測値。
/// パス、内容指紋、mtimeを一緒に保持する不変値。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PipelineHandoff {
    path: String,
    sha256: String,
    mtime_ms: String,
}
impl PipelineHandoff {
    /// ファイル境界または保存境界の観測を検査して組む。
    /// # Errors
    /// パスが空、SHA-256またはmtimeが不正な場合。
    pub fn new(
        path: String,
        sha256: String,
        mtime_ms: String,
    ) -> Result<Self, super::PipelineLinkError> {
        if path.is_empty()
            || path.contains('\0')
            || sha256.strip_prefix("sha256:").is_none_or(|hash| {
                hash.len() != 64
                    || !hash
                        .bytes()
                        .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
            })
            || mtime_ms
                .parse::<f64>()
                .ok()
                .is_none_or(|n| !n.is_finite() || n < 0.0)
        {
            return Err(super::PipelineLinkError::InvalidHandoff);
        }
        Ok(Self {
            path,
            sha256,
            mtime_ms,
        })
    }
    /// 監査・保存へ渡す相対パス。
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }
    /// 監査・保存へ渡す内容指紋。
    #[must_use]
    pub fn sha256(&self) -> &str {
        &self.sha256
    }
    /// 監査・保存へ渡すmtimeの公開表記。
    #[must_use]
    pub fn mtime_ms(&self) -> &str {
        &self.mtime_ms
    }
    pub(super) fn matches_current(&self, now: &Self) -> bool {
        self.path == now.path
            && self.sha256 == now.sha256
            && (self.millis() - now.millis()).abs() <= 0.01
    }
    pub(super) fn millis(&self) -> f64 {
        self.mtime_ms.parse().unwrap_or(f64::NEG_INFINITY)
    }
    #[expect(
        clippy::cast_precision_loss,
        reason = "本家のNumberによるmtimeミリ秒比較に揃える"
    )]
    pub(super) fn was_written_since(&self, at: chrono::DateTime<chrono::Utc>) -> bool {
        self.millis() >= at.timestamp() as f64 * 1000.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn snapshot(time: &str) -> PipelineHandoff {
        PipelineHandoff::new(
            "record/scan.md".into(),
            format!("sha256:{}", "a".repeat(64)),
            time.into(),
        )
        .unwrap()
    }
    #[test]
    fn invalid_recorded_values_cannot_form_a_handoff() {
        for (path, sha, mtime) in [
            ("", "a", "1000"),
            ("record\0", "a", "1000"),
            ("record", "sha256:bad", "1000"),
            ("record", "a", "NaN"),
            ("record", "a", "-1"),
            ("record", "a", "infinity"),
        ] {
            let sha = if sha == "a" {
                format!("sha256:{}", "a".repeat(64))
            } else {
                sha.into()
            };
            assert!(PipelineHandoff::new(path.into(), sha, mtime.into()).is_err());
        }
    }
    #[test]
    fn current_handoff_requires_the_same_bytes_and_bounded_timestamp_difference() {
        assert!(snapshot("1000.125").matches_current(&snapshot("1000.13")));
        assert!(!snapshot("1000.125").matches_current(&snapshot("1000.14")));
        let changed = PipelineHandoff::new(
            "record/scan.md".into(),
            format!("sha256:{}", "b".repeat(64)),
            "1000.125".into(),
        )
        .unwrap();
        assert!(!snapshot("1000.125").matches_current(&changed));
        let floor = chrono::DateTime::from_timestamp(1, 0).unwrap();
        assert!(snapshot("1000").was_written_since(floor));
        assert!(!snapshot("999.99").was_written_since(floor));
    }
}
