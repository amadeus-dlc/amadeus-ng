//! `CodekbArtifactNameError` — [`CodekbArtifactName::parse`](super::CodekbArtifactName::parse) の拒否理由。

/// codekb の成果物名として受理できなかった理由。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodekbArtifactNameError {
    /// 正準の 9 綴りのどれでもない。
    Unknown(String),
}

impl std::fmt::Display for CodekbArtifactNameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CodekbArtifactNameError::Unknown(given) => {
                write!(f, "not a CodeKB artifact name: {given:?}")
            }
        }
    }
}

impl std::error::Error for CodekbArtifactNameError {}
