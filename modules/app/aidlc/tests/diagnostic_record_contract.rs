//! U3が呼ぶ診断記録APIを、既存SQLiteと通常RMUへ接続する契約。
#![allow(clippy::unwrap_used)]
use core_command_domain::{
    orchestration::{HealthCheckResult, IntentExecutionId},
    workspace::{SpaceName, StorePath},
};
use core_command_interface_adapter::orchestration::IntentExecutionRepositoryImpl;
use core_command_use_case::orchestration::RecordHealthCheckUseCase;
use std::{
    fs,
    path::{Path, PathBuf},
};

struct Fixture {
    root: tempfile::TempDir,
    execution: IntentExecutionId,
    record: PathBuf,
}
impl Fixture {
    async fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let data = root.path().join(".claude/tools/data");
        fs::create_dir_all(&data).unwrap();
        for name in ["stage-graph.json", "scope-grid.json", "harness.json"] {
            fs::copy(
                repo.join("tests/golden/upstream-a277af21/data").join(name),
                data.join(name),
            )
            .unwrap();
        }
        fs::create_dir_all(root.path().join(".claude/scopes")).unwrap();
        fs::copy(
            repo.join(".claude/scopes/aidlc-bugfix.md"),
            root.path().join(".claude/scopes/aidlc-bugfix.md"),
        )
        .unwrap();
        let args = [
            "intent-create",
            "--scope",
            "bugfix",
            "--label",
            "diagnostic",
            "--arguments",
            "Record diagnostic check",
        ]
        .map(str::to_string);
        let created = aidlc::runtime::run("aidlc-utility", &args, root.path()).await;
        assert_eq!(created.code(), 0, "{created:?}");
        let intents = root.path().join("aidlc/spaces/default/intents");
        let record = intents.join(
            fs::read_to_string(intents.join("active-intent"))
                .unwrap()
                .trim(),
        );
        let cursor = aidlc::execution_cursor::ExecutionCursor::read(&record)
            .unwrap()
            .unwrap();
        Self {
            root,
            execution: cursor.execution_id().clone(),
            record,
        }
    }
    fn store(&self) -> StorePath {
        StorePath::for_space(&self.root.path().join("aidlc"), &SpaceName::default())
    }
    fn count(&self) -> i64 {
        rusqlite::Connection::open(self.store().as_path())
            .unwrap()
            .query_row("SELECT count(*) FROM journal", [], |row| row.get(0))
            .unwrap()
    }
    fn audit_path(&self) -> PathBuf {
        fs::read_dir(self.record.join("audit"))
            .unwrap()
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .find(|path| path.extension().is_some_and(|ext| ext == "md"))
            .unwrap()
    }
    async fn project(&self) -> Result<(), String> {
        use core_read_model_updater::orchestration::{
            JournalReaderImpl, OrchestrationReadModelUpdater, ProjectionName, ProjectionTargets,
            ReadModelUpdater, SteeringReadModelUpdater, SteeringSource, StructuredReadModelUpdater,
        };
        let memory = self.root.path().join("aidlc/spaces/default/memory");
        let targets = ProjectionTargets::new(
            self.record.join("aidlc-state.md"),
            self.audit_path(),
            memory.clone(),
        );
        let reader = JournalReaderImpl::open(&self.store()).map_err(|error| error.to_string())?;
        let projection =
            ProjectionName::parse(&format!("orchestration-{}", self.execution)).unwrap();
        OrchestrationReadModelUpdater::new(
            reader,
            projection,
            targets,
            SteeringReadModelUpdater::open(
                self.store().as_path(),
                SteeringSource::new(memory).relative_to(self.root.path().to_path_buf()),
            )
            .map_err(|error| error.to_string())?,
            StructuredReadModelUpdater::open(self.store().as_path())
                .map_err(|error| error.to_string())?,
        )
        .for_execution(self.execution.clone())
        .update_read_models()
        .await
        .map_err(|error| error.to_string())
    }
}

#[tokio::test]
async fn a_diagnostic_check_appends_one_fact_to_the_existing_execution() {
    let fixture = Fixture::new().await;
    let before = fixture.count();
    let repository = IntentExecutionRepositoryImpl::open(&fixture.store()).unwrap();
    let result: Result<(), _> = RecordHealthCheckUseCase::new(repository)
        .execute(
            &fixture.execution,
            HealthCheckResult::new(12, 2),
            chrono::Utc::now(),
        )
        .await;
    result.unwrap();
    assert_eq!(fixture.count(), before + 1);
    assert!(fixture.record.join("aidlc-state.md").exists());
}

#[tokio::test]
async fn projection_appends_the_c7_audit_bytes_without_changing_workflow_state() {
    let fixture = Fixture::new().await;
    let before_audit = fs::read_to_string(fixture.audit_path()).unwrap();
    let before_state = fs::read(fixture.record.join("aidlc-state.md")).unwrap();
    let at: chrono::DateTime<chrono::Utc> = "2026-09-09T10:11:12Z".parse().unwrap();
    RecordHealthCheckUseCase::new(IntentExecutionRepositoryImpl::open(&fixture.store()).unwrap())
        .execute(&fixture.execution, HealthCheckResult::new(12, 2), at)
        .await
        .unwrap();
    fixture.project().await.unwrap();
    let after_audit = fs::read_to_string(fixture.audit_path()).unwrap();
    assert_eq!(
        after_audit.strip_prefix(&before_audit).unwrap(),
        "\n## Health Check\n**Timestamp**: 2026-09-09T10:11:12Z\n**Event**: HEALTH_CHECKED\n**Request**: /aidlc --doctor\n**Details**: 12 passed, 2 failed\n\n---\n"
    );
    assert_eq!(
        fs::read(fixture.record.join("aidlc-state.md")).unwrap(),
        before_state
    );
}

#[tokio::test]
async fn repeated_diagnostics_keep_distinct_facts_and_actual_failure_counts() {
    use core_command_domain::orchestration::IntentExecutionEvent;
    use core_read_model_updater::orchestration::{GlobalSeqNr, JournalReader, JournalReaderImpl};
    let fixture = Fixture::new().await;
    let mut command = RecordHealthCheckUseCase::new(
        IntentExecutionRepositoryImpl::open(&fixture.store()).unwrap(),
    );
    for result in [HealthCheckResult::new(5, 0), HealthCheckResult::new(0, 3)] {
        command
            .execute(&fixture.execution, result, chrono::Utc::now())
            .await
            .unwrap();
    }
    let history = JournalReaderImpl::open(&fixture.store())
        .unwrap()
        .events_after(GlobalSeqNr::ZERO)
        .await
        .unwrap();
    let facts: Vec<_> = history
        .executions()
        .iter()
        .filter_map(|entry| match entry.event() {
            IntentExecutionEvent::HealthChecked(fact) => Some(fact),
            _ => None,
        })
        .collect();
    assert_eq!(facts.len(), 2);
    assert_ne!(facts.first().unwrap().id(), facts.last().unwrap().id());
    assert_eq!(
        facts.first().unwrap().result(),
        &HealthCheckResult::new(5, 0)
    );
    assert_eq!(
        facts.last().unwrap().result(),
        &HealthCheckResult::new(0, 3)
    );
    fixture.project().await.unwrap();
    let audit = fs::read_to_string(fixture.audit_path()).unwrap();
    assert_eq!(audit.matches("**Event**: HEALTH_CHECKED").count(), 2);
    assert!(audit.contains("**Details**: 5 passed, 0 failed\n"));
    assert!(audit.contains("**Details**: 0 passed, 3 failed\n"));
}

#[tokio::test]
async fn a_storage_failure_is_returned_without_a_successful_diagnostic_fact() {
    use core_command_use_case::orchestration::HealthCheckError;
    let fixture = Fixture::new().await;
    let before = fixture.count();
    let before_audit = fs::read(fixture.audit_path()).unwrap();
    let db = rusqlite::Connection::open(fixture.store().as_path()).unwrap();
    db.execute_batch("CREATE TRIGGER fail_diagnostic_record BEFORE INSERT ON journal BEGIN SELECT RAISE(ABORT, 'diagnostic persistence unavailable'); END").unwrap();
    let result = RecordHealthCheckUseCase::new(
        IntentExecutionRepositoryImpl::open(&fixture.store()).unwrap(),
    )
    .execute(
        &fixture.execution,
        HealthCheckResult::new(4, 0),
        chrono::Utc::now(),
    )
    .await;
    assert!(
        matches!(result, Err(HealthCheckError::Repository(_))),
        "{result:?}"
    );
    assert_eq!(fixture.count(), before);
    assert_eq!(fs::read(fixture.audit_path()).unwrap(), before_audit);
}

#[tokio::test]
async fn an_unknown_execution_is_not_created_by_recording_a_diagnostic() {
    use core_command_use_case::orchestration::{HealthCheckError, RepositoryError};
    let fixture = Fixture::new().await;
    let before = fixture.count();
    let absent = IntentExecutionId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000").unwrap();
    let result = RecordHealthCheckUseCase::new(
        IntentExecutionRepositoryImpl::open(&fixture.store()).unwrap(),
    )
    .execute(&absent, HealthCheckResult::new(1, 1), chrono::Utc::now())
    .await;
    assert!(
        matches!(
            result,
            Err(HealthCheckError::Repository(
                RepositoryError::NotFound { .. }
            ))
        ),
        "{result:?}"
    );
    assert_eq!(fixture.count(), before);
}

#[tokio::test]
async fn failed_projection_retries_the_saved_fact_without_duplicate_audit() {
    use core_read_model_updater::orchestration::{
        JournalReader, JournalReaderImpl, ProjectionName,
    };
    let fixture = Fixture::new().await;
    let before_audit = fs::read_to_string(fixture.audit_path()).unwrap();
    let reader = JournalReaderImpl::open(&fixture.store()).unwrap();
    let projection =
        ProjectionName::parse(&format!("orchestration-{}", fixture.execution)).unwrap();
    let before_checkpoint = reader.checkpoint(&projection).await.unwrap();
    RecordHealthCheckUseCase::new(IntentExecutionRepositoryImpl::open(&fixture.store()).unwrap())
        .execute(
            &fixture.execution,
            HealthCheckResult::new(4, 2),
            chrono::Utc::now(),
        )
        .await
        .unwrap();
    let saved = fixture.count();
    let db = rusqlite::Connection::open(fixture.store().as_path()).unwrap();
    db.execute_batch("CREATE TRIGGER fail_diagnostic_projection BEFORE INSERT ON amadeus_projection_checkpoint BEGIN SELECT RAISE(ABORT, 'diagnostic projection unavailable'); END").unwrap();
    assert!(fixture.project().await.is_err());
    assert_eq!(fixture.count(), saved);
    // 通常RMUはファイル公開後にcheckpointを確定する。失敗時のpending計画を
    // 再利用して公開済み監査を二重追記しないことが、既存の復旧契約である。
    let partial_audit = fs::read_to_string(fixture.audit_path()).unwrap();
    assert!(partial_audit.starts_with(&before_audit));
    assert_eq!(
        partial_audit.matches("**Event**: HEALTH_CHECKED").count(),
        1
    );
    assert_eq!(
        reader.checkpoint(&projection).await.unwrap(),
        before_checkpoint
    );
    assert!(
        reader
            .pending_publication(&projection)
            .await
            .unwrap()
            .is_some()
    );
    db.execute_batch("DROP TRIGGER fail_diagnostic_projection")
        .unwrap();
    fixture.project().await.unwrap();
    let projected = fs::read_to_string(fixture.audit_path()).unwrap();
    assert_eq!(projected, partial_audit);
    assert!(
        reader
            .pending_publication(&projection)
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(projected.matches("**Event**: HEALTH_CHECKED").count(), 1);
    fixture.project().await.unwrap();
    assert_eq!(fs::read_to_string(fixture.audit_path()).unwrap(), projected);
    assert_eq!(fixture.count(), saved);
}

#[tokio::test]
async fn diagnosing_a_parked_workflow_does_not_resume_it() {
    let fixture = Fixture::new().await;
    let parked = aidlc::runtime::run("aidlc", &["park".to_string()], fixture.root.path()).await;
    assert_eq!(parked.code(), 0, "{parked:?}");
    let before = fs::read(fixture.record.join("aidlc-state.md")).unwrap();
    RecordHealthCheckUseCase::new(IntentExecutionRepositoryImpl::open(&fixture.store()).unwrap())
        .execute(
            &fixture.execution,
            HealthCheckResult::new(7, 0),
            chrono::Utc::now(),
        )
        .await
        .unwrap();
    fixture.project().await.unwrap();
    assert_eq!(
        fs::read(fixture.record.join("aidlc-state.md")).unwrap(),
        before
    );
    assert!(
        fs::read_to_string(fixture.audit_path())
            .unwrap()
            .contains("**Details**: 7 passed, 0 failed\n")
    );
}
