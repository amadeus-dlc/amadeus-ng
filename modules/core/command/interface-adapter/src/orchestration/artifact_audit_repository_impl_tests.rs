//! 両backendへ同じ履歴・破損を投入するRepository契約。
use super::*;
use chrono::{DateTime, Utc};
use core_command_domain::workspace::{ArtifactWriteObservation, HookHealthTarget, SpaceName};
fn at() -> DateTime<Utc> {
    "2026-09-09T01:00:00Z".parse().unwrap()
}
fn genesis(name: &str) -> (ArtifactAudit, ArtifactAuditEvent) {
    ArtifactAudit::start(
        ArtifactWriteObservation::new(
            HookHealthTarget::new(SpaceName::parse(name).unwrap(), None),
            "Write".into(),
            "/a.md".into(),
            "construction".into(),
            true,
        ),
        at(),
    )
    .unwrap()
}
fn malformed<T: serde::Serialize + serde::de::DeserializeOwned>(dto: &T, key: &str) -> T {
    use core_infrastructure::canon_json::{SerializationProfile, serialize, to_value};
    let text = serialize(
        &to_value(dto).unwrap(),
        SerializationProfile::ContractCompact,
    );
    let mut value: serde_json::Value = serde_json::from_str(&text).unwrap();
    value
        .as_object_mut()
        .unwrap()
        .insert(key.into(), serde_json::Value::String("invalid-id".into()));
    serde_json::from_value(value).unwrap()
}
async fn backend_contract<S>(mut repository: ArtifactAuditRepositoryImpl<S>)
where
    S: EventStore<
            AID = ArtifactAuditAggregateKeyDto,
            A = ArtifactAuditDto,
            P = ArtifactAuditEventDto,
        > + Clone,
{
    // 最新スナップショット以降だけのイベントを、別Repositoryから読む。
    let (aggregate, event) = genesis("delta");
    repository.store(&event, &aggregate).await.unwrap();
    let mut aggregate = repository.find_by_id(aggregate.id()).await.unwrap();
    let event = aggregate
        .record(
            ArtifactWriteObservation::new(
                aggregate.target().clone(),
                "Edit".into(),
                "/b.md".into(),
                "construction".into(),
                false,
            ),
            at() + chrono::Duration::seconds(1),
        )
        .unwrap();
    let key = ArtifactAuditAggregateKeyDto::of(aggregate.id());
    let mut store = repository.store.clone();
    store
        .persist_event(
            EventEnvelope::new(
                key.clone(),
                2,
                at() + chrono::Duration::seconds(1),
                ArtifactAuditEventDto::of(&event),
            )
            .with_manifest(MANIFEST),
            1,
        )
        .await
        .unwrap();
    let before = store
        .get_events_by_id_since_seq_nr(&key, 1)
        .await
        .unwrap()
        .len();
    let restored = repository
        .reopened()
        .find_by_id(aggregate.id())
        .await
        .unwrap();
    assert_eq!(restored.seq_nr(), 2);
    assert_eq!(restored.last_file(), "/b.md");
    assert_eq!(restored.last_tool(), "Edit");
    assert_eq!(
        store
            .get_events_by_id_since_seq_nr(&key, 1)
            .await
            .unwrap()
            .len(),
        before
    );
    assert_eq!(restored, aggregate.with_version(2));

    // 種別が異なる履歴を、同じ集約型として適用しない。
    let (aggregate, event) = genesis("manifest");
    repository.store(&event, &aggregate).await.unwrap();
    let mut aggregate = repository.find_by_id(aggregate.id()).await.unwrap();
    let event = aggregate
        .record(
            ArtifactWriteObservation::new(
                aggregate.target().clone(),
                "Edit".into(),
                "/b.md".into(),
                "construction".into(),
                false,
            ),
            at() + chrono::Duration::seconds(1),
        )
        .unwrap();
    let key = ArtifactAuditAggregateKeyDto::of(aggregate.id());
    store
        .persist_event(
            EventEnvelope::new(key, 2, at(), ArtifactAuditEventDto::of(&event))
                .with_manifest("foreign-event/1"),
            1,
        )
        .await
        .unwrap();
    assert!(matches!(
        repository.reopened().find_by_id(aggregate.id()).await,
        Err(RepositoryError::Corrupt {
            seq_nr: Some(2),
            ..
        })
    ));

    // DTOとして読めても、ドメインへ戻せない値はCorruptとして拒否する。
    let (aggregate, event) = genesis("malformed");
    repository.store(&event, &aggregate).await.unwrap();
    let mut aggregate = repository.find_by_id(aggregate.id()).await.unwrap();
    let event = aggregate
        .record(
            ArtifactWriteObservation::new(
                aggregate.target().clone(),
                "Edit".into(),
                "/b.md".into(),
                "construction".into(),
                false,
            ),
            at() + chrono::Duration::seconds(1),
        )
        .unwrap();
    let payload = malformed(&ArtifactAuditEventDto::of(&event), "aggregate_id");
    store
        .persist_event(
            EventEnvelope::new(
                ArtifactAuditAggregateKeyDto::of(aggregate.id()),
                2,
                at(),
                payload,
            )
            .with_manifest(MANIFEST),
            1,
        )
        .await
        .unwrap();
    assert!(matches!(
        repository.reopened().find_by_id(aggregate.id()).await,
        Err(RepositoryError::Corrupt {
            seq_nr: Some(2),
            ..
        })
    ));

    // ストア鍵と異なる、単体では有効なスナップショットを受け入れない。
    let (aggregate, event) = genesis("snapshot");
    let (foreign, _) = genesis("foreign");
    store
        .persist_event_and_snapshot(
            EventEnvelope::new(
                ArtifactAuditAggregateKeyDto::of(aggregate.id()),
                1,
                at(),
                ArtifactAuditEventDto::of(&event),
            )
            .with_manifest(MANIFEST),
            ArtifactAuditDto::of(&foreign),
            0,
        )
        .await
        .unwrap();
    assert!(matches!(
        repository.find_by_id(aggregate.id()).await,
        Err(RepositoryError::Corrupt { .. })
    ));
}
#[tokio::test]
async fn memory_history_contract() {
    backend_contract(ArtifactAuditRepositoryImpl::in_memory()).await;
}
#[tokio::test]
async fn sqlite_history_contract() {
    let temp = tempfile::tempdir().unwrap();
    let path = StorePath::for_runtime(temp.path());
    backend_contract(ArtifactAuditRepositoryImpl::open(&path).unwrap()).await;
}
fn permission_denied() -> Box<dyn std::error::Error + Send + Sync> {
    Box::new(rusqlite::Error::SqliteFailure(
        rusqlite::ffi::Error {
            code: rusqlite::ErrorCode::PermissionDenied,
            extended_code: 3,
        },
        None,
    ))
}
#[test]
fn memory_errors_carry_no_dummy_location() {
    let repository = ArtifactAuditRepositoryImpl::in_memory();
    assert!(matches!(
        repository.read_error(EventStoreReadError::IOError(permission_denied())),
        RepositoryError::Io {
            kind: ErrorKind::PermissionDenied,
            path: None
        }
    ));
    assert!(matches!(
        ArtifactAuditRepositoryImpl::<ArtifactAuditMemoryStore>::io(
            EventStoreWriteError::IOError(permission_denied()),
            None
        ),
        RepositoryError::Io {
            kind: ErrorKind::PermissionDenied,
            path: None
        }
    ));
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

async fn snapshot_corruption_contract<S>(mut repository: ArtifactAuditRepositoryImpl<S>)
where
    S: EventStore<
            AID = ArtifactAuditAggregateKeyDto,
            A = ArtifactAuditDto,
            P = ArtifactAuditEventDto,
        > + Clone,
{
    let mut store = repository.store.clone();
    // DTOとして読めても、ドメインへ戻せないsnapshotは Corrupt (seq 不明) として拒否する。
    let (aggregate, event) = genesis("snapshot-a");
    store
        .persist_event_and_snapshot(
            EventEnvelope::new(
                ArtifactAuditAggregateKeyDto::of(aggregate.id()),
                1,
                at(),
                ArtifactAuditEventDto::of(&event),
            )
            .with_manifest(MANIFEST),
            malformed(&ArtifactAuditDto::of(&aggregate), "id"),
            0,
        )
        .await
        .unwrap();
    assert!(matches!(
        repository.find_by_id(aggregate.id()).await,
        Err(RepositoryError::Corrupt { seq_nr: None, .. })
    ));
    // 通番が尽きたsnapshotの後ろに差分を探しに行かず、Corrupt (seq 不明) にする。
    let (aggregate, event) = genesis("snapshot-b");
    store
        .persist_event_and_snapshot(
            EventEnvelope::new(
                ArtifactAuditAggregateKeyDto::of(aggregate.id()),
                1,
                at(),
                ArtifactAuditEventDto::of(&event),
            )
            .with_manifest(MANIFEST),
            with_exhausted_sequence(&ArtifactAuditDto::of(&aggregate)),
            0,
        )
        .await
        .unwrap();
    assert!(matches!(
        repository.find_by_id(aggregate.id()).await,
        Err(RepositoryError::Corrupt { seq_nr: None, .. })
    ));
    // 新規作成 (seq 1) に読取済み版を添える契約違反は、成功に丸めず失敗として返す。
    let (aggregate, event) = genesis("snapshot-a");
    assert!(matches!(
        repository.store(&event, &aggregate.with_version(1)).await,
        Err(RepositoryError::Io {
            kind: ErrorKind::Other,
            ..
        })
    ));
}

#[tokio::test]
async fn memory_snapshot_corruption_contract() {
    snapshot_corruption_contract(ArtifactAuditRepositoryImpl::in_memory()).await;
}

#[tokio::test]
async fn sqlite_snapshot_corruption_contract() {
    let temp = tempfile::tempdir().unwrap();
    let path = StorePath::for_runtime(temp.path());
    snapshot_corruption_contract(ArtifactAuditRepositoryImpl::open(&path).unwrap()).await;
}

#[test]
fn opening_a_missing_directory_is_reported_as_not_found() {
    let temp = tempfile::tempdir().unwrap();
    let path = StorePath::for_runtime(&temp.path().join("missing"));
    assert!(matches!(
        ArtifactAuditRepositoryImpl::open(&path),
        Err(RepositoryError::Io {
            kind: ErrorKind::NotFound,
            path: Some(ref reported)
        }) if reported == path.as_path()
    ));
}

/// 失敗の分類は、ストアの実体 (memory / SQLite) に依らず同じ語彙へ落ちる。
fn failure_vocabulary_contract<S>(
    repository: &ArtifactAuditRepositoryImpl<S>,
    path: Option<&StorePath>,
) where
    S: EventStore<
            AID = ArtifactAuditAggregateKeyDto,
            A = ArtifactAuditDto,
            P = ArtifactAuditEventDto,
        >,
{
    let expected_path = path.map(|p| p.as_path().to_path_buf());
    let located = |error: RepositoryError<_>, kind: ErrorKind| match error {
        RepositoryError::Io { kind: found, path } => found == kind && path == expected_path,
        _ => false,
    };
    assert!(located(
        repository.read_error(EventStoreReadError::OtherError("other".into())),
        ErrorKind::Other
    ));
    assert!(located(
        repository.read_error(EventStoreReadError::DeserializationError(Box::new(
            std::io::Error::other("bad payload")
        ))),
        ErrorKind::InvalidData
    ));
    assert!(located(
        repository.read_error(EventStoreReadError::IOError(Box::new(
            std::io::Error::other("not sqlite")
        ))),
        ErrorKind::Other
    ));
    assert!(located(
        ArtifactAuditRepositoryImpl::<S>::io(
            EventStoreWriteError::IOError(Box::new(std::io::Error::other("not sqlite"))),
            path
        ),
        ErrorKind::Other
    ));
    assert!(located(
        ArtifactAuditRepositoryImpl::<S>::io(
            EventStoreWriteError::OtherError("other".into()),
            path
        ),
        ErrorKind::Other
    ));
}

#[test]
fn both_backends_share_the_failure_vocabulary() {
    let temp = tempfile::tempdir().unwrap();
    let path = StorePath::for_runtime(temp.path());
    failure_vocabulary_contract(
        &ArtifactAuditRepositoryImpl::open(&path).unwrap(),
        Some(&path),
    );
    failure_vocabulary_contract(
        &ArtifactAuditRepositoryImpl::<ArtifactAuditMemoryStore>::in_memory(),
        None,
    );
}
