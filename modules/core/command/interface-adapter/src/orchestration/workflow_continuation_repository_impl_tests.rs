//! 同じ本家ストアAPIでsnapshotと差分イベントの契約を両backendへ課す。
use super::*;
use core_command_domain::orchestration::{
    ContinuationAttemptId, ContinuationPublicationObservation, ContinuationRequest,
    ContinuationSignature, IntentExecutionId,
};

fn genesis(suffix: &str) -> (WorkflowContinuation, WorkflowContinuationEvent) {
    let id = WorkflowContinuationId::for_execution(
        &IntentExecutionId::parse(&format!("0191aaaa-bbbb-7ccc-9ddd-eeeeffff{suffix}")).unwrap(),
    );
    let request = ContinuationRequest::new(
        ContinuationAttemptId::generate(),
        Some(
            ContinuationSignature::parse(&format!(
                "reverse-engineering::{}::{}",
                "a".repeat(64),
                "b".repeat(64)
            ))
            .unwrap(),
        ),
        false,
        2,
    )
    .unwrap();
    WorkflowContinuation::start(id, request, chrono::Utc::now()).unwrap()
}

async fn history_contract<S>(mut repository: WorkflowContinuationRepositoryImpl<S>)
where
    S: EventStore<
            AID = WorkflowContinuationKeyDto,
            A = WorkflowContinuationDto,
            P = WorkflowContinuationEventDto,
        > + Clone,
{
    let (aggregate, event) = genesis("0001");
    repository.store(&event, &aggregate).await.unwrap();
    let mut aggregate = repository.find_by_id(aggregate.id()).await.unwrap();
    let event = aggregate
        .record_publication(
            &ContinuationPublicationObservation::new(aggregate.last_request().id().clone(), false),
            chrono::Utc::now(),
        )
        .unwrap();
    let mut store = repository.store.clone();
    let key = WorkflowContinuationKeyDto::of(aggregate.id());
    store
        .persist_event(
            EventEnvelope::new(
                key.clone(),
                2,
                aggregate.occurred_at(),
                WorkflowContinuationEventDto::of(&event),
            )
            .with_manifest(MANIFEST),
            1,
        )
        .await
        .unwrap();
    let restored = repository.find_by_id(aggregate.id()).await.unwrap();
    assert_eq!(restored.counter().published(), Some(false));
    assert_eq!(restored.seq_nr(), 2);
    assert_eq!(
        store
            .get_events_by_id_since_seq_nr(&key, 1)
            .await
            .unwrap()
            .len(),
        2
    );

    let (aggregate, event) = genesis("0002");
    let (foreign, _) = genesis("0003");
    store
        .persist_event_and_snapshot(
            EventEnvelope::new(
                WorkflowContinuationKeyDto::of(aggregate.id()),
                1,
                aggregate.occurred_at(),
                WorkflowContinuationEventDto::of(&event),
            )
            .with_manifest(MANIFEST),
            WorkflowContinuationDto::of(&foreign),
            0,
        )
        .await
        .unwrap();
    assert!(
        matches!(
            repository.find_by_id(aggregate.id()).await,
            Err(RepositoryError::Corrupt { .. })
        ),
        "要求IDと別の有効snapshotを返さない"
    );
}

#[tokio::test]
async fn memory_checks_delta_replay_and_snapshot_identity() {
    history_contract(WorkflowContinuationRepositoryImpl::in_memory()).await;
}

#[tokio::test]
async fn sqlite_checks_delta_replay_and_snapshot_identity() {
    let root = tempfile::tempdir().unwrap();
    history_contract(
        WorkflowContinuationRepositoryImpl::open(&StorePath::for_runtime(root.path())).unwrap(),
    )
    .await;
}

fn malformed<T: serde::Serialize + serde::de::DeserializeOwned>(dto: &T, key: &str) -> T {
    use core_infrastructure::canon_json::{SerializationProfile, serialize, to_value};
    let text = serialize(
        &to_value(dto).unwrap(),
        SerializationProfile::ContractCompact,
    );
    let mut value: serde_json::Value = serde_json::from_str(&text).unwrap();
    value.as_object_mut().unwrap().insert(
        key.into(),
        serde_json::Value::String("unknown-event".into()),
    );
    serde_json::from_value(value).unwrap()
}

fn with_exhausted_sequence<T: serde::Serialize + serde::de::DeserializeOwned>(dto: &T) -> T {
    use core_infrastructure::canon_json::{SerializationProfile, serialize, to_value};
    let text = serialize(
        &to_value(dto).unwrap(),
        SerializationProfile::ContractCompact,
    );
    let mut value: serde_json::Value = serde_json::from_str(&text).unwrap();
    value
        .as_object_mut()
        .unwrap()
        .insert("seq_nr".into(), serde_json::Value::from(u64::MAX));
    serde_json::from_value(value).unwrap()
}

fn next_publication(aggregate: &mut WorkflowContinuation) -> WorkflowContinuationEvent {
    aggregate
        .record_publication(
            &ContinuationPublicationObservation::new(aggregate.last_request().id().clone(), false),
            chrono::Utc::now(),
        )
        .unwrap()
}

async fn corruption_contract<S>(mut repository: WorkflowContinuationRepositoryImpl<S>)
where
    S: EventStore<
            AID = WorkflowContinuationKeyDto,
            A = WorkflowContinuationDto,
            P = WorkflowContinuationEventDto,
        > + Clone,
{
    let mut store = repository.store.clone();

    // 種別が異なる履歴を、同じ集約型として適用しない。
    let (aggregate, event) = genesis("0011");
    repository.store(&event, &aggregate).await.unwrap();
    let mut aggregate = repository.find_by_id(aggregate.id()).await.unwrap();
    let event = next_publication(&mut aggregate);
    store
        .persist_event(
            EventEnvelope::new(
                WorkflowContinuationKeyDto::of(aggregate.id()),
                2,
                aggregate.occurred_at(),
                WorkflowContinuationEventDto::of(&event),
            )
            .with_manifest("foreign-event/1"),
            1,
        )
        .await
        .unwrap();
    assert!(matches!(
        repository.find_by_id(aggregate.id()).await,
        Err(RepositoryError::Corrupt {
            seq_nr: Some(2),
            ..
        })
    ));

    // DTOとして読めても、ドメインへ戻せないイベントは Corrupt として拒否する。
    let (aggregate, event) = genesis("0012");
    repository.store(&event, &aggregate).await.unwrap();
    let mut aggregate = repository.find_by_id(aggregate.id()).await.unwrap();
    let event = next_publication(&mut aggregate);
    store
        .persist_event(
            EventEnvelope::new(
                WorkflowContinuationKeyDto::of(aggregate.id()),
                2,
                aggregate.occurred_at(),
                malformed(&WorkflowContinuationEventDto::of(&event), "aggregate_id"),
            )
            .with_manifest(MANIFEST),
            1,
        )
        .await
        .unwrap();
    assert!(matches!(
        repository.find_by_id(aggregate.id()).await,
        Err(RepositoryError::Corrupt {
            seq_nr: Some(2),
            ..
        })
    ));

    // DTOとして読めても、ドメインへ戻せないsnapshotは Corrupt (seq 不明) として拒否する。
    let (aggregate, event) = genesis("0013");
    store
        .persist_event_and_snapshot(
            EventEnvelope::new(
                WorkflowContinuationKeyDto::of(aggregate.id()),
                1,
                aggregate.occurred_at(),
                WorkflowContinuationEventDto::of(&event),
            )
            .with_manifest(MANIFEST),
            malformed(&WorkflowContinuationDto::of(&aggregate), "id"),
            0,
        )
        .await
        .unwrap();
    assert!(matches!(
        repository.find_by_id(aggregate.id()).await,
        Err(RepositoryError::Corrupt { seq_nr: None, .. })
    ));

    // 通番が尽きたsnapshotの後ろに差分を探しに行かず、Corrupt (seq 不明) にする。
    let (aggregate, event) = genesis("0017");
    store
        .persist_event_and_snapshot(
            EventEnvelope::new(
                WorkflowContinuationKeyDto::of(aggregate.id()),
                1,
                aggregate.occurred_at(),
                WorkflowContinuationEventDto::of(&event),
            )
            .with_manifest(MANIFEST),
            with_exhausted_sequence(&WorkflowContinuationDto::of(&aggregate)),
            0,
        )
        .await
        .unwrap();
    assert!(matches!(
        repository.find_by_id(aggregate.id()).await,
        Err(RepositoryError::Corrupt { seq_nr: None, .. })
    ));

    // 別集約のイベントを、この集約の履歴として書かない。
    let (aggregate, _) = genesis("0014");
    let (_, foreign_event) = genesis("0015");
    assert!(matches!(
        repository.store(&foreign_event, &aggregate).await,
        Err(RepositoryError::Corrupt {
            seq_nr: Some(1),
            ..
        })
    ));

    // 新規作成 (seq 1) に読取済み版を添える契約違反は、成功に丸めず失敗として返す。
    let (aggregate, event) = genesis("0016");
    assert!(matches!(
        repository.store(&event, &aggregate.with_version(1)).await,
        Err(RepositoryError::Io {
            kind: ErrorKind::Other,
            ..
        })
    ));
}

#[tokio::test]
async fn memory_rejects_corrupt_history_and_foreign_writes() {
    corruption_contract(WorkflowContinuationRepositoryImpl::in_memory()).await;
}

#[tokio::test]
async fn sqlite_rejects_corrupt_history_and_foreign_writes() {
    let root = tempfile::tempdir().unwrap();
    corruption_contract(
        WorkflowContinuationRepositoryImpl::open(&StorePath::for_runtime(root.path())).unwrap(),
    )
    .await;
}

#[tokio::test]
async fn sqlite_reports_the_store_location_on_failures() {
    let temp = tempfile::tempdir().unwrap();
    let path = StorePath::for_runtime(temp.path());
    let mut repository = WorkflowContinuationRepositoryImpl::open(&path).unwrap();
    let (aggregate, event) = genesis("0021");
    repository.store(&event, &aggregate).await.unwrap();
    let mut current = repository.find_by_id(aggregate.id()).await.unwrap();
    let next = next_publication(&mut current);
    let db = rusqlite::Connection::open(path.as_path()).unwrap();

    // 他の書き手が占有している媒体は WouldBlock として返し、再実行で解ける分類にする。
    db.execute_batch("BEGIN EXCLUSIVE").unwrap();
    assert!(matches!(
        repository.find_by_id(aggregate.id()).await,
        Err(RepositoryError::Io {
            kind: ErrorKind::WouldBlock,
            path: Some(ref reported)
        }) if reported == path.as_path()
    ));
    db.execute_batch("COMMIT").unwrap();

    // 媒体の追記先が失われていれば、成功に丸めず Io として伝播する。
    db.execute_batch("ALTER TABLE journal RENAME TO journal_gone")
        .unwrap();
    let outcome = repository.store(&next, &current).await;
    assert!(
        matches!(
            outcome,
            Err(RepositoryError::Io {
                kind: ErrorKind::Other,
                path: Some(_)
            })
        ),
        "{outcome:?}"
    );
    db.execute_batch("ALTER TABLE journal_gone RENAME TO journal")
        .unwrap();

    // DTOとして読めない保存物は InvalidData であり、空の集約に丸めない。
    db.execute(
        "UPDATE snapshot SET payload = X'00' WHERE aid = ?1",
        [aggregate.id().as_str()],
    )
    .unwrap();
    assert!(matches!(
        repository.find_by_id(aggregate.id()).await,
        Err(RepositoryError::Io {
            kind: ErrorKind::InvalidData,
            path: Some(_)
        })
    ));
}

#[test]
fn opening_a_missing_directory_is_reported_as_not_found() {
    let temp = tempfile::tempdir().unwrap();
    let path = StorePath::for_runtime(&temp.path().join("missing"));
    assert!(matches!(
        WorkflowContinuationRepositoryImpl::open(&path),
        Err(RepositoryError::Io {
            kind: ErrorKind::NotFound,
            path: Some(ref reported)
        }) if reported == path.as_path()
    ));
}

/// 失敗の分類は、ストアの実体 (memory / SQLite) に依らず同じ語彙へ落ちる。
fn failure_vocabulary_contract<S>(
    repository: &WorkflowContinuationRepositoryImpl<S>,
    path: Option<&StorePath>,
) where
    S: EventStore<
            AID = WorkflowContinuationKeyDto,
            A = WorkflowContinuationDto,
            P = WorkflowContinuationEventDto,
        >,
{
    let located =
        |error: RepositoryError<_>, kind: ErrorKind, expected: Option<&StorePath>| match error {
            RepositoryError::Io { kind: found, path } => {
                found == kind && path == expected.map(|p| p.as_path().to_path_buf())
            }
            _ => false,
        };
    assert!(located(
        repository.read_error(EventStoreReadError::OtherError("other".into())),
        ErrorKind::Other,
        path
    ));
    assert!(located(
        repository.read_error(EventStoreReadError::DeserializationError(Box::new(
            std::io::Error::other("bad payload")
        ))),
        ErrorKind::InvalidData,
        path
    ));
    assert!(located(
        repository.read_error(EventStoreReadError::IOError(Box::new(
            std::io::Error::other("not sqlite")
        ))),
        ErrorKind::Other,
        path
    ));
    let temp = tempfile::tempdir().unwrap();
    let opening = StorePath::for_runtime(temp.path());
    assert!(located(
        WorkflowContinuationRepositoryImpl::<S>::io(
            EventStoreWriteError::IOError(Box::new(std::io::Error::other("not sqlite"))),
            &opening
        ),
        ErrorKind::Other,
        Some(&opening)
    ));
    assert!(located(
        WorkflowContinuationRepositoryImpl::<S>::io(
            EventStoreWriteError::OtherError("other".into()),
            &opening
        ),
        ErrorKind::Other,
        Some(&opening)
    ));
}

#[test]
fn both_backends_share_the_failure_vocabulary() {
    let temp = tempfile::tempdir().unwrap();
    let path = StorePath::for_runtime(temp.path());
    failure_vocabulary_contract(
        &WorkflowContinuationRepositoryImpl::open(&path).unwrap(),
        Some(&path),
    );
    failure_vocabulary_contract(
        &WorkflowContinuationRepositoryImpl::<WorkflowContinuationMemoryStore>::in_memory(),
        None,
    );
}
