//! 表の DAO で書く更新器が所有する接続を開く 1 か所。
//!
//! 更新器は接続を 1 本所有し、`BEGIN IMMEDIATE` で開いたトランザクションを表の DAO へ渡す
//! (`coding-rules/read-model-updater-structure.md`「トランザクションの受け渡し」)。その接続の
//! 開き方 (ストアを作らない・URI を許す・書込ロックを待つ上限) を更新器ごとに書き下さない。

use std::path::Path;
use std::time::Duration;

use rusqlite::{Connection, OpenFlags};

use super::JournalReadError;
use super::store_failure::SqliteResultExt;

/// 書込ロックを待つ上限 (他の更新器と同じ既定)。
const BUSY_TIMEOUT: Duration = Duration::from_millis(5000);

/// 既存の共有ストアへ接続する。ストア自体は作らない (作るのは本家のイベントストアである)。
///
/// # Errors
///
/// ストアへ接続できない、待ち時間を設定できない場合 (`Io`)。
pub(super) fn open(path: &Path) -> Result<Connection, JournalReadError> {
    let connection = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_WRITE
            | OpenFlags::SQLITE_OPEN_URI
            | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .at_store(path)?;
    connection.busy_timeout(BUSY_TIMEOUT).at_store(path)?;
    Ok(connection)
}
