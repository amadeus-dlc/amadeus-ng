//! 実SQLiteで、共有承認の壊れた保存物の拒否と媒体失敗の伝播を検査する。
use super::*;
use chrono::{DateTime, Utc};
use core_command_domain::orchestration::{
    CodeGenerationAuthority, IntentId, PlanApprovalEvidence, PlanApprovalOperationId,
    PlanChallenge, PlanSession, PlanTarget,
};

fn at() -> DateTime<Utc> {
    "2026-09-10T01:00:00Z".parse().unwrap()
}
fn challenge() -> PlanChallenge {
    let authority = CodeGenerationAuthority::new(
        &PlanTarget::stage_level(),
        &IntentId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000").unwrap(),
        format!("sha256:{}", "a".repeat(64)),
        "WORKFLOW_STARTED:2026-09-08T01:00:00Z#1".to_string(),
        "b".repeat(64),
        2,
    )
    .unwrap();
    let evidence = PlanApprovalEvidence::new(
        authority,
        format!("sha256:{}", "c".repeat(64)),
        "aidlc/spaces/default/intents/example/construction/code-generation/code-generation-questions.md".to_string(),
        "d".repeat(64),
        "e".repeat(64),
    )
    .unwrap();
    PlanChallenge::issue(
        evidence,
        PlanSession::new("session".to_string()).unwrap(),
        ["Approve Plan".to_string(), "Request Changes".to_string()],
        false,
    )
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
/// 作成済みの共有承認と、その次に保存できる発行イベントを実SQLiteへ用意する。
async fn created(
    path: &StorePath,
) -> (
    PlanApprovalRuntimeRepositoryImpl,
    PlanApprovalRuntime,
    PlanApprovalEvent,
) {
    let mut repository = PlanApprovalRuntimeRepositoryImpl::open(path).unwrap();
    let (runtime, created) = PlanApprovalRuntime::create(at());
    repository.store(&created, &runtime).await.unwrap();
    let mut runtime = repository.find_by_id(runtime.id()).await.unwrap();
    let issued = runtime
        .issue_challenge(PlanApprovalOperationId::generate(), challenge(), at())
        .unwrap();
    (repository, runtime, issued)
}

#[tokio::test]
async fn corrupt_history_is_rejected_instead_of_replayed() {
    let temp = tempfile::tempdir().unwrap();
    let path = StorePath::for_runtime(temp.path());
    let (repository, runtime, issued) = created(&path).await;
    let id = *runtime.id();
    let key = PlanApprovalRuntimeKeyDto::of(&id);
    let db = rusqlite::Connection::open(path.as_path()).unwrap();

    // 通番・種別が食い違う履歴は、その封筒の通番付きで Corrupt にする。
    let mut raw = EventStoreForSqlite::<
        PlanApprovalRuntimeKeyDto,
        PlanApprovalRuntimeDto,
        PlanApprovalEventDto,
    >::new(path.as_path())
    .unwrap();
    raw.persist_event(
        EventEnvelope::new(key.clone(), 2, at(), PlanApprovalEventDto::of(&issued))
            .with_manifest("foreign-event/1"),
        1,
    )
    .await
    .unwrap();
    assert!(matches!(
        repository.find_by_id(&id).await,
        Err(RepositoryError::Corrupt {
            seq_nr: Some(2),
            ..
        })
    ));

    // DTOとして読めても、ドメインへ戻せないイベントは Corrupt として拒否する。
    let broken_event = {
        use core_infrastructure::canon_json::{SerializationProfile, serialize, to_value};
        serialize(
            &to_value(&malformed(
                &PlanApprovalEventDto::of(&issued),
                "aggregate_id",
            ))
            .unwrap(),
            SerializationProfile::ContractCompact,
        )
    };
    db.execute(
        "UPDATE journal SET manifest = ?1, payload = ?2 WHERE seq_nr = 2",
        rusqlite::params![EVENT_MANIFEST, broken_event.as_bytes()],
    )
    .unwrap();
    assert!(matches!(
        repository.find_by_id(&id).await,
        Err(RepositoryError::Corrupt {
            seq_nr: Some(2),
            ..
        })
    ));
    db.execute("DELETE FROM journal WHERE seq_nr = 2", [])
        .unwrap();

    // DTOとして読めても、ドメインへ戻せないsnapshotは Corrupt (seq 不明) として拒否する。
    let broken = {
        use core_infrastructure::canon_json::{SerializationProfile, serialize, to_value};
        serialize(
            &to_value(&malformed(&PlanApprovalRuntimeDto::of(&runtime), "id")).unwrap(),
            SerializationProfile::ContractCompact,
        )
    };
    db.execute(
        "UPDATE snapshot SET payload = ?1 WHERE aid = ?2",
        rusqlite::params![broken.as_bytes(), key.to_string()],
    )
    .unwrap();
    assert!(matches!(
        repository.find_by_id(&id).await,
        Err(RepositoryError::Corrupt { seq_nr: None, .. })
    ));

    // DTOとして読めない保存物も、空の集約に丸めず Corrupt にする。
    db.execute(
        "UPDATE snapshot SET payload = X'00' WHERE aid = ?1",
        [key.to_string()],
    )
    .unwrap();
    assert!(matches!(
        repository.find_by_id(&id).await,
        Err(RepositoryError::Corrupt { seq_nr: None, .. })
    ));

    // 履歴だけが残り基底が失われた状態は、NotFound ではなく Corrupt である。
    db.execute("DELETE FROM snapshot", []).unwrap();
    assert!(matches!(
        repository.find_by_id(&id).await,
        Err(RepositoryError::Corrupt { seq_nr: None, .. })
    ));
}

#[tokio::test]
async fn store_failures_are_propagated_not_swallowed() {
    let temp = tempfile::tempdir().unwrap();
    let path = StorePath::for_runtime(temp.path());
    let (mut repository, runtime, issued) = created(&path).await;
    let db = rusqlite::Connection::open(path.as_path()).unwrap();

    // 新規作成 (seq 1) に読取済み版を添える契約違反は、Corrupt として返す。
    let (fresh, event) = PlanApprovalRuntime::create(at());
    let temp2 = tempfile::tempdir().unwrap();
    let mut other =
        PlanApprovalRuntimeRepositoryImpl::open(&StorePath::for_runtime(temp2.path())).unwrap();
    assert!(matches!(
        other.store(&event, &fresh.with_version(1)).await,
        Err(RepositoryError::Corrupt {
            seq_nr: Some(1),
            ..
        })
    ));

    // 他の書き手が占有している媒体は WouldBlock として返し、再実行で解ける分類にする。
    db.execute_batch("BEGIN EXCLUSIVE").unwrap();
    assert!(matches!(
        repository.find_by_id(runtime.id()).await,
        Err(RepositoryError::Io {
            kind: ErrorKind::WouldBlock,
            path: Some(ref reported)
        }) if reported == path.as_path()
    ));
    db.execute_batch("COMMIT").unwrap();

    // 媒体の追記先が失われていれば、成功に丸めず Io として伝播する。
    db.execute_batch("ALTER TABLE journal RENAME TO journal_gone")
        .unwrap();
    let outcome = repository.store(&issued, &runtime).await;
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
}

#[test]
fn opening_a_missing_directory_is_reported_as_not_found() {
    let temp = tempfile::tempdir().unwrap();
    let path = StorePath::for_runtime(&temp.path().join("missing"));
    assert!(matches!(
        PlanApprovalRuntimeRepositoryImpl::open(&path),
        Err(RepositoryError::Io {
            kind: ErrorKind::NotFound,
            path: Some(ref reported)
        }) if reported == path.as_path()
    ));
}

#[test]
fn read_errors_keep_the_store_location() {
    let temp = tempfile::tempdir().unwrap();
    let path = StorePath::for_runtime(temp.path());
    let repository = PlanApprovalRuntimeRepositoryImpl::open(&path).unwrap();
    assert!(matches!(
        repository.read_error(EventStoreReadError::OtherError("other".into())),
        RepositoryError::Io {
            kind: ErrorKind::Other,
            path: Some(ref reported)
        } if reported == path.as_path()
    ));
    assert!(matches!(
        repository.read_error(EventStoreReadError::IOError(Box::new(
            std::io::Error::other("not sqlite")
        ))),
        RepositoryError::Io {
            kind: ErrorKind::Other,
            path: Some(_)
        }
    ));
}
