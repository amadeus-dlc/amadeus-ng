//! 構造化面 (系統 (2) — ジャーナル由来の `read_*` 20 表) を表の DAO で書く手順。
//!
//! 更新器 ([`super::StructuredReadModelUpdater`]) の内部である。接続を持たず、更新器が開いた
//! トランザクション (読取は接続) の上で、ジャーナルの読み手と表ごとの DAO を順に呼び、表を
//! またぐ検査 (共有面の記録と 20 表の内容の照合・処理したシーケンス番号とジャーナルの行の
//! アンカー照合) を行う。DAO は 1 表の I/O だけを持ち、これらの検査は持たない
//! (`coding-rules/read-model-updater-structure.md` 原則 3)。
//!
//! 手順を更新器から切り出してあるのは、同じ手順を公開 (Markdown 面 — `JournalReader::publish`、
//! Issue #153 の PR5 で更新器へ移す) が自分のトランザクションの中で使うからである。ファイルの
//! 公開と構造化面の確定を 1 つの IMMEDIATE トランザクションに閉じる今の形を保つため、手順は
//! トランザクションを受け取る形にしてある。trait は持たない (ポートではない) — 複数の表へ書く
//! 大きなポート (`ReadModelWriter` の類) を作らない、という原則 1 に当たらない。
//!
//! ```text
//! advance (1 つの IMMEDIATE トランザクションの中)
//!   checkpoint DAO.find + JournalReader.anchor_at   → 現在の番号 (アンカー照合つき)
//!   JournalReader.anchor_at(to)                     → 前進先の行の識別子
//!   head DAO.find + 20 表の find_content             → 共有面の記録の検査
//!   20 表の delete_all / insert                      → 共有面より新しければ差し替え
//!   head DAO.save                                    → 共有面の記録 (位置・世代・ダイジェスト)
//!   checkpoint DAO.save                              → 処理したシーケンス番号 (表の後)
//! ```

use rusqlite::{Connection, Transaction};

use crate::read_tables::ReadTables;

use super::journal_reader_impl::corrupt_error;
use super::store_failure::SqliteResultExt;
use super::structured_surface_content::StructuredSurfaceContent;
use super::{
    AnswerResultDao, AnswerResultDaoImpl, ArtifactAuditDao, ArtifactAuditDaoImpl, CorruptCause,
    DefinitionDao, DefinitionDaoImpl, DefinitionScopeDao, DefinitionScopeDaoImpl,
    DefinitionScopeKeywordDao, DefinitionScopeKeywordDaoImpl, DefinitionScopePhaseEntryDao,
    DefinitionScopePhaseEntryDaoImpl, DefinitionScopeStageDao, DefinitionScopeStageDaoImpl,
    DefinitionStageDao, DefinitionStageDaoImpl, ExecutionDao, ExecutionDaoImpl, ExecutionStageDao,
    ExecutionStageDaoImpl, GlobalSeqNr, IntentDao, IntentDaoImpl, IntentStageDao,
    IntentStageDaoImpl, JournalReadError, JumpResultDao, JumpResultDaoImpl, NextAnswerDao,
    NextAnswerDaoImpl, NextJumpDao, NextJumpDaoImpl, NextJumpPhaseDao, NextJumpPhaseDaoImpl,
    ProjectionCheckpointDao, ProjectionCheckpointDaoImpl, ProjectionCheckpointRow, ProjectionName,
    PublicationBatch, ReadModelHeadDao, ReadModelHeadDaoImpl, ReadModelHeadRow,
    ReadModelUpdateError, ReportResultDao, ReportResultDaoImpl, RunStageDao, RunStageDaoImpl,
    ScopeChangeDao, ScopeChangeDaoImpl, SessionAuditDao, SessionAuditDaoImpl,
    StructuredJournalReader, StructuredJournalReaderImpl,
};

/// 集約に属さない値 (位置・記録) の識別子欄に置く印。
const NO_AGGREGATE: &str = "-";

/// 内容の照合で書いた行を捨てるためのセーブポイント (トランザクションの制御であって表の I/O
/// ではない)。
const COMPARE_SAVEPOINT: &str = "SAVEPOINT amadeus_compare_projection";

/// 照合のために書いた行を捨て、セーブポイントを閉じる。
const COMPARE_ROLLBACK: &str =
    "ROLLBACK TO amadeus_compare_projection; RELEASE amadeus_compare_projection";

/// 共有面の記録が内容と合わない (`Corrupt (ProjectionSnapshotMismatch)`)。
fn snapshot_mismatch() -> JournalReadError {
    corrupt_error(NO_AGGREGATE, None, CorruptCause::ProjectionSnapshotMismatch)
}

/// 位置 (`u64`) を記録の列 (`i64`) の値へ写す。収まらない位置は `Corrupt` で止める。
fn position_value(position: GlobalSeqNr) -> Result<i64, JournalReadError> {
    i64::try_from(position.to_u64())
        .map_err(|_| corrupt_error(NO_AGGREGATE, None, CorruptCause::InvariantViolation))
}

/// 構造化面を書く手順 — ジャーナルの読み手と、表ごとの DAO を型引数で持つ (スタティック
/// ディスパッチ)。既定の型引数は実物の組であり、`StructuredSurface::default()` がそれを組む。
///
/// 型引数は、ジャーナルの読み手 `J`・チェックポイントの表の DAO `K`・共有面の記録の表の DAO
/// `H` と、20 表の DAO (`read_session_audit` から `read_scope_change` まで) である。
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct StructuredSurface<
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
    journal: J,
    checkpoints: K,
    head: H,
    session_audits: Sa,
    artifact_audits: Aa,
    answer_results: Ar,
    jump_results: Jr,
    report_results: Rr,
    definitions: De,
    definition_stages: Ds,
    definition_scopes: Dc,
    definition_scope_keywords: Dk,
    definition_scope_stages: Dt,
    definition_scope_phase_entries: Dp,
    intents: In,
    intent_stages: Is,
    executions: Ex,
    execution_stages: Es,
    next_answers: Na,
    next_jumps: Nj,
    next_jump_phases: Np,
    run_stages: Rs,
    scope_changes: Sc,
}

impl<J, K, H, Sa, Aa, Ar, Jr, Rr, De, Ds, Dc, Dk, Dt, Dp, In, Is, Ex, Es, Na, Nj, Np, Rs, Sc>
    StructuredSurface<
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
    /// ジャーナルの読み手 (更新器が差分の探りと全履歴の読取に使う)。
    pub(crate) const fn journal(&self) -> &J {
        &self.journal
    }

    // ---- 表の用意 (DDL の正本は各表の DAO) ----

    /// この面の表 (チェックポイント・共有面の記録・20 表) が揃っているか。
    /// `sqlite_master` の読取だけで、書込ロックを取らない。
    pub(crate) fn tables_exist(&self, connection: &Connection) -> Result<bool, JournalReadError> {
        Ok(self.checkpoints.table_exists(connection)?
            && self.head.table_exists(connection)?
            && self.session_audits.table_exists(connection)?
            && self.artifact_audits.table_exists(connection)?
            && self.answer_results.table_exists(connection)?
            && self.jump_results.table_exists(connection)?
            && self.report_results.table_exists(connection)?
            && self.definitions.table_exists(connection)?
            && self.definition_stages.table_exists(connection)?
            && self.definition_scopes.table_exists(connection)?
            && self.definition_scope_keywords.table_exists(connection)?
            && self.definition_scope_stages.table_exists(connection)?
            && self
                .definition_scope_phase_entries
                .table_exists(connection)?
            && self.intents.table_exists(connection)?
            && self.intent_stages.table_exists(connection)?
            && self.executions.table_exists(connection)?
            && self.execution_stages.table_exists(connection)?
            && self.next_answers.table_exists(connection)?
            && self.next_jumps.table_exists(connection)?
            && self.next_jump_phases.table_exists(connection)?
            && self.run_stages.table_exists(connection)?
            && self.scope_changes.table_exists(connection)?)
    }

    /// この面の表を (無ければ) 作る。冪等。
    pub(crate) fn create_tables(
        &self,
        transaction: &mut Transaction<'_>,
    ) -> Result<(), JournalReadError> {
        self.checkpoints.create_table(transaction)?;
        self.head.create_table(transaction)?;
        self.session_audits.create_table(transaction)?;
        self.artifact_audits.create_table(transaction)?;
        self.answer_results.create_table(transaction)?;
        self.jump_results.create_table(transaction)?;
        self.report_results.create_table(transaction)?;
        self.definitions.create_table(transaction)?;
        self.definition_stages.create_table(transaction)?;
        self.definition_scopes.create_table(transaction)?;
        self.definition_scope_keywords.create_table(transaction)?;
        self.definition_scope_stages.create_table(transaction)?;
        self.definition_scope_phase_entries
            .create_table(transaction)?;
        self.intents.create_table(transaction)?;
        self.intent_stages.create_table(transaction)?;
        self.executions.create_table(transaction)?;
        self.execution_stages.create_table(transaction)?;
        self.next_answers.create_table(transaction)?;
        self.next_jumps.create_table(transaction)?;
        self.next_jump_phases.create_table(transaction)?;
        self.run_stages.create_table(transaction)?;
        self.scope_changes.create_table(transaction)
    }

    /// 20 表を落とす (読み面の版が動いたときの作り直しだけが呼ぶ)。チェックポイントと共有面の
    /// 記録は落とさない — チェックポイントは Markdown 面と共有の位置であり、記録は未照合へ戻す
    /// だけで足りる。
    pub(crate) fn drop_tables(
        &self,
        transaction: &mut Transaction<'_>,
    ) -> Result<(), JournalReadError> {
        self.session_audits.drop_table(transaction)?;
        self.artifact_audits.drop_table(transaction)?;
        self.answer_results.drop_table(transaction)?;
        self.jump_results.drop_table(transaction)?;
        self.report_results.drop_table(transaction)?;
        self.definitions.drop_table(transaction)?;
        self.definition_stages.drop_table(transaction)?;
        self.definition_scopes.drop_table(transaction)?;
        self.definition_scope_keywords.drop_table(transaction)?;
        self.definition_scope_stages.drop_table(transaction)?;
        self.definition_scope_phase_entries
            .drop_table(transaction)?;
        self.intents.drop_table(transaction)?;
        self.intent_stages.drop_table(transaction)?;
        self.executions.drop_table(transaction)?;
        self.execution_stages.drop_table(transaction)?;
        self.next_answers.drop_table(transaction)?;
        self.next_jumps.drop_table(transaction)?;
        self.next_jump_phases.drop_table(transaction)?;
        self.run_stages.drop_table(transaction)?;
        self.scope_changes.drop_table(transaction)
    }

    // ---- 共有面の記録 ----

    /// 共有面の記録 (まだ置かれていなければ `None`)。
    pub(crate) fn head(
        &self,
        connection: &Connection,
    ) -> Result<Option<ReadModelHeadRow>, JournalReadError> {
        self.head.find(connection)
    }

    /// 共有面の記録が無ければ、未照合の初期値 (位置 0・世代 1・現行の変換・空のダイジェスト) を
    /// 置く。在れば触らない。
    pub(crate) fn ensure_head(
        &self,
        transaction: &mut Transaction<'_>,
    ) -> Result<(), JournalReadError> {
        if self.head.find(transaction)?.is_some() {
            return Ok(());
        }
        self.head.save(
            transaction,
            &ReadModelHeadRow::new(
                0,
                1,
                PublicationBatch::current_transform_revision(),
                String::new(),
                false,
            ),
        )
    }

    /// 共有面の記録を未照合へ戻す (次の書込の前に、歴史から描き直した行と照合させる)。
    pub(crate) fn invalidate_head(
        &self,
        transaction: &mut Transaction<'_>,
    ) -> Result<(), JournalReadError> {
        let Some(head) = self.head.find(transaction)? else {
            return Ok(());
        };
        self.head.save(
            transaction,
            &ReadModelHeadRow::new(
                head.position(),
                head.generation(),
                head.revision().to_string(),
                head.content_digest().to_string(),
                false,
            ),
        )
    }

    /// 共有面を描き直す必要があるか (旧い変換で描かれた・記録が無い・旧いストアから持ち越した
    /// 未照合の記録で、しかも何かが描かれている)。
    pub(crate) fn needs_rebuild(&self, connection: &Connection) -> Result<bool, JournalReadError> {
        Ok(match self.head.find(connection)? {
            None => true,
            Some(head) => {
                head.revision() != PublicationBatch::current_transform_revision()
                    || (!head.is_verified() && self.known_position(connection)? > 0)
            }
        })
    }

    /// 投影が 1 度でも進んだか (進んだチェックポイントが在るか)。チェックポイントの表が
    /// まだ無いストアは未投影である。
    pub(crate) fn projected_before(
        &self,
        connection: &Connection,
    ) -> Result<bool, JournalReadError> {
        if !self.checkpoints.table_exists(connection)? {
            return Ok(false);
        }
        Ok(self
            .checkpoints
            .find_max_position(connection)?
            .is_some_and(|position| position > GlobalSeqNr::ZERO))
    }

    /// 記録を持たない旧いストアで、共有面がどの位置まで描かれていたかの推定 — チェック
    /// ポイントと、集約そのものを表す 3 表 (`read_execution` / `read_intent` /
    /// `read_definition`) の `as_of` のうち最も進んだもの (何も無ければ 0)。
    fn known_position(&self, connection: &Connection) -> Result<i64, JournalReadError> {
        [
            self.checkpoints.find_max_position(connection)?,
            self.executions.find_max_as_of(connection)?,
            self.intents.find_max_as_of(connection)?,
            self.definitions.find_max_as_of(connection)?,
        ]
        .into_iter()
        .flatten()
        .max()
        .map_or(Ok(0), position_value)
    }

    /// 20 表の内容と記録を照らし合わせる (旧 `shared_projection::verify`)。
    ///
    /// 照合済みの記録は、20 表の内容のダイジェスト・変換の版・位置と世代の値域を見る。
    /// 旧いストアから持ち越した未照合の記録は、記録の位置 (と推定位置の大きいほう) までの
    /// 歴史から描き直した行と 20 表を比べ、一致したときだけ照合済みとして記録し直す。
    ///
    /// # Errors
    ///
    /// 記録が無い・食い違う (`Corrupt (ProjectionSnapshotMismatch)`)、読み書きの失敗 (`Io`)。
    pub(crate) fn verify_head(
        &self,
        transaction: &mut Transaction<'_>,
    ) -> Result<ReadModelHeadRow, JournalReadError> {
        let head = self.head.find(transaction)?.ok_or_else(snapshot_mismatch)?;
        if !head.is_verified() {
            let position = head.position().max(self.known_position(transaction)?);
            let to = GlobalSeqNr::new(u64::try_from(position).map_err(|_| snapshot_mismatch())?);
            let history = self.journal.events_through(transaction, to)?;
            if history.scanned_to().unwrap_or(GlobalSeqNr::ZERO) != to {
                return Err(snapshot_mismatch());
            }
            let expected = ReadTables::project(&history).map_err(|_| snapshot_mismatch())?;
            if !self.matches(transaction, &expected)? {
                return Err(snapshot_mismatch());
            }
            return self.record_head(transaction, position);
        }
        let actual = self.content(transaction)?.digest()?;
        if head.position() < 0
            || head.generation() <= 0
            || head.revision() != PublicationBatch::current_transform_revision()
            || head.content_digest() != actual
        {
            return Err(snapshot_mismatch());
        }
        Ok(head)
    }

    /// 20 表の現在の内容から共有面の記録を作り直す (世代を 1 つ進め、照合済みとする)。
    fn record_head(
        &self,
        transaction: &mut Transaction<'_>,
        position: i64,
    ) -> Result<ReadModelHeadRow, JournalReadError> {
        let generation = self
            .head
            .find(transaction)?
            .map_or(0, |head| head.generation())
            .checked_add(1)
            .ok_or_else(snapshot_mismatch)?;
        let head = ReadModelHeadRow::new(
            position,
            generation,
            PublicationBatch::current_transform_revision(),
            self.content(transaction)?.digest()?,
            true,
        );
        self.head.save(transaction, &head)?;
        Ok(head)
    }

    // ---- 処理したシーケンス番号 ----

    /// 投影 `projection` が処理したシーケンス番号 (未登録は `ZERO`)。
    ///
    /// 正の番号は、保存したアンカーをジャーナルの同じ位置の行と照らし合わせてから返す —
    /// 位置の振り直しやジャーナルの改変を、静かな欠落・重複ではなく明示的な
    /// `Corrupt (CheckpointAnchorMismatch)` にする。チェックポイントの表とジャーナルをまたぐ
    /// 検査なので、DAO ではなくここに置く。
    ///
    /// # Errors
    ///
    /// 読めない (`Io`)、保存値が負・アンカーが欠ける・食い違う (`Corrupt`) 場合。
    pub(crate) fn checkpoint(
        &self,
        connection: &Connection,
        projection: &ProjectionName,
    ) -> Result<GlobalSeqNr, JournalReadError> {
        let Some(saved) = self.checkpoints.find(connection, projection)? else {
            return Ok(GlobalSeqNr::ZERO);
        };
        if saved.position() == GlobalSeqNr::ZERO {
            return Ok(GlobalSeqNr::ZERO);
        }
        // 正の番号には前進が必ずアンカーを書く — 欠けは直接改変の兆候。
        let Some(expected) = saved.anchor() else {
            return Err(corrupt_error(
                NO_AGGREGATE,
                None,
                CorruptCause::CheckpointAnchorMismatch,
            ));
        };
        let actual = self.journal.anchor_at(connection, saved.position())?;
        if actual.as_ref() == Some(expected) {
            Ok(saved.position())
        } else {
            Err(corrupt_error(
                expected.aggregate_id(),
                Some(expected.seq_nr()),
                CorruptCause::CheckpointAnchorMismatch,
            ))
        }
    }

    /// 構造化面を `tables` で確定し、投影 `projection` の処理したシーケンス番号を `to` へ
    /// 進める (旧 `JournalReaderImpl::advance_on`)。呼び手のトランザクションの中で行い、
    /// 確定 (`commit`) は呼び手が決める。
    ///
    /// - 現在の番号より前へは戻さない (`CheckpointRegression`)。
    /// - ジャーナルに無い位置へは進めない — 以後の照合が必ず失敗するので、前進の時点で止める
    ///   (`ZERO` はアンカー無しの明示登録)。
    /// - 共有面は space 共有である。`to` が共有面の位置より新しければ 20 表を差し替えて記録を
    ///   作り直し、同じ位置なら `tables` が現在の 20 表と一致することを確かめ (食い違えば
    ///   `ProjectionSnapshotMismatch`)、古ければ共有面を維持する (個別の番号が遅れていても
    ///   共有の行集合は後退させない)。
    /// - 番号は表の後に保存する (原則 6 — 表が先、番号が後)。どこかで失敗すれば呼び手の
    ///   トランザクションは確定されず、表も番号も動かない。
    ///
    /// # Errors
    ///
    /// 上の拒否 (`CheckpointRegression` / `Corrupt`) と、読み書きの失敗 (`Io`)。
    pub(crate) fn advance(
        &self,
        transaction: &mut Transaction<'_>,
        projection: &ProjectionName,
        to: GlobalSeqNr,
        tables: &ReadTables,
    ) -> Result<(), JournalReadError> {
        let target = position_value(to)?;
        let current = self.checkpoint(transaction, projection)?;
        if to < current {
            return Err(JournalReadError::CheckpointRegression {
                projection: projection.clone(),
                current,
                requested: to,
            });
        }
        let anchor = if to == GlobalSeqNr::ZERO {
            None
        } else {
            Some(self.journal.anchor_at(transaction, to)?.ok_or_else(|| {
                corrupt_error(NO_AGGREGATE, None, CorruptCause::CheckpointAnchorMismatch)
            })?)
        };
        let shared = self.verify_head(transaction)?.position();
        if target == shared && !self.matches(transaction, tables)? {
            return Err(snapshot_mismatch());
        }
        if target > shared {
            self.replace(transaction, tables)?;
            self.record_head(transaction, target)?;
        }
        self.checkpoints.save(
            transaction,
            &ProjectionCheckpointRow::new(projection.clone(), to, anchor),
        )
    }

    /// 現在の全履歴から共有面を描き直す (旧 `JournalReaderImpl::rebuild_read_model` の
    /// トランザクションの中身)。個別のチェックポイントは変えない。
    ///
    /// 記録やチェックポイントが走査済みの位置より先を名乗っていれば、歴史が切り落とされた
    /// 兆候なので描かずに止める。
    ///
    /// # Errors
    ///
    /// 歴史の欠落・破損 (`Read`)、投影できない (`ReadTables`)、読み書きの失敗 (`Read`)。
    pub(crate) fn rebuild(
        &self,
        transaction: &mut Transaction<'_>,
    ) -> Result<GlobalSeqNr, ReadModelUpdateError> {
        let history = self.journal.events_after(transaction, GlobalSeqNr::ZERO)?;
        let last = history.scanned_to().unwrap_or(GlobalSeqNr::ZERO);
        let recorded = self
            .checkpoints
            .find_max_position(transaction)?
            .map_or(Ok(0), position_value)?;
        let published = match self.head.find(transaction)? {
            Some(head) => head.position(),
            None => self.known_position(transaction)?,
        };
        if position_value(last)? < recorded.max(published) {
            return Err(
                corrupt_error(NO_AGGREGATE, None, CorruptCause::CheckpointAnchorMismatch).into(),
            );
        }
        let tables = ReadTables::project(&history)?;
        self.replace(transaction, &tables)?;
        self.record_head(transaction, position_value(last)?)?;
        Ok(last)
    }

    // ---- 20 表の読み書き ----

    /// 20 表の行を `tables` に差し替える (旧 `read_tables::replace_all`)。
    ///
    /// 監査だけを描いた断面 (`ReadTables::project_audit_only`) は既存の行を消さずに書き足す
    /// (監査の 2 表は主キーで上書きする)。それ以外は全行を消してから足す。`as_of` は断面全体の
    /// 性質なので、全表に同じ値を書く。
    pub(crate) fn replace(
        &self,
        transaction: &mut Transaction<'_>,
        tables: &ReadTables,
    ) -> Result<(), JournalReadError> {
        if !tables.preserves_existing() {
            self.session_audits.delete_all(transaction)?;
            self.artifact_audits.delete_all(transaction)?;
            self.answer_results.delete_all(transaction)?;
            self.report_results.delete_all(transaction)?;
            self.jump_results.delete_all(transaction)?;
            self.definitions.delete_all(transaction)?;
            self.definition_stages.delete_all(transaction)?;
            self.definition_scopes.delete_all(transaction)?;
            self.definition_scope_keywords.delete_all(transaction)?;
            self.definition_scope_stages.delete_all(transaction)?;
            self.definition_scope_phase_entries
                .delete_all(transaction)?;
            self.intents.delete_all(transaction)?;
            self.intent_stages.delete_all(transaction)?;
            self.executions.delete_all(transaction)?;
            self.execution_stages.delete_all(transaction)?;
            self.next_answers.delete_all(transaction)?;
            self.next_jumps.delete_all(transaction)?;
            self.next_jump_phases.delete_all(transaction)?;
            self.run_stages.delete_all(transaction)?;
            self.scope_changes.delete_all(transaction)?;
        }
        let as_of = tables.as_of().unwrap_or(GlobalSeqNr::ZERO);
        self.session_audits
            .save(transaction, tables.session_audits(), as_of)?;
        self.artifact_audits
            .save(transaction, tables.artifact_audits(), as_of)?;
        self.answer_results
            .insert(transaction, tables.answer_results(), as_of)?;
        self.jump_results
            .insert(transaction, tables.jump_results(), as_of)?;
        self.report_results
            .insert(transaction, tables.report_results(), as_of)?;
        self.definitions
            .insert(transaction, tables.definitions(), as_of)?;
        self.definition_stages
            .insert(transaction, tables.definition_stages(), as_of)?;
        self.definition_scopes
            .insert(transaction, tables.definition_scopes(), as_of)?;
        self.definition_scope_keywords.insert(
            transaction,
            tables.definition_scope_keywords(),
            as_of,
        )?;
        self.definition_scope_stages.insert(
            transaction,
            tables.definition_scope_stages(),
            as_of,
        )?;
        self.definition_scope_phase_entries.insert(
            transaction,
            tables.definition_scope_phase_entries(),
            as_of,
        )?;
        self.intents.insert(transaction, tables.intents(), as_of)?;
        self.intent_stages
            .insert(transaction, tables.intent_stages(), as_of)?;
        self.executions
            .insert(transaction, tables.executions(), as_of)?;
        self.execution_stages
            .insert(transaction, tables.execution_stages(), as_of)?;
        self.next_answers
            .insert(transaction, tables.next_answers(), as_of)?;
        self.next_jumps
            .insert(transaction, tables.next_jumps(), as_of)?;
        self.run_stages
            .insert(transaction, tables.run_stages(), as_of)?;
        self.scope_changes
            .insert(transaction, tables.scope_changes(), as_of)?;
        self.next_jump_phases
            .insert(transaction, tables.next_jump_phases(), as_of)
    }

    /// 20 表の現在の内容。
    pub(crate) fn content(
        &self,
        connection: &Connection,
    ) -> Result<StructuredSurfaceContent, JournalReadError> {
        Ok(StructuredSurfaceContent::new([
            self.session_audits.find_content(connection)?,
            self.artifact_audits.find_content(connection)?,
            self.answer_results.find_content(connection)?,
            self.report_results.find_content(connection)?,
            self.jump_results.find_content(connection)?,
            self.definitions.find_content(connection)?,
            self.definition_stages.find_content(connection)?,
            self.definition_scopes.find_content(connection)?,
            self.definition_scope_keywords.find_content(connection)?,
            self.definition_scope_stages.find_content(connection)?,
            self.definition_scope_phase_entries
                .find_content(connection)?,
            self.intents.find_content(connection)?,
            self.intent_stages.find_content(connection)?,
            self.executions.find_content(connection)?,
            self.execution_stages.find_content(connection)?,
            self.next_answers.find_content(connection)?,
            self.next_jumps.find_content(connection)?,
            self.next_jump_phases.find_content(connection)?,
            self.run_stages.find_content(connection)?,
            self.scope_changes.find_content(connection)?,
        ]))
    }

    /// `tables` を書いたら現在の 20 表と同じ内容になるか。
    ///
    /// 比べるためにセーブポイントの中で実際に書き、内容を読んでから書いた行を捨てる — 行の値が
    /// 格納型まで含めてどう保存されるかの正本は DAO の書込であり、それを別に書き下さない。
    /// 呼び手のトランザクションの内容は変わらない。
    fn matches(
        &self,
        transaction: &mut Transaction<'_>,
        tables: &ReadTables,
    ) -> Result<bool, JournalReadError> {
        let actual = self.content(transaction)?;
        transaction
            .execute_batch(COMPARE_SAVEPOINT)
            .at_connection(transaction)?;
        let expected = self
            .replace(transaction, tables)
            .and_then(|()| self.content(transaction));
        let cleanup = transaction
            .execute_batch(COMPARE_ROLLBACK)
            .at_connection(transaction);
        let expected = expected?;
        cleanup?;
        Ok(actual == expected)
    }
}

#[cfg(test)]
mod tests {
    use std::io::ErrorKind;

    use rusqlite::TransactionBehavior;

    use super::*;
    use crate::orchestration::JournalBatch;
    use crate::orchestration::journal_reader_impl::tests::opened_store;
    use crate::orchestration::read_model_schema;
    use core_command_domain::workspace::StorePath;

    /// 失敗経路の試験が使う投影名。
    fn projection() -> ProjectionName {
        ProjectionName::parse("state-file").expect("投影名は kebab")
    }

    /// 前進と一緒に渡す構造化リードモデル。単調性・アンカー照合を見る試験は行の中身に依存
    /// しないので、空の履歴からの投影 (= 全表 0 行) で足りる。行の往復そのものは
    /// `tests/structured_read_model_updater_contract.rs` が見る。
    fn empty_tables() -> ReadTables {
        ReadTables::project(&JournalBatch::empty()).expect("空も投影できる")
    }

    /// 走査位置だけを名乗る空の断面 (共有面の位置を動かす試験に使う)。
    fn empty_tables_at(position: u64) -> ReadTables {
        ReadTables::project(&JournalBatch::new(
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Some(GlobalSeqNr::new(position)),
        ))
        .expect("投影")
    }

    /// 本家のストアを開き (表を作らせ)、読み面の表を用意した接続を返す。
    fn prepared(dir: &tempfile::TempDir) -> (StorePath, Connection) {
        let (store, path) = opened_store(dir);
        drop(store);
        let mut connection = Connection::open(path.as_path()).expect("接続");
        read_model_schema::prepare(&mut connection, path.as_path()).expect("表を用意する");
        (path, connection)
    }

    /// 生の SQL で表や行を壊すための接続。
    fn raw(path: &StorePath) -> Connection {
        Connection::open(path.as_path()).expect("生の接続")
    }

    /// 前進先になるジャーナル行を置く (中身は読まないので最小の材料でよい)。
    fn journal_rows(path: &StorePath, count: usize) {
        let conn = raw(path);
        for seq_nr in 1..=count {
            conn.execute(
                "INSERT INTO journal(pkey, skey, aid, seq_nr, payload, occurred_at)
                 VALUES ('p', ?1, 'intent-x', ?1, X'7B7D', 0)",
                [seq_nr.to_string()],
            )
            .expect("行を置く");
        }
    }

    /// 実物の組の手順。
    fn surface() -> StructuredSurface {
        StructuredSurface::default()
    }

    /// 更新器と同じく IMMEDIATE で開き、投影 `name` を前進して確定する。
    fn advance_as(
        connection: &mut Connection,
        name: &str,
        to: GlobalSeqNr,
        tables: &ReadTables,
    ) -> Result<(), JournalReadError> {
        let projection = ProjectionName::parse(name).expect("投影名");
        let mut transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .expect("書込トランザクション");
        surface().advance(&mut transaction, &projection, to, tables)?;
        transaction.commit().expect("確定");
        Ok(())
    }

    fn advance(
        connection: &mut Connection,
        to: GlobalSeqNr,
        tables: &ReadTables,
    ) -> Result<(), JournalReadError> {
        advance_as(connection, "state-file", to, tables)
    }

    fn assert_io_other<T: std::fmt::Debug>(result: &Result<T, JournalReadError>) {
        assert!(
            matches!(
                result,
                Err(JournalReadError::Io {
                    kind: ErrorKind::Other,
                    ..
                })
            ),
            "{result:?}"
        );
    }

    #[test]
    fn a_position_beyond_the_column_range_is_refused_before_the_query() {
        let dir = tempfile::tempdir().expect("一時 dir");
        let (_path, mut connection) = prepared(&dir);
        assert_eq!(
            advance(&mut connection, GlobalSeqNr::new(u64::MAX), &empty_tables()),
            Err(corrupt_error(
                NO_AGGREGATE,
                None,
                CorruptCause::InvariantViolation
            ))
        );
    }

    #[test]
    fn a_missing_checkpoint_table_is_reported_as_io_on_both_faces() {
        let dir = tempfile::tempdir().expect("一時 dir");
        let (path, mut connection) = prepared(&dir);
        raw(&path)
            .execute_batch("DROP TABLE amadeus_projection_checkpoint")
            .expect("表を落とす");
        assert_io_other(&surface().checkpoint(&connection, &projection()));
        assert_io_other(&advance(
            &mut connection,
            GlobalSeqNr::new(1),
            &empty_tables(),
        ));
    }

    #[test]
    fn an_unregistered_projection_reads_as_zero() {
        let dir = tempfile::tempdir().expect("一時 dir");
        let (_path, connection) = prepared(&dir);
        assert_eq!(
            surface().checkpoint(&connection, &projection()),
            Ok(GlobalSeqNr::ZERO)
        );
    }

    #[test]
    fn a_positive_checkpoint_without_an_anchor_is_a_mismatch() {
        // 正のチェックポイントには前進が必ずアンカーを書く。欠けた行は直接改変の兆候。
        let dir = tempfile::tempdir().expect("一時 dir");
        let (path, connection) = prepared(&dir);
        raw(&path)
            .execute(
                "INSERT INTO amadeus_projection_checkpoint(projection, last_global_seq)
                 VALUES ('state-file', 3)",
                [],
            )
            .expect("アンカー無しの正値を置く");
        assert_eq!(
            surface().checkpoint(&connection, &projection()),
            Err(corrupt_error(
                NO_AGGREGATE,
                None,
                CorruptCause::CheckpointAnchorMismatch
            ))
        );
    }

    #[test]
    fn a_negative_anchor_seq_nr_is_corrupt() {
        let dir = tempfile::tempdir().expect("一時 dir");
        let (path, connection) = prepared(&dir);
        raw(&path)
            .execute(
                "INSERT INTO amadeus_projection_checkpoint(projection, last_global_seq, anchor_aid, anchor_seq_nr)
                 VALUES ('state-file', 3, 'intent-x', -5)",
                [],
            )
            .expect("負のアンカーを置く");
        assert_eq!(
            surface().checkpoint(&connection, &projection()),
            Err(corrupt_error(
                "intent-x",
                None,
                CorruptCause::InvariantViolation
            ))
        );
    }

    #[test]
    fn an_anchor_that_disagrees_with_the_journal_row_is_a_mismatch() {
        // 位置の振り直し・ジャーナルの改変の兆候。保存したアンカーを添えて止める。
        let dir = tempfile::tempdir().expect("一時 dir");
        let (path, connection) = prepared(&dir);
        journal_rows(&path, 1);
        raw(&path)
            .execute(
                "INSERT INTO amadeus_projection_checkpoint(projection, last_global_seq, anchor_aid, anchor_seq_nr)
                 VALUES ('state-file', 1, 'intent-y', 1)",
                [],
            )
            .expect("食い違うアンカーを置く");
        assert_eq!(
            surface().checkpoint(&connection, &projection()),
            Err(corrupt_error(
                "intent-y",
                Some(1),
                CorruptCause::CheckpointAnchorMismatch
            ))
        );
    }

    #[test]
    fn advancing_to_zero_writes_a_row_without_an_anchor_and_reads_back_zero() {
        // ZERO は「まだ何も投影していない」の明示登録 — journal に対応行が無いので
        // アンカーも無し。読み返しは照合をスキップして ZERO を返す。
        let dir = tempfile::tempdir().expect("一時 dir");
        let (_path, mut connection) = prepared(&dir);
        advance(&mut connection, GlobalSeqNr::ZERO, &empty_tables()).expect("ZERO への前進は通る");
        assert_eq!(
            surface().checkpoint(&connection, &projection()),
            Ok(GlobalSeqNr::ZERO)
        );
    }

    #[test]
    fn advancing_writes_the_anchor_of_the_journal_row_and_reads_it_back() {
        let dir = tempfile::tempdir().expect("一時 dir");
        let (path, mut connection) = prepared(&dir);
        journal_rows(&path, 1);
        advance(&mut connection, GlobalSeqNr::new(1), &empty_tables_at(1)).expect("前進");
        assert_eq!(
            surface().checkpoint(&connection, &projection()),
            Ok(GlobalSeqNr::new(1))
        );
        let anchor: (String, i64) = raw(&path)
            .query_row(
                "SELECT anchor_aid, anchor_seq_nr FROM amadeus_projection_checkpoint",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("行");
        assert_eq!(anchor, ("intent-x".to_string(), 1));
    }

    #[test]
    fn advancing_to_a_position_not_in_the_journal_is_refused() {
        // journal に無い位置へ進めると以後の照合が必ず失敗するため、前進の時点で止める。
        let dir = tempfile::tempdir().expect("一時 dir");
        let (_path, mut connection) = prepared(&dir);
        assert_eq!(
            advance(&mut connection, GlobalSeqNr::new(1), &empty_tables()),
            Err(corrupt_error(
                NO_AGGREGATE,
                None,
                CorruptCause::CheckpointAnchorMismatch
            ))
        );
    }

    #[test]
    fn a_regression_is_refused_and_moves_nothing() {
        let dir = tempfile::tempdir().expect("一時 dir");
        let (path, mut connection) = prepared(&dir);
        journal_rows(&path, 2);
        advance(&mut connection, GlobalSeqNr::new(2), &empty_tables_at(2)).expect("前進");
        assert_eq!(
            advance(&mut connection, GlobalSeqNr::new(1), &empty_tables_at(1)),
            Err(JournalReadError::CheckpointRegression {
                projection: projection(),
                current: GlobalSeqNr::new(2),
                requested: GlobalSeqNr::new(1),
            })
        );
        assert_eq!(
            surface().checkpoint(&connection, &projection()),
            Ok(GlobalSeqNr::new(2))
        );
    }

    #[test]
    fn a_negative_checkpoint_row_is_corrupt() {
        let dir = tempfile::tempdir().expect("一時 dir");
        let (path, connection) = prepared(&dir);
        raw(&path)
            .execute(
                "INSERT INTO amadeus_projection_checkpoint(projection, last_global_seq)
                 VALUES ('state-file', -1)",
                [],
            )
            .expect("負値を置く");
        assert_eq!(
            surface().checkpoint(&connection, &projection()),
            Err(corrupt_error(
                NO_AGGREGATE,
                None,
                CorruptCause::InvariantViolation
            ))
        );
    }

    #[test]
    fn a_checkpoint_row_whose_columns_have_the_wrong_type_is_reported_as_io() {
        for row in [
            "('state-file', 3, X'FF', 3)",
            "('state-file', 'x', NULL, NULL)",
            "('state-file', 3, 'intent-x', 'not-a-number')",
        ] {
            let dir = tempfile::tempdir().expect("一時 dir");
            let (path, connection) = prepared(&dir);
            raw(&path)
                .execute(
                    &format!(
                        "INSERT INTO amadeus_projection_checkpoint(projection, last_global_seq, anchor_aid, anchor_seq_nr)
                         VALUES {row}"
                    ),
                    [],
                )
                .expect("型の違う行を置く");
            assert_io_other(&surface().checkpoint(&connection, &projection()));
        }
    }

    #[test]
    fn a_journal_row_whose_columns_have_the_wrong_type_fails_anchor_verification_as_io() {
        for journal in [
            "('p', 's', X'FF', 1, X'7B7D', 0)",
            "('p', 's', 'intent-x', 'x', X'7B7D', 0)",
        ] {
            let dir = tempfile::tempdir().expect("一時 dir");
            let (path, connection) = prepared(&dir);
            let conn = raw(&path);
            conn.execute(
                &format!(
                    "INSERT INTO journal(pkey, skey, aid, seq_nr, payload, occurred_at) VALUES {journal}"
                ),
                [],
            )
            .expect("型の違う行を置く");
            conn.execute(
                "INSERT INTO amadeus_projection_checkpoint(projection, last_global_seq, anchor_aid, anchor_seq_nr)
                 VALUES ('state-file', 1, 'intent-x', 1)",
                [],
            )
            .expect("正のチェックポイントを置く");
            assert_io_other(&surface().checkpoint(&connection, &projection()));
        }
    }

    #[test]
    fn advancing_over_a_journal_row_whose_columns_have_the_wrong_type_is_reported_as_io() {
        for journal in [
            "('p', 's', X'FF', 1, X'7B7D', 0)",
            "('p', 's', 'intent-x', 'x', X'7B7D', 0)",
        ] {
            let dir = tempfile::tempdir().expect("一時 dir");
            let (path, mut connection) = prepared(&dir);
            raw(&path)
                .execute(
                    &format!(
                        "INSERT INTO journal(pkey, skey, aid, seq_nr, payload, occurred_at) VALUES {journal}"
                    ),
                    [],
                )
                .expect("型の違う行を置く");
            assert_io_other(&advance(
                &mut connection,
                GlobalSeqNr::new(1),
                &empty_tables(),
            ));
        }
    }

    #[test]
    fn a_failing_checkpoint_write_is_reported_as_io_and_leaves_the_tables_unchanged() {
        // 番号の保存 (表の後) の失敗経路。トリガで書込を落とし、握り潰されないこと、同じ
        // トランザクションで書いた共有面の記録も確定されないことを確かめる。
        let dir = tempfile::tempdir().expect("一時 dir");
        let (path, mut connection) = prepared(&dir);
        journal_rows(&path, 1);
        raw(&path)
            .execute_batch(
                "CREATE TRIGGER checkpoint_write_fails
                 BEFORE INSERT ON amadeus_projection_checkpoint
                 BEGIN SELECT RAISE(ABORT, 'boom'); END",
            )
            .expect("書込を落とすトリガを置く");
        let head_before = surface().head(&connection).expect("記録");

        assert_io_other(&advance(
            &mut connection,
            GlobalSeqNr::new(1),
            &empty_tables_at(1),
        ));
        assert_eq!(
            surface().head(&connection).expect("記録"),
            head_before,
            "共有面の記録も動かない"
        );
    }

    #[test]
    fn an_older_projection_keeps_the_newer_shared_surface() {
        // 共有面は space 共有である。個別の番号が遅れていても、共有の行集合は後退させない。
        let dir = tempfile::tempdir().expect("一時 dir");
        let (path, mut connection) = prepared(&dir);
        journal_rows(&path, 2);
        advance(&mut connection, GlobalSeqNr::new(2), &empty_tables_at(2)).expect("前進");
        let head = surface().head(&connection).expect("記録").expect("在る");
        assert_eq!(head.position(), 2);

        advance_as(
            &mut connection,
            "other",
            GlobalSeqNr::new(1),
            &empty_tables_at(1),
        )
        .expect("古い位置への前進");
        assert_eq!(
            surface().head(&connection).expect("記録"),
            Some(head),
            "共有面の記録は動かない"
        );
        let as_of: Vec<i64> = raw(&path)
            .prepare("SELECT DISTINCT as_of FROM read_intent")
            .expect("文")
            .query_map([], |row| row.get(0))
            .expect("引ける")
            .map(Result::unwrap)
            .collect();
        assert!(as_of.is_empty(), "空の断面なので行は無い");
    }

    #[test]
    fn a_tampered_shared_surface_fails_verification() {
        let dir = tempfile::tempdir().expect("一時 dir");
        let (path, mut connection) = prepared(&dir);
        advance(&mut connection, GlobalSeqNr::ZERO, &empty_tables()).expect("前進");
        raw(&path)
            .execute(
                "INSERT INTO read_scope_change(id, execution_id, scope, kind, as_of)
                 VALUES ('tampered', 'e', 's', 'k', 0)",
                [],
            )
            .expect("記録に無い行を足す");
        let mut transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .expect("Tx");
        assert_eq!(
            surface().verify_head(&mut transaction),
            Err(snapshot_mismatch())
        );
    }

    #[test]
    fn a_same_position_candidate_that_differs_from_the_shared_rows_is_refused() {
        let dir = tempfile::tempdir().expect("一時 dir");
        let (path, mut connection) = prepared(&dir);
        advance(&mut connection, GlobalSeqNr::ZERO, &empty_tables()).expect("前進");
        // 共有面 (位置 0) と同じ位置で、行の違う候補を作る — 記録と照合したうえで、候補を
        // 書いた内容が現在の 20 表と一致しないので確定しない。
        let mut transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .expect("Tx");
        let candidate = ReadTables::project_audit_only(&JournalBatch::new(
            Vec::new(),
            Vec::new(),
            Vec::new(),
            None,
        ))
        .expect("投影");
        assert_eq!(
            surface().advance(
                &mut transaction,
                &projection(),
                GlobalSeqNr::ZERO,
                &candidate
            ),
            Ok(()),
            "中身が同じ (空) なら通る"
        );
        drop(transaction);
        raw(&path)
            .execute(
                "INSERT INTO read_scope_change(id, execution_id, scope, kind, as_of)
                 VALUES ('shared-only', 'e', 's', 'k', 0)",
                [],
            )
            .expect("共有面にだけ在る行");
        // 記録のダイジェストを今の内容に合わせ直す (照合は通し、一致の検査だけを見る)。
        let mut transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .expect("Tx");
        surface()
            .record_head(&mut transaction, 0)
            .expect("記録し直す");
        assert_eq!(
            surface().advance(
                &mut transaction,
                &projection(),
                GlobalSeqNr::ZERO,
                &empty_tables()
            ),
            Err(snapshot_mismatch())
        );
    }
}
