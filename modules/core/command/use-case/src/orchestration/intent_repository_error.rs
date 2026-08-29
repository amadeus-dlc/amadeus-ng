//! `IntentRepositoryError` — `IntentRepository` の失敗 (材料のみ)。

use std::fmt;
use std::io::ErrorKind;
use std::path::PathBuf;

use core_command_domain::orchestration::IntentId;

use super::corrupt_cause::CorruptCause;

/// `IntentRepository` の失敗 (材料のみ — 逐語文言はアダプタ層)。
///
/// [`RepositoryError`] と**同じ名前の別の型**ではなく、別の面の別の型である。実行の
/// Repository が持つ `Conflict` を**ここは持たない** — 当面のポートは読取専用
/// (`find_by_id` だけ) であり、CAS を伴う書込が無い以上、楽観 version の競合は
/// **構成不能**だからである。無用な変種は「この面ではありえない」という情報を消してしまう
/// (`corrupt_cause.rs` と同じ考え方)。書込 (`store`) を足す U7 で必要になれば、そのときに
/// 変種を足す (additive-safe)。
///
/// [`RepositoryError`]: super::repository_error::RepositoryError
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntentRepositoryError {
    /// この識別子の intent がストアに無い。
    NotFound {
        /// 探した intent 識別子。
        intent_id: IntentId,
    },
    /// ストア I/O の失敗 (`ErrorKind` を保持)。
    Io {
        /// OS / ドライバ由来の分類。
        kind: ErrorKind,
        /// 対象パス (分からない場合は `None`)。
        path: Option<PathBuf>,
    },
    /// 行は読めたが intent へ写せない (復号不能・Always Valid 違反)。
    Corrupt {
        /// 対象の intent 識別子。
        intent_id: IntentId,
        /// 原因の分類。
        cause: CorruptCause,
    },
}

impl fmt::Display for IntentRepositoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IntentRepositoryError::NotFound { intent_id } => write!(f, "not found: {intent_id}"),
            IntentRepositoryError::Io { kind, path } => write!(
                f,
                "io: {kind:?} at {}",
                path.as_ref()
                    .map_or_else(|| "-".to_string(), |p| p.display().to_string())
            ),
            IntentRepositoryError::Corrupt { intent_id, cause } => {
                write!(f, "corrupt: intent {intent_id}, cause {cause}")
            }
        }
    }
}

impl std::error::Error for IntentRepositoryError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn intent_id() -> IntentId {
        IntentId::parse("01a02785-1bd8-76eb-aeea-5aa303ebd5b6").expect("UUIDv7")
    }

    #[test]
    fn every_variant_renders_its_material() {
        assert_eq!(
            IntentRepositoryError::NotFound {
                intent_id: intent_id()
            }
            .to_string(),
            "not found: 01a02785-1bd8-76eb-aeea-5aa303ebd5b6"
        );
        assert_eq!(
            IntentRepositoryError::Io {
                kind: ErrorKind::PermissionDenied,
                path: Some(PathBuf::from("/tmp/store.db")),
            }
            .to_string(),
            "io: PermissionDenied at /tmp/store.db"
        );
        assert_eq!(
            IntentRepositoryError::Io {
                kind: ErrorKind::NotFound,
                path: None,
            }
            .to_string(),
            "io: NotFound at -"
        );
        assert_eq!(
            IntentRepositoryError::Corrupt {
                intent_id: intent_id(),
                cause: CorruptCause::UndecodablePayload,
            }
            .to_string(),
            "corrupt: intent 01a02785-1bd8-76eb-aeea-5aa303ebd5b6, cause undecodable payload"
        );
    }

    #[test]
    fn the_rejection_is_a_std_error() {
        let error: Box<dyn std::error::Error> = Box::new(IntentRepositoryError::NotFound {
            intent_id: intent_id(),
        });
        assert_eq!(
            error.to_string(),
            "not found: 01a02785-1bd8-76eb-aeea-5aa303ebd5b6"
        );
    }

    #[test]
    fn failures_compare_by_value() {
        assert_eq!(
            IntentRepositoryError::NotFound {
                intent_id: intent_id()
            },
            IntentRepositoryError::NotFound {
                intent_id: intent_id()
            }
        );
    }
}
