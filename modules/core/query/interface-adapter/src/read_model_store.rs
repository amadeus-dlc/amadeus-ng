//! `read_*` 表への読取専用接続 (クレート内部の部品 — 公開型ではない)。
//!
//! DAO 実装 10 本が共有する唯一の I/O 面である。**媒体の選択はここに閉じる** — ポート面は
//! DTO しか語らないので、格納形式を替えても差し替わるのはこのクレートだけである
//! (`coding-rules/gateway-taxonomy.md` §3 の DAO 項)。
//!
//! # 読取専用で開く
//!
//! `SQLITE_OPEN_READ_ONLY` は「リードモデルは更新できない」(`cqrs-boundaries.md` 規則 6) の
//! **媒体側の裏取り**である。ポート面に更新動詞が無いことが型の保証で、この接続フラグが
//! 実行時の保証にあたる。作成フラグも外してあるので、存在しないパスに空 DB を作って
//! 「行が無い」と答えることは起きない — 開けないものは読取失敗である。
//!
//! 待ちの上限を置くのは、投影の差し替え (RMU の 1 トランザクション) と読取がかち合うのが
//! 通常の運用だからである。待ち切れなければ `WouldBlock` として上がる (再実行で解ける)。

use std::path::{Path, PathBuf};
use std::time::Duration;

use core_query_use_case::orchestration::ReadModelReadError;
use rusqlite::{Connection, OpenFlags, OptionalExtension, Row, ToSql};

use super::read_model_failure::read_failure;

/// 書込ロックを待つ上限 (RMU の差し替えは 1 トランザクションで短い)。
const BUSY_TIMEOUT: Duration = Duration::from_millis(5_000);

/// 読取専用に開いた 1 つのストア。
#[derive(Debug)]
pub(crate) struct ReadModelStore {
    connection: Connection,
    path: PathBuf,
}

impl ReadModelStore {
    /// ストアファイルを読取専用で開く。
    pub(crate) fn open(path: &Path) -> Result<ReadModelStore, ReadModelReadError> {
        let connection = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_ONLY
                | OpenFlags::SQLITE_OPEN_URI
                | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .map_err(|error| read_failure(&error, path))?;
        connection
            .busy_timeout(BUSY_TIMEOUT)
            .map_err(|error| read_failure(&error, path))?;
        Ok(ReadModelStore {
            connection,
            path: path.to_path_buf(),
        })
    }

    /// 高々 1 行を引く (行が無いのは失敗ではない)。
    pub(crate) fn find_one<T>(
        &self,
        sql: &str,
        params: &[&dyn ToSql],
        map: impl FnOnce(&Row<'_>) -> rusqlite::Result<T>,
    ) -> Result<Option<T>, ReadModelReadError> {
        self.connection
            .query_row(sql, params, map)
            .optional()
            .map_err(|error| read_failure(&error, &self.path))
    }
}
