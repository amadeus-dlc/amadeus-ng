//! 読み面の表の用意 — ストアを開く段で `read_*` 表と構造化面の管理表を揃え、読み面の版を守る
//! **1 か所** (公開型ゼロの内部モジュール)。
//!
//! # なぜ 1 か所に集めるか
//!
//! 表の DDL の正本はそれぞれの表の DAO であり、表を書く更新器は開く段で自分の表を揃える
//! (`SteeringReadModelUpdater::open` ほか)。それとは別に、ストアを開く段で**すべての**読み面の
//! 表を揃える場所が 1 つ要る。理由は 2 つある。
//!
//! 1. **クエリ側は、面ごとの更新器がまだ一度も走っていないストアでも表を引く** (例: 実行の
//!    無い段階の `next` が `read_pipeline_progress` を引く)。表が無いと「行が無い」ではなく
//!    読取の失敗になる。
//! 2. **読み面の版 (`PRAGMA user_version`) は読み面全体の性質である**。版が動いたら列の形が
//!    変わった表を落として作り直す必要があるが、どの表の形が変わったかは版からは分からない
//!    ので、すべての `read_*` 表を落として作り直す。落とした表は、面ごとの更新器が次に
//!    走るまで待たずに、ここで空の表として作り直す (1 と同じ理由)。
//!
//! 呼ぶのは構造化面の更新器の開く段 ([`super::StructuredReadModelUpdater::open`]) だけである。
//! 構造化面の更新器は、ストアを読み書きするどの経路 (取得ループ・初回の定義の用意・テスト
//! 契約や計画承認の前の共有面の点検) でも最初に開かれる。以前はこの用意を
//! `JournalReaderImpl::open` が持っていた (Issue #153 の PR2 で入れた暫定) が、ジャーナルの
//! 読み手はリードモデル側の表を知らないのが正しい形なので、ここへ移した。
//!
//! # 書込ロックは要るときだけ取る
//!
//! 版が現行で、表がすべて揃い、共有面の記録も在れば、`sqlite_master` と `PRAGMA` と記録の行を
//! 読むだけで戻る (書込トランザクションを開かない)。欠けがあるか版が動いていれば
//! `BEGIN IMMEDIATE` で開く (DEFERRED で開くと、読んでから書込へ昇格するときに busy timeout を
//! 待たずに即失敗しうる — #134)。
//!
//! # 版が動いたときの作り直し
//!
//! 行の正本はジャーナルなので、読み面は捨てて描き直せる。作り直しは次を**1 つの IMMEDIATE
//! トランザクション**で行う — 途中で失敗すれば何も変わらず、版も上がらない (次に開いたときに
//! 同じ作り直しをやり直す)。
//!
//! 1. 投影が 1 度でも進んだストア (進んだチェックポイントが在る) なら、ジャーナルの全履歴を
//!    読んで構造化面の行を組む。描けない歴史 (切り落とし・復号不能) は作り直しでも直らない
//!    ので、ここで止める。未投影のストアは描く中身が無く、次の更新が普通に描く。
//! 2. `read_*` 表をすべて落とし (構造化面の 20 表と参照入力由来の 6 表)、作り直す。
//! 3. 1 で組んだ行を構造化面へ書く。参照入力由来の表は空のまま — 面ごとの更新器が保存済みの
//!    出所を `None` と見て描き直す。
//! 4. 版を記録し、共有面の記録を未照合へ戻す (次の書込の前に歴史と照合させる)。
//!
//! チェックポイントは戻さない — Markdown 面と共有の位置であり、戻すと未投影区間が全履歴に
//! なって監査シャードに同じブロックが二度並ぶ。

use std::path::Path;

use rusqlite::{Connection, TransactionBehavior};

use crate::read_tables::ReadTables;

use super::journal_reader_impl::corrupt_error;
use super::store_failure::{InStore as _, SqliteResultExt};
use super::structured_surface::StructuredSurface;
use super::{
    CodeGenerationApprovalDao as _, CodeGenerationApprovalDaoImpl, CorruptCause, GlobalSeqNr,
    JournalReadError, PipelineProgressDao as _, PipelineProgressDaoImpl, PlanFingerprintDao as _,
    PlanFingerprintDaoImpl, ReadSchemaVersionDao as _, ReadSchemaVersionDaoImpl,
    SteeringPartDao as _, SteeringPartDaoImpl, SteeringPlanDao as _, SteeringPlanDaoImpl,
    StructuredJournalReader as _, TestingContractDao as _, TestingContractDaoImpl,
};

/// 読み面スキーマの版 (`PRAGMA user_version` に保存する値)。
///
/// **列の形を変えたら必ず 1 つ上げる。** `CREATE TABLE IF NOT EXISTS` は既存の表には何もしない
/// ので、列を足す・落とす・型を変える改訂は旧スキーマの表が残ったままになり、`INSERT` が
/// `no such column` で落ちる (b47 の `read_next_answer.gated INTEGER` → `gate TEXT` が実例)。
/// 版が動いていれば [`prepare`] が落として作り直す。
///
/// 行の正本はジャーナルであって読み面ではないので、**作り直しは情報を失わない**。
/// 「後方互換を残さない」(`coding-rules/no-backward-compatibility.md`) はコードの規則であり、
/// 機械が読む媒体を捨てて描き直すのはその帰結である。
pub(super) const READ_SCHEMA_VERSION: i64 = 7;

/// 集約に属さない値の識別子欄に置く印。
const NO_AGGREGATE: &str = "-";

/// 読み面の表を揃え、版が動いていれば作り直す (モジュール doc を参照)。
///
/// # Errors
///
/// 読めない・書けない (`Io`)、版が動いたストアの歴史を描き直せない (`Corrupt`) 場合。
pub(super) fn prepare(connection: &mut Connection, path: &Path) -> Result<(), JournalReadError> {
    prepare_tables(connection, path).in_store(path)
}

/// [`prepare`] の本体 (失敗の場所は呼び手が開いたストアの綴りに揃える前のもの)。
fn prepare_tables(connection: &mut Connection, path: &Path) -> Result<(), JournalReadError> {
    let surface: StructuredSurface = StructuredSurface::default();
    let version = ReadSchemaVersionDaoImpl;
    if version.find(connection)? == READ_SCHEMA_VERSION
        && surface.tables_exist(connection)?
        && reference_tables_exist(connection)?
        && surface.head(connection)?.is_some()
    {
        return Ok(());
    }
    let mut transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .at_store(path)?;
    if version.find(&transaction)? == READ_SCHEMA_VERSION {
        surface.create_tables(&mut transaction)?;
        create_reference_tables(&mut transaction)?;
        surface.ensure_head(&mut transaction)?;
        return transaction.commit().at_store(path);
    }
    // 版が動いた。先に歴史から行を組む (描けなければ何も落とさずに止める)。旧い形の表へは
    // DDL を打たない — 落としてから作り直す。
    let tables = if surface.projected_before(&transaction)? {
        let history = surface
            .journal()
            .events_after(&transaction, GlobalSeqNr::ZERO)?;
        Some(
            ReadTables::project(&history)
                .map_err(|_| corrupt_error(NO_AGGREGATE, None, CorruptCause::InvariantViolation))?,
        )
    } else {
        None
    };
    surface.drop_tables(&mut transaction)?;
    drop_reference_tables(&mut transaction)?;
    surface.create_tables(&mut transaction)?;
    create_reference_tables(&mut transaction)?;
    if let Some(tables) = &tables {
        surface.replace(&mut transaction, tables)?;
    }
    version.save(&mut transaction, READ_SCHEMA_VERSION)?;
    surface.ensure_head(&mut transaction)?;
    surface.invalidate_head(&mut transaction)?;
    transaction.commit().at_store(path)
}

/// 参照入力由来の 6 表 (steering 2 表・テスト契約・計画指紋・Code Generation 開始可否・
/// Pipeline 進捗) が揃っているか。書込ロックを取らない。
fn reference_tables_exist(connection: &Connection) -> Result<bool, JournalReadError> {
    Ok(SteeringPlanDaoImpl.table_exists(connection)?
        && SteeringPartDaoImpl.table_exists(connection)?
        && TestingContractDaoImpl.table_exists(connection)?
        && PlanFingerprintDaoImpl.table_exists(connection)?
        && CodeGenerationApprovalDaoImpl.table_exists(connection)?
        && PipelineProgressDaoImpl.table_exists(connection)?)
}

/// 参照入力由来の 6 表を (無ければ) 作る。DDL の正本はそれぞれの表の DAO である。
fn create_reference_tables(
    transaction: &mut rusqlite::Transaction<'_>,
) -> Result<(), JournalReadError> {
    SteeringPlanDaoImpl.create_table(transaction)?;
    SteeringPartDaoImpl.create_table(transaction)?;
    TestingContractDaoImpl.create_table(transaction)?;
    PlanFingerprintDaoImpl.create_table(transaction)?;
    CodeGenerationApprovalDaoImpl.create_table(transaction)?;
    PipelineProgressDaoImpl.create_table(transaction)
}

/// 参照入力由来の 6 表を落とす (版が動いたときだけ)。
fn drop_reference_tables(
    transaction: &mut rusqlite::Transaction<'_>,
) -> Result<(), JournalReadError> {
    SteeringPlanDaoImpl.drop_table(transaction)?;
    SteeringPartDaoImpl.drop_table(transaction)?;
    TestingContractDaoImpl.drop_table(transaction)?;
    PlanFingerprintDaoImpl.drop_table(transaction)?;
    CodeGenerationApprovalDaoImpl.drop_table(transaction)?;
    PipelineProgressDaoImpl.drop_table(transaction)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::orchestration::journal_reader_impl::tests::opened_store;

    /// 本家のストアを開き (表を作らせ)、その場所へつないだ接続を返す。
    fn store(dir: &tempfile::TempDir) -> (std::path::PathBuf, Connection) {
        let (store, path) = opened_store(dir);
        drop(store);
        let connection = Connection::open(path.as_path()).expect("接続");
        (path.as_path().to_path_buf(), connection)
    }

    fn count(connection: &Connection, table: &str) -> i64 {
        connection
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .expect("数えられる")
    }

    #[test]
    fn a_fresh_store_gets_every_table_the_current_version_and_an_unverified_record() {
        let dir = tempfile::tempdir().expect("一時 dir");
        let (path, mut connection) = store(&dir);
        prepare(&mut connection, &path).expect("用意する");
        let surface: StructuredSurface = StructuredSurface::default();
        assert!(surface.tables_exist(&connection).expect("読める"));
        assert!(reference_tables_exist(&connection).expect("読める"));
        assert_eq!(
            ReadSchemaVersionDaoImpl.find(&connection),
            Ok(READ_SCHEMA_VERSION)
        );
        let head = surface
            .head(&connection)
            .expect("読める")
            .expect("記録が在る");
        assert!(!head.is_verified(), "次の書込の前に歴史と照合させる");
        let tables = connection
            .prepare(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name LIKE 'read_%'",
            )
            .expect("文")
            .query_row([], |row| row.get::<_, i64>(0))
            .expect("数えられる");
        assert_eq!(tables, 26, "構造化面の 20 表と参照入力由来の 6 表");
    }

    #[test]
    fn preparing_a_store_whose_tables_exist_does_not_wait_for_a_write_lock() {
        // 読取だけの動詞も開く段を通る。表が在るのに書込ロックを取りに行くと、別の書き手が
        // いる間は busy timeout まで待たされ、最後は `WouldBlock` で落ちる。
        let dir = tempfile::tempdir().expect("一時 dir");
        let (path, mut connection) = store(&dir);
        prepare(&mut connection, &path).expect("初回は作る");
        connection
            .busy_timeout(Duration::from_millis(20))
            .expect("待ち時間");
        let holder = Connection::open(&path).expect("握る側の接続");
        holder.execute_batch("BEGIN IMMEDIATE").expect("書込ロック");

        let result = prepare(&mut connection, &path);

        holder.execute_batch("END").expect("手放す");
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn a_missing_table_is_created_again_under_a_write_lock() {
        let dir = tempfile::tempdir().expect("一時 dir");
        let (path, mut connection) = store(&dir);
        prepare(&mut connection, &path).expect("初回は作る");
        connection
            .execute_batch("DROP TABLE read_plan_fingerprint; DROP TABLE read_run_stage")
            .expect("2 表を落とす");
        assert!(!PlanFingerprintDaoImpl.table_exists(&connection).unwrap());
        prepare(&mut connection, &path).expect("作り直す");
        assert!(PlanFingerprintDaoImpl.table_exists(&connection).unwrap());
        assert_eq!(count(&connection, "read_run_stage"), 0, "空の表として在る");
    }

    #[test]
    fn a_missing_record_of_the_shared_surface_is_put_back() {
        let dir = tempfile::tempdir().expect("一時 dir");
        let (path, mut connection) = store(&dir);
        prepare(&mut connection, &path).expect("初回は作る");
        connection
            .execute_batch("DELETE FROM amadeus_read_model_head")
            .expect("記録を消す");
        prepare(&mut connection, &path).expect("置き直す");
        let surface: StructuredSurface = StructuredSurface::default();
        assert!(surface.head(&connection).expect("読める").is_some());
    }

    #[test]
    fn a_version_change_discards_the_reference_rows_and_keeps_the_checkpoints() {
        // 版は読み面全体の性質である。参照入力由来の表も落として空で作り直す (面ごとの
        // 更新器が保存済みの出所を `None` と見て描き直す)。チェックポイントは Markdown 面と
        // 共有の位置なので戻さない。
        let dir = tempfile::tempdir().expect("一時 dir");
        let (path, mut connection) = store(&dir);
        prepare(&mut connection, &path).expect("初回は作る");
        connection.execute_batch(
            "INSERT INTO read_testing_contract (id,contract,rendered,error,source_digest,as_of) VALUES ('bare-space','old','old',NULL,'old',1);
             INSERT INTO read_plan_fingerprint (id,execution_id,target_id,fingerprint,error,source_digest,as_of) VALUES ('old','execution','stage:code-generation','old',NULL,'old',1);
             INSERT INTO amadeus_projection_checkpoint(projection, last_global_seq) VALUES ('state-file', 0);
             PRAGMA user_version = 0;",
        )
        .expect("旧い版の断面を作る");

        prepare(&mut connection, &path).expect("作り直す");

        for table in ["read_testing_contract", "read_plan_fingerprint"] {
            assert_eq!(count(&connection, table), 0, "{table} も作り直しの対象");
        }
        assert_eq!(count(&connection, "amadeus_projection_checkpoint"), 1);
        assert_eq!(
            ReadSchemaVersionDaoImpl.find(&connection),
            Ok(READ_SCHEMA_VERSION)
        );
    }
}
