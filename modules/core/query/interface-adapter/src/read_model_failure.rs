//! リードモデル読取の失敗を `std::io::ErrorKind` の語彙へ落とす 1 か所 (公開型ゼロ)。
//!
//! コマンド側と RMU にも同名の写像がある。**意図的な複製**である — 「エラー分類・I/O 写像は
//! 側ごとに専用化」が正本の裁定であり (`coding-rules/cqrs-boundaries.md`)、30 行の写像に
//! 面の正確さを売り渡さない。
//!
//! こちらは**読取面が踏む経路だけ**を持つ。行の整数が `u32` に収まらない
//! (`IntegralValueOutOfRange`) を `InvalidData` に落とすのはこの側だけである — 書込側は
//! 収まる値しか書かないので、収まらない値に出会ったら行が壊れている。

use std::io::ErrorKind;
use std::path::Path;

use core_query_use_case::orchestration::ReadModelReadError;
use rusqlite::ErrorCode;

/// rusqlite の失敗を、所在を添えたポート面の失敗へ写す。
pub(crate) fn read_failure(error: &rusqlite::Error, path: &Path) -> ReadModelReadError {
    ReadModelReadError::new(io_kind(error), Some(path.to_path_buf()))
}

/// rusqlite の失敗を `std::io::ErrorKind` の語彙へ落とす。
///
/// `SQLITE_BUSY` / `SQLITE_LOCKED` を `WouldBlock` に写すのは、これが「壊れた」ではなく
/// 「いま他の書き手がいる」という**再実行で解ける**分類だからである (NFR3.5)。
const fn io_kind(error: &rusqlite::Error) -> ErrorKind {
    let rusqlite::Error::SqliteFailure(inner, _) = error else {
        // 行の整数が期待の幅に収まらないのは行の破損である (書込側は収まる値しか書かない)。
        return match error {
            rusqlite::Error::IntegralValueOutOfRange(_, _)
            | rusqlite::Error::InvalidColumnType(_, _, _) => ErrorKind::InvalidData,
            _ => ErrorKind::Other,
        };
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
            io_kind(&failure(ErrorCode::CannotOpen)),
            ErrorKind::NotFound
        );
        assert_eq!(
            io_kind(&failure(ErrorCode::ReadOnly)),
            ErrorKind::PermissionDenied
        );
        assert_eq!(
            io_kind(&failure(ErrorCode::DatabaseCorrupt)),
            ErrorKind::InvalidData
        );
        assert_eq!(
            io_kind(&failure(ErrorCode::OperationInterrupted)),
            ErrorKind::Interrupted
        );
        assert_eq!(io_kind(&failure(ErrorCode::DiskFull)), ErrorKind::Other);
    }

    #[test]
    fn a_row_whose_integer_does_not_fit_is_corrupt_data() {
        assert_eq!(
            io_kind(&rusqlite::Error::IntegralValueOutOfRange(0, i64::MAX)),
            ErrorKind::InvalidData
        );
    }

    #[test]
    fn a_failure_that_is_not_from_sqlite_is_not_classified() {
        assert_eq!(
            io_kind(&rusqlite::Error::QueryReturnedNoRows),
            ErrorKind::Other
        );
    }

    #[test]
    fn the_failure_carries_the_place_it_tried_to_read() {
        let error = read_failure(
            &failure(ErrorCode::CannotOpen),
            Path::new("/r/store.sqlite3"),
        );
        assert_eq!(error.kind(), ErrorKind::NotFound);
        assert_eq!(error.path(), Some(Path::new("/r/store.sqlite3")));
    }
}
