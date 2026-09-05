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
//! ## `VACUUM` はカーソルを動かさない
//!
//! `journal` に `INTEGER PRIMARY KEY` は無いので、SQLite の仕様上 `VACUUM` は rowid を
//! 振り直し得る。ただし振り直しが**値を変える**のは行削除で隙間ができた場合だけであり、
//! `journal` は削除ゼロの純追記 (`DELETE` 文は本家の snapshot 表にしか無い) なので rowid は
//! 隙間の無い連番 1..N のまま — 再構築後も同値に保たれ、保存済みチェックポイントは有効で
//! あり続ける。この前提は回帰テスト
//! (`journal_reader_impl_test.rs::a_vacuum_rebuild_does_not_move_the_cursor`) で
//! 実挙動に釘留めしている。本家に削除経路が増えたらこの前提ごと見直すこと
//! (スキーマガードと同じ運用)。
//!
//! さらに多層防御として、チェックポイント表に**アンカー (aid, seq_nr)** を併記する —
//! `advance_checkpoint` が前進先の journal 行の識別子を保存し、読取はアンカーを journal の
//! 同 rowid と照合して、食い違い (振り直し・改変の兆候) を
//! `Corrupt (CheckpointAnchorMismatch)` で明示的に拒否する。前提が破れても静かな欠落・
//! 重複にはならない。
//!
//! 本家スキーマへの結合は次の 2 つで守る:
//!
//! 1. 版の**完全固定** (`event-store-adapter-rs = "=3.0.0"` — ADR-010 決定 4)
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

use chrono::{DateTime, Utc};
use rusqlite::{Connection, OpenFlags, OptionalExtension, TransactionBehavior, params};

use super::corrupt_cause::CorruptCause;
use super::definition_entry::DefinitionEntry;
use super::global_seq_nr::GlobalSeqNr;
use super::journal_batch::JournalBatch;
use super::journal_entry::JournalEntry;
use super::journal_read_error::JournalReadError;
use super::journal_reader::JournalReader;
use super::projection_name::ProjectionName;
use super::store_failure::io_kind;
use core_command_domain::orchestration::{
    Intent, IntentExecutionEvent, IntentExecutionId, IntentId,
};
use core_command_domain::workflow_definition::{WorkflowDefinitionEvent, WorkflowDefinitionId};

use super::dto::{
    DtoDecodeError, IntentEventDto, IntentExecutionEventDto, WorkflowDefinitionEventDto,
};
use crate::read_tables::{
    READ_SCHEMA_VERSION, ReadTables, SteeringTables, ensure_tables, read_schema_version,
    recreate_tables, replace_all, replace_steering, set_schema_version,
};
use core_command_domain::workspace::StorePath;

/// 書込ロックを待つ既定の上限 (BR2.1)。読取専用の接続でも、チェックポイントの前進だけは
/// 書込なので待ち時間が要る。
const DEFAULT_BUSY_TIMEOUT: Duration = Duration::from_millis(5000);

/// 本家の追記専用ジャーナル表 (`rowid` がコミット順の単調カーソルになる)。
const UPSTREAM_JOURNAL_TABLE: &str = "journal";

/// チェックポイント表の DDL (冪等)。
const CREATE_CHECKPOINT_TABLE: &str = "CREATE TABLE IF NOT EXISTS amadeus_projection_checkpoint (
  projection      TEXT    PRIMARY KEY,
  last_global_seq INTEGER NOT NULL,
  anchor_aid      TEXT,
  anchor_seq_nr   INTEGER
)";

/// 全集約横断の差分読取 (`rowid` 昇順)。
///
/// v3 で journal は `occurred_at` (epoch ナノ秒) と `manifest` (型判別子) を持つ。封筒の材料は
/// 列から読む — payload には輸送のメタデータが入らなくなったためである (ADR-010 / B7)。
const SELECT_EVENTS_AFTER: &str = "SELECT rowid, aid, seq_nr, payload, occurred_at, manifest
     FROM journal WHERE rowid > ?1 AND (?2 IS NULL OR rowid <= ?2) ORDER BY rowid";

/// 投影のチェックポイント。
const SELECT_CHECKPOINT: &str = "SELECT last_global_seq, anchor_aid, anchor_seq_nr
     FROM amadeus_projection_checkpoint WHERE projection = ?1";

/// チェックポイント位置の journal 行の識別子 (アンカーの記録・照合の両方が使う)。
const SELECT_ANCHOR_ROW: &str = "SELECT aid, seq_nr FROM journal WHERE rowid = ?1";

/// 保存済み steering 面の出所 (全行に同じ値が書かれているので 1 行で足りる)。
const SELECT_STEERING_SOURCE: &str =
    "SELECT source_digest FROM read_steering_plan ORDER BY phase LIMIT 1";

/// チェックポイントの前進 (未登録なら登録)。
const UPSERT_CHECKPOINT: &str =
    "INSERT INTO amadeus_projection_checkpoint(projection, last_global_seq, anchor_aid, anchor_seq_nr)
     VALUES (?1, ?2, ?3, ?4)
     ON CONFLICT(projection) DO UPDATE SET
       last_global_seq = excluded.last_global_seq,
       anchor_aid = excluded.anchor_aid,
       anchor_seq_nr = excluded.anchor_seq_nr";

/// 進んだチェックポイントの本数 (読み面の作り直しが要るストアかの判別に使う)。
const SELECT_ADVANCED_CHECKPOINTS: &str =
    "SELECT COUNT(*) FROM amadeus_projection_checkpoint WHERE last_global_seq > 0";

/// 集約に属さない行 (チェックポイント・カーソル) の識別子欄に置く印。
const NO_AGGREGATE: &str = "-";

/// rusqlite の失敗を `Io { kind, path }` へ写す (材料のみ — 文言は運ばない)。
pub(super) fn map_sqlite_error(error: &rusqlite::Error, path: &Path) -> JournalReadError {
    JournalReadError::Io {
        kind: io_kind(error),
        path: Some(path.to_path_buf()),
    }
}

/// 行の材料を添えて `Corrupt` を組む。
pub(super) fn corrupt_error(
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
    /// 発生時刻 (epoch ナノ秒 — v3 の `occurred_at` 列)。
    occurred_at: i64,
    /// 型判別子 (v3 の `manifest` 列。本家の既定は空文字列)。
    manifest: String,
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
        let mut connection = Connection::open_with_flags(
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
        // 構造化リードモデルの 17 表も我々の表である (本家の DDL とは衝突しない
        // `read_` 接頭)。版が一致していれば冪等な `CREATE TABLE IF NOT EXISTS` だけ、
        // 動いていれば落として作り直しジャーナルから描き直す。
        let schema_changed = read_schema_version(&connection)
            .map_err(|e| map_sqlite_error(&e, path.as_path()))?
            != READ_SCHEMA_VERSION;
        JournalReaderImpl::ensure_read_schema(&mut connection, path.as_path())?;
        super::publication_store::initialize(&connection, path.as_path())?;
        super::shared_projection::initialize(&connection, path.as_path())?;
        if schema_changed {
            super::shared_projection::invalidate(&connection, path.as_path())?;
        }
        let mut reader = JournalReaderImpl {
            path: path.clone(),
            connection,
        };
        if super::shared_projection::read(&reader.connection, path.as_path())?
            .is_some_and(|head| !head.is_current())
        {
            reader.rebuild_read_model().map_err(|error| match error {
                super::CatchUpError::Read(inner) => inner,
                _ => corrupt_error(NO_AGGREGATE, None, CorruptCause::ProjectionSnapshotMismatch),
            })?;
        }
        Ok(reader)
    }

    /// 読んでいるストアファイルの場所。
    #[must_use]
    pub const fn path(&self) -> &StorePath {
        &self.path
    }

    /// 新規イベントの有無によらず、現在の全履歴から共有構造化面を再生成する。
    /// 個別ファイルのチェックポイントは変更しない。
    ///
    /// # Errors
    /// 履歴欠落・破損、投影不能、DB更新の失敗。
    pub fn rebuild_read_model(&mut self) -> Result<GlobalSeqNr, super::CatchUpError> {
        let path = self.path.clone();
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| map_sqlite_error(&e, path.as_path()))?;
        let history = Self::scan_from(&transaction, path.as_path(), GlobalSeqNr::ZERO)?;
        let last = history.scanned_to().unwrap_or(GlobalSeqNr::ZERO);
        let recorded: i64 = transaction
            .query_row(
                "SELECT COALESCE(MAX(last_global_seq),0) FROM amadeus_projection_checkpoint",
                [],
                |row| row.get(0),
            )
            .map_err(|e| map_sqlite_error(&e, path.as_path()))?;
        let head = super::shared_projection::read(&transaction, path.as_path())?;
        let published = match head {
            Some(head) => head.position(),
            None => super::shared_projection::known_position(&transaction, path.as_path())?,
        };
        let recorded = recorded.max(published);
        if to_i64(last.to_u64())? < recorded {
            return Err(
                corrupt_error(NO_AGGREGATE, None, CorruptCause::CheckpointAnchorMismatch).into(),
            );
        }
        let tables = ReadTables::project(&history)?;
        replace_all(&transaction, &tables).map_err(|e| map_sqlite_error(&e, path.as_path()))?;
        super::shared_projection::record(&transaction, path.as_path(), to_i64(last.to_u64())?)?;
        transaction
            .commit()
            .map_err(|e| map_sqlite_error(&e, path.as_path()))?;
        Ok(last)
    }

    /// 失われた出力を最後の確定済み計画から復元する。存在する本文は保持する。
    ///
    /// # Errors
    /// 対象・確定位置の不一致、保存済み計画や履歴の破損、公開時の失敗。
    pub fn restore_missing_files(
        &mut self,
        projection: &ProjectionName,
        targets: &super::ProjectionTargets,
    ) -> Result<bool, super::CatchUpError> {
        if super::publication_store::pending(&self.connection, self.path.as_path(), projection)?
            .is_some()
        {
            return Ok(false);
        }
        let target = super::PublicationBatch::rebuild(GlobalSeqNr::ZERO, GlobalSeqNr::ZERO, vec![])
            .for_targets(targets)?;
        let Some(binding) = target.target_binding() else {
            return Ok(false);
        };
        let Some(previous) = super::publication_store::snapshot(
            &self.connection,
            self.path.as_path(),
            projection,
            binding,
        )?
        else {
            return Ok(false);
        };
        if !previous.matches_targets(targets) || !previous.uses_current_transform() {
            return Ok(false);
        }
        let mut missing = false;
        let mut files = Vec::new();
        for file in previous.files() {
            if let Some((restored, absent)) = file.restore_missing()? {
                missing |= absent;
                files.push(restored);
            }
        }
        if !missing {
            return Ok(false);
        }
        let checkpoint = Self::read_checkpoint(&self.connection, projection, self.path.as_path())?;
        if checkpoint < previous.to() {
            return Err(super::CatchUpError::PublicationConflict {
                path: self.path.as_path().to_path_buf(),
            });
        }
        // 共有面は現在の履歴から修復し、個別ファイルは保存済み断面を回復する。
        self.rebuild_read_model()?;
        let history = Self::scan_range(
            &self.connection,
            self.path.as_path(),
            GlobalSeqNr::ZERO,
            Some(checkpoint),
        )?;
        let tables = ReadTables::project(&history)?;
        let batch =
            super::PublicationBatch::rebuild(checkpoint, checkpoint, files).for_targets(targets)?;
        super::publication_store::publish(
            &mut self.connection,
            self.path.as_path(),
            projection,
            &batch,
            &tables,
        )?;
        Ok(true)
    }

    /// 競合した未完計画を、現在内容を保持する新世代へ置換して再開する。
    /// 自動的に対応を証明できない編集は競合のまま残す。
    ///
    /// # Errors
    /// 対象不一致・曖昧な編集・履歴の破損・公開の失敗。
    pub fn resolve_publication(
        &mut self,
        projection: &ProjectionName,
        targets: &super::ProjectionTargets,
    ) -> Result<bool, super::CatchUpError> {
        let Some(previous) =
            super::publication_store::pending(&self.connection, self.path.as_path(), projection)?
        else {
            return Ok(false);
        };
        if !previous.matches_targets(targets) {
            return Err(super::CatchUpError::PublicationConflict {
                path: targets.state_file().to_path_buf(),
            });
        }
        let files = previous
            .files()
            .iter()
            .map(super::PublicationFile::rebase)
            .collect::<Result<Vec<_>, _>>()?;
        let batch = super::PublicationBatch::rebuild(previous.from(), previous.to(), files)
            .for_targets(targets)?
            .replacing(previous.request_id());
        let history = Self::scan_range(
            &self.connection,
            self.path.as_path(),
            GlobalSeqNr::ZERO,
            Some(previous.to()),
        )?;
        let tables = ReadTables::project(&history)?;
        super::publication_store::publish(
            &mut self.connection,
            self.path.as_path(),
            projection,
            &batch,
            &tables,
        )?;
        Ok(true)
    }

    /// 読み面 17 表を**版付きで**用意する (取得ループの入口 = 開く段で 1 度)。
    ///
    /// `PRAGMA user_version` に [`READ_SCHEMA_VERSION`] を持ち、保存値が現行と同じなら
    /// 冪等な `CREATE TABLE IF NOT EXISTS` だけを打つ。違うときは 17 表を落として作り直し、
    /// **その場でジャーナル全履歴から描き直す**。
    ///
    /// # なぜ作り直しが要るか
    ///
    /// `CREATE TABLE IF NOT EXISTS` は既存の表に何もしない。列の形が変わる改訂
    /// (b47 の `read_next_answer.gated INTEGER` → `gate TEXT`) を旧スキーマのストアへ
    /// 持ち込むと、表は旧いまま残り `INSERT` が `no such column` で落ちる。行の正本は
    /// ジャーナルなので、読み面は捨てて描き直せる。
    ///
    /// # なぜチェックポイントを戻さないか
    ///
    /// チェックポイントは Markdown 面 (状態ファイル・監査シャード) と**共有**である。
    /// 戻すと未投影区間が全履歴になり、監査シャードに同じブロックがもう一度並ぶ
    /// ([`crate::orchestration::ReadModelUpdater::catch_up`] の「書いてから進める」)。
    /// 読み面だけを作り直したいので、ここで全履歴を引いて `replace_all` し、
    /// チェックポイントには触れない。参照入力由来の 2 表 (`read_steering_*`) は空のまま
    /// 戻るが、次の `catch_up` が保存済み `source_digest` を `None` と見て描き直す。
    ///
    /// # 描き直すのは「投影済みのストア」だけ
    ///
    /// チェックポイントがまだ 1 つも進んでいないストア (鋳造直後・未投影) には作り直す
    /// 中身が無く、次の `catch_up` が全履歴から普通に描く。開く段で毎回ジャーナル全体を
    /// 復号すると、読むだけの動詞まで復号の失敗で倒れるようになるので、**進んだ
    /// チェックポイントが在るときだけ**その場の描き直しに入る。
    fn ensure_read_schema(
        connection: &mut Connection,
        path: &Path,
    ) -> Result<(), JournalReadError> {
        // SQLite の失敗はどれも同じ写像なので、写す口は 1 つに束ねる。
        let io = |error: rusqlite::Error| map_sqlite_error(&error, path);
        let stored = read_schema_version(connection).map_err(io)?;
        if stored == READ_SCHEMA_VERSION {
            return ensure_tables(connection).map_err(io);
        }
        recreate_tables(connection).map_err(io)?;
        if !JournalReaderImpl::projected_before(connection, path)? {
            return set_schema_version(connection, READ_SCHEMA_VERSION).map_err(io);
        }
        let history = JournalReaderImpl::scan_from(connection, path, GlobalSeqNr::ZERO)?;
        // 描けない歴史 (切り落とし・復号不能) は作り直しでも直らない — 版を上げずに止める。
        let tables = ReadTables::project(&history)
            .map_err(|_| corrupt_error(NO_AGGREGATE, None, CorruptCause::InvariantViolation))?;
        // 行の全差し替えと版の記録は同じ Tx に閉じる — 途中で落ちたら版も上がらず、
        // 次の起動が同じ作り直しをやり直す。
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(io)?;
        replace_all(&transaction, &tables).map_err(io)?;
        set_schema_version(&transaction, READ_SCHEMA_VERSION).map_err(io)?;
        transaction.commit().map_err(io)
    }

    /// このストアで投影が 1 度でも進んだか (進んだチェックポイントが在るか)。
    ///
    /// 読み面の作り直しが要るのは、**旧スキーマの表がすでに描かれている**ストアだけで
    /// ある。未投影のストアは作り直す中身を持たない。
    fn projected_before(connection: &Connection, path: &Path) -> Result<bool, JournalReadError> {
        let count: i64 = connection
            .query_row(SELECT_ADVANCED_CHECKPOINTS, [], |row| row.get(0))
            .map_err(|error| map_sqlite_error(&error, path))?;
        Ok(count > 0)
    }

    /// ジャーナルを `after` の先から走査する (同期の核)。
    ///
    /// [`JournalReader::events_after`] の本体そのものであり、開く段のスキーマ作り直し
    /// ([`JournalReaderImpl::ensure_read_schema`]) も同じ核を使う — 版が動いたときに
    /// 読み面を全履歴から描き直すのに、非同期の口を通す必要が無いためである。
    pub(super) fn advance_on(
        connection: &rusqlite::Transaction<'_>,
        path: &Path,
        projection: &ProjectionName,
        to: GlobalSeqNr,
        tables: &ReadTables,
    ) -> Result<(), JournalReadError> {
        let target = to_i64(to.to_u64())?;
        let current = JournalReaderImpl::read_checkpoint(connection, projection, path)?;
        if to < current {
            return Err(JournalReadError::CheckpointRegression {
                projection: projection.clone(),
                current,
                requested: to,
            });
        }
        // 前進先の journal 行の識別子をアンカーとして併記する。journal に無い位置へは
        // 進めない — 進めると以後の照合が必ず失敗する (ZERO はアンカー無し)。
        let anchor: Option<(String, i64)> = if target == 0 {
            None
        } else {
            let row: Option<(String, i64)> = connection
                .query_row(SELECT_ANCHOR_ROW, params![target], |row| {
                    Ok((row.get(0)?, row.get(1)?))
                })
                .optional()
                .map_err(|error| map_sqlite_error(&error, path))?;
            match row {
                Some(found) => Some(found),
                None => {
                    return Err(corrupt_error(
                        NO_AGGREGATE,
                        None,
                        CorruptCause::CheckpointAnchorMismatch,
                    ));
                }
            }
        };
        let (anchor_aid, anchor_seq_nr) = match &anchor {
            Some((aid, seq_nr)) => (Some(aid.as_str()), Some(*seq_nr)),
            None => (None, None),
        };
        // 行の全差し替えとチェックポイントの前進は**同じ Tx** である (裁定 §3)。
        // 上の単調性・アンカー照合で早期 return したときは行も 1 つも変わっていない
        // (Tx は commit されずに落ちる)。
        // 個別カーソルが遅れていても、space共有の行集合は後退させない。
        // 古い候補を確定する場合も、共有面の現物と耐久headの一致を検査する。
        let head = super::shared_projection::verify(connection, path)?;
        let shared = head.position();
        if target == shared
            && !crate::read_tables::matches_rows(connection, tables)
                .map_err(|error| map_sqlite_error(&error, path))?
        {
            return Err(corrupt_error(
                NO_AGGREGATE,
                None,
                CorruptCause::ProjectionSnapshotMismatch,
            ));
        }
        if target > shared {
            replace_all(connection, tables).map_err(|error| map_sqlite_error(&error, path))?;
            super::shared_projection::record(connection, path, target)?;
        }
        connection
            .execute(
                UPSERT_CHECKPOINT,
                params![projection.as_str(), target, anchor_aid, anchor_seq_nr],
            )
            .map_err(|error| map_sqlite_error(&error, path))?;
        Ok(())
    }

    fn scan_from(
        connection: &Connection,
        path: &Path,
        after: GlobalSeqNr,
    ) -> Result<JournalBatch, JournalReadError> {
        Self::scan_range(connection, path, after, None)
    }

    pub(super) fn scan_range(
        connection: &Connection,
        path: &Path,
        after: GlobalSeqNr,
        through: Option<GlobalSeqNr>,
    ) -> Result<JournalBatch, JournalReadError> {
        let through = through.map(|value| to_i64(value.to_u64())).transpose()?;
        let from = to_i64(after.to_u64())?;
        let rows = {
            let mut statement = connection
                .prepare(SELECT_EVENTS_AFTER)
                .map_err(|error| map_sqlite_error(&error, path))?;
            let mapped = statement
                .query_map(params![from, through], |row| {
                    Ok(JournalRow {
                        rowid: row.get::<_, i64>(0)?,
                        aggregate_id: row.get::<_, String>(1)?,
                        seq_nr: row.get::<_, i64>(2)?,
                        payload: row.get::<_, Vec<u8>>(3)?,
                        occurred_at: row.get::<_, i64>(4)?,
                        manifest: row.get::<_, String>(5)?,
                    })
                })
                .map_err(|error| map_sqlite_error(&error, path))?;
            let mut collected = Vec::new();
            for row in mapped {
                collected.push(row.map_err(|error| map_sqlite_error(&error, path))?);
            }
            collected
        };

        let scanned_to = rows
            .last()
            .map(|row| to_u64(row.rowid, &row.aggregate_id).map(GlobalSeqNr::new))
            .transpose()?;
        let mut entries = Vec::new();
        let mut intents = Vec::new();
        let mut definitions = Vec::new();
        for row in &rows {
            // 同居する 3 ストリームを判別子で振り分ける (issue #50 / #56、定義は 2026-08-31)。
            if row.manifest == DEFINITION_EVENT_MANIFEST {
                definitions.push(decode_definition_row(row)?);
                continue;
            }
            if row.manifest == INTENT_EVENT_MANIFEST {
                intents.push(decode_intent_row(row)?);
            } else {
                entries.push(decode_entry(row)?);
            }
        }
        Ok(JournalBatch::new(entries, intents, definitions, scanned_to))
    }

    /// 現在のチェックポイント (未登録は `ZERO`)。読取・前進の両方が使う。
    ///
    /// 正のチェックポイントは保存済みアンカー (aid, seq_nr) を journal の同 rowid と照合して
    /// から返す — 食い違いは `Corrupt (CheckpointAnchorMismatch)` (PR #30 レビュー裁定)。
    pub(super) fn read_checkpoint(
        connection: &Connection,
        projection: &ProjectionName,
        path: &Path,
    ) -> Result<GlobalSeqNr, JournalReadError> {
        let raw: Option<(i64, Option<String>, Option<i64>)> = connection
            .query_row(SELECT_CHECKPOINT, params![projection.as_str()], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })
            .optional()
            .map_err(|error| map_sqlite_error(&error, path))?;
        let Some((value, anchor_aid, anchor_seq_nr)) = raw else {
            return Ok(GlobalSeqNr::ZERO);
        };
        let checkpoint = GlobalSeqNr::new(to_u64(value, NO_AGGREGATE)?);
        if checkpoint == GlobalSeqNr::ZERO {
            return Ok(checkpoint);
        }
        JournalReaderImpl::verify_anchor(connection, path, checkpoint, anchor_aid, anchor_seq_nr)?;
        Ok(checkpoint)
    }

    /// 保存済みアンカーを journal の同 rowid と照合する。
    ///
    /// rowid が振り直される・ジャーナルが改変されると、`rowid > チェックポイント` の差分読取は
    /// 欠落や重複を起こす。照合はそれを静かな破損ではなく明示エラーにする。
    fn verify_anchor(
        connection: &Connection,
        path: &Path,
        checkpoint: GlobalSeqNr,
        anchor_aid: Option<String>,
        anchor_seq_nr: Option<i64>,
    ) -> Result<(), JournalReadError> {
        // 正のチェックポイントには advance が必ずアンカーを書く — 欠けは直接改変の兆候。
        let (Some(expected_aid), Some(expected_seq_nr)) = (anchor_aid, anchor_seq_nr) else {
            return Err(corrupt_error(
                NO_AGGREGATE,
                None,
                CorruptCause::CheckpointAnchorMismatch,
            ));
        };
        let expected_seq_nr = usize::try_from(expected_seq_nr)
            .map_err(|_| corrupt_error(&expected_aid, None, CorruptCause::InvariantViolation))?;
        let target = to_i64(checkpoint.to_u64())?;
        let actual: Option<(String, i64)> = connection
            .query_row(SELECT_ANCHOR_ROW, params![target], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .optional()
            .map_err(|error| map_sqlite_error(&error, path))?;
        let matches = actual.as_ref().is_some_and(|(aid, seq_nr)| {
            *aid == expected_aid && usize::try_from(*seq_nr) == Ok(expected_seq_nr)
        });
        if matches {
            Ok(())
        } else {
            Err(corrupt_error(
                &expected_aid,
                Some(expected_seq_nr),
                CorruptCause::CheckpointAnchorMismatch,
            ))
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

/// ジャーナル 1 行を読取レコードへ写す。
///
/// payload の形式は**本家のシリアライザと同じ** — 既定の `JsonEventSerializer` は
/// `serde_json::to_vec(payload)` で書くので、読み側も素の serde で戻す。輸送のメタデータは
/// payload ではなく**列**から来る (ADR-010 / B7)。
///
/// 行が名乗る識別子は `IntentExecutionId` として妥当とは限らない (破損・直接改変) ので、ここで
/// 検証して初めてドメイン型になる。写せない行は `Corrupt(InvariantViolation)` —
/// 列の値をドメインへ運べない、という他の変換 (`to_u64` / 負の `seq_nr`) と同じ扱いである。
///
/// `manifest` の不一致・欠落は `Corrupt(UndecodablePayload)` である。この列は payload の型と
/// 読み方の版を名乗る値なので、名乗りが違えば中身を解釈してはならない (旧 `schema_version`
/// 検査 (B6 CodeRabbit #466) の後継。payload 内メタとの二重照合 (#500) はメタが payload から
/// 消えたことで不要になった)。
/// ジャーナル行 `manifest` 列に期待する型判別子 — **読む側の正本**。
///
/// 書く側 (command interface-adapter) は自前の同値の定数を持つ。共有しないのは
/// `coding-rules/cqrs-boundaries.md` の側ごと専用化に従うためで、両者が一致していることは
/// 横断適合テスト (`journal_protocol_conformance` / ゴールデンパリティ) が固定する。
const EVENT_MANIFEST: &str = "intent-execution-event/1";

/// intent 自身のジャーナル行の型判別子 — 同じストアファイルに同居する別ストリーム
/// (issue #50: intent の `Created` は実行と同じ journal 表に書かれる)。
///
/// この行は**消費する** (issue #56) — `Created` の誕生材料を検査付き再構成で [`Intent`] へ
/// 戻し、バッチの `intents` として返す。状態ファイルの骨格 (全ステージ行・表示属性・走査
/// 結果) を描く材料の正本である。未知の判別子は従来どおり `Corrupt` に落ちる。
///
/// [`Intent`]: core_command_domain::orchestration::Intent
const INTENT_EVENT_MANIFEST: &str = "intent-event/1";

/// 定義ジャーナル行の型判別子 — 同じストアファイルに同居する**第 3 のストリーム**
/// (2026-08-31 のオーナー裁定で `WorkflowDefinition` の Repository がイベントストア形に
/// なったため)。
///
/// この行は**消費する** (b39)。かつては「orchestration の読み面 (`aidlc-state.md` と監査
/// シャード) に定義の確立・改訂は現れない」という理由で読み飛ばしていたが、構造化リード
/// モデル (`read_definition*` 表) が定義の内容を必要とするので、その暫定措置は撤去した
/// (`coding-rules/cqrs-boundaries.md` 規則 3 の 2026-09-02 追記 — 投影核は集約を `replay`
/// で起こしてクエリメソッドを呼ぶ)。復号は [`decode_definition_row`] が行い、結果は
/// バッチの `definitions` として返る。
///
/// **既知の判別子として明示的に振り分ける**ことは変わらない。「知らない判別子は黙って
/// 扱う」にしてしまうと、[`decode_entry`] が守っている「名乗りが違う行の中身は解釈
/// しない」guard が崩れる — 未知の判別子は従来どおり `Corrupt` に落ちる。
const DEFINITION_EVENT_MANIFEST: &str = "workflow-definition-event/1";

/// 定義ジャーナル 1 行を読取レコードへ写す。
///
/// 実行の行 ([`decode_entry`]) / intent の行 ([`decode_intent_row`]) と同じ検査態度である:
/// 名乗り (`manifest`) を照合し、行の `aid` を文法検査でドメイン型にし、payload はこの側の
/// DTO ([`WorkflowDefinitionEventDto`]) で受けてから検査付き再構成でドメインへ写す。
///
/// 定義 id の出所は**行の `aid` 列**である。payload 側も全変種が `aggregate_id` を運ぶので
/// (b40 — イベントはエンティティ)、両者の一致を全変種で検査する。食い違う行はどちらかが
/// 嘘をついており、解釈せず `Corrupt` で止める (intent の行・実行の行と同じ規律)。
fn decode_definition_row(row: &JournalRow) -> Result<DefinitionEntry, JournalReadError> {
    let row_seq = usize::try_from(row.seq_nr)
        .map_err(|_| corrupt_error(&row.aggregate_id, None, CorruptCause::InvariantViolation))?;
    if row.manifest != DEFINITION_EVENT_MANIFEST {
        return Err(corrupt_error(
            &row.aggregate_id,
            Some(row_seq),
            CorruptCause::UndecodablePayload,
        ));
    }
    let definition_id = WorkflowDefinitionId::parse(&row.aggregate_id).map_err(|_| {
        corrupt_error(
            &row.aggregate_id,
            Some(row_seq),
            CorruptCause::InvariantViolation,
        )
    })?;
    let event = serde_json::from_slice::<WorkflowDefinitionEventDto>(&row.payload)
        .map_err(|_| corrupt_error(&row.aggregate_id, None, CorruptCause::UndecodablePayload))?
        .to_domain()
        .map_err(|error| corrupt_error(&row.aggregate_id, Some(row_seq), decode_cause(&error)))?;
    // 行の `aid` と payload の `aggregate_id` を**全変種で**照合する — 食い違う行はどちらかが
    // 嘘をついており、解釈せず `Corrupt` で止める (intent 行・実行の行と同じ規律)。b40 で
    // `Redefined` も系譜 ID を運ぶようになったので、かつての「改訂は照合相手が無いので行の
    // `aid` が正」という片肺は解消した。
    if event.aggregate_id() != &definition_id {
        return Err(corrupt_error(
            &row.aggregate_id,
            Some(row_seq),
            CorruptCause::InvariantViolation,
        ));
    }
    // 誕生は必ず通番 1、改訂は 2 以上である。食い違う行は**復号の境界で**止める — そのまま
    // 通すと通番の飛びが `replay` まで届き、壊れた歴史としてパニックになる (再構成は失敗を
    // 返さない)。破損は復号で `Corrupt` として返すのが本層の役目である (intent 行と同じ)。
    let genesis = matches!(event, WorkflowDefinitionEvent::Defined(_));
    if genesis != (row_seq == 1) {
        return Err(corrupt_error(
            &row.aggregate_id,
            Some(row_seq),
            CorruptCause::InvariantViolation,
        ));
    }
    let global = GlobalSeqNr::new(to_u64(row.rowid, &row.aggregate_id)?);
    Ok(DefinitionEntry::new(
        global,
        definition_id,
        row_seq,
        occurred_at_of(row.occurred_at),
        event,
    ))
}

/// intent ジャーナル 1 行を集約値へ写す。
///
/// 実行の行 ([`decode_entry`]) と同じ検査態度である: 行が名乗る識別子は文法検査を通し、
/// payload はこの側の DTO ([`IntentEventDto`]) で受けてから検査付き再構成でドメインへ
/// 写す。行の名乗り (`aid`) と誕生材料の識別子が食い違う行は、どちらかが噓をついている —
/// 解釈せず `Corrupt` で止める。
fn decode_intent_row(row: &JournalRow) -> Result<Intent, JournalReadError> {
    let row_seq = usize::try_from(row.seq_nr)
        .map_err(|_| corrupt_error(&row.aggregate_id, None, CorruptCause::InvariantViolation))?;
    // intent のイベントは現状 `Created` 1 種 = 必ず genesis (通番 1)。それ以外の通番を名乗る
    // 行は破損した歴史であり、payload を解釈する前に止める (CodeRabbit 指摘)。変種が増えた
    // ときはこの前提ごと見直す (`IntentEventDto` の網羅がビルドで教える)。
    if row_seq != 1 {
        return Err(corrupt_error(
            &row.aggregate_id,
            Some(row_seq),
            CorruptCause::InvariantViolation,
        ));
    }
    let intent_id = IntentId::parse(&row.aggregate_id).map_err(|_| {
        corrupt_error(
            &row.aggregate_id,
            Some(row_seq),
            CorruptCause::InvariantViolation,
        )
    })?;
    let intent = serde_json::from_slice::<IntentEventDto>(&row.payload)
        .map_err(|_| corrupt_error(&row.aggregate_id, None, CorruptCause::UndecodablePayload))?
        .to_domain()
        .map_err(|error| corrupt_error(&row.aggregate_id, Some(row_seq), decode_cause(&error)))?;
    // `Intent::from((Created, _))` は `aggregate_id` をそのまま集約 id にするので、集約の id を
    // 見ることが payload の `aggregate_id` の照合である (b40 の一般形)。
    if intent.id() != &intent_id {
        return Err(corrupt_error(
            &row.aggregate_id,
            Some(row_seq),
            CorruptCause::InvariantViolation,
        ));
    }
    Ok(intent)
}

/// 復号の失敗を `Corrupt` の原因へ写す。
const fn decode_cause(error: &DtoDecodeError) -> CorruptCause {
    match error {
        DtoDecodeError::Malformed { .. } => CorruptCause::UndecodablePayload,
        DtoDecodeError::InvariantViolation => CorruptCause::InvariantViolation,
    }
}

fn decode_entry(row: &JournalRow) -> Result<JournalEntry, JournalReadError> {
    let row_seq = usize::try_from(row.seq_nr)
        .map_err(|_| corrupt_error(&row.aggregate_id, None, CorruptCause::InvariantViolation))?;
    let execution_id = IntentExecutionId::parse(&row.aggregate_id).map_err(|_| {
        corrupt_error(
            &row.aggregate_id,
            Some(row_seq),
            CorruptCause::InvariantViolation,
        )
    })?;
    if row.manifest != EVENT_MANIFEST {
        return Err(corrupt_error(
            &row.aggregate_id,
            Some(row_seq),
            CorruptCause::UndecodablePayload,
        ));
    }
    // 行のバイトは**この側の DTO** で受けてからドメインイベントへ写す
    // (`coding-rules/cqrs-boundaries.md` — 側ごと専用化)。
    let event = serde_json::from_slice::<IntentExecutionEventDto>(&row.payload)
        .map_err(|_| corrupt_error(&row.aggregate_id, None, CorruptCause::UndecodablePayload))?
        .to_domain()
        .map_err(|error| corrupt_error(&row.aggregate_id, Some(row_seq), decode_cause(&error)))?;
    // 行の `aid` と payload の `aggregate_id` を**全変種で**照合する — 食い違う行はどちらかが
    // 嘘をついている。解釈せず止める (intent 行・定義行と同じ規律)。b40 で全変種が
    // `aggregate_id` を運ぶようになったので、かつて genesis だけだった照合が全変種に広がった。
    if event.aggregate_id() != &execution_id {
        return Err(corrupt_error(
            &row.aggregate_id,
            Some(row_seq),
            CorruptCause::InvariantViolation,
        ));
    }
    // 誕生 (`Started`) は必ず通番 1、以降のイベントは 2 以上である。食い違う行は**復号の
    // 境界で**止める — 通番の飛びをそのまま通すと `replay` まで届き、壊れた歴史として
    // パニックになる (再構成は失敗を返さない)。破損は `Corrupt` として返すのが本層の
    // 役目である (定義行・intent 行と同じ検査態度)。
    let genesis = matches!(event, IntentExecutionEvent::Started(_));
    if genesis != (row_seq == 1) {
        return Err(corrupt_error(
            &row.aggregate_id,
            Some(row_seq),
            CorruptCause::InvariantViolation,
        ));
    }
    let global = GlobalSeqNr::new(to_u64(row.rowid, &row.aggregate_id)?);
    Ok(JournalEntry::new(
        global,
        execution_id,
        row_seq,
        occurred_at_of(row.occurred_at),
        event,
    ))
}

/// `occurred_at` 列 (epoch ナノ秒) をドメイン供給値へ戻す。
///
/// 本家 v3 は `timestamp_nanos_opt` で書き `DateTime::from_timestamp_nanos` で戻すので、
/// 表現可能な範囲 (およそ 1677〜2262 年) では往復が完全である。
const fn occurred_at_of(nanos: i64) -> DateTime<Utc> {
    DateTime::from_timestamp_nanos(nanos)
}

impl JournalReader for JournalReaderImpl {
    async fn pending_publication(
        &self,
        projection: &ProjectionName,
    ) -> Result<Option<super::PublicationBatch>, JournalReadError> {
        super::publication_store::pending(&self.connection, self.path.as_path(), projection)
    }

    async fn events_through(&self, to: GlobalSeqNr) -> Result<JournalBatch, JournalReadError> {
        Self::scan_range(
            &self.connection,
            self.path.as_path(),
            GlobalSeqNr::ZERO,
            Some(to),
        )
    }

    async fn publish(
        &mut self,
        projection: &ProjectionName,
        batch: &super::PublicationBatch,
        tables: &ReadTables,
    ) -> Result<(), super::CatchUpError> {
        super::publication_store::publish(
            &mut self.connection,
            self.path.as_path(),
            projection,
            batch,
            tables,
        )
    }

    async fn events_after(&self, after: GlobalSeqNr) -> Result<JournalBatch, JournalReadError> {
        JournalReaderImpl::scan_from(&self.connection, self.path.as_path(), after)
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
        tables: &ReadTables,
    ) -> Result<(), JournalReadError> {
        let path = self.path.clone();
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| map_sqlite_error(&e, path.as_path()))?;
        Self::advance_on(&transaction, path.as_path(), projection, to, tables)?;
        transaction
            .commit()
            .map_err(|e| map_sqlite_error(&e, path.as_path()))
    }

    async fn steering_source_digest(&self) -> Result<Option<String>, JournalReadError> {
        self.connection
            .query_row(SELECT_STEERING_SOURCE, [], |row| row.get(0))
            .optional()
            .map_err(|error| map_sqlite_error(&error, self.path.as_path()))
    }

    async fn replace_steering(&mut self, tables: &SteeringTables) -> Result<(), JournalReadError> {
        let path = self.path.clone();
        // 読み取ってから書くので `BEGIN IMMEDIATE` で書込ロックを最初に取る (BR2.3)。
        // チェックポイントの前進とは**別の Tx** である — 参照入力はジャーナルの走査位置と
        // 無関係に変わるので束ねる理由が無く、束ねると規則を 1 文字直すたびにチェック
        // ポイントの書込ロックを取り合うことになる。
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| map_sqlite_error(&error, path.as_path()))?;
        replace_steering(&transaction, tables)
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
    use core_command_domain::orchestration::{
        IntentEventId, IntentExecutionEvent, IntentExecutionEventId, Unparked,
    };

    /// 投影チェックポイントの表 (**我々の表**。本家の `journal` / `snapshot` と衝突しない)。
    const CHECKPOINT_TABLE: &str = "amadeus_projection_checkpoint";
    use core_command_domain::workspace::SpaceName;
    use event_store_adapter_rs::EventStoreForSqlite;
    use event_store_adapter_rs::types::AggregateId;

    /// b40 のテスト用固定イベント識別子 (同じ材料から組んだイベントを同値に保つため)。
    fn event_id() -> IntentExecutionEventId {
        IntentExecutionEventId::parse("0191aaaa-bbbb-7ccc-9ddd-eeeeffff0002").expect("UUIDv7")
    }

    /// b40 のテスト用集約識別子 (行の `aid` と payload の `aggregate_id` を揃える)。
    fn execution_id() -> IntentExecutionId {
        IntentExecutionId::parse("01a02785-1bd8-76eb-aeea-5aa303ebd5b6").expect("実行 id")
    }

    /// b40 のテスト用固定イベント識別子 (intent 面)。
    fn intent_event_id() -> IntentEventId {
        IntentEventId::parse("0191aaaa-bbbb-7ccc-9ddd-eeeeffff0001").expect("UUIDv7")
    }

    /// 本家 `AggregateId` を満たすテスト用のストア鍵。
    ///
    /// RMU の本番経路は `rusqlite` で `journal` 表を直接読むので本家ストアには触れない。
    /// ここで本家ストアを使うのは**表を実物の DDL で作らせるため**だけなので、鍵も payload も
    /// 型境界を満たせば足りる。
    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    struct StoreKey(String);

    impl std::fmt::Display for StoreKey {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str(&self.0)
        }
    }

    impl AggregateId for StoreKey {
        fn type_name(&self) -> String {
            "IntentExecution".to_string()
        }

        fn value(&self) -> String {
            self.0.clone()
        }
    }

    /// 本家の SQLite ストア (表を作らせるためだけに開く — 行は `rusqlite` で直接入れる)。
    type UpstreamStore = EventStoreForSqlite<StoreKey, serde_json::Value, serde_json::Value>;

    #[test]
    fn the_store_key_reports_the_aggregate_type_name_and_the_raw_value() {
        // 本家がパーティション鍵を組む材料である。表を実物の DDL で作らせるために要る。
        const RAW: &str = "0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000";
        let key = StoreKey(RAW.to_string());
        assert_eq!(key.type_name(), "IntentExecution");
        assert_eq!(key.value(), RAW);
        assert_eq!(key.to_string(), RAW);
    }

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

    /// **本家 v3.0.0 の `journal` スキーマ (ピン留め)。**
    ///
    /// `rowid` をカーソルに使ってよい根拠そのものである — 列構成が変わったり、
    /// `WITHOUT ROWID` になったり、削除経路が増えたりしたら前提が崩れる。
    /// v3 で `manifest TEXT NOT NULL DEFAULT ''` が増え、`occurred_at` はナノ秒になった。
    const PINNED_JOURNAL_DDL: &str = "CREATE TABLE journal (\n  \
        pkey TEXT NOT NULL,\n  \
        skey TEXT NOT NULL,\n  \
        aid TEXT NOT NULL,\n  \
        seq_nr INTEGER NOT NULL,\n  \
        payload BLOB NOT NULL,\n  \
        occurred_at INTEGER NOT NULL,\n  \
        manifest TEXT NOT NULL DEFAULT '',\n  \
        PRIMARY KEY (pkey, skey)\n)";

    /// 同じくピン留めした `journal` の一意索引。
    const PINNED_JOURNAL_INDEX_DDL: &str =
        "CREATE UNIQUE INDEX journal_aid_seq_nr_idx ON journal (aid, seq_nr)";

    #[test]
    fn the_upstream_journal_schema_is_the_pinned_one() {
        // スキーマガード (ADR-010 決定 4)。ここが落ちたら**本家スキーマが変わった**という
        // ことなので、`=3.0.0` の固定と `rowid` をカーソルに使う前提を見直すこと。
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
            "本家スキーマが変わった。event-store-adapter-rs の =3.0.0 固定を見直せ"
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
            "本家スキーマが変わった。event-store-adapter-rs の =3.0.0 固定を見直せ"
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
    fn opening_creates_our_tables_next_to_the_upstream_ones() {
        // 同じ DB ファイルに 3 種の表が同居する: 本家の 2 つ (`journal` / `snapshot`)、
        // 我々のチェックポイント表、そして構造化リードモデルの 17 表 (`read_` 接頭)。
        // 名前が衝突しないことが同居の前提なので、集合そのものを固定する。
        let dir = tempfile::tempdir().expect("一時 dir");
        let (_store, path) = opened_store(&dir);
        let journal_reader = JournalReaderImpl::open(&path).expect("開ける");
        assert_eq!(journal_reader.path(), &path);

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

        let upstream: Vec<&str> = tables
            .iter()
            .map(String::as_str)
            .filter(|name| {
                !name.starts_with("read_")
                    && *name != CHECKPOINT_TABLE
                    && *name != "amadeus_publication"
                    && *name != "amadeus_publication_file"
                    && *name != "amadeus_publication_history"
                    && *name != "amadeus_publication_history_file"
                    && *name != "amadeus_publication_snapshot"
                    && *name != "amadeus_publication_snapshot_file"
                    && *name != "amadeus_read_model_head"
            })
            .collect();
        assert_eq!(upstream, ["journal", "snapshot"], "本家の表は 2 つだけ");
        for name in [
            "amadeus_publication",
            "amadeus_publication_file",
            "amadeus_publication_history",
            "amadeus_publication_history_file",
            "amadeus_publication_snapshot",
            "amadeus_publication_snapshot_file",
            "amadeus_read_model_head",
        ] {
            assert!(
                tables.iter().any(|table| table == name),
                "公開計画の表がある: {name}"
            );
        }
        assert!(
            tables.iter().any(|name| name == CHECKPOINT_TABLE),
            "チェックポイント表がある"
        );
        assert_eq!(
            tables
                .iter()
                .filter(|name| name.starts_with("read_"))
                .count(),
            17,
            "構造化リードモデルは 17 表 (ジャーナル由来 15 + 参照入力由来 2)"
        );
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
        let mut journal_reader = JournalReaderImpl::open(&path).expect("開ける");
        assert!(
            journal_reader
                .events_after(GlobalSeqNr::new(u64::MAX))
                .await
                .is_err()
        );
        assert!(
            journal_reader
                .advance_checkpoint(&projection(), GlobalSeqNr::new(u64::MAX), &empty_tables())
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn a_missing_journal_table_is_reported_as_io() {
        let dir = tempfile::tempdir().expect("一時 dir");
        let (_store, path) = opened_store(&dir);
        let journal_reader = JournalReaderImpl::open(&path).expect("開ける");
        raw(&path)
            .execute_batch("DROP TABLE journal")
            .expect("表を落とす");

        let error = journal_reader
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
        let mut journal_reader = JournalReaderImpl::open(&path).expect("開ける");
        raw(&path)
            .execute_batch("DROP TABLE amadeus_projection_checkpoint")
            .expect("表を落とす");

        let error = journal_reader
            .checkpoint(&projection())
            .await
            .expect_err("表が無い");
        assert_eq!(
            error,
            JournalReadError::Io {
                kind: ErrorKind::Other,
                path: Some(path.as_path().to_path_buf()),
            }
        );
        let error = journal_reader
            .advance_checkpoint(&projection(), GlobalSeqNr::new(1), &empty_tables())
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
    async fn a_positive_checkpoint_without_an_anchor_is_a_mismatch() {
        // 正のチェックポイントには advance が必ずアンカーを書く。欠けた行は直接改変の兆候。
        let dir = tempfile::tempdir().expect("一時 dir");
        let (_store, path) = opened_store(&dir);
        let journal_reader = JournalReaderImpl::open(&path).expect("開ける");
        raw(&path)
            .execute(
                "INSERT INTO amadeus_projection_checkpoint(projection, last_global_seq)
                 VALUES ('state-file', 3)",
                [],
            )
            .expect("アンカー無しの正値を置く");

        let error = journal_reader
            .checkpoint(&projection())
            .await
            .expect_err("照合できない");
        assert_eq!(
            error,
            JournalReadError::Corrupt {
                aggregate_id: "-".to_string(),
                seq_nr: None,
                cause: CorruptCause::CheckpointAnchorMismatch,
            }
        );
    }

    #[tokio::test]
    async fn a_negative_anchor_seq_nr_is_corrupt() {
        let dir = tempfile::tempdir().expect("一時 dir");
        let (_store, path) = opened_store(&dir);
        let journal_reader = JournalReaderImpl::open(&path).expect("開ける");
        raw(&path)
            .execute(
                "INSERT INTO amadeus_projection_checkpoint(projection, last_global_seq, anchor_aid, anchor_seq_nr)
                 VALUES ('state-file', 3, 'intent-x', -5)",
                [],
            )
            .expect("負のアンカーを置く");

        let error = journal_reader
            .checkpoint(&projection())
            .await
            .expect_err("負の通番は無い");
        assert_eq!(
            error,
            JournalReadError::Corrupt {
                aggregate_id: "intent-x".to_string(),
                seq_nr: None,
                cause: CorruptCause::InvariantViolation,
            }
        );
    }

    #[tokio::test]
    async fn advancing_to_zero_writes_a_row_without_an_anchor_and_reads_back_zero() {
        // ZERO は「まだ何も投影していない」の明示登録 — journal に対応行が無いので
        // アンカーも無し。読み返しは照合をスキップして ZERO を返す。
        let dir = tempfile::tempdir().expect("一時 dir");
        let (_store, path) = opened_store(&dir);
        let mut journal_reader = JournalReaderImpl::open(&path).expect("開ける");

        journal_reader
            .advance_checkpoint(&projection(), GlobalSeqNr::ZERO, &empty_tables())
            .await
            .expect("ZERO への前進は通る");
        let saved = journal_reader
            .checkpoint(&projection())
            .await
            .expect("読める");
        assert_eq!(saved, GlobalSeqNr::ZERO);
    }

    #[tokio::test]
    async fn advancing_to_a_position_not_in_the_journal_is_refused() {
        // journal に無い位置へ進めると以後の照合が必ず失敗するため、前進の時点で止める。
        let dir = tempfile::tempdir().expect("一時 dir");
        let (_store, path) = opened_store(&dir);
        let mut journal_reader = JournalReaderImpl::open(&path).expect("開ける");

        let error = journal_reader
            .advance_checkpoint(&projection(), GlobalSeqNr::new(1), &empty_tables())
            .await
            .expect_err("空のジャーナルに位置 1 は無い");
        assert_eq!(
            error,
            JournalReadError::Corrupt {
                aggregate_id: "-".to_string(),
                seq_nr: None,
                cause: CorruptCause::CheckpointAnchorMismatch,
            }
        );
    }

    #[tokio::test]
    async fn a_negative_checkpoint_row_is_corrupt() {
        let dir = tempfile::tempdir().expect("一時 dir");
        let (_store, path) = opened_store(&dir);
        let journal_reader = JournalReaderImpl::open(&path).expect("開ける");
        raw(&path)
            .execute(
                "INSERT INTO amadeus_projection_checkpoint(projection, last_global_seq)
                 VALUES ('state-file', -1)",
                [],
            )
            .expect("負値を置く");

        let error = journal_reader
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
    async fn a_checkpoint_row_whose_anchor_aid_is_not_text_is_reported_as_io() {
        let dir = tempfile::tempdir().expect("一時 dir");
        let (_store, path) = opened_store(&dir);
        let journal_reader = JournalReaderImpl::open(&path).expect("開ける");
        raw(&path)
            .execute(
                "INSERT INTO amadeus_projection_checkpoint(projection, last_global_seq, anchor_aid, anchor_seq_nr)
                 VALUES ('state-file', 3, X'FF', 3)",
                [],
            )
            .expect("UTF-8 でないアンカーを置く");

        let error = journal_reader
            .checkpoint(&projection())
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
    async fn a_journal_row_whose_aid_is_not_text_fails_anchor_verification_as_io() {
        let dir = tempfile::tempdir().expect("一時 dir");
        let (_store, path) = opened_store(&dir);
        let journal_reader = JournalReaderImpl::open(&path).expect("開ける");
        let conn = raw(&path);
        conn.execute(
            "INSERT INTO journal(pkey, skey, aid, seq_nr, payload, occurred_at)
             VALUES ('p', 's', X'FF', 1, X'7B7D', 0)",
            [],
        )
        .expect("UTF-8 でない aid の行を置く");
        conn.execute(
            "INSERT INTO amadeus_projection_checkpoint(projection, last_global_seq, anchor_aid, anchor_seq_nr)
             VALUES ('state-file', 1, 'intent-x', 1)",
            [],
        )
        .expect("正のチェックポイントを置く");

        let error = journal_reader
            .checkpoint(&projection())
            .await
            .expect_err("照合先の列を読めない");
        assert_eq!(
            error,
            JournalReadError::Io {
                kind: ErrorKind::Other,
                path: Some(path.as_path().to_path_buf()),
            }
        );
    }

    #[tokio::test]
    async fn advancing_over_a_journal_row_whose_aid_is_not_text_is_reported_as_io() {
        let dir = tempfile::tempdir().expect("一時 dir");
        let (_store, path) = opened_store(&dir);
        let mut journal_reader = JournalReaderImpl::open(&path).expect("開ける");
        raw(&path)
            .execute(
                "INSERT INTO journal(pkey, skey, aid, seq_nr, payload, occurred_at)
                 VALUES ('p', 's', X'FF', 1, X'7B7D', 0)",
                [],
            )
            .expect("UTF-8 でない aid の行を置く");

        let error = journal_reader
            .advance_checkpoint(&projection(), GlobalSeqNr::new(1), &empty_tables())
            .await
            .expect_err("アンカー列を読めない");
        assert_eq!(
            error,
            JournalReadError::Io {
                kind: ErrorKind::Other,
                path: Some(path.as_path().to_path_buf()),
            }
        );
    }

    #[tokio::test]
    async fn a_checkpoint_row_whose_value_is_not_an_integer_is_reported_as_io() {
        let dir = tempfile::tempdir().expect("一時 dir");
        let (_store, path) = opened_store(&dir);
        let journal_reader = JournalReaderImpl::open(&path).expect("開ける");
        raw(&path)
            .execute(
                "INSERT INTO amadeus_projection_checkpoint(projection, last_global_seq)
                 VALUES ('state-file', 'x')",
                [],
            )
            .expect("整数でないチェックポイント値を置く");

        let error = journal_reader
            .checkpoint(&projection())
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
    async fn a_checkpoint_row_whose_anchor_seq_nr_is_not_an_integer_is_reported_as_io() {
        let dir = tempfile::tempdir().expect("一時 dir");
        let (_store, path) = opened_store(&dir);
        let journal_reader = JournalReaderImpl::open(&path).expect("開ける");
        raw(&path)
            .execute(
                "INSERT INTO amadeus_projection_checkpoint(projection, last_global_seq, anchor_aid, anchor_seq_nr)
                 VALUES ('state-file', 3, 'intent-x', 'not-a-number')",
                [],
            )
            .expect("整数でないアンカー通番を置く");

        let error = journal_reader
            .checkpoint(&projection())
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
    async fn a_journal_row_whose_seq_nr_is_not_an_integer_fails_anchor_verification_as_io() {
        let dir = tempfile::tempdir().expect("一時 dir");
        let (_store, path) = opened_store(&dir);
        let journal_reader = JournalReaderImpl::open(&path).expect("開ける");
        let conn = raw(&path);
        conn.execute(
            "INSERT INTO journal(pkey, skey, aid, seq_nr, payload, occurred_at)
             VALUES ('p', 's', 'intent-x', 'x', X'7B7D', 0)",
            [],
        )
        .expect("整数でない seq_nr の行を置く");
        conn.execute(
            "INSERT INTO amadeus_projection_checkpoint(projection, last_global_seq, anchor_aid, anchor_seq_nr)
             VALUES ('state-file', 1, 'intent-x', 1)",
            [],
        )
        .expect("正のチェックポイントを置く");

        let error = journal_reader
            .checkpoint(&projection())
            .await
            .expect_err("照合先の列を読めない");
        assert_eq!(
            error,
            JournalReadError::Io {
                kind: ErrorKind::Other,
                path: Some(path.as_path().to_path_buf()),
            }
        );
    }

    #[tokio::test]
    async fn advancing_over_a_journal_row_whose_seq_nr_is_not_an_integer_is_reported_as_io() {
        let dir = tempfile::tempdir().expect("一時 dir");
        let (_store, path) = opened_store(&dir);
        let mut journal_reader = JournalReaderImpl::open(&path).expect("開ける");
        raw(&path)
            .execute(
                "INSERT INTO journal(pkey, skey, aid, seq_nr, payload, occurred_at)
                 VALUES ('p', 's', 'intent-x', 'x', X'7B7D', 0)",
                [],
            )
            .expect("整数でない seq_nr の行を置く");

        let error = journal_reader
            .advance_checkpoint(&projection(), GlobalSeqNr::new(1), &empty_tables())
            .await
            .expect_err("アンカー列を読めない");
        assert_eq!(
            error,
            JournalReadError::Io {
                kind: ErrorKind::Other,
                path: Some(path.as_path().to_path_buf()),
            }
        );
    }

    #[tokio::test]
    async fn a_row_whose_seq_nr_is_not_an_integer_is_reported_as_io() {
        let dir = tempfile::tempdir().expect("一時 dir");
        let (_store, path) = opened_store(&dir);
        let journal_reader = JournalReaderImpl::open(&path).expect("開ける");
        raw(&path)
            .execute(
                "INSERT INTO journal(pkey, skey, aid, seq_nr, payload, occurred_at)
                 VALUES ('p', 's', 'intent-x', 'x', X'7B7D', 0)",
                [],
            )
            .expect("整数でない seq_nr の行を置く");

        let error = journal_reader
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
    async fn a_failing_checkpoint_write_is_reported_as_io() {
        // UPSERT 自体の失敗経路。トリガで書込を落とし、握り潰されないことを確かめる。
        let dir = tempfile::tempdir().expect("一時 dir");
        let (_store, path) = opened_store(&dir);
        let mut journal_reader = JournalReaderImpl::open(&path).expect("開ける");
        let conn = raw(&path);
        conn.execute(
            "INSERT INTO journal(pkey, skey, aid, seq_nr, payload, occurred_at)
             VALUES ('p', 's', 'intent-x', 1, X'7B7D', 0)",
            [],
        )
        .expect("前進先の行を置く");
        conn.execute_batch(
            "CREATE TRIGGER checkpoint_write_fails
             BEFORE INSERT ON amadeus_projection_checkpoint
             BEGIN SELECT RAISE(ABORT, 'boom'); END",
        )
        .expect("書込を落とすトリガを置く");
        drop(conn);

        let error = journal_reader
            .advance_checkpoint(&projection(), GlobalSeqNr::new(1), &empty_tables())
            .await
            .expect_err("書込が落ちる");
        assert_eq!(
            error,
            JournalReadError::Io {
                kind: ErrorKind::Other,
                path: Some(path.as_path().to_path_buf()),
            }
        );
    }

    #[tokio::test]
    async fn a_row_whose_aggregate_id_is_not_text_is_reported_as_io() {
        let dir = tempfile::tempdir().expect("一時 dir");
        let (_store, path) = opened_store(&dir);
        let journal_reader = JournalReaderImpl::open(&path).expect("開ける");
        raw(&path)
            .execute(
                "INSERT INTO journal(pkey, skey, aid, seq_nr, payload, occurred_at)
                 VALUES ('p', 's', X'FF', 1, X'7B7D', 0)",
                [],
            )
            .expect("UTF-8 でない aid を置く");

        let error = journal_reader
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
        let journal_reader = JournalReaderImpl::open(&path).expect("開ける");
        raw(&path)
            .execute(
                "INSERT INTO journal(pkey, skey, aid, seq_nr, payload, occurred_at)
                 VALUES ('p', 's', 'agg', 1, 42, 0)",
                [],
            )
            .expect("BLOB でない payload を置く");

        let error = journal_reader
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
        let mut journal_reader =
            JournalReaderImpl::open_with_busy_timeout(&path, Duration::from_millis(20))
                .expect("開ける");

        let holder = raw(&path);
        holder
            .execute_batch("BEGIN EXCLUSIVE")
            .expect("書込ロックを握る");

        let error = journal_reader
            .advance_checkpoint(&projection(), GlobalSeqNr::new(1), &empty_tables())
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

    /// 前進と一緒に渡す構造化リードモデル。
    ///
    /// 単調性・アンカー照合を見る試験は行の中身に依存しないので、空の履歴からの投影
    /// (= 全表 0 行) で足りる。行の往復そのものは `journal_reader_impl_test.rs` が見る。
    fn empty_tables() -> ReadTables {
        ReadTables::project(&JournalBatch::empty()).expect("空も投影できる")
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

    /// 本家シリアライザと同じ形式の payload バイト列。
    fn payload_bytes() -> Vec<u8> {
        #[allow(
            clippy::disallowed_methods,
            reason = "本家シリアライザと同形式のフィクスチャ生成 (BR1.7 の射程外)"
        )]
        serde_json::to_vec(&IntentExecutionEventDto::of(
            &IntentExecutionEvent::Unparked(Unparked::new(event_id(), execution_id())),
        ))
        .unwrap()
    }

    /// 正常な 1 行 (個々のフィールドを崩して境界を踏むための素体)。
    ///
    /// 通番が 2 なのは payload が誕生イベントでないからである — 誕生 (`Started`) は通番 1
    /// でしか現れず、`Unparked` が通番 1 を名乗る行は復号の境界で破損として止まる。
    fn sound_row() -> JournalRow {
        JournalRow {
            rowid: 1,
            seq_nr: 2,
            aggregate_id: "01a02785-1bd8-76eb-aeea-5aa303ebd5b6".to_string(),
            payload: payload_bytes(),
            occurred_at: 1_756_425_600_000_000_000,
            manifest: EVENT_MANIFEST.to_string(),
        }
    }

    #[expect(
        clippy::disallowed_methods,
        reason = "契約 JSON ではなく行のバイトそのものを組む (BR1.7 の射程外)"
    )]
    #[test]
    fn a_row_that_decodes_but_cannot_be_carried_into_the_domain_is_corrupt() {
        // JSON としては読めて DTO にもなるが、閉集合外の綴りを名乗る行。ドメインへ写す時点で
        // 止まるので、壊れた値が投影核に流れ込まない。
        let tampered = String::from_utf8(
            serde_json::to_vec(&IntentExecutionEventDto::of(&IntentExecutionEvent::Parked(
                core_command_domain::orchestration::Parked::new(
                    event_id(),
                    execution_id(),
                    core_command_domain::workflow_definition::StageSlug::parse("intent-capture")
                        .expect("slug"),
                ),
            )))
            .unwrap(),
        )
        .unwrap()
        .replace(r#""intent-capture""#, r#""Not A Slug""#);
        let row = JournalRow {
            payload: tampered.into_bytes(),
            ..sound_row()
        };
        assert_eq!(
            decode_entry(&row).expect_err("閉集合外の綴り"),
            JournalReadError::Corrupt {
                aggregate_id: "01a02785-1bd8-76eb-aeea-5aa303ebd5b6".to_string(),
                seq_nr: Some(2),
                cause: CorruptCause::UndecodablePayload,
            }
        );
    }

    #[test]
    fn a_sound_row_becomes_a_journal_entry_with_every_material() {
        let entry = decode_entry(&sound_row()).expect("読める行");
        assert_eq!(entry.global_seq(), GlobalSeqNr::new(1));
        assert_eq!(
            entry.execution_id().as_str(),
            "01a02785-1bd8-76eb-aeea-5aa303ebd5b6"
        );
        assert_eq!(entry.seq_nr(), 2);
        assert_eq!(
            entry.event(),
            &IntentExecutionEvent::Unparked(Unparked::new(event_id(), execution_id()))
        );
        assert_eq!(
            entry.occurred_at().timestamp_nanos_opt(),
            Some(1_756_425_600_000_000_000),
            "発生時刻はナノ秒のまま往復する"
        );
    }

    #[test]
    fn a_row_whose_aggregate_id_is_not_an_intent_id_is_corrupt() {
        // 列は TEXT なので、行が名乗る識別子が `IntentExecutionId` として妥当とは限らない。
        // 我々の型へ写せない行はここで止める (旧 #500 の payload 内メタ照合の後継 —
        // payload からメタが消えたので、照合の相手は列そのものになった)。
        let row = JournalRow {
            aggregate_id: "not-a-uuid-v7".to_string(),
            ..sound_row()
        };
        assert_eq!(
            decode_entry(&row).expect_err("IntentExecutionId にならない"),
            JournalReadError::Corrupt {
                aggregate_id: "not-a-uuid-v7".to_string(),
                seq_nr: Some(2),
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
        assert_eq!(
            error,
            JournalReadError::Io {
                kind: ErrorKind::NotFound,
                path: Some(path.as_path().to_path_buf()),
            },
            "表の不在は NotFound"
        );
    }

    #[test]
    fn opening_a_missing_store_does_not_create_the_file() {
        // 読取側の接続はストアファイルを作らない (B6 CodeRabbit #511)。
        let dir = tempfile::tempdir().unwrap();
        let path = StorePath::for_space(&dir.path().join("aidlc"), &SpaceName::default());
        std::fs::create_dir_all(path.as_path().parent().unwrap()).unwrap();
        let error = JournalReaderImpl::open(&path).expect_err("無いストアは開けない");
        assert_eq!(
            error,
            JournalReadError::Io {
                kind: ErrorKind::NotFound,
                path: Some(path.as_path().to_path_buf()),
            }
        );
        assert!(!path.as_path().exists(), "空の SQLite ファイルを作らない");
    }

    #[test]
    fn a_row_with_a_negative_sequence_number_is_corrupt() {
        // journal.seq_nr は本家スキーマ上 INTEGER — 負値は書込経路からは生まれないが、
        // 破損検出の境界なので usize への写しの失敗も Corrupt に畳む。
        let row = JournalRow {
            seq_nr: -1,
            ..sound_row()
        };
        assert_eq!(
            decode_entry(&row).expect_err("負の通番"),
            JournalReadError::Corrupt {
                aggregate_id: "01a02785-1bd8-76eb-aeea-5aa303ebd5b6".to_string(),
                seq_nr: None,
                cause: CorruptCause::InvariantViolation,
            }
        );
    }

    #[test]
    fn a_row_whose_manifest_is_not_ours_is_corrupt() {
        // manifest は payload の型と読み方の版を名乗る列。名乗りが違う行の中身は解釈しない
        // (旧 `schema_version` 検査 (#466) の後継)。
        for foreign in ["", "intent-execution-event/2", "some-other-type/1"] {
            let row = JournalRow {
                manifest: foreign.to_string(),
                ..sound_row()
            };
            assert_eq!(
                decode_entry(&row).expect_err("名乗りが違う"),
                JournalReadError::Corrupt {
                    aggregate_id: "01a02785-1bd8-76eb-aeea-5aa303ebd5b6".to_string(),
                    seq_nr: Some(2),
                    cause: CorruptCause::UndecodablePayload,
                },
                "manifest = {foreign:?}"
            );
        }
    }

    #[tokio::test]
    async fn a_definition_row_that_cannot_be_decoded_stops_the_read() {
        // 定義のジャーナルは同じストアファイルに同居し、いまは**消費する** (b39)。
        // 復号できない payload は飛ばさない — 中身を解釈できないまま先へ進めない。
        let dir = tempfile::tempdir().expect("一時 dir");
        let (_store, path) = opened_store(&dir);
        let journal_reader = JournalReaderImpl::open(&path).expect("開ける");
        let connection = raw(&path);
        connection
            .execute(
                "INSERT INTO journal(pkey, skey, aid, seq_nr, payload, occurred_at, manifest)
                 VALUES ('p', 's', 'claude', 1, X'7B7D', 0, ?1)",
                params![DEFINITION_EVENT_MANIFEST],
            )
            .expect("空オブジェクトの定義行を置く");

        assert_eq!(
            journal_reader
                .events_after(GlobalSeqNr::ZERO)
                .await
                .expect_err("`{}` は定義イベントとして読めない"),
            JournalReadError::Corrupt {
                aggregate_id: "claude".to_string(),
                seq_nr: None,
                cause: CorruptCause::UndecodablePayload,
            }
        );

        // 未知の判別子も従来どおり落ちる — 名乗りが違う行の中身は解釈しない。
        connection
            .execute("DELETE FROM journal", [])
            .expect("行を片付ける");
        connection
            .execute(
                "INSERT INTO journal(pkey, skey, aid, seq_nr, payload, occurred_at, manifest)
                 VALUES ('p2', 's2', 'claude', 2, X'7B7D', 0, 'some-other-type/1')",
                [],
            )
            .expect("未知の判別子の行を置く");
        assert!(matches!(
            journal_reader
                .events_after(GlobalSeqNr::ZERO)
                .await
                .expect_err("未知の判別子は落ちる"),
            JournalReadError::Corrupt { .. }
        ));
    }

    #[test]
    fn a_payload_that_is_not_an_event_is_corrupt() {
        let row = JournalRow {
            payload: b"{not json".to_vec(),
            ..sound_row()
        };
        assert_eq!(
            decode_entry(&row).expect_err("復号できない"),
            JournalReadError::Corrupt {
                aggregate_id: "01a02785-1bd8-76eb-aeea-5aa303ebd5b6".to_string(),
                seq_nr: None,
                cause: CorruptCause::UndecodablePayload,
            }
        );
    }

    #[test]
    fn a_row_whose_rowid_is_negative_is_corrupt() {
        // rowid は問い合わせ上 0 以上しか返らないが、u64 への写しの失敗を静かに丸めない。
        let row = JournalRow {
            rowid: -1,
            ..sound_row()
        };
        assert_eq!(
            decode_entry(&row).expect_err("負の rowid"),
            JournalReadError::Corrupt {
                aggregate_id: "01a02785-1bd8-76eb-aeea-5aa303ebd5b6".to_string(),
                seq_nr: None,
                cause: CorruptCause::InvariantViolation,
            }
        );
    }

    /// intent 行のフィクスチャ (誕生の材料 = 検査付き再構成で戻る集約値)。
    fn birth_created() -> core_command_domain::orchestration::Created {
        use core_command_domain::orchestration::{
            Created, IntentId, StageDisplay, StageEntry, StartRequest, WorkspaceScan,
        };
        use core_command_domain::workflow_definition::{
            BrownfieldGreenfield, DefinitionRevision, PhaseId, PlanAction, StageNumber, StageSlug,
            WorkflowDefinitionId,
        };
        Created::new(
            intent_event_id(),
            IntentId::parse("01a02785-1bd8-76eb-aeea-5aa303ebd5b6").unwrap(),
            WorkflowDefinitionId::parse("claude").unwrap(),
            DefinitionRevision::parse(&format!("sha256:{}", "0".repeat(64))).unwrap(),
            StartRequest::new("classic", "unit"),
            vec![StageEntry::new(
                StageSlug::parse("state-init").unwrap(),
                PhaseId::Initialization,
                PlanAction::Execute,
                false,
                StageDisplay::new(
                    StageNumber::parse("0.1").unwrap(),
                    "State Init",
                    "orchestrator",
                )
                .unwrap(),
            )],
            WorkspaceScan::new(
                BrownfieldGreenfield::Greenfield,
                "Unknown",
                "Unknown",
                "Unknown",
            )
            .unwrap(),
        )
    }

    /// 誕生イベント (行を組む材料)。
    fn birth_event() -> core_command_domain::orchestration::IntentEvent {
        core_command_domain::orchestration::IntentEvent::Created(birth_created())
    }

    /// 誕生記録から起こした集約 (復号結果との突き合わせ用)。
    fn birth_intent() -> core_command_domain::orchestration::Intent {
        core_command_domain::orchestration::Intent::from((birth_created(), birth_at()))
    }

    /// 誕生の発生時刻 (`birth_created` と対で行を組む)。
    fn birth_at() -> chrono::DateTime<chrono::Utc> {
        occurred_at_of(0)
    }

    #[expect(
        clippy::disallowed_methods,
        reason = "本家シリアライザと同形式のフィクスチャ生成 (BR1.7 の射程外)"
    )]
    fn intent_row() -> JournalRow {
        JournalRow {
            rowid: 1,
            seq_nr: 1,
            aggregate_id: "01a02785-1bd8-76eb-aeea-5aa303ebd5b6".to_string(),
            payload: serde_json::to_vec(&IntentEventDto::of(&birth_event(), birth_at())).unwrap(),
            occurred_at: 1_756_425_600_000_000_000,
            manifest: INTENT_EVENT_MANIFEST.to_string(),
        }
    }

    #[test]
    fn an_intent_row_decodes_into_the_birth_material() {
        assert_eq!(decode_intent_row(&intent_row()).unwrap(), birth_intent());
    }

    #[test]
    fn an_intent_row_whose_aid_is_not_an_identifier_is_corrupt() {
        let row = JournalRow {
            aggregate_id: "not-a-uuid".to_string(),
            ..intent_row()
        };
        assert_eq!(
            decode_intent_row(&row).unwrap_err(),
            JournalReadError::Corrupt {
                aggregate_id: "not-a-uuid".to_string(),
                seq_nr: Some(1),
                cause: CorruptCause::InvariantViolation,
            }
        );
    }

    #[test]
    fn an_intent_row_whose_aid_disagrees_with_its_birth_material_is_corrupt() {
        // 行の名乗り (aid) と誕生材料の識別子が食い違う — どちらかが噓をついているので
        // 解釈せず止める。
        let row = JournalRow {
            aggregate_id: "018f3b2c-4d5e-7f60-8abc-def012345678".to_string(),
            ..intent_row()
        };
        assert_eq!(
            decode_intent_row(&row).unwrap_err(),
            JournalReadError::Corrupt {
                aggregate_id: "018f3b2c-4d5e-7f60-8abc-def012345678".to_string(),
                seq_nr: Some(1),
                cause: CorruptCause::InvariantViolation,
            }
        );
    }

    #[test]
    fn an_intent_row_whose_material_cannot_be_carried_into_the_domain_is_corrupt() {
        // JSON としては読めて DTO にもなるが、識別子の文法違反でドメインへ戻せない行。
        let sound = intent_row();
        let tampered = String::from_utf8(sound.payload.clone())
            .unwrap()
            .replace("01a02785-1bd8-76eb-aeea-5aa303ebd5b6", "not-a-uuid");
        let row = JournalRow {
            payload: tampered.into_bytes(),
            ..sound
        };
        assert_eq!(
            decode_intent_row(&row).unwrap_err(),
            JournalReadError::Corrupt {
                aggregate_id: "01a02785-1bd8-76eb-aeea-5aa303ebd5b6".to_string(),
                seq_nr: Some(1),
                cause: CorruptCause::UndecodablePayload,
            }
        );
    }

    #[test]
    fn an_intent_row_that_is_not_the_genesis_sequence_is_corrupt() {
        // `Created` は必ず通番 1 — それ以外を名乗る行は payload を解釈する前に止める。
        let row = JournalRow {
            seq_nr: 2,
            ..intent_row()
        };
        assert_eq!(
            decode_intent_row(&row).unwrap_err(),
            JournalReadError::Corrupt {
                aggregate_id: "01a02785-1bd8-76eb-aeea-5aa303ebd5b6".to_string(),
                seq_nr: Some(2),
                cause: CorruptCause::InvariantViolation,
            }
        );
    }

    /// 定義ストリームの 1 行 (名乗りと `aid` の検査を踏むための素体)。
    ///
    /// payload は `{}` — 名乗りの照合も `aid` の文法検査も**payload を読む前**に効くので、
    /// 素体が復号できない形であること自体が「中身を解釈していない」ことの証明になる。
    fn definition_row() -> JournalRow {
        JournalRow {
            rowid: 1,
            seq_nr: 1,
            aggregate_id: "claude".to_string(),
            payload: b"{}".to_vec(),
            occurred_at: 1_756_425_600_000_000_000,
            manifest: DEFINITION_EVENT_MANIFEST.to_string(),
        }
    }

    #[test]
    fn a_definition_row_that_names_another_manifest_is_refused_before_its_payload_is_read() {
        // 本番経路は判別子で振り分けるのでここへは来ないはずの行だが、来たら中身を
        // 解釈しない (`decode_entry` と同じ検査態度 — 名乗りが違う行は破損として止める)。
        let row = JournalRow {
            manifest: EVENT_MANIFEST.to_string(),
            ..definition_row()
        };
        assert_eq!(
            decode_definition_row(&row).unwrap_err(),
            JournalReadError::Corrupt {
                aggregate_id: "claude".to_string(),
                seq_nr: Some(1),
                cause: CorruptCause::UndecodablePayload,
            }
        );
    }

    #[test]
    fn a_definition_row_whose_aid_is_not_a_definition_id_is_corrupt() {
        // 系譜 ID の出所は行の `aid` である (改訂は識別子を運ばない)。文法を外れた `aid` は
        // 読み替えず止める — 空白だけの `aid` は `WorkflowDefinitionId` にならない。
        let row = JournalRow {
            aggregate_id: "   ".to_string(),
            ..definition_row()
        };
        assert_eq!(
            decode_definition_row(&row).unwrap_err(),
            JournalReadError::Corrupt {
                aggregate_id: "   ".to_string(),
                seq_nr: Some(1),
                cause: CorruptCause::InvariantViolation,
            }
        );
    }

    /// 1 ノードだけの定義内容 (誕生と改訂で共有する素体)。
    fn definition_content() -> (
        core_command_domain::workflow_definition::StageGraph,
        core_command_domain::workflow_definition::ScopeGrid,
        std::collections::BTreeMap<String, core_command_domain::workflow_definition::ScopeMetadata>,
    ) {
        use core_command_domain::workflow_definition::{
            ExecutionKind, PhaseId, ScopeGrid, StageGraph, StageMode, StageNodeBuilder,
            StageNumber, StageSlug,
        };
        let graph = StageGraph::new(vec![
            StageNodeBuilder::new(
                StageSlug::parse("state-init").expect("slug は文法内"),
                StageNumber::parse("0.1").expect("番号は文法内"),
                "State Init".to_string(),
                PhaseId::Initialization,
                ExecutionKind::Always,
                StageMode::Inline,
            )
            .build(),
        ])
        .expect("1 ノードのグラフ");
        let grid = ScopeGrid::from_graph(&graph);
        (graph, grid, std::collections::BTreeMap::new())
    }

    /// テストの定義内容版 (同じ文字で埋めた 64 桁)。
    fn definition_revision(
        fill: char,
    ) -> core_command_domain::workflow_definition::DefinitionRevision {
        core_command_domain::workflow_definition::DefinitionRevision::parse(&format!(
            "sha256:{}",
            fill.to_string().repeat(64)
        ))
        .expect("テストの revision")
    }

    /// b40 のテスト用固定イベント識別子 (定義面)。
    fn definition_event_id() -> core_command_domain::workflow_definition::WorkflowDefinitionEventId
    {
        core_command_domain::workflow_definition::WorkflowDefinitionEventId::parse(
            "0191aaaa-bbbb-7ccc-9ddd-eeeeffff0003",
        )
        .expect("UUIDv7")
    }

    /// 誕生イベント (`Defined` — 通番 1 でしか現れない)。
    fn defined_event() -> WorkflowDefinitionEvent {
        use core_command_domain::workflow_definition::Defined;
        let (graph, grid, scopes) = definition_content();
        WorkflowDefinitionEvent::Defined(Defined::new(
            definition_event_id(),
            WorkflowDefinitionId::parse("claude").expect("定義 id"),
            definition_revision('0'),
            graph,
            grid,
            scopes,
        ))
    }

    /// 改訂イベント (`Redefined` — 通番 2 以上でしか現れない)。
    fn redefined_event() -> WorkflowDefinitionEvent {
        use core_command_domain::workflow_definition::Redefined;
        let (graph, grid, scopes) = definition_content();
        WorkflowDefinitionEvent::Redefined(Redefined::new(
            definition_event_id(),
            WorkflowDefinitionId::parse("claude").expect("定義 id"),
            definition_revision('1'),
            graph,
            grid,
            scopes,
        ))
    }

    #[expect(
        clippy::disallowed_methods,
        reason = "契約 JSON ではなく行のバイトそのものを組む (BR1.7 の射程外)"
    )]
    #[test]
    fn a_definition_row_whose_sequence_contradicts_its_event_is_corrupt() {
        // 誕生 (`Defined`) は必ず通番 1、改訂 (`Redefined`) は 2 以上である。食い違う行は
        // 復号の境界で止める — 通番の飛びは `replay` まで運ぶとパニックになり、破損した
        // 行と区別がつかなくなる (intent 行と同じ検査態度)。
        let birth = WorkflowDefinitionEventDto::of(&defined_event());
        let revision = WorkflowDefinitionEventDto::of(&redefined_event());

        for (seq_nr, dto, label) in [
            (2_i64, &birth, "誕生が通番 2 を名乗る"),
            (1_i64, &revision, "改訂が通番 1 を名乗る"),
        ] {
            let row = JournalRow {
                seq_nr,
                payload: serde_json::to_vec(dto).unwrap(),
                ..definition_row()
            };
            assert_eq!(
                decode_definition_row(&row).unwrap_err(),
                JournalReadError::Corrupt {
                    aggregate_id: "claude".to_string(),
                    seq_nr: Some(usize::try_from(seq_nr).unwrap()),
                    cause: CorruptCause::InvariantViolation,
                },
                "{label}"
            );
        }
    }

    #[expect(
        clippy::disallowed_methods,
        reason = "契約 JSON ではなく行のバイトそのものを組む (BR1.7 の射程外)"
    )]
    #[test]
    fn a_definition_row_whose_sequence_agrees_with_its_event_is_read() {
        // 上の裏返し — 正しい組は素通りする (検査が広すぎないことを固定する)。
        let birth = JournalRow {
            seq_nr: 1,
            payload: serde_json::to_vec(&WorkflowDefinitionEventDto::of(&defined_event())).unwrap(),
            ..definition_row()
        };
        assert_eq!(decode_definition_row(&birth).unwrap().seq_nr(), 1);

        let revision = JournalRow {
            seq_nr: 2,
            payload: serde_json::to_vec(&WorkflowDefinitionEventDto::of(&redefined_event()))
                .unwrap(),
            ..definition_row()
        };
        assert_eq!(decode_definition_row(&revision).unwrap().seq_nr(), 2);
    }

    /// 実行ストリームの誕生イベント (`Started` — 通番 1 でしか現れない)。
    fn started_event() -> IntentExecutionEvent {
        use core_command_domain::orchestration::IntentExecution;
        let (_, event) = IntentExecution::start(
            IntentExecutionId::parse("01a02785-1bd8-76eb-aeea-5aa303ebd5b6").expect("実行 id"),
            &birth_intent(),
            occurred_at_of(1_756_425_600_000_000_000),
        );
        event
    }

    #[expect(
        clippy::disallowed_methods,
        reason = "契約 JSON ではなく行のバイトそのものを組む (BR1.7 の射程外)"
    )]
    #[test]
    fn an_execution_row_whose_sequence_contradicts_its_event_is_corrupt() {
        // 誕生 (`Started`) は必ず通番 1、以降のイベントは 2 以上である。食い違う行は復号の
        // 境界で止める — 通番の飛びをそのまま通すと `replay` まで届いてパニックになり、
        // 破損した行と区別がつかなくなる (定義行・intent 行と同じ検査態度)。
        for (seq_nr, dto, label) in [
            (
                2_i64,
                IntentExecutionEventDto::of(&started_event()),
                "誕生が通番 2 を名乗る",
            ),
            (
                1_i64,
                IntentExecutionEventDto::of(&IntentExecutionEvent::Unparked(Unparked::new(
                    event_id(),
                    execution_id(),
                ))),
                "誕生でないイベントが通番 1 を名乗る",
            ),
        ] {
            let row = JournalRow {
                seq_nr,
                payload: serde_json::to_vec(&dto).unwrap(),
                ..sound_row()
            };
            assert_eq!(
                decode_entry(&row).unwrap_err(),
                JournalReadError::Corrupt {
                    aggregate_id: "01a02785-1bd8-76eb-aeea-5aa303ebd5b6".to_string(),
                    seq_nr: Some(usize::try_from(seq_nr).unwrap()),
                    cause: CorruptCause::InvariantViolation,
                },
                "{label}"
            );
        }
    }

    #[expect(
        clippy::disallowed_methods,
        reason = "契約 JSON ではなく行のバイトそのものを組む (BR1.7 の射程外)"
    )]
    #[test]
    fn an_execution_row_whose_sequence_agrees_with_its_event_is_read() {
        // 上の裏返し — 正しい組は素通りする (検査が広すぎないことを固定する)。
        let birth = JournalRow {
            seq_nr: 1,
            payload: serde_json::to_vec(&IntentExecutionEventDto::of(&started_event())).unwrap(),
            ..sound_row()
        };
        assert_eq!(decode_entry(&birth).unwrap().seq_nr(), 1);
        assert_eq!(decode_entry(&sound_row()).unwrap().seq_nr(), 2);
    }

    #[expect(
        clippy::disallowed_methods,
        reason = "契約 JSON ではなく行のバイトそのものを組む (BR1.7 の射程外)"
    )]
    #[test]
    fn an_execution_row_whose_birth_plan_is_broken_is_corrupt() {
        // 計画そのものの不変条件を破る誕生行は復号の境界で止まる — 通すと集約の再構成まで
        // 届いてクラッシュする。DTO 側の拒否 (`InvariantViolation`) がここで `Corrupt` に写る
        // ことを、この層の面で固定する。
        use core_command_domain::orchestration::Started;
        let broken = IntentExecutionEvent::Started(Started::new(
            event_id(),
            IntentExecutionId::parse("01a02785-1bd8-76eb-aeea-5aa303ebd5b6").expect("実行 id"),
            IntentId::parse("01a02785-1bd8-76eb-aeea-5aa303ebd5b6").expect("intent id"),
            Vec::new(),
        ));
        let row = JournalRow {
            seq_nr: 1,
            payload: serde_json::to_vec(&IntentExecutionEventDto::of(&broken)).unwrap(),
            ..sound_row()
        };
        assert_eq!(
            decode_entry(&row).unwrap_err(),
            JournalReadError::Corrupt {
                aggregate_id: "01a02785-1bd8-76eb-aeea-5aa303ebd5b6".to_string(),
                seq_nr: Some(1),
                cause: CorruptCause::InvariantViolation,
            }
        );
    }

    #[test]
    fn every_decode_cause_maps_to_its_corrupt_classification() {
        assert_eq!(
            decode_cause(&DtoDecodeError::malformed("id", "x")),
            CorruptCause::UndecodablePayload
        );
        assert_eq!(
            decode_cause(&DtoDecodeError::InvariantViolation),
            CorruptCause::InvariantViolation
        );
    }
}
