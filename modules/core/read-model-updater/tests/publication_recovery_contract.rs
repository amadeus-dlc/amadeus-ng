//! 公開計画の復旧契約。SQLiteに保存した計画と利用者のファイルを公開APIから検証する。
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use std::fs;
use std::io::ErrorKind;
use std::path::PathBuf;

use core_command_domain::workspace::{SpaceName, StorePath};
use core_read_model_updater::orchestration::{
    CatchUpError, GlobalSeqNr, JournalBatch, JournalReadError, JournalReader, JournalReaderImpl,
    ProjectionName, ProjectionTargets, PublicationBatch, PublicationFile,
};
use core_read_model_updater::read_tables::ReadTables;
use rusqlite::Connection;
use tempfile::TempDir;

struct Fixture {
    root: TempDir,
    store: StorePath,
    state: PathBuf,
    audit: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let store = StorePath::for_space(&root.path().join("aidlc"), &SpaceName::default());
        fs::create_dir_all(store.as_path().parent().unwrap()).unwrap();
        drop(support::open_store(&store));
        let state = root.path().join("state.md");
        let audit = root.path().join("audit.md");
        fs::write(&state, "before\n").unwrap();
        Self {
            root,
            store,
            state,
            audit,
        }
    }

    fn reader(&self) -> JournalReaderImpl {
        JournalReaderImpl::open(&self.store).unwrap()
    }

    fn raw(&self) -> Connection {
        Connection::open(self.store.as_path()).unwrap()
    }

    fn shared_head(&self) -> (i64, i64, String, String, bool) {
        self.raw().query_row(
            "SELECT position,generation,revision,content_digest,verified FROM amadeus_read_model_head WHERE singleton=1",
            [], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
        ).unwrap()
    }

    fn targets(&self) -> ProjectionTargets {
        ProjectionTargets::new(&self.state, &self.audit, self.root.path().join("memory"))
    }

    fn batch(&self) -> PublicationBatch {
        PublicationBatch::rebuild(
            GlobalSeqNr::ZERO,
            GlobalSeqNr::ZERO,
            vec![
                PublicationFile::replacement(&self.state, "before\n", "after\n"),
                PublicationFile::audit(&self.audit, "event\n").unwrap(),
            ],
        )
        .for_targets(&self.targets())
        .unwrap()
    }

    fn block_checkpoint(&self) {
        self.raw().execute_batch("CREATE TRIGGER fail_checkpoint BEFORE INSERT ON amadeus_projection_checkpoint BEGIN SELECT RAISE(ABORT,'checkpoint unavailable'); END").unwrap();
    }

    fn unblock_checkpoint(&self) {
        self.raw()
            .execute_batch("DROP TRIGGER fail_checkpoint")
            .unwrap();
    }
}

fn projection() -> ProjectionName {
    ProjectionName::parse("publication-contract").unwrap()
}

fn empty_tables() -> ReadTables {
    ReadTables::project(&JournalBatch::empty()).unwrap()
}

#[tokio::test]
async fn malformed_saved_plans_are_rejected_without_modifying_files() {
    // ヘッダ、ファイル順序、対象パス、内容の各破損を独立に検査する。
    for corruption in [
        "UPDATE amadeus_publication SET format_version=2",
        "UPDATE amadeus_publication SET start_position=-1",
        "UPDATE amadeus_publication SET target_position=-1",
        "UPDATE amadeus_publication SET start_position=1,target_position=0",
        "UPDATE amadeus_publication SET generation=0",
        "UPDATE amadeus_publication SET generation=-1",
        "UPDATE amadeus_publication SET request_id='not-a-uuid'",
        "UPDATE amadeus_publication SET file_count=3",
        "UPDATE amadeus_publication_file SET ordinal=-1 WHERE ordinal=0",
        "UPDATE amadeus_publication_file SET path='relative.md' WHERE ordinal=0",
        "DELETE FROM amadeus_publication_file WHERE ordinal=1",
        "UPDATE amadeus_publication SET plan_digest='tampered'",
    ] {
        let fixture = Fixture::new();
        let mut reader = fixture.reader();
        fixture.block_checkpoint();
        let batch = fixture.batch();
        assert!(
            reader
                .publish(&projection(), &batch, &empty_tables())
                .await
                .is_err()
        );
        let state = fs::read(&fixture.state).unwrap();
        let audit = fs::read(&fixture.audit).unwrap();
        fixture
            .raw()
            .execute_batch(&format!("PRAGMA ignore_check_constraints=ON; {corruption}"))
            .unwrap();

        assert!(
            matches!(
                reader.pending_publication(&projection()).await,
                Err(JournalReadError::Io {
                    kind: ErrorKind::InvalidData,
                    ..
                })
            ),
            "{corruption}"
        );
        assert!(
            reader
                .resolve_publication(&projection(), &fixture.targets())
                .is_err(),
            "{corruption}"
        );
        assert_eq!(fs::read(&fixture.state).unwrap(), state, "{corruption}");
        assert_eq!(fs::read(&fixture.audit).unwrap(), audit, "{corruption}");
        assert_eq!(
            reader.checkpoint(&projection()).await.unwrap(),
            GlobalSeqNr::ZERO,
            "{corruption}"
        );
    }
}

#[tokio::test]
async fn a_matching_concurrent_request_finishes_the_original_saved_bytes() {
    let fixture = Fixture::new();
    let mut reader = fixture.reader();
    fixture.block_checkpoint();
    let original = fixture.batch();
    assert!(
        reader
            .publish(&projection(), &original, &empty_tables())
            .await
            .is_err()
    );
    let saved = reader
        .pending_publication(&projection())
        .await
        .unwrap()
        .unwrap();
    let audit = fs::read(&fixture.audit).unwrap();
    fixture.unblock_checkpoint();
    let contender = PublicationBatch::rebuild(
        GlobalSeqNr::ZERO,
        GlobalSeqNr::ZERO,
        vec![
            PublicationFile::replacement(&fixture.state, "before\n", "different computation\n"),
            PublicationFile::audit(&fixture.audit, "another event\n").unwrap(),
        ],
    )
    .for_targets(&fixture.targets())
    .unwrap();
    assert_ne!(contender.request_id(), saved.request_id());

    fixture
        .reader()
        .publish(&projection(), &contender, &empty_tables())
        .await
        .unwrap();

    assert!(
        reader
            .pending_publication(&projection())
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(fs::read_to_string(&fixture.state).unwrap(), "after\n");
    assert_eq!(fs::read(&fixture.audit).unwrap(), audit);
}

#[tokio::test]
async fn unrelated_candidates_cannot_replace_an_unfinished_plan() {
    let fixture = Fixture::new();
    let mut reader = fixture.reader();
    fixture.block_checkpoint();
    let original = fixture.batch();
    assert!(
        reader
            .publish(&projection(), &original, &empty_tables())
            .await
            .is_err()
    );
    let saved = reader
        .pending_publication(&projection())
        .await
        .unwrap()
        .unwrap();
    fixture.unblock_checkpoint();
    let wrong_range = PublicationBatch::rebuild(
        GlobalSeqNr::ZERO,
        GlobalSeqNr::new(1),
        saved.files().to_vec(),
    )
    .for_targets(&fixture.targets())
    .unwrap();
    let wrong_paths = PublicationBatch::rebuild(GlobalSeqNr::ZERO, GlobalSeqNr::ZERO, vec![])
        .for_targets(&fixture.targets())
        .unwrap();
    let other_targets = ProjectionTargets::new(
        fixture.root.path().join("other.md"),
        &fixture.audit,
        fixture.root.path().join("memory"),
    );
    let wrong_binding = original.clone().for_targets(&other_targets).unwrap();
    for candidate in [wrong_range, wrong_paths, wrong_binding] {
        assert!(matches!(
            reader
                .publish(&projection(), &candidate, &empty_tables())
                .await,
            Err(CatchUpError::PublicationConflict { .. })
        ));
        assert_eq!(
            reader.pending_publication(&projection()).await.unwrap(),
            Some(saved.clone())
        );
    }
    reader
        .publish(&projection(), &saved, &empty_tables())
        .await
        .unwrap();
    assert_eq!(fs::read_to_string(&fixture.state).unwrap(), "after\n");
}

#[tokio::test]
async fn resending_an_archived_committed_request_does_not_reapply_it() {
    let fixture = Fixture::new();
    let mut reader = fixture.reader();
    let original = fixture.batch();
    reader
        .publish(&projection(), &original, &empty_tables())
        .await
        .unwrap();
    let successor = PublicationBatch::rebuild(
        GlobalSeqNr::ZERO,
        GlobalSeqNr::ZERO,
        vec![PublicationFile::replacement(
            &fixture.state,
            "after\n",
            "successor\n",
        )],
    )
    .for_targets(&fixture.targets())
    .unwrap();
    reader
        .publish(&projection(), &successor, &empty_tables())
        .await
        .unwrap();
    let audit = fs::read(&fixture.audit).unwrap();

    reader
        .publish(&projection(), &original, &empty_tables())
        .await
        .unwrap();

    assert_eq!(fs::read_to_string(&fixture.state).unwrap(), "successor\n");
    assert_eq!(fs::read(&fixture.audit).unwrap(), audit);
    assert!(
        reader
            .pending_publication(&projection())
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn resolving_preserves_prefixes_and_suffixes_before_and_after_application() {
    for (current, expected) in [
        ("before\nuser suffix\n", "after\nuser suffix\n"),
        ("user prefix\nbefore\n", "user prefix\nafter\n"),
        ("after\nuser suffix\n", "after\nuser suffix\n"),
        ("user prefix\nafter\n", "user prefix\nafter\n"),
    ] {
        let fixture = Fixture::new();
        let mut reader = fixture.reader();
        fixture.block_checkpoint();
        let batch = fixture.batch();
        assert!(
            reader
                .publish(&projection(), &batch, &empty_tables())
                .await
                .is_err()
        );
        fixture.unblock_checkpoint();
        fs::write(&fixture.state, current).unwrap();
        let audit = fs::read(&fixture.audit).unwrap();

        assert!(
            reader
                .resolve_publication(&projection(), &fixture.targets())
                .unwrap()
        );

        assert_eq!(fs::read_to_string(&fixture.state).unwrap(), expected);
        assert_eq!(fs::read(&fixture.audit).unwrap(), audit);
        assert!(
            !reader
                .resolve_publication(&projection(), &fixture.targets())
                .unwrap()
        );
    }
}

#[tokio::test]
async fn ambiguous_edits_and_deleted_outputs_remain_pending_without_overwrite() {
    for contents in [Some("unrelated user document\n"), None] {
        let fixture = Fixture::new();
        let mut reader = fixture.reader();
        fixture.block_checkpoint();
        let batch = fixture.batch();
        assert!(
            reader
                .publish(&projection(), &batch, &empty_tables())
                .await
                .is_err()
        );
        let saved = reader
            .pending_publication(&projection())
            .await
            .unwrap()
            .unwrap();
        fixture.unblock_checkpoint();
        if let Some(text) = contents {
            fs::write(&fixture.state, text).unwrap();
        } else {
            fs::remove_file(&fixture.state).unwrap();
        }
        let audit = fs::read(&fixture.audit).unwrap();

        assert!(
            matches!(reader.resolve_publication(&projection(), &fixture.targets()),
            Err(CatchUpError::PublicationConflict { path }) if path == fixture.state)
        );
        assert!(
            !reader
                .restore_missing_files(&projection(), &fixture.targets())
                .unwrap()
        );

        assert_eq!(fs::read_to_string(&fixture.state).ok().as_deref(), contents);
        assert_eq!(fs::read(&fixture.audit).unwrap(), audit);
        assert_eq!(
            reader.pending_publication(&projection()).await.unwrap(),
            Some(saved)
        );
    }
}

#[tokio::test]
async fn recovery_rejects_targets_that_do_not_own_the_saved_files() {
    let fixture = Fixture::new();
    let mut reader = fixture.reader();
    fixture.block_checkpoint();
    assert!(
        reader
            .publish(&projection(), &fixture.batch(), &empty_tables())
            .await
            .is_err()
    );
    fixture.unblock_checkpoint();
    let other = ProjectionTargets::new(
        fixture.root.path().join("other.md"),
        &fixture.audit,
        fixture.root.path().join("memory"),
    );

    assert!(matches!(
        reader.resolve_publication(&projection(), &other),
        Err(CatchUpError::PublicationConflict { .. })
    ));
    assert_eq!(fs::read_to_string(&fixture.state).unwrap(), "after\n");
    assert!(
        reader
            .pending_publication(&projection())
            .await
            .unwrap()
            .is_some()
    );
}

#[tokio::test]
async fn restoring_a_missing_audit_retains_the_users_existing_state_and_memory() {
    let fixture = Fixture::new();
    let targets = fixture.targets();
    fs::create_dir_all(targets.team_md().parent().unwrap()).unwrap();
    fs::write(targets.team_md(), "rules\n").unwrap();
    let batch = PublicationBatch::rebuild(
        GlobalSeqNr::ZERO,
        GlobalSeqNr::ZERO,
        vec![
            PublicationFile::replacement(&fixture.state, "before\n", "after\n"),
            PublicationFile::memory(targets.team_md(), "rules\n", "affirmed rules\n"),
            PublicationFile::audit(&fixture.audit, "event\n").unwrap(),
        ],
    )
    .for_targets(&targets)
    .unwrap();
    let mut reader = fixture.reader();
    assert!(
        !reader
            .restore_missing_files(&projection(), &targets)
            .unwrap()
    );
    reader
        .publish(&projection(), &batch, &empty_tables())
        .await
        .unwrap();
    let audit = fs::read(&fixture.audit).unwrap();
    fs::write(&fixture.state, "user state\n").unwrap();
    fs::write(targets.team_md(), "user rules\n").unwrap();
    fs::remove_file(&fixture.audit).unwrap();

    assert!(
        reader
            .restore_missing_files(&projection(), &targets)
            .unwrap()
    );

    assert_eq!(fs::read(&fixture.audit).unwrap(), audit);
    assert_eq!(fs::read_to_string(&fixture.state).unwrap(), "user state\n");
    assert_eq!(
        fs::read_to_string(targets.team_md()).unwrap(),
        "user rules\n"
    );
}

#[tokio::test]
async fn invalid_candidates_leave_no_pending_plan_or_file_changes() {
    let fixture = Fixture::new();
    let mut reader = fixture.reader();
    let relative = PublicationBatch::rebuild(
        GlobalSeqNr::ZERO,
        GlobalSeqNr::ZERO,
        vec![PublicationFile::replacement(
            std::path::Path::new("relative.md"),
            "before",
            "after",
        )],
    );
    let invalid_range = PublicationBatch::rebuild(GlobalSeqNr::new(1), GlobalSeqNr::ZERO, vec![]);
    let wrong_start = PublicationBatch::new(GlobalSeqNr::new(1), GlobalSeqNr::new(2), vec![]);
    let unrepresentable_position =
        PublicationBatch::new(GlobalSeqNr::ZERO, GlobalSeqNr::new(u64::MAX), vec![]);
    for candidate in [
        relative,
        invalid_range,
        wrong_start,
        unrepresentable_position,
    ] {
        assert!(
            reader
                .publish(&projection(), &candidate, &empty_tables())
                .await
                .is_err()
        );
        assert!(
            reader
                .pending_publication(&projection())
                .await
                .unwrap()
                .is_none()
        );
        assert_eq!(fs::read_to_string(&fixture.state).unwrap(), "before\n");
        assert!(!fixture.audit.exists());
    }
}

#[tokio::test]
async fn a_committed_generation_cannot_be_submitted_to_another_projection() {
    let fixture = Fixture::new();
    let mut reader = fixture.reader();
    fixture.block_checkpoint();
    assert!(
        reader
            .publish(&projection(), &fixture.batch(), &empty_tables())
            .await
            .is_err()
    );
    let accepted = reader
        .pending_publication(&projection())
        .await
        .unwrap()
        .unwrap();
    fixture.unblock_checkpoint();
    let other = ProjectionName::parse("other-projection").unwrap();

    assert!(matches!(
        reader.publish(&other, &accepted, &empty_tables()).await,
        Err(CatchUpError::PublicationConflict { .. })
    ));
    assert!(reader.pending_publication(&other).await.unwrap().is_none());
    assert_eq!(
        reader.pending_publication(&projection()).await.unwrap(),
        Some(accepted)
    );
}

#[tokio::test]
async fn a_publication_already_covered_by_the_cursor_does_not_touch_files() {
    let fixture = Fixture::new();
    support::seed_intent(&fixture.store).await;
    let mut reader = fixture.reader();
    let history = reader.events_after(GlobalSeqNr::ZERO).await.unwrap();
    let tables = ReadTables::project(&history).unwrap();
    let last = history.scanned_to().unwrap();
    reader
        .advance_checkpoint(&projection(), last, &tables)
        .await
        .unwrap();
    let obsolete = PublicationBatch::new(
        GlobalSeqNr::ZERO,
        last,
        vec![PublicationFile::replacement(
            &fixture.state,
            "before\n",
            "obsolete\n",
        )],
    );

    reader
        .publish(&projection(), &obsolete, &tables)
        .await
        .unwrap();

    assert_eq!(fs::read_to_string(&fixture.state).unwrap(), "before\n");
    assert!(
        reader
            .pending_publication(&projection())
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn failures_while_preparing_a_plan_roll_back_without_publishing() {
    for trigger in [
        "CREATE TRIGGER fail_prepare BEFORE INSERT ON amadeus_publication BEGIN SELECT RAISE(ABORT,'no plan'); END",
        "CREATE TRIGGER fail_prepare BEFORE INSERT ON amadeus_publication_file BEGIN SELECT RAISE(ABORT,'no file plan'); END",
    ] {
        let fixture = Fixture::new();
        let mut reader = fixture.reader();
        let batch = fixture.batch();
        fixture.raw().execute_batch(trigger).unwrap();

        let error = reader
            .publish(&projection(), &batch, &empty_tables())
            .await
            .unwrap_err();

        assert_eq!(
            error,
            CatchUpError::Read(JournalReadError::Io {
                kind: ErrorKind::Other,
                path: Some(fixture.store.as_path().to_path_buf()),
            })
        );
        assert_eq!(fs::read_to_string(&fixture.state).unwrap(), "before\n");
        assert!(!fixture.audit.exists());
        assert!(
            reader
                .pending_publication(&projection())
                .await
                .unwrap()
                .is_none()
        );
        fixture
            .raw()
            .execute_batch("DROP TRIGGER fail_prepare")
            .unwrap();
        reader
            .publish(&projection(), &batch, &empty_tables())
            .await
            .unwrap();
        assert_eq!(fs::read_to_string(&fixture.state).unwrap(), "after\n");
    }
}

#[tokio::test]
async fn failures_after_file_publication_keep_a_resumable_plan() {
    for trigger in [
        "CREATE TRIGGER fail_finish BEFORE UPDATE OF committed ON amadeus_publication WHEN NEW.committed=1 BEGIN SELECT RAISE(ABORT,'no confirmation'); END",
        "CREATE TRIGGER fail_finish BEFORE INSERT ON amadeus_publication_snapshot BEGIN SELECT RAISE(ABORT,'no snapshot'); END",
        "CREATE TRIGGER fail_finish BEFORE INSERT ON amadeus_publication_snapshot_file BEGIN SELECT RAISE(ABORT,'no snapshot file'); END",
    ] {
        let fixture = Fixture::new();
        let mut reader = fixture.reader();
        let batch = fixture.batch();
        fixture.raw().execute_batch(trigger).unwrap();

        assert_eq!(
            reader.publish(&projection(), &batch, &empty_tables()).await,
            Err(CatchUpError::Read(JournalReadError::Io {
                kind: ErrorKind::Other,
                path: Some(fixture.store.as_path().to_path_buf()),
            }))
        );

        let pending = reader
            .pending_publication(&projection())
            .await
            .unwrap()
            .unwrap();
        let audit = fs::read(&fixture.audit).unwrap();
        assert_eq!(fs::read_to_string(&fixture.state).unwrap(), "after\n");
        fixture
            .raw()
            .execute_batch("DROP TRIGGER fail_finish")
            .unwrap();

        fixture
            .reader()
            .publish(&projection(), &pending, &empty_tables())
            .await
            .unwrap();
        assert!(
            reader
                .pending_publication(&projection())
                .await
                .unwrap()
                .is_none()
        );
        assert_eq!(fs::read(&fixture.audit).unwrap(), audit);
        assert_eq!(fs::read_to_string(&fixture.state).unwrap(), "after\n");
    }
}

#[tokio::test]
async fn a_busy_store_reports_contention_before_saving_or_writing_files() {
    let fixture = Fixture::new();
    let mut reader =
        JournalReaderImpl::open_with_busy_timeout(&fixture.store, std::time::Duration::ZERO)
            .unwrap();
    let connection = fixture.raw();
    connection.execute_batch("BEGIN IMMEDIATE").unwrap();
    let batch = fixture.batch();

    let error = reader
        .publish(&projection(), &batch, &empty_tables())
        .await
        .unwrap_err();

    assert!(
        matches!(error, CatchUpError::Read(JournalReadError::Io { kind: ErrorKind::WouldBlock, path: Some(path) }) if path == fixture.store.as_path())
    );
    assert_eq!(fs::read_to_string(&fixture.state).unwrap(), "before\n");
    assert!(
        reader
            .pending_publication(&projection())
            .await
            .unwrap()
            .is_none()
    );
    connection.execute_batch("ROLLBACK").unwrap();
    reader
        .publish(&projection(), &batch, &empty_tables())
        .await
        .unwrap();
}

#[tokio::test]
async fn a_shared_head_cannot_claim_missing_history_or_overflow_its_generation() {
    for mutation in [
        "UPDATE amadeus_read_model_head SET position=1,verified=0",
        "UPDATE amadeus_read_model_head SET generation=9223372036854775807,verified=0",
    ] {
        let fixture = Fixture::new();
        let mut reader = fixture.reader();
        fixture.raw().execute_batch(mutation).unwrap();

        let error = reader
            .publish(&projection(), &fixture.batch(), &empty_tables())
            .await
            .unwrap_err();

        assert!(matches!(
            error,
            CatchUpError::Read(JournalReadError::Corrupt { .. })
        ));
        assert_eq!(fs::read_to_string(&fixture.state).unwrap(), "before\n");
        assert!(!fixture.audit.exists());
        assert!(
            reader
                .pending_publication(&projection())
                .await
                .unwrap()
                .is_some()
        );
    }
}

#[tokio::test]
async fn an_old_transform_is_rebuilt_at_the_write_boundary() {
    let fixture = Fixture::new();
    support::seed_intent(&fixture.store).await;
    let mut reader = fixture.reader();
    let history = reader.events_after(GlobalSeqNr::ZERO).await.unwrap();
    let tables = ReadTables::project(&history).unwrap();
    let last = history.scanned_to().unwrap();
    reader
        .advance_checkpoint(&projection(), last, &tables)
        .await
        .unwrap();
    drop(reader);
    fixture
        .raw()
        .execute_batch(
            "DELETE FROM read_intent; UPDATE amadeus_read_model_head SET revision='old-transform'",
        )
        .unwrap();

    let mut reopened = fixture.reader();
    let untouched: i64 = fixture
        .raw()
        .query_row("SELECT count(*) FROM read_intent", [], |r| r.get(0))
        .unwrap();
    assert_eq!(untouched, 0, "openだけでは全履歴を投影しない");
    assert_eq!(reopened.checkpoint(&projection()).await.unwrap(), last);
    core_read_model_updater::orchestration::ReadModelUpdater::<JournalReaderImpl>::catch_up_structured(&mut reopened, &projection()).await.unwrap();

    // 同位置の正しい断面を再提示できることが、再生成された内容一致の検収。
    reopened
        .advance_checkpoint(&projection(), last, &tables)
        .await
        .unwrap();
    assert_eq!(reopened.checkpoint(&projection()).await.unwrap(), last);
    assert_eq!(fs::read_to_string(&fixture.state).unwrap(), "before\n");
}

#[tokio::test]
async fn explicit_resolution_completes_a_partial_audit_without_duplicate_bytes() {
    let fixture = Fixture::new();
    let mut reader = fixture.reader();
    fixture.block_checkpoint();
    assert!(
        reader
            .publish(&projection(), &fixture.batch(), &empty_tables())
            .await
            .is_err()
    );
    let complete = fs::read(&fixture.audit).unwrap();
    let partial = complete.strip_suffix(b"event\n").unwrap();
    fs::write(&fixture.audit, partial).unwrap();
    fixture.unblock_checkpoint();

    assert!(
        reader
            .resolve_publication(&projection(), &fixture.targets())
            .unwrap()
    );

    assert_eq!(fs::read(&fixture.audit).unwrap(), complete);
    assert_eq!(fs::read_to_string(&fixture.state).unwrap(), "after\n");
}

#[tokio::test]
async fn a_legacy_unverified_head_must_match_its_reconstructed_rows() {
    let fixture = Fixture::new();
    support::seed_intent(&fixture.store).await;
    let mut reader = fixture.reader();
    fixture
        .raw()
        .execute_batch("UPDATE amadeus_read_model_head SET position=1,verified=0")
        .unwrap();

    let error = reader
        .publish(&projection(), &fixture.batch(), &empty_tables())
        .await
        .unwrap_err();

    assert!(matches!(
        error,
        CatchUpError::Read(JournalReadError::Corrupt { .. })
    ));
    assert_eq!(fs::read_to_string(&fixture.state).unwrap(), "before\n");
    assert!(
        reader
            .pending_publication(&projection())
            .await
            .unwrap()
            .is_some()
    );
}

#[tokio::test]
async fn preparing_an_old_transform_preserves_history_failure_classification() {
    for corrupt_payload in [false, true] {
        let fixture = Fixture::new();
        // Intentの誕生を欠く実行ストリームだけを保存する。
        support::seed(&mut support::open_store(&fixture.store)).await;
        let reader = fixture.reader();
        fixture
            .raw()
            .execute_batch("UPDATE amadeus_read_model_head SET revision='old-transform'")
            .unwrap();
        if corrupt_payload {
            fixture
                .raw()
                .execute_batch("UPDATE journal SET payload=X'00'")
                .unwrap();
        }
        drop(reader);

        let mut reopened = JournalReaderImpl::open(&fixture.store).expect("openでは再投影しない");
        let failure = reopened.prepare_read_model().unwrap_err();
        if corrupt_payload {
            assert!(matches!(
                failure,
                CatchUpError::Read(JournalReadError::Corrupt { .. })
            ));
        } else {
            assert!(
                matches!(failure, CatchUpError::ReadTables(_)),
                "{failure:?}"
            );
        }
        assert_eq!(fs::read_to_string(&fixture.state).unwrap(), "before\n");
    }
}

#[tokio::test]
async fn restoration_refuses_a_checkpoint_behind_its_saved_snapshot() {
    let fixture = Fixture::new();
    support::seed_intent(&fixture.store).await;
    let mut reader = fixture.reader();
    let history = reader.events_after(GlobalSeqNr::ZERO).await.unwrap();
    let last = history.scanned_to().unwrap();
    let tables = ReadTables::project(&history).unwrap();
    let batch = PublicationBatch::new(
        GlobalSeqNr::ZERO,
        last,
        vec![PublicationFile::replacement(
            &fixture.state,
            "before\n",
            "after\n",
        )],
    )
    .for_targets(&fixture.targets())
    .unwrap();
    reader
        .publish(&projection(), &batch, &tables)
        .await
        .unwrap();
    fs::remove_file(&fixture.state).unwrap();
    fixture.raw().execute_batch("UPDATE amadeus_projection_checkpoint SET last_global_seq=0,anchor_aid=NULL,anchor_seq_nr=NULL").unwrap();

    assert!(matches!(
        reader.restore_missing_files(&projection(), &fixture.targets()),
        Err(CatchUpError::PublicationConflict { .. })
    ));

    assert!(!fixture.state.exists());
}

#[tokio::test]
async fn catch_up_refuses_a_saved_plan_for_different_targets_before_publication() {
    use core_read_model_updater::orchestration::{ReadModelUpdater, SteeringSource};
    let fixture = Fixture::new();
    let mut reader = fixture.reader();
    fixture.block_checkpoint();
    assert!(
        reader
            .publish(&projection(), &fixture.batch(), &empty_tables())
            .await
            .is_err()
    );
    fixture.unblock_checkpoint();
    let other_state = fixture.root.path().join("other-state.md");
    fs::write(&other_state, "other user's state\n").unwrap();
    let targets = ProjectionTargets::new(
        &other_state,
        &fixture.audit,
        fixture.root.path().join("memory"),
    );
    let mut updater = ReadModelUpdater::new(
        reader,
        projection(),
        targets,
        SteeringSource::new(fixture.root.path().join("memory")),
    );

    assert_eq!(
        updater.catch_up().await,
        Err(CatchUpError::PublicationConflict {
            path: other_state.clone()
        })
    );

    assert_eq!(
        fs::read_to_string(&other_state).unwrap(),
        "other user's state\n"
    );
    assert_eq!(fs::read_to_string(&fixture.state).unwrap(), "after\n");
}

#[tokio::test]
async fn catch_up_refuses_a_saved_cut_that_has_disappeared_from_history() {
    use core_read_model_updater::orchestration::{ReadModelUpdater, SteeringSource};
    let fixture = Fixture::new();
    let mut reader = fixture.reader();
    let batch = PublicationBatch::new(
        GlobalSeqNr::ZERO,
        GlobalSeqNr::new(1),
        vec![PublicationFile::replacement(
            &fixture.state,
            "before\n",
            "after\n",
        )],
    )
    .for_targets(&fixture.targets())
    .unwrap();
    assert!(
        reader
            .publish(&projection(), &batch, &empty_tables())
            .await
            .is_err()
    );
    let saved = reader
        .pending_publication(&projection())
        .await
        .unwrap()
        .unwrap();
    let mut updater = ReadModelUpdater::new(
        reader,
        projection(),
        fixture.targets(),
        SteeringSource::new(fixture.root.path().join("memory")),
    );

    assert_eq!(updater.catch_up().await, Err(CatchUpError::PlanUnavailable));

    assert_eq!(
        fixture
            .reader()
            .pending_publication(&projection())
            .await
            .unwrap(),
        Some(saved)
    );
    assert_eq!(fs::read_to_string(&fixture.state).unwrap(), "after\n");
}

#[tokio::test]
async fn typed_shared_row_corruption_blocks_old_and_same_position_publications_until_rebuilt() {
    for (mutation, column, corrupted_type, restored_type) in [
        (
            "UPDATE read_execution SET cursor_index=0.5",
            "cursor_index",
            "real",
            "integer",
        ),
        (
            "UPDATE read_execution SET scope=CAST(scope AS BLOB)",
            "scope",
            "blob",
            "text",
        ),
    ] {
        for same_position in [false, true] {
            let fixture = Fixture::new();
            support::seed_intent(&fixture.store).await;
            support::seed(&mut support::open_store(&fixture.store)).await;
            let mut reader = fixture.reader();
            let history = reader.events_after(GlobalSeqNr::ZERO).await.unwrap();
            let last = history.scanned_to().unwrap();
            let complete = ReadTables::project(&history).unwrap();
            let ahead = ProjectionName::parse("ahead").unwrap();
            reader
                .advance_checkpoint(&ahead, last, &complete)
                .await
                .unwrap();
            let head = fixture.shared_head();
            let target = if same_position {
                last
            } else {
                GlobalSeqNr::new(last.to_u64() - 1)
            };
            let candidate_tables =
                ReadTables::project(&reader.events_through(target).await.unwrap()).unwrap();
            let batch = PublicationBatch::new(
                GlobalSeqNr::ZERO,
                target,
                vec![PublicationFile::replacement(
                    &fixture.state,
                    "before\n",
                    "after\n",
                )],
            )
            .for_targets(&fixture.targets())
            .unwrap();
            fixture.raw().execute_batch(mutation).unwrap();
            let sqlite_type = || {
                fixture
                    .raw()
                    .query_row(
                        &format!("SELECT typeof({column}) FROM read_execution"),
                        [],
                        |row| row.get::<_, String>(0),
                    )
                    .unwrap()
            };
            assert_eq!(
                sqlite_type(),
                corrupted_type,
                "SQLiteの型親和性で元の型へ戻されていない"
            );

            assert!(matches!(
                reader
                    .publish(&projection(), &batch, &candidate_tables)
                    .await,
                Err(CatchUpError::Read(JournalReadError::Corrupt { .. }))
            ));

            assert_eq!(fs::read_to_string(&fixture.state).unwrap(), "before\n");
            assert_eq!(
                reader.checkpoint(&projection()).await.unwrap(),
                GlobalSeqNr::ZERO
            );
            assert_eq!(fixture.shared_head(), head);
            assert_eq!(sqlite_type(), corrupted_type);
            let saved = reader
                .pending_publication(&projection())
                .await
                .unwrap()
                .unwrap();
            assert_eq!(reader.rebuild_read_model().unwrap(), last);
            assert_eq!(sqlite_type(), restored_type);
            reader
                .publish(&projection(), &saved, &candidate_tables)
                .await
                .unwrap();
            assert_eq!(fs::read_to_string(&fixture.state).unwrap(), "after\n");
            assert_eq!(reader.checkpoint(&projection()).await.unwrap(), target);
            assert_eq!(reader.checkpoint(&ahead).await.unwrap(), last);
            assert_eq!(
                fixture.shared_head().0,
                i64::try_from(last.to_u64()).unwrap()
            );
        }
    }
}

#[tokio::test]
async fn a_same_position_comparison_failure_preserves_rows_head_and_checkpoint() {
    let fixture = Fixture::new();
    support::seed_intent(&fixture.store).await;
    support::seed(&mut support::open_store(&fixture.store)).await;
    let mut reader = fixture.reader();
    let history = reader.events_after(GlobalSeqNr::ZERO).await.unwrap();
    let last = history.scanned_to().unwrap();
    let tables = ReadTables::project(&history).unwrap();
    let ahead = ProjectionName::parse("ahead").unwrap();
    reader
        .advance_checkpoint(&ahead, last, &tables)
        .await
        .unwrap();
    let head = fixture.shared_head();
    let execution_before: (String, String, i64) = fixture
        .raw()
        .query_row("SELECT id,status,seq_nr FROM read_execution", [], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })
        .unwrap();
    let stages_before: i64 = fixture
        .raw()
        .query_row("SELECT COUNT(*) FROM read_intent_stage", [], |row| {
            row.get(0)
        })
        .unwrap();
    // DELETEとIntent/Executionの再挿入が済んだ後で、比較用のステージ挿入を拒否する。
    fixture.raw().execute_batch("CREATE TRIGGER fail_compare BEFORE INSERT ON read_execution_stage BEGIN SELECT RAISE(ABORT,'comparison insert unavailable'); END").unwrap();

    assert_eq!(
        reader
            .advance_checkpoint(&projection(), last, &tables)
            .await,
        Err(JournalReadError::Io {
            kind: ErrorKind::Other,
            path: Some(fixture.store.as_path().to_path_buf())
        })
    );

    let execution_after: (String, String, i64) = fixture
        .raw()
        .query_row("SELECT id,status,seq_nr FROM read_execution", [], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })
        .unwrap();
    assert_eq!(execution_after, execution_before);
    assert_eq!(
        fixture
            .raw()
            .query_row("SELECT COUNT(*) FROM read_intent_stage", [], |row| row
                .get::<_, i64>(0))
            .unwrap(),
        stages_before
    );
    assert_eq!(fixture.shared_head(), head);
    assert_eq!(
        reader.checkpoint(&projection()).await.unwrap(),
        GlobalSeqNr::ZERO
    );
    assert_eq!(reader.checkpoint(&ahead).await.unwrap(), last);
    fixture
        .raw()
        .execute_batch("DROP TRIGGER fail_compare")
        .unwrap();
    // 同じ候補の再試行は全表の一致比較を通る。
    reader
        .advance_checkpoint(&projection(), last, &tables)
        .await
        .unwrap();
    assert_eq!(reader.checkpoint(&projection()).await.unwrap(), last);
    assert_eq!(fixture.shared_head(), head);
}

#[tokio::test]
async fn losing_the_shared_head_during_checkpoint_write_rolls_back_and_keeps_the_plan() {
    let fixture = Fixture::new();
    let mut reader = fixture.reader();
    let before_head = fixture.shared_head();
    let batch = fixture.batch();
    fixture.raw().execute_batch("CREATE TRIGGER remove_head AFTER INSERT ON amadeus_projection_checkpoint BEGIN DELETE FROM amadeus_read_model_head; END").unwrap();

    assert_eq!(
        reader.publish(&projection(), &batch, &empty_tables()).await,
        Err(CatchUpError::PublicationConflict {
            path: fixture.store.as_path().to_path_buf()
        })
    );

    assert_eq!(fixture.shared_head(), before_head);
    let checkpoint_rows: i64 = fixture
        .raw()
        .query_row(
            "SELECT COUNT(*) FROM amadeus_projection_checkpoint",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(checkpoint_rows, 0);
    let pending = reader
        .pending_publication(&projection())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(pending.request_id(), batch.request_id());
    assert_eq!(fs::read_to_string(&fixture.state).unwrap(), "after\n");
    let audit = fs::read(&fixture.audit).unwrap();
    fixture
        .raw()
        .execute_batch("DROP TRIGGER remove_head")
        .unwrap();

    fixture
        .reader()
        .publish(&projection(), &pending, &empty_tables())
        .await
        .unwrap();

    assert!(
        reader
            .pending_publication(&projection())
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(fs::read(&fixture.audit).unwrap(), audit);
    assert_eq!(fs::read_to_string(&fixture.state).unwrap(), "after\n");
    assert!(fixture.shared_head().4);
}

/// headが失われても、公開行の終点より古い履歴への巻戻りを拒否する。
#[tokio::test]
async fn rebuilding_without_a_head_preserves_the_known_published_cut() {
    let fixture = Fixture::new();
    support::seed_intent(&fixture.store).await;
    let mut reader = fixture.reader();
    let history = reader.events_after(GlobalSeqNr::ZERO).await.unwrap();
    let last = history.scanned_to().unwrap();
    reader
        .advance_checkpoint(&projection(), last, &ReadTables::project(&history).unwrap())
        .await
        .unwrap();
    fixture
        .raw()
        .execute_batch(
            "DELETE FROM amadeus_read_model_head; DELETE FROM amadeus_projection_checkpoint",
        )
        .unwrap();
    assert_eq!(reader.rebuild_read_model().unwrap(), last);
    assert_eq!(fixture.shared_head().0, 1);
    fixture
        .raw()
        .execute_batch("DELETE FROM amadeus_read_model_head; DELETE FROM journal")
        .unwrap();
    assert!(matches!(
        reader.rebuild_read_model(),
        Err(CatchUpError::Read(JournalReadError::Corrupt {
            cause: core_read_model_updater::orchestration::CorruptCause::CheckpointAnchorMismatch,
            ..
        }))
    ));
    let rows: i64 = fixture
        .raw()
        .query_row("SELECT count(*) FROM read_intent WHERE as_of=1", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(rows, 1, "公開済みの行を消して成功したことにしない");
}

/// 欠落ファイルの復元も、反映後の確定失敗から保存計画で再開できる。
#[tokio::test]
async fn an_interrupted_restoration_keeps_a_resumable_plan() {
    let fixture = Fixture::new();
    let mut reader = fixture.reader();
    reader
        .publish(&projection(), &fixture.batch(), &empty_tables())
        .await
        .unwrap();
    let state = fs::read(&fixture.state).unwrap();
    let audit = fs::read(&fixture.audit).unwrap();
    fs::remove_file(&fixture.state).unwrap();
    fixture.block_checkpoint();
    let error = reader
        .restore_missing_files(&projection(), &fixture.targets())
        .unwrap_err();
    assert!(matches!(
        error,
        CatchUpError::Read(JournalReadError::Io {
            kind: ErrorKind::Other,
            ..
        })
    ));
    assert_eq!(fs::read(&fixture.state).unwrap(), state);
    assert_eq!(fs::read(&fixture.audit).unwrap(), audit);
    let pending = reader
        .pending_publication(&projection())
        .await
        .unwrap()
        .unwrap();
    fixture.unblock_checkpoint();
    reader
        .publish(&projection(), &pending, &empty_tables())
        .await
        .unwrap();
    assert!(
        reader
            .pending_publication(&projection())
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        !reader
            .restore_missing_files(&projection(), &fixture.targets())
            .unwrap()
    );
    assert_eq!(fs::read(&fixture.state).unwrap(), state);
    assert_eq!(fs::read(&fixture.audit).unwrap(), audit);
}

/// 利用者編集を保持した解決計画が確定に失敗しても、新世代を再開できる。
#[tokio::test]
async fn an_interrupted_resolution_keeps_user_text_and_a_resumable_generation() {
    let fixture = Fixture::new();
    let mut reader = fixture.reader();
    fixture.block_checkpoint();
    assert!(
        reader
            .publish(&projection(), &fixture.batch(), &empty_tables())
            .await
            .is_err()
    );
    let initial = reader
        .pending_publication(&projection())
        .await
        .unwrap()
        .unwrap();
    let audit = fs::read(&fixture.audit).unwrap();
    fs::write(&fixture.state, "after\nuser addition\n").unwrap();
    assert!(matches!(
        reader.resolve_publication(&projection(), &fixture.targets()),
        Err(CatchUpError::Read(JournalReadError::Io {
            kind: ErrorKind::Other,
            ..
        }))
    ));
    let replacement = reader
        .pending_publication(&projection())
        .await
        .unwrap()
        .unwrap();
    assert!(replacement.generation() > initial.generation());
    assert_eq!(
        fs::read_to_string(&fixture.state).unwrap(),
        "after\nuser addition\n"
    );
    fixture.unblock_checkpoint();
    reader
        .publish(&projection(), &replacement, &empty_tables())
        .await
        .unwrap();
    assert_eq!(fs::read(&fixture.audit).unwrap(), audit);
    assert_eq!(
        fs::read_to_string(&fixture.state).unwrap(),
        "after\nuser addition\n"
    );
    assert!(
        reader
            .pending_publication(&projection())
            .await
            .unwrap()
            .is_none()
    );
}

/// 解決計画を作る前に履歴の破損を検出し、未完計画と現物を保持する。
#[tokio::test]
async fn resolution_refuses_corrupt_history_before_replacing_the_saved_plan() {
    let fixture = Fixture::new();
    let mut reader = fixture.reader();
    support::seed_intent(&fixture.store).await;
    let history = reader.events_after(GlobalSeqNr::ZERO).await.unwrap();
    let last = history.scanned_to().unwrap();
    let tables = ReadTables::project(&history).unwrap();
    let batch = PublicationBatch::new(
        GlobalSeqNr::ZERO,
        last,
        vec![PublicationFile::replacement(
            &fixture.state,
            "before\n",
            "after\n",
        )],
    )
    .for_targets(&fixture.targets())
    .unwrap();
    fixture.block_checkpoint();
    assert!(
        reader
            .publish(&projection(), &batch, &tables)
            .await
            .is_err()
    );
    let saved = reader
        .pending_publication(&projection())
        .await
        .unwrap()
        .unwrap();
    fixture.unblock_checkpoint();
    fixture
        .raw()
        .execute_batch("UPDATE journal SET payload=X'00'")
        .unwrap();
    assert!(matches!(
        reader.resolve_publication(&projection(), &fixture.targets()),
        Err(CatchUpError::Read(JournalReadError::Corrupt { .. }))
    ));
    assert_eq!(
        reader.pending_publication(&projection()).await.unwrap(),
        Some(saved)
    );
    assert_eq!(fs::read_to_string(&fixture.state).unwrap(), "after\n");
    assert_eq!(
        reader.checkpoint(&projection()).await.unwrap(),
        GlobalSeqNr::ZERO
    );
}
