//! 報告事実と結果行をチェックポイントと同時に公開する契約。
#![allow(clippy::unwrap_used, clippy::expect_used)]
mod support;
use core_command_domain::orchestration::{ReportId, ReportRequest, Verdict};
use core_command_domain::workspace::{SpaceName, StorePath};
use core_read_model_updater::orchestration::{
    GlobalSeqNr, JournalReader, JournalReaderImpl, ProjectionName,
};
use core_read_model_updater::read_tables::ReadTables;
use rusqlite::Connection;

struct Fixture {
    root: tempfile::TempDir,
    store: support::UpstreamStore,
    writer: support::JournalWriter,
    reader: JournalReaderImpl,
    path: StorePath,
    projection: ProjectionName,
}
impl Fixture {
    async fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let path = StorePath::for_space(&root.path().join("aidlc"), &SpaceName::default());
        std::fs::create_dir_all(path.as_path().parent().unwrap()).unwrap();
        let mut store = support::open_store(&path);
        support::seed_intent(&path).await;
        let writer = support::JournalWriter::start(&mut store, support::execution_id()).await;
        let reader = JournalReaderImpl::open(&path).unwrap();
        // 読み面の表は構造化面の更新器の開く段が用意する (本番と同じ順)。
        support::prepare_read_model(&path);
        let projection = ProjectionName::parse("report-result-contract").unwrap();
        Self {
            root,
            store,
            writer,
            reader,
            path,
            projection,
        }
    }
    fn raw(&self) -> Connection {
        Connection::open(
            StorePath::for_space(&self.root.path().join("aidlc"), &SpaceName::default()).as_path(),
        )
        .unwrap()
    }
    async fn report(&mut self, verdict: Verdict, stage: Option<&str>) -> ReportId {
        let report_id = ReportId::generate();
        self.writer
            .advance(&mut self.store, |aggregate| {
                Ok(aggregate
                    .apply_report(
                        report_id.clone(),
                        &support::intent(),
                        &ReportRequest::new(
                            verdict,
                            stage.map(|s| {
                                core_command_domain::workflow_definition::StageSlug::parse(s)
                                    .unwrap()
                            }),
                            Some("Approve".into()),
                            None,
                            false,
                        ),
                        None,
                        support::at(),
                    )
                    .unwrap())
            })
            .await;
        report_id
    }
    async fn tables(&self) -> (GlobalSeqNr, ReadTables) {
        let history = self.reader.events_after(GlobalSeqNr::ZERO).await.unwrap();
        (
            history.scanned_to().unwrap(),
            ReadTables::project(&history).unwrap(),
        )
    }
    async fn project(&mut self) -> GlobalSeqNr {
        let (position, _) = self.tables().await;
        support::advance_structured(&self.path, &self.projection)
            .await
            .unwrap();
        position
    }
    fn result(
        &self,
        id: &ReportId,
    ) -> (
        String,
        String,
        String,
        String,
        Option<String>,
        Option<String>,
    ) {
        self.raw().query_row("SELECT execution_id, stage, result_kind, steps, no_op_reason, current_stage FROM read_report_result WHERE report_id=?1", [id.as_str()], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?))).unwrap()
    }
}

#[tokio::test]
async fn a_persisted_report_is_projected_with_its_callers_identity() {
    let mut fixture = Fixture::new().await;
    let report_id = fixture.report(Verdict::AwaitingApproval, None).await;
    let position = fixture.project().await;
    assert_eq!(
        fixture.result(&report_id),
        (
            support::EXECUTION.into(),
            "intent-capture".into(),
            "committed".into(),
            "[\"gate-start\"]".into(),
            None,
            None
        )
    );
    assert_eq!(
        fixture
            .reader
            .checkpoint(&fixture.projection)
            .await
            .unwrap(),
        position
    );
}

#[tokio::test]
async fn a_later_report_does_not_replace_an_earlier_result() {
    let mut fixture = Fixture::new().await;
    let first = fixture.report(Verdict::AwaitingApproval, None).await;
    fixture.project().await;
    let first_result = fixture.result(&first);
    let next = fixture.report(Verdict::Forward, None).await;
    fixture.project().await;
    assert_eq!(fixture.result(&first), first_result);
    assert_eq!(fixture.result(&next).3, "[\"approve\"]");
}

#[tokio::test]
async fn all_three_no_op_reasons_preserve_the_reported_and_current_stages() {
    let mut fixture = Fixture::new().await;
    fixture.report(Verdict::AwaitingApproval, None).await;
    let awaiting = fixture.report(Verdict::AwaitingApproval, None).await;
    fixture.report(Verdict::Forward, None).await;
    let moved = fixture
        .report(Verdict::Forward, Some("intent-capture"))
        .await;
    fixture
        .report(Verdict::Forward, Some("scope-definition"))
        .await;
    let completed = fixture.report(Verdict::Forward, None).await;
    fixture.project().await;
    assert_eq!(
        fixture.result(&awaiting).4.as_deref(),
        Some("already_awaiting")
    );
    assert_eq!(fixture.result(&moved).1, "intent-capture");
    assert_eq!(
        fixture.result(&moved).4.as_deref(),
        Some("already_completed_moved_on")
    );
    assert_eq!(
        fixture.result(&moved).5.as_deref(),
        Some("scope-definition")
    );
    assert_eq!(
        fixture.result(&completed).4.as_deref(),
        Some("workflow_already_completed")
    );
}

#[tokio::test]
async fn checkpoint_failure_rolls_back_results_and_retry_publishes_them() {
    let mut fixture = Fixture::new().await;
    let old_position = fixture.project().await;
    let report = fixture.report(Verdict::AwaitingApproval, None).await;
    let raw = fixture.raw();
    raw.execute_batch("CREATE TRIGGER fail_report_checkpoint BEFORE INSERT ON amadeus_projection_checkpoint BEGIN SELECT RAISE(ABORT,'checkpoint unavailable'); END").unwrap();
    assert!(
        support::advance_structured(&fixture.path, &fixture.projection)
            .await
            .is_err()
    );
    assert_eq!(
        fixture
            .reader
            .checkpoint(&fixture.projection)
            .await
            .unwrap(),
        old_position
    );
    assert_eq!(
        raw.query_row("SELECT count(*) FROM read_report_result", [], |row| row
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
    raw.execute_batch("DROP TRIGGER fail_report_checkpoint")
        .unwrap();
    fixture.project().await;
    assert_eq!(fixture.result(&report).2, "committed");
}

#[tokio::test]
async fn repeated_projection_keeps_exactly_one_result_per_report() {
    let mut fixture = Fixture::new().await;
    let report = fixture.report(Verdict::AwaitingApproval, None).await;
    let position = fixture.project().await;
    let result = fixture.result(&report);
    assert_eq!(fixture.project().await, position);
    assert_eq!(fixture.result(&report), result);
    assert_eq!(
        fixture
            .raw()
            .query_row("SELECT count(*) FROM read_report_result", [], |row| row
                .get::<_, i64>(0))
            .unwrap(),
        1
    );
}
