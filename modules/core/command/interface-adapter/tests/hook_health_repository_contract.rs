//! HookHealthの保存→再オープン→再生契約。
#![allow(clippy::unwrap_used)]
use chrono::{DateTime, Utc};
use core_command_domain::workspace::StorePath;
use core_command_domain::workspace::{
    HookHealth, HookHealthEvent, HookHealthTarget, HookName, SpaceName,
};
use core_command_interface_adapter::orchestration::HookHealthRepositoryImpl;
use core_command_use_case::orchestration::HookHealthRepository;
#[tokio::test]
async fn heartbeat_and_drop_are_persisted_as_separate_events_and_replayed() {
    let temp = tempfile::tempdir().unwrap();
    let path = StorePath::for_runtime(temp.path());
    let target = HookHealthTarget::new(SpaceName::parse("default").unwrap(), None);
    let hook = HookName::parse("write-audit-log").unwrap();
    let at: DateTime<Utc> = "2026-09-08T01:00:00Z".parse().unwrap();
    let (health, started) = HookHealth::start(target, hook, at).unwrap();
    let mut repository = HookHealthRepositoryImpl::open(&path).unwrap();
    repository.store(&started, &health).await.unwrap();
    let mut loaded = repository.find_by_id(health.id()).await.unwrap();
    let heartbeat = loaded
        .observe_heartbeat(at + chrono::Duration::seconds(1))
        .unwrap();
    repository.store(&heartbeat, &loaded).await.unwrap();
    let mut loaded = repository.find_by_id(health.id()).await.unwrap();
    let dropped = loaded
        .record_drop("EISDIR\\nwrite failed", at + chrono::Duration::seconds(2))
        .unwrap();
    repository.store(&dropped, &loaded).await.unwrap();
    let restored = repository.find_by_id(health.id()).await.unwrap();
    assert_eq!(restored.seq_nr(), 3);
    assert_eq!(restored.drops(), 1);
    assert_eq!(
        restored.latest_drop().unwrap().as_str(),
        "EISDIR\\nwrite failed"
    );
    assert!(matches!(started, HookHealthEvent::Started(_)));
    let db = rusqlite::Connection::open(path.as_path()).unwrap();
    let drop_at: i64 = db
        .query_row(
            "SELECT occurred_at FROM journal WHERE aid=?1 AND seq_nr=3",
            [health.id().as_str()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        drop_at,
        (at + chrono::Duration::seconds(2))
            .timestamp_nanos_opt()
            .unwrap(),
        "dropの時刻を直前のheartbeatへ戻さない"
    );
}

async fn common_contract<R: HookHealthRepository>(mut repository: R) {
    use core_command_use_case::orchestration::RepositoryError;
    let at: DateTime<Utc> = "2026-09-09T01:00:00Z".parse().unwrap();
    let (initial, event) = HookHealth::start(
        HookHealthTarget::new(SpaceName::default(), None),
        HookName::parse("write-audit-log").unwrap(),
        at,
    )
    .unwrap();
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
    let stale = held.clone();
    let next = held
        .observe_heartbeat(at + chrono::Duration::seconds(1))
        .unwrap();
    repository.store(&next, &held).await.unwrap();
    let mut stale = stale;
    let event = stale.record_drop("late", at).unwrap();
    assert!(matches!(
        repository.store(&event, &stale).await,
        Err(RepositoryError::Conflict {
            expected: 1,
            actual: 2
        })
    ));
    let restored = repository.find_by_id(held.id()).await.unwrap();
    assert_eq!(restored, held.with_version(2));
    let (_, foreign) = HookHealth::start(
        HookHealthTarget::new(SpaceName::default(), None),
        HookName::parse("record-human-turn").unwrap(),
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

async fn first_drop_contract<R: HookHealthRepository>(mut repository: R) {
    use core_command_use_case::orchestration::RecordHookDropUseCase;
    let target = HookHealthTarget::new(SpaceName::default(), None);
    let hook = HookName::parse("session-end").unwrap();
    let id = core_command_domain::workspace::HookHealthId::for_hook(&target, &hook);
    let at: DateTime<Utc> = "2026-09-09T01:00:00Z".parse().unwrap();
    // 同じrepositoryを借りるportの実装はないため、ここでは公開集約→保存の共通契約を確認する。
    let (initial, event) =
        HookHealth::start_with_drop(target.clone(), hook.clone(), "unknown stamp", at).unwrap();
    repository.store(&event, &initial).await.unwrap();
    let mut restored = repository.find_by_id(&id).await.unwrap();
    assert_eq!(restored.heartbeat(), None);
    assert_eq!(restored.drops(), 1);
    let next = restored
        .record_drop("another", at + chrono::Duration::seconds(1))
        .unwrap();
    repository.store(&next, &restored).await.unwrap();
    let last = repository.find_by_id(&id).await.unwrap();
    assert_eq!(last.heartbeat(), None);
    assert_eq!(last.drops(), 2);
    assert_eq!(last.observed_at(), at + chrono::Duration::seconds(1));
    // 更新ユースケースも同じ保存ポートを消費して後続dropを追加できる。
    RecordHookDropUseCase::new(repository)
        .execute(&target, &hook, "third", at + chrono::Duration::seconds(2))
        .await
        .unwrap();
}

#[tokio::test]
async fn sqlite_accepts_a_first_drop_without_fabricating_a_heartbeat() {
    let temporary = tempfile::tempdir().unwrap();
    first_drop_contract(
        HookHealthRepositoryImpl::open(&StorePath::for_runtime(temporary.path())).unwrap(),
    )
    .await;
}

#[tokio::test]
async fn memory_accepts_the_same_first_drop_contract() {
    first_drop_contract(HookHealthRepositoryImpl::in_memory()).await;
}
#[tokio::test]
async fn sqlite_satisfies_the_common_repository_contract() {
    let temp = tempfile::tempdir().unwrap();
    let path = StorePath::for_runtime(temp.path());
    common_contract(HookHealthRepositoryImpl::open(&path).unwrap()).await;
}

#[tokio::test]
async fn memory_satisfies_the_same_repository_contract() {
    common_contract(HookHealthRepositoryImpl::in_memory()).await;
}
