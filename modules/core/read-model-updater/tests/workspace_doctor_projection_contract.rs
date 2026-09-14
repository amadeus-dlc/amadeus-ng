//! 自己診断の投影契約 — ジャーナルの診断事実から `read_doctor_*` の行を作る。
#![allow(clippy::unwrap_used, clippy::indexing_slicing)]
use chrono::{DateTime, Utc};
use core_command_domain::workspace::{
    DefinitionAssets, DoctorObservation, HeartbeatObservation, HookHealthTarget, HookWiring,
    NativeEntryPoints, SpaceName, StorePath, WorkspaceDoctor, WorkspaceDoctorEvent, WorkspaceShell,
};
use core_infrastructure::collections::FirstClassCollection as _;
use core_read_model_updater::orchestration::WorkspaceDoctorReadModelUpdater;
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

async fn project(
    path: &std::path::Path,
    history: &[(HookHealthTarget, bool)],
) -> (Projected, Vec<WorkspaceDoctor>) {
    let mut store = Store::new(path).unwrap();
    let mut aggregates: Vec<WorkspaceDoctor> = Vec::new();
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
    WorkspaceDoctorReadModelUpdater::open(path)
        .unwrap()
        .catch_up()
        .unwrap();
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
async fn the_projection_is_a_full_recomputation_so_running_it_twice_is_the_same() {
    let temp = tempfile::tempdir().unwrap();
    let path = StorePath::for_runtime(temp.path());
    let (projected, aggregates) = project(path.as_path(), &[(target("default"), true)]).await;
    let before = projected.checks(aggregates[0].id().as_str());
    let rows_before = projected.count("read_doctor_check");
    WorkspaceDoctorReadModelUpdater::open(path.as_path())
        .unwrap()
        .catch_up()
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
    let (_, aggregates) = project(path.as_path(), &[(target("default"), true)]).await;
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
        .catch_up()
        .unwrap_err();
    assert!(
        format!("{error}").contains("undecodable payload"),
        "実際: {error}"
    );
    let _ = aggregates;
}
