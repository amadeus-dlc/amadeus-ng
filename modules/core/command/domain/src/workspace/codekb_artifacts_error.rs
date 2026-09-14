//! `CodekbArtifactsError` — [`CodekbArtifacts::of`](super::CodekbArtifacts::of) の拒否理由。

/// 9 成果物の集合として受理できなかった理由 (材料のみ — 逐語文言は出す側が組む)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodekbArtifactsError {
    /// 正準の 9 つちょうどではない (欠けている・余分がある・重複している)。
    NotTheCanonicalNine {
        /// 実際に在った綴り (辞書順)。
        found: Vec<String>,
    },
}

impl std::fmt::Display for CodekbArtifactsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CodekbArtifactsError::NotTheCanonicalNine { found } => {
                write!(
                    f,
                    "not the canonical nine CodeKB artifacts: {}",
                    found.join(", ")
                )
            }
        }
    }
}

impl std::error::Error for CodekbArtifactsError {}
