//! ストア層の失敗を `std::io::ErrorKind` の語彙へ落とす 1 か所 (NFR3.5 / 監査 C24)。
//!
//! クエリ側にも同名の写像がある。**意図的な複製**である — 両側は互いを知らないので
//! (`coding-rules/cqrs-boundaries.md`)、共有すればどちらかが相手を `Cargo.toml` に書くことに
//! なる。30 行の写像に側の独立を売り渡さない、という裁定 (構成案 §3)。
//!
//! 複製とはいえ、こちらは**コマンド側が実際に踏む経路**を持つ — 本家ストアが
//! `Box<dyn Error>` に包んで運ぶ失敗を開けて分類する。生の rusqlite を直接触るのはクエリ側
//! だけなので、そちらの複製には `io_kind_of_source` が無い。

use std::io::ErrorKind;

use rusqlite::ErrorCode;

/// rusqlite の失敗を `std::io::ErrorKind` の語彙へ落とす。
///
/// `SQLITE_BUSY` / `SQLITE_LOCKED` を `WouldBlock` に写すのは、これが「壊れた」ではなく
/// 「いま他の書き手がいる」という**再実行で解ける**分類だからである (NFR3.5)。
const fn io_kind(error: &rusqlite::Error) -> ErrorKind {
    let rusqlite::Error::SqliteFailure(inner, _) = error else {
        return ErrorKind::Other;
    };
    match inner.code {
        ErrorCode::DatabaseBusy | ErrorCode::DatabaseLocked => ErrorKind::WouldBlock,
        ErrorCode::CannotOpen | ErrorCode::NotFound => ErrorKind::NotFound,
        ErrorCode::PermissionDenied
        | ErrorCode::ReadOnly
        | ErrorCode::AuthorizationForStatementDenied => ErrorKind::PermissionDenied,
        ErrorCode::DatabaseCorrupt | ErrorCode::NotADatabase => ErrorKind::InvalidData,
        ErrorCode::OperationInterrupted => ErrorKind::Interrupted,
        _ => ErrorKind::Other,
    }
}

/// 本家の失敗が抱えている下位エラーを同じ語彙へ落とす。
///
/// 本家は rusqlite の失敗を `Box<dyn Error>` に包んで運ぶので、そこまで降りて分類する。
/// SQLite 由来でなければ分類しない (`Other`)。
pub(crate) fn io_kind_of_source(source: &(dyn std::error::Error + 'static)) -> ErrorKind {
    source.downcast_ref::<rusqlite::Error>().map_or_else(
        || ErrorKind::Other,
        |error: &rusqlite::Error| io_kind(error),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn failure(code: ErrorCode) -> rusqlite::Error {
        rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error {
                code,
                extended_code: 0,
            },
            None,
        )
    }

    #[test]
    fn the_error_codes_map_to_the_io_vocabulary() {
        assert_eq!(
            io_kind(&failure(ErrorCode::DatabaseBusy)),
            ErrorKind::WouldBlock
        );
        assert_eq!(
            io_kind(&failure(ErrorCode::DatabaseLocked)),
            ErrorKind::WouldBlock
        );
        assert_eq!(
            io_kind(&failure(ErrorCode::CannotOpen)),
            ErrorKind::NotFound
        );
        assert_eq!(io_kind(&failure(ErrorCode::NotFound)), ErrorKind::NotFound);
        assert_eq!(
            io_kind(&failure(ErrorCode::PermissionDenied)),
            ErrorKind::PermissionDenied
        );
        assert_eq!(
            io_kind(&failure(ErrorCode::ReadOnly)),
            ErrorKind::PermissionDenied
        );
        assert_eq!(
            io_kind(&failure(ErrorCode::AuthorizationForStatementDenied)),
            ErrorKind::PermissionDenied
        );
        assert_eq!(
            io_kind(&failure(ErrorCode::DatabaseCorrupt)),
            ErrorKind::InvalidData
        );
        assert_eq!(
            io_kind(&failure(ErrorCode::NotADatabase)),
            ErrorKind::InvalidData
        );
        assert_eq!(
            io_kind(&failure(ErrorCode::OperationInterrupted)),
            ErrorKind::Interrupted
        );
        assert_eq!(io_kind(&failure(ErrorCode::DiskFull)), ErrorKind::Other);
    }

    #[test]
    fn a_failure_that_is_not_from_sqlite_is_not_classified() {
        assert_eq!(
            io_kind(&rusqlite::Error::QueryReturnedNoRows),
            ErrorKind::Other
        );
    }

    #[test]
    fn a_boxed_sqlite_failure_keeps_its_classification() {
        let boxed: Box<dyn std::error::Error + Send + Sync> =
            Box::new(failure(ErrorCode::DatabaseBusy));
        assert_eq!(io_kind_of_source(boxed.as_ref()), ErrorKind::WouldBlock);
    }

    #[test]
    fn a_boxed_error_from_elsewhere_is_not_classified() {
        let boxed: Box<dyn std::error::Error + Send + Sync> =
            Box::new(std::io::Error::other("どこか別の層"));
        assert_eq!(io_kind_of_source(boxed.as_ref()), ErrorKind::Other);
    }
}
