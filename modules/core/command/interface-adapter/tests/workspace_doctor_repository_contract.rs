//! `WorkspaceDoctor` の保存 → 再構成 → 再生契約。
//!
//! 格納先は一時ストア (プロセス内の共有キャッシュ SQLite) 1 種類しか無い — 診断は正本の
//! ストアへ書かないので、永続ストアを開く口は存在しない (オーナー裁定 2026-09-12)。
//! 保存・再構成・replay・楽観版・不在時の `NotFound` は媒体に依らない契約なので、ここで
//! 固定するのはその振る舞いである。
#![allow(clippy::unwrap_used, clippy::indexing_slicing)]
use chrono::{DateTime, Utc};
use core_command_domain::workspace::{
    DefinitionAssets, DoctorObservation, HeartbeatObservation, HookHealthTarget, HookWiring,
    NativeEntryPoints, SpaceName, WorkspaceDoctor, WorkspaceDoctorId, WorkspaceShell,
};
use core_command_interface_adapter::orchestration::WorkspaceDoctorRepositoryImpl;
use core_command_use_case::orchestration::{RepositoryError, WorkspaceDoctorRepository};
use core_infrastructure::collections::FirstClassCollection as _;

fn at() -> DateTime<Utc> {
    "2026-09-12T00:00:00Z".parse().unwrap()
}

fn target() -> HookHealthTarget {
    HookHealthTarget::new(SpaceName::parse("default").unwrap(), None)
}

fn other_target() -> HookHealthTarget {
    HookHealthTarget::new(SpaceName::parse("team-a").unwrap(), None)
}

/// 最小の観測。`bun_found` だけを振って行の成否を変える。
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

fn labels(doctor: &WorkspaceDoctor) -> Vec<String> {
    doctor.checks().fold_left(Vec::new(), |mut rows, check| {
        rows.push(format!(
            "{} {}",
            if check.is_passed() { "ok" } else { "ng" },
            check.label()
        ));
        rows
    })
}

#[tokio::test]
async fn a_first_diagnosis_is_persisted_and_replayed_with_its_rows() {
    let mut repository = WorkspaceDoctorRepositoryImpl::open_ephemeral().unwrap();
    let (doctor, event) = WorkspaceDoctor::start(target(), &observation(true), at()).unwrap();
    repository.store(&event, &doctor).await.unwrap();

    let restored = repository.find_by_id(doctor.id()).await.unwrap();
    assert_eq!(restored.seq_nr(), 1);
    assert_eq!(restored.target(), doctor.target());
    assert_eq!(restored.checks(), doctor.checks());
    assert_eq!(restored.diagnosed_at(), at());
    assert!(
        labels(&restored)
            .iter()
            .any(|row| row.starts_with("ok bun"))
    );
}

#[tokio::test]
async fn a_second_diagnosis_advances_the_sequence_and_replaces_the_rows() {
    let mut repository = WorkspaceDoctorRepositoryImpl::open_ephemeral().unwrap();
    let (doctor, first) = WorkspaceDoctor::start(target(), &observation(true), at()).unwrap();
    repository.store(&first, &doctor).await.unwrap();

    let mut loaded = repository.find_by_id(doctor.id()).await.unwrap();
    let later = at() + chrono::Duration::seconds(30);
    let second = loaded.diagnose(&observation(false), later).unwrap();
    repository.store(&second, &loaded).await.unwrap();

    let restored = repository.find_by_id(doctor.id()).await.unwrap();
    assert_eq!(restored.seq_nr(), 2);
    assert_eq!(restored.diagnosed_at(), later);
    assert!(
        labels(&restored)
            .iter()
            .any(|row| row.starts_with("ng bun"))
    );
    assert_eq!(restored.checks(), second.checks());
}

#[tokio::test]
async fn an_unknown_target_is_not_found_rather_than_an_empty_report() {
    let repository = WorkspaceDoctorRepositoryImpl::open_ephemeral().unwrap();
    let id = WorkspaceDoctorId::for_target(&target());
    assert!(matches!(
        repository.find_by_id(&id).await,
        Err(RepositoryError::NotFound { .. })
    ));
}

#[tokio::test]
async fn storing_the_same_event_twice_is_a_version_conflict() {
    let mut repository = WorkspaceDoctorRepositoryImpl::open_ephemeral().unwrap();
    let (doctor, event) = WorkspaceDoctor::start(target(), &observation(true), at()).unwrap();
    repository.store(&event, &doctor).await.unwrap();
    assert!(
        matches!(
            repository.store(&event, &doctor).await,
            Err(RepositoryError::Conflict { .. })
        ),
        "同じ版で二度書けない"
    );
}

#[tokio::test]
async fn an_event_from_another_aggregate_is_refused_before_it_reaches_the_store() {
    let mut repository = WorkspaceDoctorRepositoryImpl::open_ephemeral().unwrap();
    let (doctor, _) = WorkspaceDoctor::start(target(), &observation(true), at()).unwrap();
    let (_, foreign) = WorkspaceDoctor::start(other_target(), &observation(true), at()).unwrap();
    assert!(matches!(
        repository.store(&foreign, &doctor).await,
        Err(RepositoryError::Corrupt { .. })
    ));
}

#[tokio::test]
async fn the_journal_row_names_the_workspace_doctor_manifest() {
    let mut repository = WorkspaceDoctorRepositoryImpl::open_ephemeral().unwrap();
    let (doctor, event) = WorkspaceDoctor::start(target(), &observation(true), at()).unwrap();
    repository.store(&event, &doctor).await.unwrap();
    let db = rusqlite::Connection::open_with_flags(
        repository.location(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY
            | rusqlite::OpenFlags::SQLITE_OPEN_URI
            | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .unwrap();
    let (manifest, occurred_at): (String, i64) = db
        .query_row(
            "SELECT manifest, occurred_at FROM journal WHERE aid=?1 AND seq_nr=1",
            [doctor.id().as_str()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(manifest, "workspace-doctor-event/1");
    assert_eq!(occurred_at, at().timestamp_nanos_opt().unwrap());
}

#[tokio::test]
async fn an_ephemeral_store_runs_the_same_flow_without_creating_a_file() {
    let temp = tempfile::tempdir().unwrap();
    let before = std::fs::read_dir(temp.path()).unwrap().count();
    let mut repository = WorkspaceDoctorRepositoryImpl::open_ephemeral().unwrap();
    let (doctor, event) = WorkspaceDoctor::start(target(), &observation(true), at()).unwrap();
    repository.store(&event, &doctor).await.unwrap();
    let restored = repository.find_by_id(doctor.id()).await.unwrap();
    assert_eq!(restored.checks(), doctor.checks());
    assert_eq!(
        std::fs::read_dir(temp.path()).unwrap().count(),
        before,
        "一時ストアはファイルを作らない"
    );
    assert!(
        !std::path::Path::new(repository.location()).exists(),
        "場所は実ファイルではない: {}",
        repository.location().display()
    );
}

#[tokio::test]
async fn the_ephemeral_location_is_readable_from_another_connection_of_the_same_process() {
    let mut repository = WorkspaceDoctorRepositoryImpl::open_ephemeral().unwrap();
    let (doctor, event) = WorkspaceDoctor::start(target(), &observation(true), at()).unwrap();
    repository.store(&event, &doctor).await.unwrap();
    let reader = rusqlite::Connection::open_with_flags(
        repository.location(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY
            | rusqlite::OpenFlags::SQLITE_OPEN_URI
            | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .unwrap();
    let manifest: String = reader
        .query_row(
            "SELECT manifest FROM journal WHERE aid=?1",
            [doctor.id().as_str()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(manifest, "workspace-doctor-event/1");
    let other = WorkspaceDoctorRepositoryImpl::open_ephemeral().unwrap();
    assert_ne!(
        other.location(),
        repository.location(),
        "一時ストアは実行ごとに別のデータベース"
    );
}
