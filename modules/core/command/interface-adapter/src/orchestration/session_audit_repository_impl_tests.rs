//! 同じ本家ストアAPIでsnapshotと差分イベント・破損・永続化失敗の契約を両backendへ課す。
use super::*;
use core_command_domain::workspace::{
    AuditFieldKey, AuditFields, EventType, HookHealthTarget, SessionAuditObservation,
    SessionAuditObservationId, SessionAuditRecord, SpaceName,
};
use event_store_adapter_rs::types::EventStore;

fn at() -> chrono::DateTime<chrono::Utc> {
    "2026-09-10T01:00:00Z".parse().unwrap()
}
fn observation(space: &str, source: &str) -> SessionAuditObservation {
    SessionAuditObservation::new(
        SessionAuditObservationId::generate(),
        HookHealthTarget::new(SpaceName::parse(space).unwrap(), None),
        SessionAuditRecord::new(
            EventType::SessionStarted,
            AuditFields::new().with(AuditFieldKey::parse("Source").unwrap(), source),
        )
        .unwrap(),
        "Running".into(),
    )
}
fn genesis(space: &str) -> (SessionAudit, SessionAuditEvent) {
    SessionAudit::start(&observation(space, "startup"), at())
        .unwrap()
        .unwrap()
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

async fn backend_contract<S>(mut repository: SessionAuditRepositoryImpl<S>)
where
    S: EventStore<AID = SessionAuditKeyDto, A = SessionAuditDto, P = SessionAuditEventDto> + Clone,
{
    // 未保存の集約は NotFound であり、空の集約を捏造しない。
    let (aggregate, event) = genesis("absent");
    assert!(matches!(
        repository.find_by_id(aggregate.id()).await,
        Err(RepositoryError::NotFound { .. })
    ));
    // 保存 → 再構成で最後の記録・観測IDが往復する。
    repository.store(&event, &aggregate).await.unwrap();
    let restored = repository.find_by_id(aggregate.id()).await.unwrap();
    assert_eq!(restored, aggregate.clone().with_version(1));
    // 同じ版からの二重保存は Conflict であり、実在版を報告する。
    assert!(matches!(
        repository.store(&event, &aggregate).await,
        Err(RepositoryError::Conflict {
            expected: 0,
            actual: 1
        })
    ));

    // 最新snapshot以降の差分イベントだけを再生する。
    let (aggregate, event) = genesis("delta");
    repository.store(&event, &aggregate).await.unwrap();
    let mut aggregate = repository.find_by_id(aggregate.id()).await.unwrap();
    let event = aggregate
        .record(&observation("delta", "resume"), at())
        .unwrap()
        .unwrap();
    let key = SessionAuditKeyDto::of(aggregate.id());
    let mut store = repository.store.clone();
    store
        .persist_event(
            EventEnvelope::new(key.clone(), 2, at(), SessionAuditEventDto::of(&event))
                .with_manifest(MANIFEST),
            1,
        )
        .await
        .unwrap();
    let restored = repository.find_by_id(aggregate.id()).await.unwrap();
    assert_eq!(restored.seq_nr(), 2);
    assert_eq!(restored.observation_id(), event.observation_id());
    assert_eq!(restored.last_record(), event.record());
    assert_eq!(restored, aggregate.with_version(2));

    // 種別が異なる履歴を、同じ集約型として適用しない。
    let (aggregate, event) = genesis("manifest");
    repository.store(&event, &aggregate).await.unwrap();
    let mut aggregate = repository.find_by_id(aggregate.id()).await.unwrap();
    let event = aggregate
        .record(&observation("manifest", "resume"), at())
        .unwrap()
        .unwrap();
    store
        .persist_event(
            EventEnvelope::new(
                SessionAuditKeyDto::of(aggregate.id()),
                2,
                at(),
                SessionAuditEventDto::of(&event),
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
    let (aggregate, event) = genesis("malformed-event");
    repository.store(&event, &aggregate).await.unwrap();
    let mut aggregate = repository.find_by_id(aggregate.id()).await.unwrap();
    let event = aggregate
        .record(&observation("malformed-event", "resume"), at())
        .unwrap()
        .unwrap();
    store
        .persist_event(
            EventEnvelope::new(
                SessionAuditKeyDto::of(aggregate.id()),
                2,
                at(),
                malformed(&SessionAuditEventDto::of(&event), "target"),
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
    let (aggregate, event) = genesis("malformed-snapshot");
    store
        .persist_event_and_snapshot(
            EventEnvelope::new(
                SessionAuditKeyDto::of(aggregate.id()),
                1,
                at(),
                SessionAuditEventDto::of(&event),
            )
            .with_manifest(MANIFEST),
            malformed(&SessionAuditDto::of(&aggregate), "target"),
            0,
        )
        .await
        .unwrap();
    assert!(matches!(
        repository.find_by_id(aggregate.id()).await,
        Err(RepositoryError::Corrupt { seq_nr: None, .. })
    ));

    // 通番が尽きたsnapshotの後ろに差分を探しに行かず、Corrupt (seq 不明) にする。
    let (aggregate, event) = genesis("exhausted");
    store
        .persist_event_and_snapshot(
            EventEnvelope::new(
                SessionAuditKeyDto::of(aggregate.id()),
                1,
                at(),
                SessionAuditEventDto::of(&event),
            )
            .with_manifest(MANIFEST),
            with_exhausted_sequence(&SessionAuditDto::of(&aggregate)),
            0,
        )
        .await
        .unwrap();
    assert!(matches!(
        repository.find_by_id(aggregate.id()).await,
        Err(RepositoryError::Corrupt { seq_nr: None, .. })
    ));

    // ストア鍵と異なる、単体では有効なsnapshotを受け入れない。
    let (aggregate, event) = genesis("snapshot");
    let (foreign, _) = genesis("foreign");
    store
        .persist_event_and_snapshot(
            EventEnvelope::new(
                SessionAuditKeyDto::of(aggregate.id()),
                1,
                at(),
                SessionAuditEventDto::of(&event),
            )
            .with_manifest(MANIFEST),
            SessionAuditDto::of(&foreign),
            0,
        )
        .await
        .unwrap();
    assert!(matches!(
        repository.find_by_id(aggregate.id()).await,
        Err(RepositoryError::Corrupt {
            seq_nr: Some(1),
            ..
        })
    ));

    // 別集約のイベントを、この集約の履歴として書かない。
    let (aggregate, _) = genesis("write-target");
    let (_, foreign_event) = genesis("write-foreign");
    assert!(matches!(
        repository.store(&foreign_event, &aggregate).await,
        Err(RepositoryError::Corrupt {
            seq_nr: Some(1),
            ..
        })
    ));

    // 新規作成 (seq 1) に読取済み版を添える契約違反は、成功に丸めず失敗として返す。
    let (aggregate, event) = genesis("contract");
    assert!(matches!(
        repository.store(&event, &aggregate.with_version(1)).await,
        Err(RepositoryError::Io {
            kind: ErrorKind::Other,
            ..
        })
    ));
}

#[tokio::test]
async fn memory_history_contract() {
    backend_contract(SessionAuditRepositoryImpl::in_memory()).await;
}

#[tokio::test]
async fn sqlite_history_contract() {
    let temp = tempfile::tempdir().unwrap();
    let path = StorePath::for_runtime(temp.path());
    backend_contract(SessionAuditRepositoryImpl::open(&path).unwrap()).await;
}

#[tokio::test]
async fn sqlite_reports_the_store_location_on_failures() {
    let temp = tempfile::tempdir().unwrap();
    let path = StorePath::for_runtime(temp.path());
    let mut repository = SessionAuditRepositoryImpl::open(&path).unwrap();
    let (aggregate, event) = genesis("default");
    repository.store(&event, &aggregate).await.unwrap();
    let db = rusqlite::Connection::open(path.as_path()).unwrap();

    // 追記が媒体で失敗したら、部分的な事実を残さず Io として伝播する。
    let mut current = repository.find_by_id(aggregate.id()).await.unwrap();
    let next = current
        .record(&observation("default", "resume"), at())
        .unwrap()
        .unwrap();
    db.execute_batch(
        "CREATE TRIGGER fail_audit BEFORE UPDATE ON snapshot \
         BEGIN SELECT RAISE(ABORT, 'injected audit failure'); END;",
    )
    .unwrap();
    let outcome = repository.store(&next, &current).await;
    assert!(
        matches!(
            outcome,
            Err(RepositoryError::Io { path: Some(ref reported), .. }) if reported == path.as_path()
        ),
        "{outcome:?}"
    );
    db.execute_batch("DROP TRIGGER fail_audit").unwrap();
    assert_eq!(
        repository
            .find_by_id(aggregate.id())
            .await
            .unwrap()
            .seq_nr(),
        1
    );

    // 他の書き手が占有している媒体は WouldBlock として返し、再実行で解ける分類にする。
    db.execute_batch("BEGIN EXCLUSIVE").unwrap();
    assert!(matches!(
        repository.find_by_id(aggregate.id()).await,
        Err(RepositoryError::Io {
            kind: ErrorKind::WouldBlock,
            path: Some(_)
        })
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
        SessionAuditRepositoryImpl::open(&path),
        Err(RepositoryError::Io {
            kind: ErrorKind::NotFound,
            path: Some(ref reported)
        }) if reported == path.as_path()
    ));
}

/// 失敗の分類は、ストアの実体 (memory / SQLite) に依らず同じ語彙へ落ちる。
fn failure_vocabulary_contract<S>(
    repository: &SessionAuditRepositoryImpl<S>,
    path: Option<&StorePath>,
) where
    S: EventStore<AID = SessionAuditKeyDto, A = SessionAuditDto, P = SessionAuditEventDto>,
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
        SessionAuditRepositoryImpl::<S>::io(
            EventStoreWriteError::IOError(Box::new(std::io::Error::other("not sqlite"))),
            &opening
        ),
        ErrorKind::Other,
        Some(&opening)
    ));
    assert!(located(
        SessionAuditRepositoryImpl::<S>::io(
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
        &SessionAuditRepositoryImpl::open(&path).unwrap(),
        Some(&path),
    );
    failure_vocabulary_contract(
        &SessionAuditRepositoryImpl::<SessionAuditMemoryStore>::in_memory(),
        None,
    );
}
