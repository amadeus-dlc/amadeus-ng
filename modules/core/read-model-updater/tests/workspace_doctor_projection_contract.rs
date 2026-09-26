//! 自己診断の投影契約 — ジャーナルの診断事実から `read_doctor_*` の行を作る。
#![allow(clippy::unwrap_used, clippy::indexing_slicing)]
use chrono::{DateTime, Utc};
use core_command_domain::workspace::{
    DefinitionAssets, DoctorObservation, HeartbeatObservation, HookHealthTarget, HookWiring,
    NativeEntryPoints, SpaceName, StorePath, WorkspaceDoctor, WorkspaceDoctorEvent, WorkspaceShell,
};
use core_infrastructure::collections::FirstClassCollection as _;
use core_read_model_updater::orchestration::{ReadModelUpdater, WorkspaceDoctorReadModelUpdater};
use event_store_adapter_rs::EventStoreForSqlite;
use event_store_adapter_rs::event_envelope::EventEnvelope;
use event_store_adapter_rs::types::{AggregateId, EventStore};
use serde::{Deserialize, Serialize};

const MANIFEST: &str = "workspace-doctor-event/1";

fn at() -> DateTime<Utc> {
    "2026-09-12T00:00:00Z".parse().unwrap()
}

fn target(space: &str) -> HookHealthTarget {
    HookHealthTarget::new(SpaceName::parse(space).unwrap(), None)
}

fn observation(bun_found: bool) -> DoctorObservation {
    DoctorObservation::new(
        bun_found,
        NativeEntryPoints::new(
            Ok("/opt/aidlc/target/release/aidlc".to_string()),
            Vec::new(),
        ),
        HookWiring::new(
            true,
            Ok(Vec::new()),
            None,
            false,
            Ok(Vec::new()),
            Vec::new(),
            core_command_domain::workspace::HookBindingDeclaration::Absent,
        ),
        HeartbeatObservation::new(false, false, Vec::new(), 0, false, None),
        WorkspaceShell::new(true, true),
        DefinitionAssets::new(
            Ok(Vec::new()),
            Ok(Vec::new()),
            Ok(Vec::new()),
            Ok(Vec::new()),
            Ok(Vec::new()),
        ),
        None,
    )
}

// テストが書く行は**本家のイベントストアが実際に書いた行**である (期待値を手で書き下さない)。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
struct KeyDto(String);
impl std::fmt::Display for KeyDto {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl AggregateId for KeyDto {
    fn type_name(&self) -> String {
        "WorkspaceDoctor".to_string()
    }
    fn value(&self) -> String {
        self.0.clone()
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CheckDto {
    check_id: String,
    passed: bool,
    label: String,
    fix: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct EventDto {
    id: String,
    aggregate_id: String,
    target: String,
    checks: Vec<CheckDto>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SnapshotDto {
    id: String,
    seq_nr: usize,
}

fn event_dto(event: &WorkspaceDoctorEvent) -> EventDto {
    EventDto {
        id: event.id().to_string(),
        aggregate_id: event.aggregate_id().to_string(),
        target: event.target().relative_directory(),
        checks: event.checks().fold_left(Vec::new(), |mut rows, check| {
            rows.push(CheckDto {
                check_id: check.id().as_str().to_string(),
                passed: check.is_passed(),
                label: check.label().to_string(),
                fix: check.fix().map(str::to_string),
            });
            rows
        }),
    }
}

type Store = EventStoreForSqlite<KeyDto, SnapshotDto, EventDto>;

async fn append(store: &mut Store, doctor: &WorkspaceDoctor, event: &WorkspaceDoctorEvent) {
    let key = KeyDto(doctor.id().as_str().to_string());
    let envelope = EventEnvelope::new(
        key,
        doctor.seq_nr(),
        doctor.diagnosed_at(),
        event_dto(event),
    )
    .with_manifest(MANIFEST);
    store
        .persist_event_and_snapshot(
            envelope,
            SnapshotDto {
                id: doctor.id().as_str().to_string(),
                seq_nr: doctor.seq_nr(),
            },
            doctor.version(),
        )
        .await
        .unwrap();
}

struct Projected {
    connection: rusqlite::Connection,
}

impl Projected {
    fn report(&self, id: &str) -> (String, i64, i64, i64, i64) {
        self.connection
            .query_row(
                "SELECT target, passed, failed, exit_code, seq_nr FROM read_doctor_report WHERE id=?1",
                [id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
            )
            .unwrap()
    }
    fn checks(&self, report_id: &str) -> Vec<(i64, String, i64, String, Option<String>)> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT position, check_id, passed, label, fix FROM read_doctor_check
                 WHERE report_id=?1 ORDER BY position",
            )
            .unwrap();
        let rows = statement
            .query_map([report_id], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            })
            .unwrap();
        rows.map(Result::unwrap).collect()
    }
    fn count(&self, table: &str) -> i64 {
        self.connection
            .query_row(&format!("SELECT count(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .unwrap()
    }
}

/// 診断の履歴を本家のストアへ書く (更新器は走らせない)。
///
/// `aggregates` は試験装置が握る集約の状態で、呼び出しをまたいで続きから書ける。
async fn record(
    path: &std::path::Path,
    aggregates: &mut Vec<WorkspaceDoctor>,
    history: &[(HookHealthTarget, bool)],
) {
    let mut store = Store::new(path).unwrap();
    for (index, (target, bun_found)) in history.iter().enumerate() {
        let moment = at() + chrono::Duration::seconds(index as i64);
        match aggregates
            .iter_mut()
            .find(|doctor| doctor.target() == target)
        {
            Some(doctor) => {
                let event = doctor.diagnose(&observation(*bun_found), moment).unwrap();
                append(&mut store, doctor, &event).await;
                // 楽観版はストアが進める — 再構成せずに続ける試験装置がその歩みを写す。
                *doctor = doctor.clone().with_version(doctor.seq_nr());
            }
            None => {
                let (doctor, event) =
                    WorkspaceDoctor::start(target.clone(), &observation(*bun_found), moment)
                        .unwrap();
                append(&mut store, &doctor, &event).await;
                aggregates.push(doctor.with_version(1));
            }
        }
    }
}

/// 更新器を開き直して 1 回走らせる (再起動と同じ — 前回の状態はリードモデルにしか無い)。
async fn update(path: &std::path::Path) {
    WorkspaceDoctorReadModelUpdater::open(path)
        .unwrap()
        .update_read_models()
        .await
        .unwrap();
}

async fn project(
    path: &std::path::Path,
    history: &[(HookHealthTarget, bool)],
) -> (Projected, Vec<WorkspaceDoctor>) {
    let mut aggregates: Vec<WorkspaceDoctor> = Vec::new();
    record(path, &mut aggregates, history).await;
    update(path).await;
    (
        Projected {
            connection: rusqlite::Connection::open(path).unwrap(),
        },
        aggregates,
    )
}

#[tokio::test]
async fn a_single_diagnosis_becomes_one_report_row_and_its_checks() {
    let temp = tempfile::tempdir().unwrap();
    let path = StorePath::for_runtime(temp.path());
    let (projected, aggregates) = project(path.as_path(), &[(target("default"), true)]).await;
    let doctor = &aggregates[0];
    let id = doctor.id().as_str();
    assert_eq!(
        projected.report(id),
        (
            "spaces/default/intents".to_string(),
            i64::try_from(doctor.checks().passed()).unwrap(),
            i64::try_from(doctor.checks().failed()).unwrap(),
            i64::from(doctor.checks().exit_code()),
            1,
        )
    );
    let rows = projected.checks(id);
    assert_eq!(rows.len(), doctor.checks().len());
    assert_eq!(rows[0].0, 0);
    assert_eq!(rows[0].3, doctor.checks().at(0).unwrap().label());
}

#[tokio::test]
async fn the_rows_keep_the_aggregates_display_order_labels_and_fixes() {
    let temp = tempfile::tempdir().unwrap();
    let path = StorePath::for_runtime(temp.path());
    let (projected, aggregates) = project(path.as_path(), &[(target("default"), false)]).await;
    let doctor = &aggregates[0];
    let expected: Vec<(i64, String, i64, String, Option<String>)> =
        doctor.checks().fold_left(Vec::new(), |mut rows, check| {
            let position = i64::try_from(rows.len()).unwrap();
            rows.push((
                position,
                check.id().as_str().to_string(),
                i64::from(check.is_passed()),
                check.label().to_string(),
                check.fix().map(str::to_string),
            ));
            rows
        });
    assert_eq!(projected.checks(doctor.id().as_str()), expected);
    assert!(
        expected.iter().any(|row| row.2 == 0 && row.4.is_some()),
        "失敗行は fix を持つ: {expected:?}"
    );
}

#[tokio::test]
async fn the_counts_and_exit_code_are_burned_in_so_the_query_side_never_counts() {
    let temp = tempfile::tempdir().unwrap();
    let path = StorePath::for_runtime(temp.path());
    let (projected, aggregates) = project(path.as_path(), &[(target("default"), false)]).await;
    let (_, passed, failed, exit_code, _) = projected.report(aggregates[0].id().as_str());
    let rows = projected.checks(aggregates[0].id().as_str());
    assert_eq!(passed, rows.iter().filter(|row| row.2 == 1).count() as i64);
    assert_eq!(failed, rows.iter().filter(|row| row.2 == 0).count() as i64);
    assert_eq!(exit_code, 1, "必須失敗があれば 1");
    assert!(failed > 0);
}

#[tokio::test]
async fn a_later_diagnosis_replaces_the_rows_of_the_same_target() {
    let temp = tempfile::tempdir().unwrap();
    let path = StorePath::for_runtime(temp.path());
    let (projected, aggregates) = project(
        path.as_path(),
        &[(target("default"), false), (target("default"), true)],
    )
    .await;
    let id = aggregates[0].id().as_str();
    let (_, _, failed, _, seq_nr) = projected.report(id);
    assert_eq!(seq_nr, 2, "最新の診断だけが行になる");
    assert_eq!(
        failed,
        i64::try_from(aggregates[0].checks().failed()).unwrap(),
        "行は 2 回目の診断の答えである"
    );
    let bun = projected
        .checks(id)
        .into_iter()
        .find(|row| row.3.starts_with("bun "))
        .unwrap();
    assert_eq!(bun.2, 1, "1 回目の失敗行が残っていない");
    assert_eq!(projected.count("read_doctor_report"), 1);
    assert_eq!(
        projected.count("read_doctor_check"),
        i64::try_from(aggregates[0].checks().len()).unwrap()
    );
}

#[tokio::test]
async fn two_targets_get_two_reports_that_do_not_share_check_rows() {
    let temp = tempfile::tempdir().unwrap();
    let path = StorePath::for_runtime(temp.path());
    let (projected, aggregates) = project(
        path.as_path(),
        &[(target("default"), true), (target("team-a"), false)],
    )
    .await;
    assert_eq!(projected.count("read_doctor_report"), 2);
    let first = projected.checks(aggregates[0].id().as_str());
    let second = projected.checks(aggregates[1].id().as_str());
    assert_eq!(first.len(), second.len());
    assert_eq!(
        projected.report(aggregates[0].id().as_str()).0,
        "spaces/default/intents"
    );
    assert_eq!(
        projected.report(aggregates[1].id().as_str()).0,
        "spaces/team-a/intents"
    );
    assert_eq!(
        projected.report(aggregates[0].id().as_str()).2 + 1,
        projected.report(aggregates[1].id().as_str()).2,
        "bun を欠いた側だけ失敗が 1 本多い"
    );
}

#[tokio::test]
async fn running_the_update_twice_leaves_the_same_rows() {
    let temp = tempfile::tempdir().unwrap();
    let path = StorePath::for_runtime(temp.path());
    let (projected, aggregates) = project(path.as_path(), &[(target("default"), true)]).await;
    let before = projected.checks(aggregates[0].id().as_str());
    let rows_before = projected.count("read_doctor_check");
    WorkspaceDoctorReadModelUpdater::open(path.as_path())
        .unwrap()
        .update_read_models()
        .await
        .unwrap();
    assert_eq!(projected.checks(aggregates[0].id().as_str()), before);
    assert_eq!(projected.count("read_doctor_check"), rows_before);
}

#[tokio::test]
async fn the_checkpoint_names_the_latest_projected_sequence() {
    let temp = tempfile::tempdir().unwrap();
    let path = StorePath::for_runtime(temp.path());
    let (projected, _) = project(
        path.as_path(),
        &[(target("default"), true), (target("default"), true)],
    )
    .await;
    let last: i64 = projected
        .connection
        .query_row(
            "SELECT last_seq FROM workspace_doctor_projection_checkpoint WHERE projection='workspace-doctor'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(last, 2);
}

#[tokio::test]
async fn a_row_whose_payload_is_not_ours_is_corrupt_rather_than_skipped() {
    let temp = tempfile::tempdir().unwrap();
    let path = StorePath::for_runtime(temp.path());
    // 処理したシーケンス番号より後の行だけが読まれるので、まだ処理していない行を壊す。
    record(
        path.as_path(),
        &mut Vec::new(),
        &[(target("default"), true)],
    )
    .await;
    let connection = rusqlite::Connection::open(path.as_path()).unwrap();
    connection
        .execute(
            "UPDATE journal SET payload=?1 WHERE manifest=?2",
            rusqlite::params![br#"{"nope":1}"#.to_vec(), MANIFEST],
        )
        .unwrap();
    drop(connection);
    let error = WorkspaceDoctorReadModelUpdater::open(path.as_path())
        .unwrap()
        .update_read_models()
        .await
        .unwrap_err();
    assert!(
        format!("{error}").contains("undecodable payload"),
        "実際: {error}"
    );
}

/// 保存された処理済みシーケンス番号 (ジャーナル上の位置)。
fn saved_checkpoint(connection: &rusqlite::Connection) -> i64 {
    connection
        .query_row(
            "SELECT last_seq FROM workspace_doctor_projection_checkpoint WHERE projection='workspace-doctor'",
            [],
            |row| row.get(0),
        )
        .unwrap()
}

/// 行が書き直されたかを見分けるための印を、報告の行に直接置く。
fn mark(connection: &rusqlite::Connection, id: &str) {
    connection
        .execute(
            "UPDATE read_doctor_report SET exit_code = 99 WHERE id = ?1",
            [id],
        )
        .unwrap();
}

#[tokio::test]
async fn the_checkpoint_is_the_journal_position_rather_than_the_aggregate_sequence() {
    let temp = tempfile::tempdir().unwrap();
    let path = StorePath::for_runtime(temp.path());
    // 2 つの対象がそれぞれ 1 件ずつ — 集約内の通番はどちらも 1、ジャーナル上の位置は 1 と 2。
    let (projected, _) = project(
        path.as_path(),
        &[(target("default"), true), (target("team-a"), true)],
    )
    .await;
    assert_eq!(saved_checkpoint(&projected.connection), 2);
}

#[tokio::test]
async fn a_restarted_updater_starts_after_the_saved_sequence_and_touches_only_new_facts() {
    let temp = tempfile::tempdir().unwrap();
    let path = StorePath::for_runtime(temp.path());
    let mut aggregates = Vec::new();
    record(
        path.as_path(),
        &mut aggregates,
        &[(target("default"), true)],
    )
    .await;
    update(path.as_path()).await;
    let connection = rusqlite::Connection::open(path.as_path()).unwrap();
    assert_eq!(saved_checkpoint(&connection), 1);
    let default_id = aggregates[0].id().as_str().to_string();
    mark(&connection, &default_id);

    // 再起動 (更新器を開き直す) までの間に、別の対象の診断が 1 件増える。
    record(
        path.as_path(),
        &mut aggregates,
        &[(target("team-a"), false)],
    )
    .await;
    update(path.as_path()).await;

    let projected = Projected { connection };
    assert_eq!(
        saved_checkpoint(&projected.connection),
        2,
        "次の番号まで進む"
    );
    assert_eq!(
        projected.report(&default_id).3,
        99,
        "処理済みの事実しか持たない集約の行は書き直さない"
    );
    let team_a = aggregates[1].id().as_str();
    assert_eq!(
        projected.report(team_a).2,
        i64::try_from(aggregates[1].checks().failed()).unwrap(),
        "新しい事実は投影される"
    );
    assert_eq!(projected.checks(team_a).len(), aggregates[1].checks().len());
}

#[tokio::test]
async fn a_new_fact_for_a_known_target_is_replayed_from_its_whole_history() {
    let temp = tempfile::tempdir().unwrap();
    let path = StorePath::for_runtime(temp.path());
    let mut aggregates = Vec::new();
    record(
        path.as_path(),
        &mut aggregates,
        &[(target("default"), false)],
    )
    .await;
    update(path.as_path()).await;
    record(
        path.as_path(),
        &mut aggregates,
        &[(target("default"), true)],
    )
    .await;
    update(path.as_path()).await;
    let projected = Projected {
        connection: rusqlite::Connection::open(path.as_path()).unwrap(),
    };
    let id = aggregates[0].id().as_str();
    let (_, _, failed, exit_code, seq_nr) = projected.report(id);
    assert_eq!(seq_nr, 2, "集約は誕生から起こし直されて通番 2 になる");
    assert_eq!(
        failed,
        i64::try_from(aggregates[0].checks().failed()).unwrap()
    );
    assert_eq!(exit_code, i64::from(aggregates[0].checks().exit_code()));
    assert_eq!(saved_checkpoint(&projected.connection), 2);
    assert_eq!(projected.count("read_doctor_report"), 1);
}

#[tokio::test]
async fn an_update_without_new_facts_writes_nothing() {
    let temp = tempfile::tempdir().unwrap();
    let path = StorePath::for_runtime(temp.path());
    let (projected, aggregates) = project(path.as_path(), &[(target("default"), true)]).await;
    let id = aggregates[0].id().as_str();
    mark(&projected.connection, id);
    update(path.as_path()).await;
    update(path.as_path()).await;
    assert_eq!(projected.report(id).3, 99, "冪等 — 同じ事実を二度書かない");
    assert_eq!(saved_checkpoint(&projected.connection), 1);
}

#[tokio::test]
async fn a_batch_that_fails_midway_moves_neither_table_nor_the_checkpoint() {
    let temp = tempfile::tempdir().unwrap();
    let path = StorePath::for_runtime(temp.path());
    let (projected, mut aggregates) = project(path.as_path(), &[(target("default"), false)]).await;
    let id = aggregates[0].id().as_str().to_string();
    let before = projected.report(&id);
    let checks_before = projected.checks(&id);
    // 報告の行を書いた後、診断行の書込で落ちるようにする (トランザクションの途中の失敗)。
    projected
        .connection
        .execute_batch(
            "CREATE TRIGGER refuse_checks BEFORE INSERT ON read_doctor_check
             BEGIN SELECT RAISE(ABORT, 'refused'); END;",
        )
        .unwrap();
    record(
        path.as_path(),
        &mut aggregates,
        &[(target("default"), true)],
    )
    .await;
    let error = WorkspaceDoctorReadModelUpdater::open(path.as_path())
        .unwrap()
        .update_read_models()
        .await
        .unwrap_err();
    assert!(format!("{error}").starts_with("io:"), "実際: {error}");
    assert_eq!(projected.report(&id), before, "報告の行は確定されていない");
    assert_eq!(projected.checks(&id), checks_before, "診断行も元のまま");
    assert_eq!(saved_checkpoint(&projected.connection), 1, "番号も進まない");

    // 原因を取り除けば、同じ事実を次の実行で処理する (取りこぼさない)。
    projected
        .connection
        .execute_batch("DROP TRIGGER refuse_checks")
        .unwrap();
    update(path.as_path()).await;
    assert_eq!(projected.report(&id).4, 2);
    assert_eq!(saved_checkpoint(&projected.connection), 2);
}

#[tokio::test]
async fn the_update_waits_for_a_write_lock_held_by_another_connection() {
    // #134 の教訓の回帰。更新器は処理したシーケンス番号とジャーナルを読んでから書く。
    // DEFERRED で始めると、別の接続が書込ロックを握っている間は、読んだ後の書込昇格が
    // busy timeout を待たずに即 `SQLITE_BUSY` になる。IMMEDIATE なら最初に書込ロックを
    // 待ち、解放後に書く。
    //
    // ホルダは HOLD を「主スレッドが更新を呼ぶ直前」の合図 (合図 2) から数え、呼び出しが
    // ロック待ちを実際に観測したことを所要時間 (下限 = HOLD の半分) で確かめる
    // (`reference_surface_updater_contract.rs` の
    // `the_pipeline_update_waits_for_a_write_lock_held_by_another_connection` と同じ立て付け)。
    const HOLD: std::time::Duration = std::time::Duration::from_millis(200);
    const MIN_OBSERVED_WAIT: std::time::Duration = std::time::Duration::from_millis(100);
    let temp = tempfile::tempdir().unwrap();
    let path = StorePath::for_runtime(temp.path());
    let mut aggregates = Vec::new();
    record(
        path.as_path(),
        &mut aggregates,
        &[(target("default"), true)],
    )
    .await;
    // 表を先に作っておく。表が無いと最初の文 (`CREATE TABLE`) が読むより先に書込ロックを
    // 取りに行くので、DEFERRED でも待ててしまい、IMMEDIATE の効果を見分けられない。
    update(path.as_path()).await;
    record(path.as_path(), &mut aggregates, &[(target("team-a"), true)]).await;
    let mut updater = WorkspaceDoctorReadModelUpdater::open(path.as_path()).unwrap();

    let (locked_sender, locked_receiver) = std::sync::mpsc::channel::<()>();
    let (calling_sender, calling_receiver) = std::sync::mpsc::channel::<()>();
    let holder_path = path.as_path().to_path_buf();
    let holder = std::thread::spawn(move || {
        let connection = rusqlite::Connection::open(&holder_path).unwrap();
        connection.execute_batch("BEGIN IMMEDIATE").unwrap();
        locked_sender.send(()).unwrap();
        if calling_receiver.recv().is_ok() {
            std::thread::sleep(HOLD);
        }
        connection.execute_batch("END").unwrap();
    });
    locked_receiver.recv().unwrap();

    calling_sender.send(()).unwrap();
    let started = std::time::Instant::now();
    let result = updater.update_read_models().await;
    let waited = started.elapsed();
    drop(calling_sender);
    holder.join().unwrap();

    assert_eq!(result, Ok(()), "書込ロックの解放を待って書く");
    assert!(
        waited >= MIN_OBSERVED_WAIT,
        "ロック待ちを観測していない (所要 {waited:?})"
    );
    let projected = Projected {
        connection: rusqlite::Connection::open(path.as_path()).unwrap(),
    };
    assert_eq!(projected.count("read_doctor_report"), 2);
    assert_eq!(saved_checkpoint(&projected.connection), 2);
}
