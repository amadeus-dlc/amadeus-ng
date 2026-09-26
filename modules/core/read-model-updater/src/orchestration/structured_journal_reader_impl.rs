//! `StructuredJournalReader` の SQLite 実装 — 本家の `journal` 表を渡された接続の上で読む。

use std::path::Path;

use rusqlite::{Connection, OptionalExtension as _, params};

use super::column_value::scan_position;
use super::journal_reader_impl::corrupt_error;
use super::store_failure::SqliteResultExt;
use super::{
    CorruptCause, GlobalSeqNr, JournalAnchor, JournalBatch, JournalReadError, JournalReaderImpl,
    StructuredJournalReader,
};

/// 位置の行の識別子 (アンカーの記録・照合の両方が使う)。
const SELECT_ANCHOR_ROW: &str = "SELECT aid, seq_nr FROM journal WHERE rowid = ?1";

/// 構造化面の更新器が使うジャーナルの読み手の実装。状態を持たない。
///
/// 行の読み方 (列・`manifest` による振り分け・復号) は `JournalReaderImpl` と 1 か所で共有する —
/// 読み方の正本を 2 つにしない。
#[derive(Debug, Clone, Copy, Default)]
pub struct StructuredJournalReaderImpl;

impl StructuredJournalReaderImpl {
    /// 失敗に添えるストアの場所 (接続が知っている場所。場所を持たない接続では空)。
    fn store(connection: &Connection) -> &Path {
        Path::new(connection.path().unwrap_or_default())
    }
}

impl StructuredJournalReader for StructuredJournalReaderImpl {
    fn events_after(
        &self,
        connection: &Connection,
        after: GlobalSeqNr,
    ) -> Result<JournalBatch, JournalReadError> {
        JournalReaderImpl::scan_range(connection, Self::store(connection), after, None)
    }

    fn events_through(
        &self,
        connection: &Connection,
        through: GlobalSeqNr,
    ) -> Result<JournalBatch, JournalReadError> {
        JournalReaderImpl::scan_range(
            connection,
            Self::store(connection),
            GlobalSeqNr::ZERO,
            Some(through),
        )
    }

    fn anchor_at(
        &self,
        connection: &Connection,
        position: GlobalSeqNr,
    ) -> Result<Option<JournalAnchor>, JournalReadError> {
        let position = scan_position(position)?;
        let found: Option<(String, i64)> = connection
            .query_row(SELECT_ANCHOR_ROW, params![position], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .optional()
            .at_connection(connection)?;
        found
            .map(|(aggregate_id, seq_nr)| {
                usize::try_from(seq_nr)
                    .map(|seq_nr| JournalAnchor::new(aggregate_id.clone(), seq_nr))
                    .map_err(|_| {
                        corrupt_error(&aggregate_id, None, CorruptCause::InvariantViolation)
                    })
            })
            .transpose()
    }
}
