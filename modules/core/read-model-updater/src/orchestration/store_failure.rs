//! ジャーナル読取の失敗を `std::io::ErrorKind` の語彙へ落とす 1 か所 (NFR3.5 / 監査 C24)。
//!
//! コマンド側にも同名の写像がある。**意図的な複製**である — RMU は中間なのでコマンド側を
//! `Cargo.toml` に書くこと自体は許されるが、それでも共有しない。「エラー分類・I/O 写像は
//! 側ごとに専用化」が正本の裁定であり (`coding-rules/cqrs-boundaries.md` / 構成案 §3)、
//! 30 行の写像に面の正確さを売り渡さない。
//!
//! 複製とはいえ、こちらは**読取面が実際に踏む経路だけ**を持つ — 生の rusqlite の失敗である。
//! 本家ストアが `Box<dyn Error>` に包んで運ぶ失敗を開ける写像 (`io_kind_of_source`) は
//! コマンド側にしかない。RMU は本家のストア API を通らず、同じ DB ファイルへの別接続を
//! 自分で開くからである。

use std::io::ErrorKind;

use rusqlite::ErrorCode;

/// rusqlite の失敗を `std::io::ErrorKind` の語彙へ落とす。
///
/// `SQLITE_BUSY` / `SQLITE_LOCKED` を `WouldBlock` に写すのは、これが「壊れた」ではなく
/// 「いま他の書き手がいる」という**再実行で解ける**分類だからである (NFR3.5)。
pub(crate) const fn io_kind(error: &rusqlite::Error) -> ErrorKind {
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
}
