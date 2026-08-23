//! `EventStoreImpl` — `EventStore` と `JournalReader` の SQLite 実装 (BR2.1〜BR2.4 / BR2.6)。
//!
//! # 接続は単一所有である
//!
//! 接続は本型が**直接所有**し、内部可変性 (`RefCell`) も共有ハンドル (`Rc`) も持たない。
//! 読取 (`get_latest_snapshot_by_id` / `get_events_by_id_since_seq_nr` / `events_after` /
//! `checkpoint`) は `&self`、書込 (`persist_event` / `persist_event_and_snapshot` /
//! `advance_checkpoint` / `within_write_transaction`) は `&mut self` で、rusqlite の
//! `Connection::prepare` (`&self`) と `Connection::transaction` (`&mut self`) にそのまま
//! 対応する。可変操作を `&self` に見せて内部可変性で隠すのは「`&self` への偽装」であり
//! 禁止されている (`coding-rules/interior-mutability.md`)。
//!
//! `Clone` は実装しない — `clone` が同じ接続を指す別ハンドルを配ると `&mut self` の
//! 排他性が嘘になるからである。同じストアを指す別インスタンスがほしい場合は
//! [`EventStoreImpl::open`] で開き直す (それが「別プロセスからの再オープン」の実体でもある)。
//!
//! `Connection` は `Send` ではない。tokio の current_thread ランタイムで回す設計
//! (C3 / Q3 = A) であり、`Send` は要求しない。
//!
//! # トランザクション
//!
//! 書込はすべて `BEGIN IMMEDIATE` で始める (BR2.3)。`DEFERRED` だと読取から書込へ昇格する
//! 瞬間に `SQLITE_BUSY` が起き、しかもその時点では待てない (デッドロック回避のため SQLite が
//! 即座に諦める) ため、書込ロックは最初に取る。`rusqlite::Transaction` は drop で rollback
//! するので、途中の `Err` で抜けた経路は半端な状態を残さない (NFR3.3)。

use std::fmt;
use std::io::ErrorKind;
use std::path::Path;
use std::time::Duration;

use rusqlite::{
    Connection, ErrorCode, OptionalExtension, Transaction, TransactionBehavior, params,
};

use core_domain::orchestration::{IntentId, WorkflowExecution, WorkflowExecutionEvent};
use core_use_case::orchestration::{
    CorruptCause, EventStore, EventStoreError, GlobalSeqNr, JournalReader, ProjectionName,
};

use crate::Clock;

use super::schema::ensure_schema;
use super::store_path::StorePath;
use super::wire::{EventPayloadWire, StateWire};

/// 書込ロックを待つ既定の上限 (BR2.1)。同一ホストの並行 CLI はこの範囲で直列化され、
/// 超過は `Io(WouldBlock)` として呼出側へ返る (黙って失敗しない — NFR3.5)。
const DEFAULT_BUSY_TIMEOUT: Duration = Duration::from_millis(5000);

// ---------------------------------------------------------------------------
// SQL (C6 の 3 表に対する操作はここに集約する)
// ---------------------------------------------------------------------------

/// ジャーナルへの追記。`UNIQUE (aggregate_id, seq_nr)` 違反が競合の検出点の 1 つ。
const INSERT_JOURNAL: &str = "INSERT INTO journal
     (aggregate_id, seq_nr, schema_version, event_type, payload, occurred_at)
     VALUES (?1, ?2, ?3, ?4, ?5, ?6)";
/// genesis のスナップショット行 (主キー違反が競合の検出点)。
const INSERT_SNAPSHOT: &str = "INSERT INTO snapshot
     (aggregate_id, version, seq_nr, schema_version, payload, updated_at)
     VALUES (?1, ?2, ?3, ?4, ?5, ?6)";
/// 楽観 version 付きのスナップショット更新 (影響 0 行が競合の検出点)。
const UPDATE_SNAPSHOT: &str = "UPDATE snapshot
     SET version = ?1, seq_nr = ?2, schema_version = ?3, payload = ?4, updated_at = ?5
     WHERE aggregate_id = ?6 AND version = ?7";
/// 競合時に読む「実際の version」。
const SELECT_SNAPSHOT_VERSION: &str = "SELECT version FROM snapshot WHERE aggregate_id = ?1";
/// 再構成の起点となるスナップショット行。
const SELECT_SNAPSHOT: &str = "SELECT version, seq_nr, schema_version, payload
     FROM snapshot WHERE aggregate_id = ?1";
/// 1 集約の差分イベント (`seq_nr` 昇順)。
const SELECT_EVENTS_SINCE: &str = "SELECT seq_nr, schema_version, event_type, payload, occurred_at
     FROM journal WHERE aggregate_id = ?1 AND seq_nr > ?2 ORDER BY seq_nr";
/// 全集約横断の差分イベント (global 通番昇順)。
const SELECT_EVENTS_AFTER: &str =
    "SELECT global_seq_nr, aggregate_id, seq_nr, schema_version, event_type, payload, occurred_at
     FROM journal WHERE global_seq_nr > ?1 ORDER BY global_seq_nr";
/// 1 集約のジャーナル行数 (`NotFound` と `MissingSnapshot` の区別に使う)。
const COUNT_JOURNAL_ROWS: &str = "SELECT count(*) FROM journal WHERE aggregate_id = ?1";
/// 投影のチェックポイント。
const SELECT_CHECKPOINT: &str = "SELECT last_global_seq FROM checkpoint WHERE projection = ?1";
/// チェックポイントの前進 (未登録なら登録)。
const UPSERT_CHECKPOINT: &str = "INSERT INTO checkpoint(projection, last_global_seq, updated_at)
     VALUES (?1, ?2, ?3)
     ON CONFLICT(projection) DO UPDATE SET
       last_global_seq = excluded.last_global_seq, updated_at = excluded.updated_at";

// ---------------------------------------------------------------------------
// エラー写像
// ---------------------------------------------------------------------------

/// rusqlite の失敗を `Io { kind, path }` へ写す (材料のみ — 文言は運ばない)。
///
/// `SQLITE_BUSY` / `SQLITE_LOCKED` を `WouldBlock` に写すのは、これが「壊れた」ではなく
/// 「いま他の書き手がいる」という**再実行で解ける**分類だからである (NFR3.5)。
pub(crate) fn map_sqlite_error(error: &rusqlite::Error, path: &Path) -> EventStoreError {
    EventStoreError::Io {
        kind: io_kind(error),
        path: Some(path.to_path_buf()),
    }
}

/// rusqlite の失敗を `std::io::ErrorKind` の語彙へ落とす。
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

/// 競合の検出点かどうか (`UNIQUE` / 主キー違反)。
fn is_constraint_violation(error: &rusqlite::Error) -> bool {
    matches!(
        error,
        rusqlite::Error::SqliteFailure(inner, _) if inner.code == ErrorCode::ConstraintViolation
    )
}

/// 集約に属さない行 (チェックポイント・global 通番) の識別子欄に置く印。
///
/// `EventStoreError::Corrupt` の `aggregate_id` は「行が名乗っていた集約識別子」であり、
/// 集約を持たない行にはその材料が無い。`Display` が欠落材料に使う綴りと同じものを置いて、
/// 別の意味の値 (投影名など) を紛れ込ませない。
const NO_AGGREGATE: &str = "-";

/// 行の材料を添えて `Corrupt` を組む。
fn corrupt(aggregate_id: &str, seq_nr: Option<u64>, cause: CorruptCause) -> EventStoreError {
    EventStoreError::Corrupt {
        aggregate_id: aggregate_id.to_string(),
        seq_nr,
        cause,
    }
}

/// ドメインの `u64` を SQLite の `INTEGER` (i64) へ写す。
///
/// 収まらない値は行として表現できない — 静かに丸めず `Corrupt` で止める (NFR4.3)。
fn to_i64(value: u64, aggregate_id: &str, seq_nr: Option<u64>) -> Result<i64, EventStoreError> {
    i64::try_from(value)
        .map_err(|_| corrupt(aggregate_id, seq_nr, CorruptCause::InvariantViolation))
}

/// SQLite の `INTEGER` (i64) をドメインの `u64` へ写す (負値は行の破損)。
fn to_u64(value: i64, aggregate_id: &str, seq_nr: Option<u64>) -> Result<u64, EventStoreError> {
    u64::try_from(value)
        .map_err(|_| corrupt(aggregate_id, seq_nr, CorruptCause::InvariantViolation))
}

/// 行の `schema_version` をワイヤの版へ写す。範囲外は「対応外の版」として復号側が弾く。
fn wire_version(value: i64) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

// ---------------------------------------------------------------------------
// 時刻
// ---------------------------------------------------------------------------

/// epoch ミリ秒を ISO 8601 UTC (`YYYY-MM-DDTHH:MM:SSZ`) に描く。
///
/// 外部の日付クレートを足さないための小さな純関数である (NFR4.1 — 依存最小化)。
/// 暦の計算は Howard Hinnant の `civil_from_days` (proleptic Gregorian) をそのまま使う。
fn format_iso8601_utc(epoch_ms: u64) -> String {
    let seconds = epoch_ms / 1000;
    let (hour, minute, second) = ((seconds / 3600) % 24, (seconds / 60) % 60, seconds % 60);
    let (year, month, day) = civil_from_days(seconds / 86_400);
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

/// epoch からの日数を (年, 月, 日) に開く (1970-01-01 を 0 とする)。
const fn civil_from_days(days: u64) -> (u64, u64, u64) {
    // 3 月始まりの暦へ移して、400 年周期 (146097 日) の中の位置で解く。
    let shifted = days + 719_468;
    let era = shifted / 146_097;
    let day_of_era = shifted - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * shifted_month + 2) / 5 + 1;
    let month = if shifted_month < 10 {
        shifted_month + 3
    } else {
        shifted_month - 9
    };
    (if month <= 2 { year + 1 } else { year }, month, day)
}

// ---------------------------------------------------------------------------
// ストア
// ---------------------------------------------------------------------------

/// C6 の 3 表を持つ SQLite ストア。接続と時計を**単一所有**する。
///
/// `Clone` は持たない — 同じ可変状態 (接続) を指す別ハンドルを配らないためである
/// (`coding-rules/interior-mutability.md`)。同じストアを指す別インスタンスは
/// [`EventStoreImpl::open`] で開き直して得る。
pub struct EventStoreImpl<C> {
    path: StorePath,
    connection: Connection,
    clock: C,
}

impl<C> fmt::Debug for EventStoreImpl<C> {
    /// 場所だけを描く (接続と時計は診断の材料にならない)。
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EventStoreImpl")
            .field("path", &self.path)
            .finish_non_exhaustive()
    }
}

impl<C: Clock> EventStoreImpl<C> {
    /// ストアを開く (無ければ作る)。`busy_timeout` は既定の 5000ms (BR2.1)。
    ///
    /// 親ディレクトリ (`intents/`) は upstream の既存ディレクトリなので**作らない** —
    /// 無ければ `Io { kind: NotFound }` で止まる。`PRAGMA user_version` が 0 なら C6 の
    /// スキーマを作って 1 を刻み、1 ならそのまま使い、それ以外は `Schema` を返す。
    ///
    /// # Errors
    ///
    /// 親ディレクトリ欠落・権限・ディスク (`Io`)、対応範囲外の版 (`Schema`) を返す。
    pub fn open(path: StorePath, clock: C) -> Result<EventStoreImpl<C>, EventStoreError> {
        EventStoreImpl::open_with_busy_timeout(path, clock, DEFAULT_BUSY_TIMEOUT)
    }

    /// 書込ロックを待つ上限を指定してストアを開く。
    ///
    /// 既定 (5000ms) を変えるのは、待ち時間そのものを観測したい試験と、合成ルートが
    /// 運用envelope を調整する場合だけである。意味論は [`EventStoreImpl::open`] と同じ。
    ///
    /// # Errors
    ///
    /// [`EventStoreImpl::open`] と同じ。
    pub fn open_with_busy_timeout(
        path: StorePath,
        clock: C,
        busy_timeout: Duration,
    ) -> Result<EventStoreImpl<C>, EventStoreError> {
        if path
            .as_path()
            .parent()
            .is_some_and(|parent| !parent.as_os_str().is_empty() && !parent.is_dir())
        {
            return Err(EventStoreError::Io {
                kind: ErrorKind::NotFound,
                path: Some(path.as_path().to_path_buf()),
            });
        }
        let connection = Connection::open(path.as_path())
            .map_err(|error| map_sqlite_error(&error, path.as_path()))?;
        connection
            .busy_timeout(busy_timeout)
            .map_err(|error| map_sqlite_error(&error, path.as_path()))?;
        ensure_schema(&connection, path.as_path())?;
        Ok(EventStoreImpl {
            path,
            connection,
            clock,
        })
    }

    /// ストアファイルの場所。
    #[must_use]
    pub const fn path(&self) -> &StorePath {
        &self.path
    }

    /// `BEGIN IMMEDIATE` で開いた書込トランザクションの中で `f` を走らせる (BR2.4)。
    ///
    /// 登録簿 (`intents.json`) の read-modify-write はこの中で行う — 同じ DB を開く別プロセスの
    /// 書込は `busy_timeout` の範囲で直列化されるので、これが**唯一の直列化機構**になる
    /// (mkdir / flock のような別機構は導入しない — ADR-007)。
    ///
    /// `f` が `Err` を返せば `COMMIT` せずに抜け、トランザクションは drop で rollback する。
    ///
    /// # Errors
    ///
    /// 書込ロックを取れない (`Io { kind: WouldBlock }`)、`COMMIT` の失敗 (`Io`)、および
    /// `f` が返した失敗をそのまま返す。
    pub fn within_write_transaction<T, F>(&mut self, f: F) -> Result<T, EventStoreError>
    where
        F: FnOnce(&Transaction<'_>) -> Result<T, EventStoreError>,
    {
        let path = self.path.clone();
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| map_sqlite_error(&error, path.as_path()))?;
        let value = f(&transaction)?;
        transaction
            .commit()
            .map_err(|error| map_sqlite_error(&error, path.as_path()))?;
        Ok(value)
    }

    /// この集約のジャーナル行が 1 件も無いか (`NotFound` と `MissingSnapshot` の区別 — BR1.2)。
    ///
    /// # Errors
    ///
    /// ストア I/O (`Io`)、行数が `u64` に収まらない (`Corrupt`) を返す。
    pub(crate) fn journal_is_empty(
        &self,
        aggregate_id: &IntentId,
    ) -> Result<bool, EventStoreError> {
        let count: i64 = self
            .connection
            .query_row(COUNT_JOURNAL_ROWS, params![aggregate_id.as_str()], |row| {
                row.get(0)
            })
            .map_err(|error| map_sqlite_error(&error, self.path.as_path()))?;
        Ok(count == 0)
    }

    /// 現在の押印時刻 (`updated_at` 列の値 — BR2.6)。
    fn now(&self) -> String {
        format_iso8601_utc(self.clock.now_ms())
    }
}

// ---------------------------------------------------------------------------
// 行の読み書き (型引数 `C` に依らないので自由関数にする)
// ---------------------------------------------------------------------------

/// スナップショット行の現在 version (行が無ければ 0 — genesis の期待値)。
fn current_version(
    transaction: &Transaction<'_>,
    aggregate_id: &str,
    path: &Path,
) -> Result<u64, EventStoreError> {
    let raw: Option<i64> = transaction
        .query_row(SELECT_SNAPSHOT_VERSION, params![aggregate_id], |row| {
            row.get(0)
        })
        .optional()
        .map_err(|error| map_sqlite_error(&error, path))?;
    match raw {
        None => Ok(0),
        Some(value) => to_u64(value, aggregate_id, None),
    }
}

/// ジャーナルへ 1 行追記する。競合は呼出側が `Conflict` に組み立てる。
///
/// # Errors
///
/// ストア I/O (`Io`)、`seq_nr` が列に収まらない (`Corrupt`) を返す。競合は `Err` ではなく
/// [`JournalInsert::Conflicted`] として返る。
fn insert_journal_row(
    transaction: &Transaction<'_>,
    event: &WorkflowExecutionEvent,
    payload: &str,
    path: &Path,
) -> Result<JournalInsert, EventStoreError> {
    let aggregate_id = event.intent_id().as_str();
    let seq_nr = to_i64(event.seq_nr(), aggregate_id, Some(event.seq_nr()))?;
    let outcome = transaction.execute(
        INSERT_JOURNAL,
        params![
            aggregate_id,
            seq_nr,
            i64::from(EventPayloadWire::SCHEMA_VERSION),
            EventPayloadWire::event_type(event.payload()),
            payload,
            event.occurred_at(),
        ],
    );
    match outcome {
        Ok(_) => Ok(JournalInsert::Appended),
        Err(error) if is_constraint_violation(&error) => Ok(JournalInsert::Conflicted),
        Err(error) => Err(map_sqlite_error(&error, path)),
    }
}

/// ジャーナル行 (封筒は列) をイベントへ復号する — functional-spec §4.1。
fn decode_event(row: &JournalRow) -> Result<WorkflowExecutionEvent, EventStoreError> {
    let intent_id = IntentId::parse(&row.aggregate_id).map_err(|_| {
        corrupt(
            &row.aggregate_id,
            Some(row.seq_nr),
            CorruptCause::UndecodablePayload,
        )
    })?;
    let payload = EventPayloadWire::decode(
        &row.aggregate_id,
        row.seq_nr,
        row.schema_version,
        &row.event_type,
        &row.payload,
    )?;
    Ok(WorkflowExecutionEvent::new(
        intent_id,
        row.seq_nr,
        row.occurred_at.clone(),
        payload,
    ))
}

/// スナップショット行を集約へ復元し、**列の** version を載せる (BR1.2 の前半)。
fn decode_snapshot(
    aggregate_id: &IntentId,
    row: &SnapshotRow,
) -> Result<WorkflowExecution, EventStoreError> {
    let state = StateWire::decode(aggregate_id.as_str(), row.schema_version, &row.payload)?;
    let aggregate = WorkflowExecution::from_state(state).map_err(|_| {
        corrupt(
            aggregate_id.as_str(),
            Some(row.seq_nr),
            CorruptCause::InvariantViolation,
        )
    })?;
    Ok(aggregate.with_version(row.version))
}

/// C6 `journal` の 1 行 (読み出した生の材料)。
#[derive(Debug)]
struct JournalRow {
    aggregate_id: String,
    seq_nr: u64,
    schema_version: u32,
    event_type: String,
    payload: String,
    occurred_at: String,
}

/// ジャーナル追記の結末。競合を `Err` にせず値で返すのは、`Conflict` の材料
/// (`expected` / `actual`) を組み立てられるのが呼出側だけだからである。
#[derive(Debug, PartialEq, Eq)]
enum JournalInsert {
    /// 1 行追記した。
    Appended,
    /// `UNIQUE (aggregate_id, seq_nr)` に阻まれた (= 競合)。
    Conflicted,
}

/// C6 `snapshot` の 1 行 (読み出した生の材料)。
#[derive(Debug)]
struct SnapshotRow {
    version: u64,
    seq_nr: u64,
    schema_version: u32,
    payload: String,
}

impl<C: Clock> EventStore<IntentId, WorkflowExecution, WorkflowExecutionEvent>
    for EventStoreImpl<C>
{
    async fn persist_event(
        &mut self,
        event: &WorkflowExecutionEvent,
        version: u64,
    ) -> Result<(), EventStoreError> {
        let aggregate_id = event.intent_id();
        let payload =
            EventPayloadWire::encode(aggregate_id.as_str(), event.seq_nr(), event.payload())?;
        let path = self.path.clone();

        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| map_sqlite_error(&error, path.as_path()))?;

        // スナップショットは更新しないが、追記の前提としての楽観 version は検査する
        // (event-store-adapter-rs の `persist_event(event, version)` と同じ意味論)。
        let actual = current_version(&transaction, aggregate_id.as_str(), path.as_path())?;
        if actual != version {
            return Err(EventStoreError::Conflict {
                expected: version,
                actual,
            });
        }
        if insert_journal_row(&transaction, event, &payload, path.as_path())?
            == JournalInsert::Conflicted
        {
            return Err(EventStoreError::Conflict {
                expected: version,
                actual,
            });
        }
        transaction
            .commit()
            .map_err(|error| map_sqlite_error(&error, path.as_path()))
    }

    async fn persist_event_and_snapshot(
        &mut self,
        event: &WorkflowExecutionEvent,
        aggregate: &WorkflowExecution,
    ) -> Result<(), EventStoreError> {
        let aggregate_id = event.intent_id();
        let expected = aggregate.version();
        let new_version = event.seq_nr();
        let event_payload =
            EventPayloadWire::encode(aggregate_id.as_str(), new_version, event.payload())?;
        // 書込後の姿を写すので、payload の version も列と同じ新 version に揃える。
        let state_payload = StateWire::encode(
            aggregate_id.as_str(),
            &aggregate.clone().with_version(new_version).state(),
        )?;
        let updated_at = self.now();
        let path = self.path.clone();
        let identifier = aggregate_id.as_str();
        let new_version_column = to_i64(new_version, identifier, Some(new_version))?;
        let expected_column = to_i64(expected, identifier, Some(new_version))?;

        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| map_sqlite_error(&error, path.as_path()))?;

        // (1) ジャーナルへの追記。`UNIQUE (aggregate_id, seq_nr)` 違反は競合。
        if insert_journal_row(&transaction, event, &event_payload, path.as_path())?
            == JournalInsert::Conflicted
        {
            let actual = current_version(&transaction, identifier, path.as_path())?;
            return Err(EventStoreError::Conflict { expected, actual });
        }

        // (2) genesis だけ INSERT、それ以外は `WHERE version = expected` の UPDATE。
        if expected == 0 {
            let outcome = transaction.execute(
                INSERT_SNAPSHOT,
                params![
                    identifier,
                    new_version_column,
                    new_version_column,
                    i64::from(StateWire::SCHEMA_VERSION),
                    state_payload,
                    updated_at,
                ],
            );
            if let Err(error) = outcome {
                if !is_constraint_violation(&error) {
                    return Err(map_sqlite_error(&error, path.as_path()));
                }
                let actual = current_version(&transaction, identifier, path.as_path())?;
                return Err(EventStoreError::Conflict { expected, actual });
            }
        } else {
            let affected = transaction
                .execute(
                    UPDATE_SNAPSHOT,
                    params![
                        new_version_column,
                        new_version_column,
                        i64::from(StateWire::SCHEMA_VERSION),
                        state_payload,
                        updated_at,
                        identifier,
                        expected_column,
                    ],
                )
                .map_err(|error| map_sqlite_error(&error, path.as_path()))?;
            if affected == 0 {
                let actual = current_version(&transaction, identifier, path.as_path())?;
                return Err(EventStoreError::Conflict { expected, actual });
            }
        }

        // (3) ここまで通った経路だけが COMMIT する。
        transaction
            .commit()
            .map_err(|error| map_sqlite_error(&error, path.as_path()))
    }

    async fn get_latest_snapshot_by_id(
        &self,
        aid: &IntentId,
    ) -> Result<Option<WorkflowExecution>, EventStoreError> {
        let row = self
            .connection
            .query_row(SELECT_SNAPSHOT, params![aid.as_str()], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })
            .optional()
            .map_err(|error| map_sqlite_error(&error, self.path.as_path()))?;
        let Some((version, seq_nr, schema_version, payload)) = row else {
            return Ok(None);
        };
        let row = SnapshotRow {
            version: to_u64(version, aid.as_str(), None)?,
            seq_nr: to_u64(seq_nr, aid.as_str(), None)?,
            schema_version: wire_version(schema_version),
            payload,
        };
        decode_snapshot(aid, &row).map(Some)
    }

    async fn get_events_by_id_since_seq_nr(
        &self,
        aid: &IntentId,
        seq_nr: u64,
    ) -> Result<Vec<WorkflowExecutionEvent>, EventStoreError> {
        let since = to_i64(seq_nr, aid.as_str(), Some(seq_nr))?;
        let rows = {
            let mut statement = self
                .connection
                .prepare(SELECT_EVENTS_SINCE)
                .map_err(|error| map_sqlite_error(&error, self.path.as_path()))?;
            let mapped = statement
                .query_map(params![aid.as_str(), since], |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                    ))
                })
                .map_err(|error| map_sqlite_error(&error, self.path.as_path()))?;
            let mut collected = Vec::new();
            for row in mapped {
                collected.push(row.map_err(|error| map_sqlite_error(&error, self.path.as_path()))?);
            }
            collected
        };

        let mut events = Vec::with_capacity(rows.len());
        for (row_seq_nr, schema_version, event_type, payload, occurred_at) in rows {
            let row = JournalRow {
                aggregate_id: aid.as_str().to_string(),
                seq_nr: to_u64(row_seq_nr, aid.as_str(), None)?,
                schema_version: wire_version(schema_version),
                event_type,
                payload,
                occurred_at,
            };
            events.push(decode_event(&row)?);
        }
        Ok(events)
    }
}

impl<C: Clock> JournalReader for EventStoreImpl<C> {
    async fn events_after(
        &self,
        after: GlobalSeqNr,
    ) -> Result<Vec<(GlobalSeqNr, WorkflowExecutionEvent)>, EventStoreError> {
        let from = to_i64(after.value(), NO_AGGREGATE, None)?;
        let rows = {
            let mut statement = self
                .connection
                .prepare(SELECT_EVENTS_AFTER)
                .map_err(|error| map_sqlite_error(&error, self.path.as_path()))?;
            let mapped = statement
                .query_map(params![from], |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, i64>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, String>(5)?,
                        row.get::<_, String>(6)?,
                    ))
                })
                .map_err(|error| map_sqlite_error(&error, self.path.as_path()))?;
            let mut collected = Vec::new();
            for row in mapped {
                collected.push(row.map_err(|error| map_sqlite_error(&error, self.path.as_path()))?);
            }
            collected
        };

        let mut events = Vec::with_capacity(rows.len());
        for (global, aggregate_id, seq_nr, schema_version, event_type, payload, occurred_at) in rows
        {
            let row = JournalRow {
                seq_nr: to_u64(seq_nr, &aggregate_id, None)?,
                aggregate_id,
                schema_version: wire_version(schema_version),
                event_type,
                payload,
                occurred_at,
            };
            let event = decode_event(&row)?;
            events.push((
                GlobalSeqNr::new(to_u64(global, &row.aggregate_id, Some(row.seq_nr))?),
                event,
            ));
        }
        Ok(events)
    }

    async fn checkpoint(
        &self,
        projection: &ProjectionName,
    ) -> Result<GlobalSeqNr, EventStoreError> {
        let raw: Option<i64> = self
            .connection
            .query_row(SELECT_CHECKPOINT, params![projection.as_str()], |row| {
                row.get(0)
            })
            .optional()
            .map_err(|error| map_sqlite_error(&error, self.path.as_path()))?;
        match raw {
            None => Ok(GlobalSeqNr::ZERO),
            Some(value) => Ok(GlobalSeqNr::new(to_u64(value, NO_AGGREGATE, None)?)),
        }
    }

    async fn advance_checkpoint(
        &mut self,
        projection: &ProjectionName,
        to: GlobalSeqNr,
    ) -> Result<(), EventStoreError> {
        let target = to_i64(to.value(), NO_AGGREGATE, None)?;
        let updated_at = self.now();
        let path = self.path.clone();

        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| map_sqlite_error(&error, path.as_path()))?;

        let raw: Option<i64> = transaction
            .query_row(SELECT_CHECKPOINT, params![projection.as_str()], |row| {
                row.get(0)
            })
            .optional()
            .map_err(|error| map_sqlite_error(&error, path.as_path()))?;
        let current = match raw {
            None => GlobalSeqNr::ZERO,
            Some(value) => GlobalSeqNr::new(to_u64(value, NO_AGGREGATE, None)?),
        };
        if to < current {
            return Err(EventStoreError::CheckpointRegression {
                projection: projection.clone(),
                current,
                requested: to,
            });
        }
        transaction
            .execute(
                UPSERT_CHECKPOINT,
                params![projection.as_str(), target, updated_at],
            )
            .map_err(|error| map_sqlite_error(&error, path.as_path()))?;
        transaction
            .commit()
            .map_err(|error| map_sqlite_error(&error, path.as_path()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FakeClock;
    use core_domain::workspace::SpaceName;

    /// 一時ディレクトリ配下にストアを開く (親 dir は先に作る)。
    fn open_store(dir: &tempfile::TempDir) -> EventStoreImpl<FakeClock> {
        let path = StorePath::of(&dir.path().join("aidlc"), &SpaceName::default_space());
        std::fs::create_dir_all(path.as_path().parent().expect("親 dir")).expect("intents/ を作る");
        EventStoreImpl::open(path, FakeClock::new(0)).expect("開ける")
    }

    #[test]
    fn the_default_busy_timeout_is_five_seconds() {
        let dir = tempfile::tempdir().expect("一時 dir");
        let store = open_store(&dir);
        let timeout: i64 = store
            .connection
            .pragma_query_value(None, "busy_timeout", |row| row.get(0))
            .expect("busy_timeout");
        assert_eq!(timeout, 5000, "BR2.1 の 5000ms");
    }

    #[test]
    fn the_busy_timeout_can_be_narrowed_for_observation() {
        let dir = tempfile::tempdir().expect("一時 dir");
        let path = StorePath::of(&dir.path().join("aidlc"), &SpaceName::default_space());
        std::fs::create_dir_all(path.as_path().parent().expect("親 dir")).expect("intents/ を作る");
        let store = EventStoreImpl::open_with_busy_timeout(
            path,
            FakeClock::new(0),
            Duration::from_millis(20),
        )
        .expect("開ける");
        let timeout: i64 = store
            .connection
            .pragma_query_value(None, "busy_timeout", |row| row.get(0))
            .expect("busy_timeout");
        assert_eq!(timeout, 20);
    }

    #[test]
    fn the_journal_mode_stays_at_the_default() {
        // WAL は付随ファイル (-wal / -shm) を増やす — 逸脱台帳 # 4 のパスを 1 本に保つ (BR2.1)。
        let dir = tempfile::tempdir().expect("一時 dir");
        let store = open_store(&dir);
        let mode: String = store
            .connection
            .pragma_query_value(None, "journal_mode", |row| row.get(0))
            .expect("journal_mode");
        assert_eq!(mode.to_lowercase(), "delete");
    }

    #[test]
    fn the_same_path_can_be_opened_again_without_recreating_the_schema() {
        // `Clone` は持たない (同じ接続を指す別ハンドルを配らない — interior-mutability.md)。
        // 「同じストアを指す別インスタンス」は開き直しで得る形に一本化した。
        let dir = tempfile::tempdir().expect("一時 dir");
        let first = open_store(&dir);
        let second = EventStoreImpl::open(first.path().clone(), FakeClock::new(0))
            .expect("同じ場所を開き直せる");
        assert_eq!(first.path(), second.path(), "同じ場所を指す");
        for store in [&first, &second] {
            let stamped: i64 = store
                .connection
                .pragma_query_value(None, "user_version", |row| row.get(0))
                .expect("user_version");
            assert_eq!(stamped, 1, "刻印は 1 のまま (作り直さない)");
        }
    }

    #[test]
    fn the_epoch_is_rendered_as_iso_8601_utc() {
        assert_eq!(format_iso8601_utc(0), "1970-01-01T00:00:00Z");
        assert_eq!(
            format_iso8601_utc(1_787_443_200_000),
            "2026-08-23T00:00:00Z"
        );
        // 閏日 (2024-02-29) と年境界を跨ぐ点を固定する。
        assert_eq!(
            format_iso8601_utc(1_709_164_800_000),
            "2024-02-29T00:00:00Z"
        );
        assert_eq!(
            format_iso8601_utc(1_735_689_599_000),
            "2024-12-31T23:59:59Z"
        );
        // ミリ秒は切り捨て (秒精度の逐語形)。
        assert_eq!(format_iso8601_utc(999), "1970-01-01T00:00:00Z");
    }

    #[test]
    fn the_clock_supplies_the_stamp() {
        let dir = tempfile::tempdir().expect("一時 dir");
        let path = StorePath::of(&dir.path().join("aidlc"), &SpaceName::default_space());
        std::fs::create_dir_all(path.as_path().parent().expect("親 dir")).expect("intents/ を作る");
        let store = EventStoreImpl::open(path, FakeClock::new(1_787_443_200_000)).expect("開ける");
        assert_eq!(store.now(), "2026-08-23T00:00:00Z");
    }

    #[test]
    fn a_busy_database_is_reported_as_would_block() {
        let error = rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error {
                code: ErrorCode::DatabaseBusy,
                extended_code: 5,
            },
            None,
        );
        assert_eq!(
            map_sqlite_error(&error, Path::new("/w/store.sqlite")),
            EventStoreError::Io {
                kind: ErrorKind::WouldBlock,
                path: Some(std::path::PathBuf::from("/w/store.sqlite")),
            }
        );
    }

    #[test]
    fn the_error_codes_map_to_the_io_vocabulary() {
        let of = |code| {
            io_kind(&rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error {
                    code,
                    extended_code: 0,
                },
                None,
            ))
        };
        assert_eq!(of(ErrorCode::DatabaseLocked), ErrorKind::WouldBlock);
        assert_eq!(of(ErrorCode::CannotOpen), ErrorKind::NotFound);
        assert_eq!(of(ErrorCode::ReadOnly), ErrorKind::PermissionDenied);
        assert_eq!(of(ErrorCode::DatabaseCorrupt), ErrorKind::InvalidData);
        assert_eq!(of(ErrorCode::DiskFull), ErrorKind::Other);
        assert_eq!(
            io_kind(&rusqlite::Error::QueryReturnedNoRows),
            ErrorKind::Other,
            "SQLite 由来でない失敗は分類しない"
        );
    }

    #[test]
    fn an_interrupted_operation_keeps_its_own_io_kind() {
        // 中断は「壊れた」でも「いま他の書き手がいる」でもない第 3 の分類なので、
        // 呼出側が再実行の可否を判断できるように `Other` へ畳まずに残す (NFR3.5)。
        let error = rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error {
                code: ErrorCode::OperationInterrupted,
                extended_code: 9,
            },
            None,
        );
        assert_eq!(io_kind(&error), ErrorKind::Interrupted);
        assert_eq!(
            map_sqlite_error(&error, Path::new("/w/store.sqlite")),
            EventStoreError::Io {
                kind: ErrorKind::Interrupted,
                path: Some(std::path::PathBuf::from("/w/store.sqlite")),
            }
        );
    }

    #[test]
    fn the_debug_rendering_shows_the_location_and_hides_the_connection() {
        // 接続と時計は診断の材料にならないので描かない (`finish_non_exhaustive`)。
        let dir = tempfile::tempdir().expect("一時 dir");
        let store = open_store(&dir);
        let rendered = format!("{store:?}");
        assert!(rendered.starts_with("EventStoreImpl {"), "{rendered}");
        assert!(rendered.contains("path:"), "{rendered}");
        assert!(rendered.contains(".."), "{rendered}");
        assert!(!rendered.contains("connection"), "{rendered}");
        assert!(!rendered.contains("clock"), "{rendered}");
    }

    #[test]
    fn a_constraint_violation_is_the_conflict_detection_point() {
        let violation = rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error {
                code: ErrorCode::ConstraintViolation,
                extended_code: 2067,
            },
            None,
        );
        assert!(is_constraint_violation(&violation));
        assert!(!is_constraint_violation(
            &rusqlite::Error::QueryReturnedNoRows
        ));
    }

    #[test]
    fn a_value_that_does_not_fit_the_column_is_corrupt_rather_than_rounded() {
        let err = to_i64(u64::MAX, "agg", Some(3)).expect_err("i64 に収まらない");
        assert_eq!(
            err,
            EventStoreError::Corrupt {
                aggregate_id: "agg".to_string(),
                seq_nr: Some(3),
                cause: CorruptCause::InvariantViolation,
            }
        );
        let err = to_u64(-1, "agg", None).expect_err("負の通番は無い");
        assert!(matches!(err, EventStoreError::Corrupt { .. }));
        assert_eq!(wire_version(-1), u32::MAX, "範囲外は対応外の版として扱う");
    }
}
