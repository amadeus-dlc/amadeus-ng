//! `DefinitionArtifactsError` — 配布物を取り込めない形 (材料のみ)。

use std::error::Error;
use std::fmt;
use std::io::ErrorKind;
use std::path::PathBuf;

/// 配布物を取り込めない形 (材料のみ — 逐語文言は出す側が組む)。
///
/// `Corrupt` の**分類は契約に載せない** (裁定 6 — エラーは契約の一部であり、内部実装が
/// バレる情報を含めない)。どのファイルがどう壊れていたかはアダプタ私有の型を
/// `Error::source` の連鎖で運ぶ。
///
/// **3 入力で失敗態度が非対称なことは実装の挙動として維持される** (12 §4。この非対称そのものが
/// 観測可能な契約で、「より厳格にする」方向の改変も逸脱になる):
///
/// - `harness.json` が読めない / 不正 JSON / `name` 欠落 = **fatal**。定義 id の供給元であり、
///   失われると集約に識別子を与えられない (ADR-008)。
/// - `stage-graph.json` が読めない / 不正 JSON = **fatal**。
/// - `scope-grid.json` が読めない / 不正 = **fatal にしない**。グラフの `scopes[]` からの
///   転置導出へフォールバックする。したがって `load` はグリッド欠損では失敗しない。
/// - identity ファイルとグリッド列の不一致は**双方向とも正当**であり、どちらもエラーにしない。
///
/// `source` が比較不能なため `PartialEq` は実装しない — テストは `matches!` と `source` の
/// 文字列確認で判定する。
#[derive(Debug)]
pub enum DefinitionArtifactsError {
    /// OS 由来の読取失敗 (欠損・権限・種別違い)。**内容の破損ではない**。
    Io {
        /// OS / ドライバ由来の分類。
        kind: ErrorKind,
        /// 読もうとしたパス。
        path: PathBuf,
    },
    /// 読めたが内容が壊れている。
    Corrupt {
        /// アダプタ私有の原因 (契約は型を約束しない — 診断表示だけを運ぶ)。
        source: Box<dyn Error + Send + Sync>,
    },
}

impl fmt::Display for DefinitionArtifactsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DefinitionArtifactsError::Io { kind, path } => {
                write!(f, "io: {kind:?} at {}", path.display())
            }
            DefinitionArtifactsError::Corrupt { .. } => f.write_str("corrupt definition artifacts"),
        }
    }
}

impl Error for DefinitionArtifactsError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            DefinitionArtifactsError::Corrupt { source } => Some(source.as_ref()),
            DefinitionArtifactsError::Io { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct FakeCause;

    impl fmt::Display for FakeCause {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("stage graph at /d/stage-graph.json is not valid JSON")
        }
    }

    impl Error for FakeCause {}

    #[test]
    fn every_variant_renders_its_material() {
        assert_eq!(
            DefinitionArtifactsError::Io {
                kind: ErrorKind::NotFound,
                path: PathBuf::from("/d/stage-graph.json"),
            }
            .to_string(),
            "io: NotFound at /d/stage-graph.json"
        );
        assert_eq!(
            DefinitionArtifactsError::Corrupt {
                source: Box::new(FakeCause),
            }
            .to_string(),
            "corrupt definition artifacts"
        );
    }

    #[test]
    fn the_corrupt_cause_travels_the_source_chain() {
        // 分類は契約に載らない (裁定 6) — 原因は `Error::source` の連鎖で診断表示だけを運ぶ。
        let error = DefinitionArtifactsError::Corrupt {
            source: Box::new(FakeCause),
        };
        assert_eq!(
            Error::source(&error)
                .expect("Corrupt は原因を連鎖する")
                .to_string(),
            "stage graph at /d/stage-graph.json is not valid JSON"
        );
        assert!(
            Error::source(&DefinitionArtifactsError::Io {
                kind: ErrorKind::NotFound,
                path: PathBuf::from("/d"),
            })
            .is_none()
        );
    }
}
