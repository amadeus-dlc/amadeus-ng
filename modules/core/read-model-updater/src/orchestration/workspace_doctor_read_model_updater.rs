//! 自己診断 (`WorkspaceDoctor`) のリードモデル更新器。
//!
//! 形は「ジャーナルを読む → 投影 (純粋な変換) → DAO でリードモデルを更新する」だけである
//! (オーナー裁定 2026-09-26 — `coding-rules/read-model-updater-structure.md`)。
//!
//! ```text
//! 更新器 ── BEGIN IMMEDIATE ─────────────────────────────────────────── COMMIT
//!   │  checkpoint DAO.find      → 処理したシーケンス番号 (after)
//!   │  JournalReader.events_after(after)   → 新しい事実 (無ければ何も書かない)
//!   │  JournalReader.events_through(last)  → 触れた集約の全履歴
//!   │  投影: 集約を replay で起こし、クエリの答えを行へ写す (判断は無い)
//!   │  report DAO.save / check DAO.replace_for_report   (表ごとに 1 本)
//!   └─ checkpoint DAO.save(last)
//! ```
//!
//! 行の値はクエリ側がそのまま表示する — 数える・並べ替える・文言を組むことをクエリ側に
//! させないため、集計と終了コードまで焼き込む (`coding-rules/cqrs-boundaries.md` 規則 6)。
//!
//! # トランザクションの渡し方 (PR1 で採った形)
//!
//! 更新器が接続を 1 本所有し、`BEGIN IMMEDIATE` でトランザクションを開く。表の DAO は接続を
//! 持たず、書込メソッドがそのトランザクションを `&mut` で受け取る。読取 (チェックポイント・
//! ジャーナル) も同じトランザクションの上で行う。確定と取り消しは更新器が決める — 途中で
//! 失敗すればトランザクションは確定されずに捨てられ、2 表と処理したシーケンス番号のどれも
//! 動かない。IMMEDIATE で開くのは、読んでから書くトランザクションを DEFERRED で始めると、
//! 別の書き手がいるときの書込昇格が busy timeout を待たずに即失敗するからである (#134)。

use std::collections::{BTreeMap, BTreeSet};

use core_command_domain::workspace::WorkspaceDoctor;
use core_infrastructure::collections::FirstClassCollection as _;
use rusqlite::{Connection, OpenFlags, TransactionBehavior};

use super::journal_reader_impl::corrupt_error;
use super::store_failure::SqliteResultExt;
use super::{
    CorruptCause, DoctorCheckDao, DoctorCheckDaoImpl, DoctorCheckRow, DoctorReportDao,
    DoctorReportDaoImpl, DoctorReportRow, JournalReadError, ProjectionName, ReadModelUpdater,
    WorkspaceDoctorJournalEntry, WorkspaceDoctorJournalReader, WorkspaceDoctorJournalReaderImpl,
    WorkspaceDoctorProjectionCheckpointDao, WorkspaceDoctorProjectionCheckpointDaoImpl,
};

/// この面のチェックポイント名。
const PROJECTION: &str = "workspace-doctor";

/// 書込ロックを待つ上限 (他の更新器と同じ既定)。
const BUSY_TIMEOUT: std::time::Duration = std::time::Duration::from_millis(5000);

/// 自己診断のリードモデル更新器。
///
/// 型引数はジャーナルの読み手と、書く表ごとの DAO である (スタティックディスパッチ)。
/// 実物の組は [`WorkspaceDoctorReadModelUpdater::open`] が作る。
#[derive(Debug)]
pub struct WorkspaceDoctorReadModelUpdater<J, R, C, K> {
    connection: Connection,
    path: std::path::PathBuf,
    journal: J,
    reports: R,
    checks: C,
    checkpoints: K,
}

impl<J, R, C, K> ReadModelUpdater for WorkspaceDoctorReadModelUpdater<J, R, C, K>
where
    J: WorkspaceDoctorJournalReader,
    R: DoctorReportDao,
    C: DoctorCheckDao,
    K: WorkspaceDoctorProjectionCheckpointDao,
{
    type Error = JournalReadError;

    /// 処理したシーケンス番号より後の事実を読み、触れた集約の行を差し替え、処理した番号を
    /// 保存する。2 表と番号は 1 つのトランザクションで確定する。
    ///
    /// 内部は同期 I/O だけである。非同期なのは共通契約の境界だけ。
    ///
    /// # Errors
    ///
    /// 履歴の読取・復号・再生、表の書込、トランザクションの確定に失敗した場合。
    async fn update_read_models(&mut self) -> Result<(), JournalReadError> {
        self.update()
    }
}

impl
    WorkspaceDoctorReadModelUpdater<
        WorkspaceDoctorJournalReaderImpl,
        DoctorReportDaoImpl,
        DoctorCheckDaoImpl,
        WorkspaceDoctorProjectionCheckpointDaoImpl,
    >
{
    /// 既存の共有 DB へ接続する。DB 自体は作らない。
    ///
    /// 一時ストア (`file:...?mode=memory&cache=shared`) も同じ口で開けるよう URI を許す —
    /// 診断は初回状態でファイルを 1 つも作らない (契約 C7 DC1)。
    ///
    /// # Errors
    ///
    /// 共有 DB へ接続できない場合。
    pub fn open(path: &std::path::Path) -> Result<Self, JournalReadError> {
        let connection = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_WRITE
                | OpenFlags::SQLITE_OPEN_URI
                | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .map_err(|_| corrupt_error(PROJECTION, None, CorruptCause::InvariantViolation))?;
        connection.busy_timeout(BUSY_TIMEOUT).at_store(path)?;
        Ok(Self {
            connection,
            path: path.to_path_buf(),
            journal: WorkspaceDoctorJournalReaderImpl,
            reports: DoctorReportDaoImpl,
            checks: DoctorCheckDaoImpl,
            checkpoints: WorkspaceDoctorProjectionCheckpointDaoImpl,
        })
    }
}

impl<J, R, C, K> WorkspaceDoctorReadModelUpdater<J, R, C, K>
where
    J: WorkspaceDoctorJournalReader,
    R: DoctorReportDao,
    C: DoctorCheckDao,
    K: WorkspaceDoctorProjectionCheckpointDao,
{
    /// [`ReadModelUpdater::update_read_models`] の本体。
    fn update(&mut self) -> Result<(), JournalReadError> {
        let projection = ProjectionName::parse(PROJECTION)
            .map_err(|_| corrupt_error(PROJECTION, None, CorruptCause::InvariantViolation))?;
        let mut transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .at_store(&self.path)?;
        self.checkpoints.create_table(&mut transaction)?;
        self.reports.create_table(&mut transaction)?;
        self.checks.create_table(&mut transaction)?;

        let after = self.checkpoints.find(&transaction, &projection)?;
        let latest = self.journal.events_after(&transaction, after)?;
        if let Some(last) = latest.last().map(WorkspaceDoctorJournalEntry::position) {
            let touched: BTreeSet<&str> = latest
                .iter()
                .map(|entry| entry.event().aggregate_id().as_str())
                .collect();
            let history = self.journal.events_through(&transaction, last)?;
            for doctor in replay(&history, &touched)? {
                let (report, checks) = project(&doctor);
                self.reports.save(&mut transaction, &report)?;
                self.checks
                    .replace_for_report(&mut transaction, report.id(), &checks)?;
            }
            self.checkpoints.save(&mut transaction, &projection, last)?;
        }
        transaction.commit().at_store(&self.path)
    }
}

/// `touched` の集約を、履歴から `replay` で起こす (集約 ID の辞書順)。
fn replay(
    history: &[WorkspaceDoctorJournalEntry],
    touched: &BTreeSet<&str>,
) -> Result<Vec<WorkspaceDoctor>, JournalReadError> {
    let mut streams: BTreeMap<&str, Vec<&WorkspaceDoctorJournalEntry>> = BTreeMap::new();
    for entry in history {
        let aid = entry.event().aggregate_id().as_str();
        if touched.contains(aid) {
            streams.entry(aid).or_default().push(entry);
        }
    }
    let mut replayed = Vec::with_capacity(streams.len());
    for (aid, mut entries) in streams {
        entries.sort_by_key(|entry| entry.seq_nr());
        let genesis = entries
            .first()
            .ok_or_else(|| corrupt_error(aid, None, CorruptCause::InvariantViolation))?;
        if genesis.seq_nr() != 1 {
            return Err(corrupt_error(
                aid,
                Some(genesis.seq_nr()),
                CorruptCause::InvariantViolation,
            ));
        }
        let snapshot = WorkspaceDoctor::new(
            genesis.event().aggregate_id().clone(),
            genesis.event().target().clone(),
            genesis.event().checks().clone(),
            genesis.seq_nr(),
            0,
            genesis.occurred_at(),
        )
        .map_err(|_| {
            corrupt_error(
                aid,
                Some(genesis.seq_nr()),
                CorruptCause::InvariantViolation,
            )
        })?;
        replayed.push(WorkspaceDoctor::replay(
            snapshot,
            entries
                .into_iter()
                .skip(1)
                .map(|entry| (entry.event().clone(), entry.seq_nr(), entry.occurred_at())),
        ));
    }
    Ok(replayed)
}

/// 集約のクエリの答えを 2 表の行へ写す (純粋な変換 — 判断は集約の側にある)。
fn project(doctor: &WorkspaceDoctor) -> (DoctorReportRow, Vec<DoctorCheckRow>) {
    let id = doctor.id().as_str();
    let report = DoctorReportRow::new(
        id.to_string(),
        doctor.target().relative_directory(),
        doctor.checks().passed(),
        doctor.checks().failed(),
        doctor.checks().exit_code(),
        doctor.seq_nr(),
    );
    let checks = doctor.checks().fold_left(Vec::new(), |mut rows, check| {
        let position = rows.len();
        rows.push(DoctorCheckRow::new(
            id.to_string(),
            position,
            check.id().as_str().to_string(),
            check.is_passed(),
            check.label().to_string(),
            check.fix().map(str::to_string),
        ));
        rows
    });
    (report, checks)
}
