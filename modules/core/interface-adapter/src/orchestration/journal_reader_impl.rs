//! `JournalReaderImpl` — `JournalReader` の SQLite 実装 (BR1.4 / ADR-010 決定 4)。
//!
//! # なぜ自前で書くのか
//!
//! 集約の永続化は本家 event-store-adapter-rs が担う。しかし本家のドメインは**集約単位**の
//! 読み書きであり、全集約横断の順序読取と投影チェックポイントは利用側の関心である
//! (ADR-010 決定 4 — ライブラリ所有者の裁定でサポート外)。したがってここだけを自前で持つ。
//!
//! # 何に結合しているか
//!
//! 本家の `journal` 表を**同じ DB ファイルへの別接続**から読む。カーソルは本家 `journal` の
//! `rowid` である — この表は追記専用 (`INSERT` だけで `DELETE` が無い) であり、本家は
//! 書込を 1 本の接続に直列化するので、`rowid` はコミット順の単調増加になる。
//!
//! 本家スキーマへの結合は次の 2 つで守る:
//!
//! 1. 版の**完全固定** (`event-store-adapter-rs = "=2.0.0"` — ADR-010 決定 4)
//! 2. スキーマガードテスト ([`tests::the_upstream_journal_schema_is_the_pinned_one`]) —
//!    本家の DDL がずれたら「本家スキーマが変わった」と明示的に落ちる
//!
//! # チェックポイントは自前の表である
//!
//! 本家の表とは名前を分ける (`amadeus_projection_checkpoint`) — 同じ DB ファイルに同居
//! させても本家のスキーマ作成 (`CREATE TABLE IF NOT EXISTS`) と衝突しない。
//!
//! # 接続は単一所有である
//!
//! 読取 (`events_after` / `checkpoint`) は `&self`、書込 (`advance_checkpoint`) は
//! `&mut self` で、rusqlite の `Connection::prepare` (`&self`) と `Connection::transaction`
//! (`&mut self`) にそのまま対応する。内部可変性で `&self` を偽装しない
//! (`coding-rules/interior-mutability.md`)。

use std::io::ErrorKind;
use std::path::Path;
use std::time::Duration;

use rusqlite::{Connection, OpenFlags, OptionalExtension, TransactionBehavior, params};

use core_domain::orchestration::WorkflowExecutionEvent;
use core_use_case::orchestration::{
    CorruptCause, GlobalSeqNr, JournalReadError, JournalReader, ProjectionName,
};
use event_store_adapter_rs::types::Event as _;

use super::store_failure::io_kind;
use super::store_path::StorePath;

/// 書込ロックを待つ既定の上限 (BR2.1)。読取専用の接続でも、チェックポイントの前進だけは
/// 書込なので待ち時間が要る。
const DEFAULT_BUSY_TIMEOUT: Duration = Duration::from_millis(5000);

/// 本家の追記専用ジャーナル表 (`rowid` がコミット順の単調カーソルになる)。
const UPSTREAM_JOURNAL_TABLE: &str = "journal";

/// チェックポイント表の DDL (冪等)。
const CREATE_CHECKPOINT_TABLE: &str = "CREATE TABLE IF NOT EXISTS amadeus_projection_checkpoint (
  projection      TEXT    PRIMARY KEY,
  last_global_seq INTEGER NOT NULL
)";

/// 全集約横断の差分読取 (`rowid` 昇順)。
const SELECT_EVENTS_AFTER: &str =
    "SELECT rowid, aid, seq_nr, payload FROM journal WHERE rowid > ?1 ORDER BY rowid";

/// 投影のチェックポイント。
const SELECT_CHECKPOINT: &str =
    "SELECT last_global_seq FROM amadeus_projection_checkpoint WHERE projection = ?1";

/// チェックポイントの前進 (未登録なら登録)。
const UPSERT_CHECKPOINT: &str =
    "INSERT INTO amadeus_projection_checkpoint(projection, last_global_seq)
     VALUES (?1, ?2)
     ON CONFLICT(projection) DO UPDATE SET last_global_seq = excluded.last_global_seq";

/// 集約に属さない行 (チェックポイント・カーソル) の識別子欄に置く印。
const NO_AGGREGATE: &str = "-";

/// rusqlite の失敗を `Io { kind, path }` へ写す (材料のみ — 文言は運ばない)。
fn map_sqlite_error(error: &rusqlite::Error, path: &Path) -> JournalReadError {
    JournalReadError::Io {
        kind: io_kind(error),
        path: Some(path.to_path_buf()),
    }
}

/// 行の材料を添えて `Corrupt` を組む。
fn corrupt_error(
    aggregate_id: &str,
    seq_nr: Option<usize>,
    cause: CorruptCause,
) -> JournalReadError {
    JournalReadError::Corrupt {
        aggregate_id: aggregate_id.to_string(),
        seq_nr,
        cause,
    }
}

/// global 通番 (`u64`) を SQLite の `INTEGER` (i64) へ写す。収まらない値は行として
/// 表現できない — 静かに丸めず `Corrupt` で止める (NFR4.3)。
fn to_i64(value: u64) -> Result<i64, JournalReadError> {
    i64::try_from(value)
        .map_err(|_| corrupt_error(NO_AGGREGATE, None, CorruptCause::InvariantViolation))
}

/// SQLite の `INTEGER` (i64) を global 通番 (`u64`) へ写す。
///
/// 負値は行の破損である。読取のカーソルは常に 0 以上で、問い合わせが `rowid > カーソル` に
/// 絞るため実際には届かないが、静かに丸めないためにここで止める (NFR4.3)。
fn to_u64(value: i64, aggregate_id: &str) -> Result<u64, JournalReadError> {
    u64::try_from(value)
        .map_err(|_| corrupt_error(aggregate_id, None, CorruptCause::InvariantViolation))
}

/// 本家 `journal` の 1 行を読み終えた生の材料。
struct JournalRow {
    rowid: i64,
    aggregate_id: String,
    seq_nr: i64,
    payload: Vec<u8>,
}

/// 本家のジャーナルを横断で読み、投影チェックポイントを持つ `JournalReader` の実装。
#[derive(Debug)]
pub struct JournalReaderImpl {
    path: StorePath,
    connection: Connection,
}

impl JournalReaderImpl {
    /// 既にあるストアファイルを**読取用に開き直す** (BR1.4)。
    ///
    /// ファイルとその `journal` 表を作るのは本家のイベントストアである。ここでは作らない —
    /// 本家が所有する表を我々が先に作ると、DDL の正本が 2 か所になる。まだ存在しない
    /// ストアを開こうとしたら `Io { kind: NotFound }` で止まる。
    ///
    /// 自前のチェックポイント表だけは (無ければ) ここで作る。
    ///
    /// # Errors
    ///
    /// ファイルを開けない・本家の `journal` 表がまだ無い (`Io { kind: NotFound }`)、権限や
    /// ディスクの失敗 (`Io`) を返す。
    pub fn open(path: &StorePath) -> Result<JournalReaderImpl, JournalReadError> {
        JournalReaderImpl::open_with_busy_timeout(path, DEFAULT_BUSY_TIMEOUT)
    }

    /// 書込ロックを待つ上限を指定して開く。
    ///
    /// 既定 (5000ms) を変えるのは、待ち時間そのものを観測したい試験と、合成ルートが運用
    /// envelope を調整する場合だけである。意味論は [`JournalReaderImpl::open`] と同じ。
    ///
    /// # Errors
    ///
    /// [`JournalReaderImpl::open`] と同じ。
    pub fn open_with_busy_timeout(
        path: &StorePath,
        busy_timeout: Duration,
    ) -> Result<JournalReaderImpl, JournalReadError> {
        // 読取側の接続はストアファイルを**作らない** (SQLITE_OPEN_CREATE を外す)。
        // 存在しないパスは空 DB を作って NotFound を返すのではなく、open 自体が失敗する
        // (B6 CodeRabbit #511)。書込は checkpoint 表があるので READ_WRITE は残す。
        let connection = Connection::open_with_flags(
            path.as_path(),
            OpenFlags::SQLITE_OPEN_READ_WRITE
                | OpenFlags::SQLITE_OPEN_URI
                | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .map_err(|error| map_sqlite_error(&error, path.as_path()))?;
        connection
            .busy_timeout(busy_timeout)
            .map_err(|error| map_sqlite_error(&error, path.as_path()))?;
        if !table_exists(&connection, UPSTREAM_JOURNAL_TABLE, path.as_path())? {
            return Err(JournalReadError::Io {
                kind: ErrorKind::NotFound,
                path: Some(path.as_path().to_path_buf()),
            });
        }
        connection
            .execute_batch(CREATE_CHECKPOINT_TABLE)
            .map_err(|error| map_sqlite_error(&error, path.as_path()))?;
        Ok(JournalReaderImpl {
            path: path.clone(),
            connection,
        })
    }

    /// 読んでいるストアファイルの場所。
    #[must_use]
    pub const fn path(&self) -> &StorePath {
        &self.path
    }

    /// 現在のチェックポイント (未登録は `ZERO`)。読取・前進の両方が使う。
    fn read_checkpoint(
        connection: &Connection,
        projection: &ProjectionName,
        path: &Path,
    ) -> Result<GlobalSeqNr, JournalReadError> {
        let raw: Option<i64> = connection
            .query_row(SELECT_CHECKPOINT, params![projection.as_str()], |row| {
                row.get(0)
            })
            .optional()
            .map_err(|error| map_sqlite_error(&error, path))?;
        match raw {
            None => Ok(GlobalSeqNr::ZERO),
            Some(value) => Ok(GlobalSeqNr::new(to_u64(value, NO_AGGREGATE)?)),
        }
    }
}

/// この名前の表があるか (`sqlite_master` の問い合わせ)。
fn table_exists(
    connection: &Connection,
    name: &str,
    path: &Path,
) -> Result<bool, JournalReadError> {
    let found: Option<String> = connection
        .query_row(
            "SELECT name FROM sqlite_master WHERE type = 'table' AND name = ?1",
            params![name],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| map_sqlite_error(&error, path))?;
    Ok(found.is_some())
}

/// ジャーナル行のペイロードをイベントへ復号し、**行の識別子と照合する**。
///
/// 形式は**本家のシリアライザと同じ** — 既定の `JsonEventSerializer` は
/// `serde_json::to_vec(event)` で書くので、読み側も素の serde で戻す。
///
/// 復号が通っても、payload が名乗る集約識別子・通番が journal 列と食い違う行は
/// `Corrupt(InvariantViolation)` — 別集約・別通番のイベントを投影に流さない
/// (B6 CodeRabbit #500)。
fn decode_event(row: &JournalRow) -> Result<WorkflowExecutionEvent, JournalReadError> {
    let event = serde_json::from_slice::<WorkflowExecutionEvent>(&row.payload)
        .map_err(|_| corrupt_error(&row.aggregate_id, None, CorruptCause::UndecodablePayload))?;
    let row_seq = usize::try_from(row.seq_nr)
        .map_err(|_| corrupt_error(&row.aggregate_id, None, CorruptCause::InvariantViolation))?;
    if event.aggregate_id().as_str() != row.aggregate_id || event.seq_nr() != row_seq {
        return Err(corrupt_error(
            &row.aggregate_id,
            Some(row_seq),
            CorruptCause::InvariantViolation,
        ));
    }
    // 対応外の schema_version は「解釈できない payload」— 予約フィールドの検査経路を
    // 復元する (B6 CodeRabbit #466 が発見した実装ギャップ。C5 の宣言どおり拒否する)。
    if event.schema_version() != WorkflowExecutionEvent::SCHEMA_VERSION {
        return Err(corrupt_error(
            &row.aggregate_id,
            Some(row_seq),
            CorruptCause::UndecodablePayload,
        ));
    }
    Ok(event)
}

impl JournalReader for JournalReaderImpl {
    async fn events_after(
        &self,
        after: GlobalSeqNr,
    ) -> Result<Vec<(GlobalSeqNr, WorkflowExecutionEvent)>, JournalReadError> {
        let from = to_i64(after.to_u64())?;
        let rows = {
            let mut statement = self
                .connection
                .prepare(SELECT_EVENTS_AFTER)
                .map_err(|error| map_sqlite_error(&error, self.path.as_path()))?;
            let mapped = statement
                .query_map(params![from], |row| {
                    Ok(JournalRow {
                        rowid: row.get::<_, i64>(0)?,
                        aggregate_id: row.get::<_, String>(1)?,
                        seq_nr: row.get::<_, i64>(2)?,
                        payload: row.get::<_, Vec<u8>>(3)?,
                    })
                })
                .map_err(|error| map_sqlite_error(&error, self.path.as_path()))?;
            let mut collected = Vec::new();
            for row in mapped {
                collected.push(row.map_err(|error| map_sqlite_error(&error, self.path.as_path()))?);
            }
            collected
        };

        let mut events = Vec::with_capacity(rows.len());
        for row in &rows {
            let event = decode_event(row)?;
            let global = GlobalSeqNr::new(to_u64(row.rowid, &row.aggregate_id)?);
            events.push((global, event));
        }
        Ok(events)
    }

    async fn checkpoint(
        &self,
        projection: &ProjectionName,
    ) -> Result<GlobalSeqNr, JournalReadError> {
        JournalReaderImpl::read_checkpoint(&self.connection, projection, self.path.as_path())
    }

    async fn advance_checkpoint(
        &mut self,
        projection: &ProjectionName,
        to: GlobalSeqNr,
    ) -> Result<(), JournalReadError> {
        let target = to_i64(to.to_u64())?;
        let path = self.path.clone();

        // 読み取ってから書くので `BEGIN IMMEDIATE` で書込ロックを最初に取る (BR2.3)。
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| map_sqlite_error(&error, path.as_path()))?;

        let current = JournalReaderImpl::read_checkpoint(&transaction, projection, path.as_path())?;
        if to < current {
            return Err(JournalReadError::CheckpointRegression {
                projection: projection.clone(),
                current,
                requested: to,
            });
        }
        transaction
            .execute(UPSERT_CHECKPOINT, params![projection.as_str(), target])
            .map_err(|error| map_sqlite_error(&error, path.as_path()))?;
        transaction
            .commit()
            .map_err(|error| map_sqlite_error(&error, path.as_path()))
    }
}

#[cfg(test)]
mod tests {
    // テストは固定長フィクスチャの添字参照と unwrap / expect を許容 (オーナー規約)。
    #![allow(clippy::indexing_slicing)]

    use super::*;
    use core_domain::orchestration::{IntentId, WorkflowExecution};

    /// 投影チェックポイントの表 (**我々の表**。本家の `journal` / `snapshot` と衝突しない)。
    const CHECKPOINT_TABLE: &str = "amadeus_projection_checkpoint";
    use core_domain::workspace::SpaceName;
    use event_store_adapter_rs::EventStoreForSqlite;
    use std::num::NonZeroUsize;

    /// 本家の SQLite ストア (この型の結合先)。
    type UpstreamStore = EventStoreForSqlite<IntentId, WorkflowExecution, WorkflowExecutionEvent>;

    /// 一時ディレクトリ配下のストアの場所。
    fn store_path(dir: &tempfile::TempDir) -> StorePath {
        let path = StorePath::for_space(&dir.path().join("aidlc"), &SpaceName::default());
        std::fs::create_dir_all(path.as_path().parent().expect("親 dir")).expect("intents/ を作る");
        path
    }

    /// 本家のストアを開いて (= 表を作って) その場所を返す。
    fn opened_store(dir: &tempfile::TempDir) -> (UpstreamStore, StorePath) {
        let path = store_path(dir);
        let store = UpstreamStore::new(path.as_path()).expect("本家ストアは開ける");
        (store, path)
    }

    /// **本家 v2.0.0 の `journal` スキーマ (ピン留め)。**
    ///
    /// `rowid` をカーソルに使ってよい根拠そのものである — 列構成が変わったり、
    /// `WITHOUT ROWID` になったり、削除経路が増えたりしたら前提が崩れる。
    const PINNED_JOURNAL_DDL: &str = "CREATE TABLE journal (\n  \
        pkey TEXT NOT NULL,\n  \
        skey TEXT NOT NULL,\n  \
        aid TEXT NOT NULL,\n  \
        seq_nr INTEGER NOT NULL,\n  \
        payload BLOB NOT NULL,\n  \
        occurred_at INTEGER NOT NULL,\n  \
        PRIMARY KEY (pkey, skey)\n)";

    /// 同じくピン留めした `journal` の一意索引。
    const PINNED_JOURNAL_INDEX_DDL: &str =
        "CREATE UNIQUE INDEX journal_aid_seq_nr_idx ON journal (aid, seq_nr)";

    #[test]
    fn the_upstream_journal_schema_is_the_pinned_one() {
        // スキーマガード (ADR-010 決定 4)。ここが落ちたら**本家スキーマが変わった**という
        // ことなので、`=2.0.0` の固定と `rowid` をカーソルに使う前提を見直すこと。
        // 直すべきは期待値ではなく、`JournalReaderImpl` の読み方である。
        let dir = tempfile::tempdir().expect("一時 dir");
        let (_store, path) = opened_store(&dir);
        let conn = Connection::open(path.as_path()).expect("生の接続");

        let table: String = conn
            .query_row(
                "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = 'journal'",
                [],
                |row| row.get(0),
            )
            .expect("本家の journal 表がある");
        assert_eq!(
            table, PINNED_JOURNAL_DDL,
            "本家スキーマが変わった。event-store-adapter-rs の =2.0.0 固定を見直せ"
        );

        let index: String = conn
            .query_row(
                "SELECT sql FROM sqlite_master WHERE type = 'index' AND name = 'journal_aid_seq_nr_idx'",
                [],
                |row| row.get(0),
            )
            .expect("本家の一意索引がある");
        assert_eq!(
            index, PINNED_JOURNAL_INDEX_DDL,
            "本家スキーマが変わった。event-store-adapter-rs の =2.0.0 固定を見直せ"
        );
    }

    #[test]
    fn the_journal_table_keeps_a_rowid_so_the_cursor_is_well_defined() {
        // `WITHOUT ROWID` 表には rowid が無い。カーソルの土台なので明示的に固定する。
        let dir = tempfile::tempdir().expect("一時 dir");
        let (_store, path) = opened_store(&dir);
        let conn = Connection::open(path.as_path()).expect("生の接続");
        let rows: i64 = conn
            .query_row("SELECT count(*) FROM journal WHERE rowid >= 0", [], |row| {
                row.get(0)
            })
            .expect("rowid を持つ表である");
        assert_eq!(rows, 0);
    }

    #[test]
    fn opening_before_the_upstream_store_exists_is_a_not_found() {
        let dir = tempfile::tempdir().expect("一時 dir");
        let path = store_path(&dir);
        let error = JournalReaderImpl::open(&path).expect_err("本家の表がまだ無い");
        assert_eq!(
            error,
            JournalReadError::Io {
                kind: ErrorKind::NotFound,
                path: Some(path.as_path().to_path_buf()),
            }
        );
    }

    #[test]
    fn opening_creates_the_checkpoint_table_next_to_the_upstream_tables() {
        let dir = tempfile::tempdir().expect("一時 dir");
        let (_store, path) = opened_store(&dir);
        let reader = JournalReaderImpl::open(&path).expect("開ける");
        assert_eq!(reader.path(), &path);

        let conn = Connection::open(path.as_path()).expect("生の接続");
        let mut statement = conn
            .prepare("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")
            .expect("sqlite_master");
        let tables: Vec<String> = statement
            .query_map([], |row| row.get::<_, String>(0))
            .expect("表の一覧")
            .filter_map(Result::ok)
            .filter(|name| !name.starts_with("sqlite_"))
            .collect();
        assert_eq!(tables, [CHECKPOINT_TABLE, "journal", "snapshot"]);
    }

    #[test]
    fn opening_twice_does_not_recreate_the_checkpoint_table() {
        let dir = tempfile::tempdir().expect("一時 dir");
        let (_store, path) = opened_store(&dir);
        {
            let conn = Connection::open(path.as_path()).expect("生の接続");
            conn.execute_batch(CREATE_CHECKPOINT_TABLE)
                .expect("表を作る");
            conn.execute(
                "INSERT INTO amadeus_projection_checkpoint(projection, last_global_seq)
                 VALUES ('state-file', 3)",
                [],
            )
            .expect("行を置く");
        }
        let _reader = JournalReaderImpl::open(&path).expect("開ける");
        let conn = Connection::open(path.as_path()).expect("生の接続");
        let last: i64 = conn
            .query_row(
                "SELECT last_global_seq FROM amadeus_projection_checkpoint",
                [],
                |row| row.get(0),
            )
            .expect("行は残る");
        assert_eq!(last, 3);
    }

    /// 本家のストアを開いてから、その表を生の SQL で壊すための接続。
    fn raw(path: &StorePath) -> Connection {
        Connection::open(path.as_path()).expect("生の接続")
    }

    #[test]
    fn opening_a_path_that_is_not_a_database_file_is_an_io_failure() {
        let dir = tempfile::tempdir().expect("一時 dir");
        let path = store_path(&dir);
        // ストアファイルの場所にディレクトリを置く (SQLite は開けない)。
        std::fs::create_dir(path.as_path()).expect("ディレクトリを置く");
        let error = JournalReaderImpl::open(&path).expect_err("開けない");
        assert_eq!(
            error,
            JournalReadError::Io {
                kind: ErrorKind::NotFound,
                path: Some(path.as_path().to_path_buf()),
            }
        );
    }

    #[test]
    fn opening_a_read_only_store_cannot_create_the_checkpoint_table() {
        let dir = tempfile::tempdir().expect("一時 dir");
        let (store, path) = opened_store(&dir);
        drop(store);
        let mut mode = std::fs::metadata(path.as_path())
            .expect("メタデータ")
            .permissions();
        mode.set_readonly(true);
        std::fs::set_permissions(path.as_path(), mode).expect("読取専用にする");

        let error = JournalReaderImpl::open(&path).expect_err("表を作れない");
        assert_eq!(
            error,
            JournalReadError::Io {
                kind: ErrorKind::PermissionDenied,
                path: Some(path.as_path().to_path_buf()),
            }
        );
    }

    #[tokio::test]
    async fn a_cursor_beyond_the_column_range_is_refused_before_the_query() {
        let dir = tempfile::tempdir().expect("一時 dir");
        let (_store, path) = opened_store(&dir);
        let mut reader = JournalReaderImpl::open(&path).expect("開ける");
        assert!(
            reader
                .events_after(GlobalSeqNr::new(u64::MAX))
                .await
                .is_err()
        );
        assert!(
            reader
                .advance_checkpoint(&projection(), GlobalSeqNr::new(u64::MAX))
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn a_missing_journal_table_is_reported_as_io() {
        let dir = tempfile::tempdir().expect("一時 dir");
        let (_store, path) = opened_store(&dir);
        let reader = JournalReaderImpl::open(&path).expect("開ける");
        raw(&path)
            .execute_batch("DROP TABLE journal")
            .expect("表を落とす");

        let error = reader
            .events_after(GlobalSeqNr::ZERO)
            .await
            .expect_err("表が無い");
        assert_eq!(
            error,
            JournalReadError::Io {
                kind: ErrorKind::Other,
                path: Some(path.as_path().to_path_buf()),
            }
        );
    }

    #[tokio::test]
    async fn a_missing_checkpoint_table_is_reported_as_io_on_both_faces() {
        let dir = tempfile::tempdir().expect("一時 dir");
        let (_store, path) = opened_store(&dir);
        let mut reader = JournalReaderImpl::open(&path).expect("開ける");
        raw(&path)
            .execute_batch("DROP TABLE amadeus_projection_checkpoint")
            .expect("表を落とす");

        let error = reader
            .checkpoint(&projection())
            .await
            .expect_err("表が無い");
        assert!(
            matches!(error, JournalReadError::Io { .. }),
            "実際: {error:?}"
        );
        let error = reader
            .advance_checkpoint(&projection(), GlobalSeqNr::new(1))
            .await
            .expect_err("表が無い");
        assert!(
            matches!(error, JournalReadError::Io { .. }),
            "実際: {error:?}"
        );
    }

    #[tokio::test]
    async fn a_negative_checkpoint_row_is_corrupt() {
        let dir = tempfile::tempdir().expect("一時 dir");
        let (_store, path) = opened_store(&dir);
        let reader = JournalReaderImpl::open(&path).expect("開ける");
        raw(&path)
            .execute(
                "INSERT INTO amadeus_projection_checkpoint(projection, last_global_seq)
                 VALUES ('state-file', -1)",
                [],
            )
            .expect("負値を置く");

        let error = reader
            .checkpoint(&projection())
            .await
            .expect_err("負の通番は無い");
        assert_eq!(
            error,
            JournalReadError::Corrupt {
                aggregate_id: NO_AGGREGATE.to_string(),
                seq_nr: None,
                cause: CorruptCause::InvariantViolation,
            }
        );
    }

    #[tokio::test]
    async fn a_row_whose_aggregate_id_is_not_text_is_reported_as_io() {
        let dir = tempfile::tempdir().expect("一時 dir");
        let (_store, path) = opened_store(&dir);
        let reader = JournalReaderImpl::open(&path).expect("開ける");
        raw(&path)
            .execute(
                "INSERT INTO journal(pkey, skey, aid, seq_nr, payload, occurred_at)
                 VALUES ('p', 's', X'FF', 1, X'7B7D', 0)",
                [],
            )
            .expect("UTF-8 でない aid を置く");

        let error = reader
            .events_after(GlobalSeqNr::ZERO)
            .await
            .expect_err("列を読めない");
        assert_eq!(
            error,
            JournalReadError::Io {
                kind: ErrorKind::Other,
                path: Some(path.as_path().to_path_buf()),
            }
        );
    }

    #[tokio::test]
    async fn a_row_whose_payload_is_not_bytes_is_reported_as_io() {
        let dir = tempfile::tempdir().expect("一時 dir");
        let (_store, path) = opened_store(&dir);
        let reader = JournalReaderImpl::open(&path).expect("開ける");
        raw(&path)
            .execute(
                "INSERT INTO journal(pkey, skey, aid, seq_nr, payload, occurred_at)
                 VALUES ('p', 's', 'agg', 1, 42, 0)",
                [],
            )
            .expect("BLOB でない payload を置く");

        let error = reader
            .events_after(GlobalSeqNr::ZERO)
            .await
            .expect_err("列を読めない");
        assert_eq!(
            error,
            JournalReadError::Io {
                kind: ErrorKind::Other,
                path: Some(path.as_path().to_path_buf()),
            }
        );
    }

    #[tokio::test]
    async fn a_write_lock_held_by_another_connection_is_reported_as_would_block() {
        // BR2.1 の待ち時間そのものを観測する。既定 (5000ms) では試験が待つだけなので、
        // `open_with_busy_timeout` で上限を縮めて `WouldBlock` を実測する (NFR3.5)。
        let dir = tempfile::tempdir().expect("一時 dir");
        let (_store, path) = opened_store(&dir);
        let mut reader =
            JournalReaderImpl::open_with_busy_timeout(&path, Duration::from_millis(20))
                .expect("開ける");

        let holder = raw(&path);
        holder
            .execute_batch("BEGIN EXCLUSIVE")
            .expect("書込ロックを握る");

        let error = reader
            .advance_checkpoint(&projection(), GlobalSeqNr::new(1))
            .await
            .expect_err("他の書き手がいる");
        assert_eq!(
            error,
            JournalReadError::Io {
                kind: ErrorKind::WouldBlock,
                path: Some(path.as_path().to_path_buf()),
            }
        );
    }

    /// 失敗経路の試験が使う投影名。
    fn projection() -> ProjectionName {
        ProjectionName::parse("state-file").expect("投影名は kebab")
    }

    #[test]
    fn a_cursor_that_does_not_fit_the_column_is_corrupt_rather_than_rounded() {
        let error = to_i64(u64::MAX).expect_err("i64 に収まらない");
        assert_eq!(
            error,
            JournalReadError::Corrupt {
                aggregate_id: NO_AGGREGATE.to_string(),
                seq_nr: None,
                cause: CorruptCause::InvariantViolation,
            }
        );
        assert!(to_u64(-1, "agg").is_err(), "負の rowid は無い");
    }

    #[test]
    fn a_row_whose_payload_names_another_aggregate_is_corrupt() {
        // 復号は通るが、payload の名乗る集約が journal 列の aid と食い違う行 —
        // 別集約のイベントを投影へ流さない (B6 CodeRabbit #500)。
        let event = WorkflowExecutionEvent::new(
            IntentId::parse("01a02785-1bd8-76eb-aeea-5aa303ebd5b6").unwrap(),
            NonZeroUsize::MIN,
            chrono::DateTime::parse_from_rfc3339("2026-08-27T00:00:00Z")
                .unwrap()
                .with_timezone(&chrono::Utc),
            core_domain::orchestration::WorkflowExecutionEventPayload::Unparked,
        );
        #[allow(
            clippy::disallowed_methods,
            reason = "本家シリアライザと同形式のフィクスチャ生成 (BR1.7 の射程外)"
        )]
        let payload = serde_json::to_vec(&event).unwrap();
        let row = JournalRow {
            rowid: 1,
            seq_nr: 1,
            aggregate_id: "018f3b2c-4d5e-7f60-8abc-def012345678".to_string(),
            payload,
        };
        assert_eq!(
            decode_event(&row).expect_err("照合で落ちる"),
            JournalReadError::Corrupt {
                aggregate_id: "018f3b2c-4d5e-7f60-8abc-def012345678".to_string(),
                seq_nr: Some(1),
                cause: CorruptCause::InvariantViolation,
            }
        );
        // 通番の食い違いも同じ照合で落ちる。
        #[allow(
            clippy::disallowed_methods,
            reason = "本家シリアライザと同形式のフィクスチャ生成 (BR1.7 の射程外)"
        )]
        let payload2 = serde_json::to_vec(&event).unwrap();
        let skewed = JournalRow {
            rowid: 2,
            seq_nr: 9,
            aggregate_id: "01a02785-1bd8-76eb-aeea-5aa303ebd5b6".to_string(),
            payload: payload2,
        };
        assert_eq!(
            decode_event(&skewed).expect_err("通番不一致"),
            JournalReadError::Corrupt {
                aggregate_id: "01a02785-1bd8-76eb-aeea-5aa303ebd5b6".to_string(),
                seq_nr: Some(9),
                cause: CorruptCause::InvariantViolation,
            }
        );
    }

    #[test]
    fn opening_a_database_without_the_journal_table_is_not_found() {
        // #511 で「無いファイル」は open 段階で落ちるようになったため、この分岐
        // (ファイルは在るが本家の journal 表が無い) を踏む経路を独立に固定する。
        let dir = tempfile::tempdir().unwrap();
        let path = StorePath::for_space(&dir.path().join("aidlc"), &SpaceName::default());
        std::fs::create_dir_all(path.as_path().parent().unwrap()).unwrap();
        // journal 表を持たない有効な SQLite ファイルを作る。
        let bootstrap = Connection::open(path.as_path()).unwrap();
        bootstrap
            .execute("CREATE TABLE unrelated (x INTEGER)", [])
            .unwrap();
        drop(bootstrap);
        let error = JournalReaderImpl::open(&path).expect_err("journal 表が無い");
        assert!(
            matches!(
                error,
                JournalReadError::Io {
                    kind: ErrorKind::NotFound,
                    ..
                }
            ),
            "表の不在は NotFound: {error:?}"
        );
    }

    #[test]
    fn opening_a_missing_store_does_not_create_the_file() {
        // 読取側の接続はストアファイルを作らない (B6 CodeRabbit #511)。
        let dir = tempfile::tempdir().unwrap();
        let path = StorePath::for_space(&dir.path().join("aidlc"), &SpaceName::default());
        std::fs::create_dir_all(path.as_path().parent().unwrap()).unwrap();
        let error = JournalReaderImpl::open(&path).expect_err("無いストアは開けない");
        assert!(
            matches!(
                error,
                JournalReadError::Io {
                    kind: ErrorKind::NotFound,
                    ..
                }
            ),
            "NotFound で失敗する: {error:?}"
        );
        assert!(!path.as_path().exists(), "空の SQLite ファイルを作らない");
    }

    #[test]
    fn a_row_with_a_negative_sequence_number_is_corrupt() {
        // journal.seq_nr は本家スキーマ上 INTEGER — 負値は書込経路からは生まれないが、
        // 破損検出の境界なので usize への写しの失敗も Corrupt に畳む (#500 の照合の一部)。
        // payload は**有効なイベント**にする — 復号失敗ではなく try_from の分岐を踏むため。
        let event = WorkflowExecutionEvent::new(
            IntentId::parse("01a02785-1bd8-76eb-aeea-5aa303ebd5b6").unwrap(),
            NonZeroUsize::MIN,
            chrono::DateTime::parse_from_rfc3339("2026-08-27T00:00:00Z")
                .unwrap()
                .with_timezone(&chrono::Utc),
            core_domain::orchestration::WorkflowExecutionEventPayload::Unparked,
        );
        #[allow(
            clippy::disallowed_methods,
            reason = "本家シリアライザと同形式のフィクスチャ生成 (BR1.7 の射程外)"
        )]
        let payload = serde_json::to_vec(&event).unwrap();
        let row = JournalRow {
            rowid: 1,
            seq_nr: -1,
            aggregate_id: "01a02785-1bd8-76eb-aeea-5aa303ebd5b6".to_string(),
            payload,
        };
        assert_eq!(
            decode_event(&row).expect_err("負の通番"),
            JournalReadError::Corrupt {
                aggregate_id: "01a02785-1bd8-76eb-aeea-5aa303ebd5b6".to_string(),
                seq_nr: None,
                cause: CorruptCause::InvariantViolation,
            }
        );
    }

    #[test]
    fn a_payload_with_an_unsupported_schema_version_is_corrupt() {
        // C5 の宣言どおり、対応外の schema_version は復号成功でも拒否する (#466)。
        let event = WorkflowExecutionEvent::new(
            IntentId::parse("01a02785-1bd8-76eb-aeea-5aa303ebd5b6").unwrap(),
            NonZeroUsize::MIN,
            chrono::DateTime::parse_from_rfc3339("2026-08-27T00:00:00Z")
                .unwrap()
                .with_timezone(&chrono::Utc),
            core_domain::orchestration::WorkflowExecutionEventPayload::Unparked,
        );
        #[allow(
            clippy::disallowed_methods,
            reason = "本家シリアライザと同形式のフィクスチャ生成 (BR1.7 の射程外)"
        )]
        let json = serde_json::to_string(&event).unwrap();
        let tampered = json.replace("\"schema_version\":1", "\"schema_version\":99");
        assert_ne!(json, tampered, "書き換えが効いている");
        let row = JournalRow {
            rowid: 1,
            seq_nr: 1,
            aggregate_id: "01a02785-1bd8-76eb-aeea-5aa303ebd5b6".to_string(),
            payload: tampered.into_bytes(),
        };
        assert_eq!(
            decode_event(&row).expect_err("対応外の版"),
            JournalReadError::Corrupt {
                aggregate_id: "01a02785-1bd8-76eb-aeea-5aa303ebd5b6".to_string(),
                seq_nr: Some(1),
                cause: CorruptCause::UndecodablePayload,
            }
        );
    }

    #[test]
    fn a_payload_that_is_not_an_event_is_corrupt() {
        let row = JournalRow {
            rowid: 1,
            seq_nr: 1,
            aggregate_id: "01a02785-1bd8-76eb-aeea-5aa303ebd5b6".to_string(),
            payload: b"{not json".to_vec(),
        };
        assert_eq!(
            decode_event(&row).expect_err("復号できない"),
            JournalReadError::Corrupt {
                aggregate_id: "01a02785-1bd8-76eb-aeea-5aa303ebd5b6".to_string(),
                seq_nr: None,
                cause: CorruptCause::UndecodablePayload,
            }
        );
    }
}
