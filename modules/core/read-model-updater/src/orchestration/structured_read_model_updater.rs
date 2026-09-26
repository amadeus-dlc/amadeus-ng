//! 構造化面 (系統 (2) — ジャーナル由来の `read_*` 20 表) の更新器。
//!
//! 形は「ジャーナルを読む → 投影 (純粋な変換) → 表の DAO で書く」である
//! (`coding-rules/read-model-updater-structure.md`)。
//!
//! ```text
//! 開く段: 読み面の表の用意 (read_model_schema::prepare — 揃っていれば書込ロックを取らない)
//!
//! 更新:
//!   head DAO.find (+ 3 表の as_of)        → 共有面が古いか (旧い変換・記録なし・未照合)
//!     古ければ BEGIN IMMEDIATE ─ 全履歴 → ReadTables::project → 20 表の DAO → head DAO ─ COMMIT
//!   (投影名を束ねていれば)
//!   checkpoint DAO.find + anchor_at        → 処理したシーケンス番号 (アンカー照合つき)
//!   JournalReader.events_after(番号)       → 新しい事実が無ければここで終わる (書込ロックを取らない)
//!   JournalReader.events_after(0)          → 全履歴 → ReadTables::project
//!   BEGIN IMMEDIATE ─────────────────────────────────────────────────────── COMMIT
//!     20 表の DAO (共有面より新しければ差し替え) → head DAO → checkpoint DAO (表の後)
//! ```
//!
//! # トランザクションの渡し方 (PR1 で採った形)
//!
//! 更新器が接続を 1 本所有し、`BEGIN IMMEDIATE` でトランザクションを開く。表の DAO は接続を
//! 持たず、書込メソッドがそのトランザクションを `&mut` で受け取る。確定と取り消しは更新器が
//! 決める — 途中で失敗すればトランザクションは確定されずに捨てられ、20 表・共有面の記録・
//! 処理したシーケンス番号のどれも動かない。IMMEDIATE で開くのは、読んでから書くトランザクションを
//! DEFERRED で始めると、別の書き手がいるときの書込昇格が busy timeout を待たずに即失敗するから
//! である (#134)。

use std::path::{Path, PathBuf};

use rusqlite::{Connection, TransactionBehavior};

use crate::read_tables::ReadTables;

use super::store_failure::{InStore as _, SqliteResultExt};
use super::structured_surface::StructuredSurface;
use super::{
    AnswerResultDao, AnswerResultDaoImpl, ArtifactAuditDao, ArtifactAuditDaoImpl, DefinitionDao,
    DefinitionDaoImpl, DefinitionScopeDao, DefinitionScopeDaoImpl, DefinitionScopeKeywordDao,
    DefinitionScopeKeywordDaoImpl, DefinitionScopePhaseEntryDao, DefinitionScopePhaseEntryDaoImpl,
    DefinitionScopeStageDao, DefinitionScopeStageDaoImpl, DefinitionStageDao,
    DefinitionStageDaoImpl, ExecutionDao, ExecutionDaoImpl, ExecutionStageDao,
    ExecutionStageDaoImpl, GlobalSeqNr, IntentDao, IntentDaoImpl, IntentStageDao,
    IntentStageDaoImpl, JournalReadError, JumpResultDao, JumpResultDaoImpl, NextAnswerDao,
    NextAnswerDaoImpl, NextJumpDao, NextJumpDaoImpl, NextJumpPhaseDao, NextJumpPhaseDaoImpl,
    ProjectionCheckpointDao, ProjectionCheckpointDaoImpl, ProjectionName, ReadModelHeadDao,
    ReadModelHeadDaoImpl, ReadModelUpdateError, ReadModelUpdater, ReportResultDao,
    ReportResultDaoImpl, RunStageDao, RunStageDaoImpl, ScopeChangeDao, ScopeChangeDaoImpl,
    SessionAuditDao, SessionAuditDaoImpl, StructuredJournalReader, StructuredJournalReaderImpl,
    read_model_schema, updater_connection,
};

/// 構造化面の更新器。
///
/// 型引数はジャーナルの読み手 `J`・チェックポイントの表の DAO `K`・共有面の記録の表の DAO `H`
/// と、20 表の DAO である (スタティックディスパッチ)。既定の型引数が実物の組であり、
/// [`StructuredReadModelUpdater::open`] がそれを組む。
///
/// # 2 つの使い方
///
/// - **共有面の点検だけ** (`open` のまま) — 共有面が旧い変換で描かれていたり、旧いストアから
///   持ち越した未照合の記録だったりすれば、現在の全履歴から描き直す。読取コマンドの前・
///   取得ループの先頭で呼ぶ (古い面を読ませない)。
/// - **投影を進める** ([`StructuredReadModelUpdater::for_projection`]) — 点検に続いて、その投影の
///   処理したシーケンス番号より後に事実があれば、全履歴から描いた 20 表と番号を 1 つの
///   IMMEDIATE トランザクションで確定する。Markdown 面の投影先がまだ無い初回起動 (最初の
///   `next` が定義の行を読む前) が使う。
#[derive(Debug)]
pub struct StructuredReadModelUpdater<
    J = StructuredJournalReaderImpl,
    K = ProjectionCheckpointDaoImpl,
    H = ReadModelHeadDaoImpl,
    Sa = SessionAuditDaoImpl,
    Aa = ArtifactAuditDaoImpl,
    Ar = AnswerResultDaoImpl,
    Jr = JumpResultDaoImpl,
    Rr = ReportResultDaoImpl,
    De = DefinitionDaoImpl,
    Ds = DefinitionStageDaoImpl,
    Dc = DefinitionScopeDaoImpl,
    Dk = DefinitionScopeKeywordDaoImpl,
    Dt = DefinitionScopeStageDaoImpl,
    Dp = DefinitionScopePhaseEntryDaoImpl,
    In = IntentDaoImpl,
    Is = IntentStageDaoImpl,
    Ex = ExecutionDaoImpl,
    Es = ExecutionStageDaoImpl,
    Na = NextAnswerDaoImpl,
    Nj = NextJumpDaoImpl,
    Np = NextJumpPhaseDaoImpl,
    Rs = RunStageDaoImpl,
    Sc = ScopeChangeDaoImpl,
> {
    connection: Connection,
    path: PathBuf,
    projection: Option<ProjectionName>,
    #[allow(
        clippy::type_complexity,
        reason = "表ごとの DAO を型引数で持つ (スタティックディスパッチ) ので、20 表ぶんの型引数が並ぶ"
    )]
    surface: StructuredSurface<
        J,
        K,
        H,
        Sa,
        Aa,
        Ar,
        Jr,
        Rr,
        De,
        Ds,
        Dc,
        Dk,
        Dt,
        Dp,
        In,
        Is,
        Ex,
        Es,
        Na,
        Nj,
        Np,
        Rs,
        Sc,
    >,
}

impl StructuredReadModelUpdater {
    /// 既存の共有ストアへ接続し、読み面の表を揃える (ストア自体は作らない — 作るのは本家の
    /// イベントストアである)。
    ///
    /// 表の用意は `read_model_schema::prepare` 1 か所が持つ — 構造化面の 20 表と管理表
    /// (チェックポイント・共有面の記録) に加えて、参照入力由来の表も揃え、読み面の版が動いて
    /// いれば作り直す。表が揃っていて版も現行なら書込ロックを取らない。
    ///
    /// # Errors
    ///
    /// ストアへ接続できない・表を用意できない (`Io`)、版が動いたストアの歴史を描き直せない
    /// (`Corrupt`) 場合。
    pub fn open(path: &Path) -> Result<Self, JournalReadError> {
        let mut connection = updater_connection::open(path)?;
        read_model_schema::prepare(&mut connection, path)?;
        Ok(Self {
            connection,
            path: path.to_path_buf(),
            projection: None,
            surface: StructuredSurface::default(),
        })
    }
}

impl<J, K, H, Sa, Aa, Ar, Jr, Rr, De, Ds, Dc, Dk, Dt, Dp, In, Is, Ex, Es, Na, Nj, Np, Rs, Sc>
    StructuredReadModelUpdater<
        J,
        K,
        H,
        Sa,
        Aa,
        Ar,
        Jr,
        Rr,
        De,
        Ds,
        Dc,
        Dk,
        Dt,
        Dp,
        In,
        Is,
        Ex,
        Es,
        Na,
        Nj,
        Np,
        Rs,
        Sc,
    >
where
    J: StructuredJournalReader,
    K: ProjectionCheckpointDao,
    H: ReadModelHeadDao,
    Sa: SessionAuditDao,
    Aa: ArtifactAuditDao,
    Ar: AnswerResultDao,
    Jr: JumpResultDao,
    Rr: ReportResultDao,
    De: DefinitionDao,
    Ds: DefinitionStageDao,
    Dc: DefinitionScopeDao,
    Dk: DefinitionScopeKeywordDao,
    Dt: DefinitionScopeStageDao,
    Dp: DefinitionScopePhaseEntryDao,
    In: IntentDao,
    Is: IntentStageDao,
    Ex: ExecutionDao,
    Es: ExecutionStageDao,
    Na: NextAnswerDao,
    Nj: NextJumpDao,
    Np: NextJumpPhaseDao,
    Rs: RunStageDao,
    Sc: ScopeChangeDao,
{
    /// 更新のたびに、投影 `projection` の処理したシーケンス番号まで構造化面を進める。
    #[must_use]
    pub fn for_projection(mut self, projection: ProjectionName) -> Self {
        self.projection = Some(projection);
        self
    }

    /// 投影 `projection` が処理したシーケンス番号 (未登録は `ZERO`)。
    ///
    /// 保存したアンカーをジャーナルの同じ位置の行と照らし合わせてから返す。更新
    /// ([`ReadModelUpdater::update_read_models`]) はコマンドなので前進先を返さない — 到達点を
    /// 知りたい側はこのクエリで読む。
    ///
    /// # Errors
    ///
    /// 読めない (`Read(Io)`)、保存値が負・アンカーが欠ける・食い違う (`Read(Corrupt)`) 場合。
    pub fn checkpoint(
        &self,
        projection: &ProjectionName,
    ) -> Result<GlobalSeqNr, ReadModelUpdateError> {
        Ok(self
            .surface
            .checkpoint(&self.connection, projection)
            .in_store(&self.path)?)
    }

    /// 共有面が古ければ、現在の全履歴から描き直す (個別のチェックポイントは変えない)。
    fn refresh(&mut self) -> Result<(), ReadModelUpdateError> {
        if !self
            .surface
            .needs_rebuild(&self.connection)
            .in_store(&self.path)?
        {
            return Ok(());
        }
        let mut transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .at_store(&self.path)?;
        self.surface
            .rebuild(&mut transaction)
            .in_store(&self.path)?;
        transaction.commit().at_store(&self.path)?;
        Ok(())
    }
}

impl<J, K, H, Sa, Aa, Ar, Jr, Rr, De, Ds, Dc, Dk, Dt, Dp, In, Is, Ex, Es, Na, Nj, Np, Rs, Sc>
    ReadModelUpdater
    for StructuredReadModelUpdater<
        J,
        K,
        H,
        Sa,
        Aa,
        Ar,
        Jr,
        Rr,
        De,
        Ds,
        Dc,
        Dk,
        Dt,
        Dp,
        In,
        Is,
        Ex,
        Es,
        Na,
        Nj,
        Np,
        Rs,
        Sc,
    >
where
    J: StructuredJournalReader,
    K: ProjectionCheckpointDao,
    H: ReadModelHeadDao,
    Sa: SessionAuditDao,
    Aa: ArtifactAuditDao,
    Ar: AnswerResultDao,
    Jr: JumpResultDao,
    Rr: ReportResultDao,
    De: DefinitionDao,
    Ds: DefinitionStageDao,
    Dc: DefinitionScopeDao,
    Dk: DefinitionScopeKeywordDao,
    Dt: DefinitionScopeStageDao,
    Dp: DefinitionScopePhaseEntryDao,
    In: IntentDao,
    Is: IntentStageDao,
    Ex: ExecutionDao,
    Es: ExecutionStageDao,
    Na: NextAnswerDao,
    Nj: NextJumpDao,
    Np: NextJumpPhaseDao,
    Rs: RunStageDao,
    Sc: ScopeChangeDao,
{
    type Error = ReadModelUpdateError;

    /// 共有面を点検し (古ければ描き直す)、投影名を束ねていれば、処理したシーケンス番号より後の
    /// 事実があるときだけ全履歴から構造化面を描き、20 表と番号を同じトランザクションで進める。
    /// 差分が空なら何も書かない (書込ロックも取らない)。
    ///
    /// 内部は同期 I/O だけである。非同期なのは共通契約の境界だけ。
    ///
    /// # Errors
    ///
    /// ジャーナル・チェックポイントの読取、構造化投影、20 表と番号の同一トランザクションでの
    /// 更新に失敗した場合。
    async fn update_read_models(&mut self) -> Result<(), ReadModelUpdateError> {
        self.refresh()?;
        let Some(projection) = &self.projection else {
            return Ok(());
        };
        let checkpoint = self
            .surface
            .checkpoint(&self.connection, projection)
            .in_store(&self.path)?;
        // 差分読取は「進む先があるか」の**探り**にだけ使う。描く材料は下の全履歴の読取 1 回に
        // 揃える (2 つの読取に跨がると、その間に入った書込のぶんだけ断面がずれる)。
        if self
            .surface
            .journal()
            .events_after(&self.connection, checkpoint)
            .in_store(&self.path)?
            .scanned_to()
            .is_none()
        {
            return Ok(());
        }
        let history = self
            .surface
            .journal()
            .events_after(&self.connection, GlobalSeqNr::ZERO)
            .in_store(&self.path)?;
        let last = history
            .scanned_to()
            .ok_or(ReadModelUpdateError::HistoryDisappeared)?;
        let tables = ReadTables::project(&history)?;
        let mut transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .at_store(&self.path)?;
        self.surface
            .advance(&mut transaction, projection, last, &tables)
            .in_store(&self.path)?;
        transaction.commit().at_store(&self.path)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::io::ErrorKind;
    use std::time::Duration;

    use super::*;
    use crate::orchestration::journal_reader_impl::tests::opened_store;

    #[tokio::test]
    async fn a_write_lock_held_by_another_connection_is_reported_as_would_block() {
        // BR2.1 の待ち時間そのものを観測する。既定 (5000ms) では試験が待つだけなので、
        // 接続の上限を縮めて `WouldBlock` を実測する (NFR3.5)。
        let dir = tempfile::tempdir().expect("一時 dir");
        let (_store, path) = opened_store(&dir);
        let mut updater = StructuredReadModelUpdater::open(path.as_path())
            .expect("開ける")
            .for_projection(ProjectionName::parse("state-file").expect("投影名"));
        updater
            .connection
            .busy_timeout(Duration::from_millis(20))
            .expect("待ち時間");
        // 共有面の記録を旧い変換にして、描き直し (IMMEDIATE) を必ず起こす。
        let raw = Connection::open(path.as_path()).expect("生の接続");
        raw.execute_batch("UPDATE amadeus_read_model_head SET revision = 'old-transform'")
            .expect("旧い変換にする");
        raw.execute_batch("BEGIN IMMEDIATE")
            .expect("書込ロックを握る");

        let error = updater.update_read_models().await;

        raw.execute_batch("END").expect("手放す");
        assert!(
            matches!(
                error,
                Err(ReadModelUpdateError::Read(JournalReadError::Io {
                    kind: ErrorKind::WouldBlock,
                    ..
                }))
            ),
            "{error:?}"
        );
    }

    /// 探りでは新しい事実が見えたのに、描く材料の読取では履歴が消えている読み手。
    ///
    /// 共有ハンドルではなく `Cell` で読取回数を数えるのは、読取が `&self` だからである
    /// (試験用のフェイクであり、設計上の内部可変性ではない)。
    #[derive(Debug, Default)]
    struct VanishingJournal {
        reads: std::cell::Cell<usize>,
    }

    impl StructuredJournalReader for VanishingJournal {
        fn events_after(
            &self,
            _connection: &Connection,
            _after: GlobalSeqNr,
        ) -> Result<crate::orchestration::JournalBatch, JournalReadError> {
            let seen = self.reads.get();
            self.reads.set(seen + 1);
            let scanned_to = (seen == 0).then_some(GlobalSeqNr::new(1));
            Ok(crate::orchestration::JournalBatch::new(
                Vec::new(),
                Vec::new(),
                Vec::new(),
                scanned_to,
            ))
        }

        fn events_through(
            &self,
            connection: &Connection,
            _through: GlobalSeqNr,
        ) -> Result<crate::orchestration::JournalBatch, JournalReadError> {
            self.events_after(connection, GlobalSeqNr::ZERO)
        }

        fn anchor_at(
            &self,
            _connection: &Connection,
            _position: GlobalSeqNr,
        ) -> Result<Option<crate::orchestration::JournalAnchor>, JournalReadError> {
            Ok(None)
        }
    }

    #[tokio::test]
    async fn a_history_that_disappears_after_the_probe_is_not_a_successful_update() {
        // 差分を観測した直後に履歴が消えた場合、古い読取位置で成功したことにしない
        // (取得ループと同じ約束)。何も書かず、番号も動かない。
        let dir = tempfile::tempdir().expect("一時 dir");
        let (_store, path) = opened_store(&dir);
        drop(StructuredReadModelUpdater::open(path.as_path()).expect("表を用意する"));
        let mut updater = StructuredReadModelUpdater {
            connection: updater_connection::open(path.as_path()).expect("接続"),
            path: path.as_path().to_path_buf(),
            projection: Some(ProjectionName::parse("state-file").expect("投影名")),
            surface: StructuredSurface::<VanishingJournal>::default(),
        };

        let result = updater.update_read_models().await;

        assert_eq!(result, Err(ReadModelUpdateError::HistoryDisappeared));
        assert_eq!(
            updater.checkpoint(&ProjectionName::parse("state-file").expect("投影名")),
            Ok(GlobalSeqNr::ZERO)
        );
    }
}
