//! `SecretFile` の読取・鋳造の失敗。

use std::io;
use std::path::PathBuf;

/// 秘密鍵ファイルの読取・鋳造の失敗（**材料のみ** — 利用者向け文言は出す側が組む）。
///
/// 変種が読取と作成で分かれているのは、復旧手順が違うからである（読めないのはファイルの
/// 問題、作れないのはディレクトリの権限の問題）。`coding-rules/error-handling.md`。
#[derive(Debug)]
pub enum SecretFileError {
    /// ファイルはあるが中身が鍵として成立しない（長さ違い・非正準な綴り）。
    Corrupt {
        /// 問題のあったファイル。
        path: PathBuf,
    },
    /// ファイルはあるが読めない。
    Unreadable {
        /// 読もうとしたファイル。
        path: PathBuf,
        /// OS が返した原因。
        cause: io::Error,
    },
    /// 鋳造できない（親ディレクトリの権限・ディスクなど）。
    Uncreatable {
        /// 作ろうとしたファイル。
        path: PathBuf,
        /// OS が返した原因。
        cause: io::Error,
    },
}

impl core::fmt::Display for SecretFileError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            SecretFileError::Corrupt { path } => {
                write!(f, "corrupt secret file: {}", path.display())
            }
            SecretFileError::Unreadable { path, cause } => {
                write!(f, "unreadable secret file: {} ({cause})", path.display())
            }
            SecretFileError::Uncreatable { path, cause } => {
                write!(f, "uncreatable secret file: {} ({cause})", path.display())
            }
        }
    }
}

impl std::error::Error for SecretFileError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            SecretFileError::Corrupt { .. } => None,
            SecretFileError::Unreadable { cause, .. }
            | SecretFileError::Uncreatable { cause, .. } => Some(cause),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_error_envelope_names_the_offending_path() {
        use tempfile::tempdir;

        // 元は `secret_file.rs` の `secret_at` ヘルパー（`SecretFile::new(dir.join("nested")
        // .join(".secret"), LEN)`）で組んだパスを使っていた。テストが要るのは
        // 「秘密鍵ファイルのパス」であって `SecretFile` の構築そのものではないため、
        // ファイル分割にあたり同じパスをここで直接組む（他ファイルの private ヘルパーへ
        // 依存させない）。
        let dir = tempdir().unwrap();
        let path = dir.path().join("nested").join(".secret");
        let error = SecretFileError::Corrupt { path };
        assert!(error.to_string().contains(".secret"));
    }

    /// 3 態はそれぞれ別の文言になり、原因が在るものは `source` で辿れる（材料のみ）。
    #[test]
    fn the_three_failures_read_differently_and_chain_their_cause() {
        use std::error::Error as _;

        let corrupt = SecretFileError::Corrupt {
            path: PathBuf::from("/ws/key"),
        };
        let unreadable = SecretFileError::Unreadable {
            path: PathBuf::from("/ws/key"),
            cause: io::Error::other("EACCES"),
        };
        let uncreatable = SecretFileError::Uncreatable {
            path: PathBuf::from("/ws/key"),
            cause: io::Error::other("EROFS"),
        };

        assert_eq!(corrupt.to_string(), "corrupt secret file: /ws/key");
        assert_eq!(
            unreadable.to_string(),
            "unreadable secret file: /ws/key (EACCES)"
        );
        assert_eq!(
            uncreatable.to_string(),
            "uncreatable secret file: /ws/key (EROFS)"
        );
        assert!(corrupt.source().is_none(), "壊れた鍵に下位原因は無い");
        assert_eq!(
            unreadable.source().map(ToString::to_string),
            Some("EACCES".to_string())
        );
        assert_eq!(
            uncreatable.source().map(ToString::to_string),
            Some("EROFS".to_string())
        );
    }
}
