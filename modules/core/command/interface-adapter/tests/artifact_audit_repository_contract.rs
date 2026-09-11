//! ArtifactSavedのSQLite保存・再構成契約。
#![allow(clippy::unwrap_used)]
use chrono::{DateTime, Utc};
use core_command_domain::workspace::ArtifactAuditEvent;
use core_command_domain::workspace::{
    ArtifactAudit, ArtifactWriteObservation, HookHealthTarget, IntentDirName, SpaceName,
};
use core_command_interface_adapter::orchestration::ArtifactAuditRepositoryImpl;
use core_command_use_case::orchestration::ArtifactAuditRepository;
#[tokio::test]
async fn saved_artifacts_survive_reopen_and_keep_one_event_per_command() {
    let dir = tempfile::tempdir().unwrap();
    let path = core_command_domain::workspace::StorePath::for_runtime(dir.path());
    let target = HookHealthTarget::new(
        SpaceName::parse("default").unwrap(),
        Some(IntentDirName::parse("260909-artifact").unwrap()),
    );
    let at: DateTime<Utc> = "2026-09-09T01:00:00Z".parse().unwrap();
    let first = ArtifactWriteObservation::new(
        target.clone(),
        "Write".into(),
        "/p/a.md".into(),
        "construction > code".into(),
        true,
    );
    let (agg, created) = ArtifactAudit::start(first, at).unwrap();
    let mut repo = ArtifactAuditRepositoryImpl::open(&path).unwrap();
    repo.store(&created, &agg).await.unwrap();
    let database = rusqlite::Connection::open(path.as_path()).unwrap();
    let first_snapshot: Vec<u8> = database
        .query_row("SELECT payload FROM snapshot", [], |row| row.get(0))
        .unwrap();
    let mut agg = repo.find_by_id(agg.id()).await.unwrap();
    let second = ArtifactWriteObservation::new(
        target,
        "Edit".into(),
        "/p/a.md".into(),
        "construction > code".into(),
        false,
    );
    let event = agg
        .record(second, at + chrono::Duration::seconds(1))
        .unwrap();
    repo.store(&event, &agg).await.unwrap();
    let restored = repo.find_by_id(agg.id()).await.unwrap();
    assert_eq!(restored.seq_nr(), 2);
    assert_eq!(restored.last_tool(), "Edit");
    assert!(matches!(created, ArtifactAuditEvent::Saved(_)));

    // スナップショットだけを古い時点へ戻し、後続イベントの再構成経路を実駆動する。
    // 楽観versionとジャーナルは保存済みのまま保持する。
    database
        .execute(
            "UPDATE snapshot SET payload = ?1, seq_nr = 1",
            [first_snapshot],
        )
        .unwrap();
    let events_before: i64 = database
        .query_row("SELECT COUNT(*) FROM journal", [], |row| row.get(0))
        .unwrap();
    let reopened = ArtifactAuditRepositoryImpl::open(&path).unwrap();
    let replayed = reopened.find_by_id(agg.id()).await.unwrap();
    assert_eq!(replayed, restored);
    assert_eq!(
        database
            .query_row("SELECT COUNT(*) FROM journal", [], |row| row
                .get::<_, i64>(0))
            .unwrap(),
        events_before,
        "再構成は保存済み履歴を増やさない"
    );
}

async fn common_contract<R: ArtifactAuditRepository>(mut repository: R) {
    use core_command_use_case::orchestration::RepositoryError;
    let at: DateTime<Utc> = "2026-09-09T01:00:00Z".parse().unwrap();
    let target = HookHealthTarget::new(
        SpaceName::default(),
        Some(IntentDirName::parse("260909-audit").unwrap()),
    );
    let observation = ArtifactWriteObservation::new(
        target,
        "Write".into(),
        "/a.md".into(),
        "construction".into(),
        true,
    );
    let (initial, event) = ArtifactAudit::start(observation.clone(), at).unwrap();
    assert!(matches!(
        repository.find_by_id(initial.id()).await,
        Err(RepositoryError::NotFound { .. })
    ));
    repository.store(&event, &initial).await.unwrap();
    assert!(matches!(
        repository.store(&event, &initial).await,
        Err(RepositoryError::Conflict {
            expected: 0,
            actual: 1
        })
    ));
    let mut held = repository.find_by_id(initial.id()).await.unwrap();
    assert_eq!(held, initial.with_version(1));
    let mut stale = held.clone();
    let next = held
        .record(observation.clone(), at + chrono::Duration::seconds(1))
        .unwrap();
    repository.store(&next, &held).await.unwrap();
    let late = stale.record(observation, at).unwrap();
    assert!(matches!(
        repository.store(&late, &stale).await,
        Err(RepositoryError::Conflict {
            expected: 1,
            actual: 2
        })
    ));
    let restored = repository.find_by_id(held.id()).await.unwrap();
    assert_eq!(restored, held.with_version(2));
    let target = HookHealthTarget::new(
        SpaceName::default(),
        Some(IntentDirName::parse("260909-other").unwrap()),
    );
    let (_, foreign) = ArtifactAudit::start(
        ArtifactWriteObservation::new(
            target,
            "Write".into(),
            "/b.md".into(),
            "construction".into(),
            true,
        ),
        at,
    )
    .unwrap();
    assert!(matches!(
        repository.store(&foreign, &restored).await,
        Err(RepositoryError::Corrupt { .. })
    ));
    assert_eq!(
        repository.find_by_id(restored.id()).await.unwrap(),
        restored
    );
}
#[tokio::test]
async fn sqlite_satisfies_the_common_repository_contract() {
    let temp = tempfile::tempdir().unwrap();
    let path = core_command_domain::workspace::StorePath::for_runtime(temp.path());
    common_contract(ArtifactAuditRepositoryImpl::open(&path).unwrap()).await;
}

#[tokio::test]
async fn memory_satisfies_the_same_repository_contract() {
    common_contract(ArtifactAuditRepositoryImpl::in_memory()).await;
}
