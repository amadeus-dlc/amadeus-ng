//! 構造化リードモデルの読取失敗 — 全 DAO で 1 本。
//!
//! `read_*` 表を読む口はどれも同じ媒体 (RMU が投影を書いた 1 つのストア) を見るので、
//! 失敗の語彙をポートごとに分ける理由が無い。1 本に収束させるのは
//! `coding-rules/error-handling.md`「Repository エラーはジェネリック 1 本」と同じ趣旨で
//! ある — 契約は「読めなかった」としか約束せず、どの表のどの行がどう壊れていたかは
//! 語らない。
//!
//! **不在はここに来ない。** 行が無いのは正常な観測 (まだ投影されていない・そのキーの答えが
//! 無い) であり、各 `find` の `Ok(None)` が運ぶ。ここが捉えるのは「引けなかった」だけで
//! ある。
//!
//! 運ぶのは**材料だけ**で、利用者向けの逐語文言は出す側 (プレゼンタ) が組む。分類は
//! `std::io::ErrorKind` の語彙へ落とす — 再実行で解ける失敗 (`WouldBlock`) と壊れている
//! 失敗 (`InvalidData`) を呼び手が見分けられる最小限であり、媒体の語 (SQLite のエラー
//! コード) をポート面に漏らさない。

use std::error::Error;
use std::fmt;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

/// リードモデルを引けなかった (行の不在は失敗ではない)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadModelReadError {
    kind: ErrorKind,
    path: Option<PathBuf>,
}

impl ReadModelReadError {
    /// 失敗の分類と、読もうとした対象の所在を束ねる (**この型の唯一の構築経路**)。
    ///
    /// `path` が `None` なのは所在を名指せない失敗 (接続を開く前に潰えた等) である。
    #[must_use]
    pub const fn new(kind: ErrorKind, path: Option<PathBuf>) -> ReadModelReadError {
        ReadModelReadError { kind, path }
    }

    /// 失敗の分類。
    #[must_use]
    pub const fn kind(&self) -> ErrorKind {
        self.kind
    }

    /// 同一スナップショットの FK が指す先を引けなかった (**壊れた投影**)。
    ///
    /// ジャーナル由来の 15 表は 1 トランザクションで差し替わるので、その中の FK が宙に
    /// 浮くことは投影が壊れていない限り起きない。したがってこれは「行が無い」という
    /// 正常な観測ではなく読取失敗である (基本コンストラクタ [`ReadModelReadError::new`]
    /// へ委譲する補助コンストラクタ)。所在を名指さないのは、たどっているのがユースケース
    /// であり媒体を知らないからである。
    #[must_use]
    pub const fn broken_projection() -> ReadModelReadError {
        ReadModelReadError::new(ErrorKind::InvalidData, None)
    }

    /// 読もうとした対象の所在 (名指せないときは `None`)。
    #[must_use]
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }
}

impl fmt::Display for ReadModelReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.path {
            Some(path) => write!(f, "{}: {}", path.display(), self.kind),
            None => write!(f, "{}", self.kind),
        }
    }
}

impl Error for ReadModelReadError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_failure_describes_the_place_and_the_classification() {
        let error = ReadModelReadError::new(
            ErrorKind::WouldBlock,
            Some(PathBuf::from(
                "/r/aidlc/spaces/default/intents/store.sqlite3",
            )),
        );
        assert_eq!(error.kind(), ErrorKind::WouldBlock);
        assert_eq!(
            error.path(),
            Some(Path::new("/r/aidlc/spaces/default/intents/store.sqlite3"))
        );
        assert!(error.to_string().starts_with("/r/aidlc"));
    }

    #[test]
    fn a_dangling_foreign_key_in_one_snapshot_is_classified_as_broken_data() {
        let error = ReadModelReadError::broken_projection();
        assert_eq!(error.kind(), ErrorKind::InvalidData);
        assert_eq!(error.path(), None, "たどる側は媒体の所在を知らない");
    }

    #[test]
    fn a_failure_without_a_place_prints_the_classification_alone() {
        let error = ReadModelReadError::new(ErrorKind::InvalidData, None);
        assert_eq!(error.path(), None);
        assert_eq!(error.to_string(), ErrorKind::InvalidData.to_string());
    }
}
