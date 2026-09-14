//! `CodekbPublishRefusal` — 集約 [`Codekb`](super::Codekb) が公開を拒んだ理由。

use super::codekb_generation::CodekbGeneration;
use super::codekb_source_fingerprint::CodekbSourceFingerprint;

/// 公開の compare-and-swap が噛み合わなかった理由 (材料のみ — 逐語文言は出す側が組む)。
///
/// 3 つはこの順で検査される (upstream `handleCodekbPublish` の逐語) — ストアの世代、源の
/// 指紋、候補の鮮度印。**最初に外れた 1 つだけ**を答えるので、直し方が一意に決まる。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodekbPublishRefusal {
    /// ストアが、写しを取ったあとに書き換えられていた。
    StoreChanged {
        /// 公開が前提としていた世代。
        expected: CodekbGeneration,
        /// いま実際に在る世代。
        found: CodekbGeneration,
    },
    /// 源が、写しを取ったあとに動いていた (計算できないことも「違う」に数える)。
    SourceChanged {
        /// 公開が前提としていた指紋。
        expected: CodekbSourceFingerprint,
        /// いま実際に採れる指紋 (採れなければ `None`)。
        found: Option<CodekbSourceFingerprint>,
    },
    /// 候補の鮮度印が、いまの源と食い違っていた。
    CandidateStale {
        /// 候補が記録している指紋 (記録が無ければ `None`)。
        staged: Option<String>,
        /// いまの源から採れる指紋 (採れなければ `None`)。
        current: Option<String>,
    },
}

impl std::fmt::Display for CodekbPublishRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CodekbPublishRefusal::StoreChanged { expected, found } => {
                write!(f, "store changed: expected {expected}, found {found}")
            }
            CodekbPublishRefusal::SourceChanged { expected, found } => write!(
                f,
                "source changed: expected {expected}, found {}",
                found
                    .as_ref()
                    .map_or_else(|| "unavailable".to_string(), ToString::to_string)
            ),
            CodekbPublishRefusal::CandidateStale { staged, current } => write!(
                f,
                "candidate stale: staged {}, current {}",
                staged.as_deref().unwrap_or("unknown"),
                current.as_deref().unwrap_or("unknown")
            ),
        }
    }
}

impl std::error::Error for CodekbPublishRefusal {}
