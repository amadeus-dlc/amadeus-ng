//! 開始基準を通常の公開計画へ含め、衝突と復旧を同じ保存経路で検証する。
#![allow(clippy::unwrap_used, clippy::expect_used)]
mod support;
use core_command_domain::{
    orchestration::SourceBaseline,
    workspace::{SpaceName, StorePath},
};
use core_read_model_updater::{
    orchestration::{
        GlobalSeqNr, JournalBatch, JournalReader, JournalReaderImpl, ProjectionName,
        ProjectionTargets, PublicationBatch, PublicationFile,
    },
    read_tables::ReadTables,
};
use std::{fs, path::PathBuf};
struct Fixture {
    root: tempfile::TempDir,
    store: StorePath,
    baseline: SourceBaseline,
}
impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let store = StorePath::for_space(&root.path().join("aidlc"), &SpaceName::default());
        fs::create_dir_all(store.as_path().parent().unwrap()).unwrap();
        drop(support::open_store(&store));
        Self {
            root,
            store,
            baseline: SourceBaseline::new(Some(String::new())).unwrap(),
        }
    }
    fn reader(&self) -> JournalReaderImpl {
        JournalReaderImpl::open(&self.store).unwrap()
    }
    fn targets(&self) -> ProjectionTargets {
        ProjectionTargets::new(
            self.root.path().join("record/aidlc-state.md"),
            self.root.path().join("record/audit/test.md"),
            self.root.path().join("memory"),
        )
    }
    fn path(&self) -> PathBuf {
        self.root
            .path()
            .join("record/.aidlc-source-review/code-generation")
            .join(self.baseline.snapshot_name().unwrap())
    }
    fn batch(&self) -> PublicationBatch {
        PublicationBatch::rebuild(
            GlobalSeqNr::ZERO,
            GlobalSeqNr::ZERO,
            vec![PublicationFile::creation(
                &self.path(),
                self.baseline.listing().unwrap(),
            )],
        )
        .for_targets(&self.targets())
        .unwrap()
    }
    fn tables() -> ReadTables {
        ReadTables::project(&JournalBatch::empty()).unwrap()
    }
    fn projection() -> ProjectionName {
        ProjectionName::parse("baseline-publication").unwrap()
    }
}
#[tokio::test]
async fn published_baseline_is_restored_after_connection_loss() {
    let fixture = Fixture::new();
    let mut reader = fixture.reader();
    let batch = fixture.batch();
    assert!(batch.matches_targets(&fixture.targets()));
    reader
        .publish(&Fixture::projection(), &batch, &Fixture::tables())
        .await
        .unwrap();
    drop(reader);
    fs::remove_file(fixture.path()).unwrap();
    let mut reader = fixture.reader();
    assert!(
        reader
            .restore_missing_files(&Fixture::projection(), &fixture.targets())
            .unwrap()
    );
    assert_eq!(fs::read(fixture.path()).unwrap(), b"");
}
#[tokio::test]
async fn conflicting_snapshot_is_not_overwritten_and_pending_publication_can_resume() {
    let fixture = Fixture::new();
    fs::create_dir_all(fixture.path().parent().unwrap()).unwrap();
    fs::write(fixture.path(), "existing evidence").unwrap();
    let mut reader = fixture.reader();
    assert!(
        reader
            .publish(&Fixture::projection(), &fixture.batch(), &Fixture::tables())
            .await
            .is_err()
    );
    assert_eq!(
        fs::read_to_string(fixture.path()).unwrap(),
        "existing evidence"
    );
    drop(reader);
    // 衝突したファイルの除去は利用者が行った修復の模擬。公開処理は除去しない。
    fs::remove_file(fixture.path()).unwrap();
    let mut reader = fixture.reader();
    let pending = reader
        .pending_publication(&Fixture::projection())
        .await
        .unwrap()
        .unwrap();
    assert!(pending.matches_targets(&fixture.targets()));
    reader
        .publish(&Fixture::projection(), &pending, &Fixture::tables())
        .await
        .unwrap();
    assert_eq!(fs::read(fixture.path()).unwrap(), b"");
}
#[tokio::test]
async fn restoring_other_files_refuses_a_corrupted_baseline() {
    let fixture = Fixture::new();
    let other = fixture.targets().state_file().to_path_buf();
    let batch = PublicationBatch::rebuild(
        GlobalSeqNr::ZERO,
        GlobalSeqNr::ZERO,
        vec![
            PublicationFile::creation(&fixture.path(), ""),
            PublicationFile::creation(&other, "state"),
        ],
    )
    .for_targets(&fixture.targets())
    .unwrap();
    let mut reader = fixture.reader();
    reader
        .publish(&Fixture::projection(), &batch, &Fixture::tables())
        .await
        .unwrap();
    fs::write(fixture.path(), "tampered").unwrap();
    fs::remove_file(&other).unwrap();
    assert!(
        reader
            .restore_missing_files(&Fixture::projection(), &fixture.targets())
            .is_err()
    );
    assert_eq!(fs::read_to_string(fixture.path()).unwrap(), "tampered");
    assert!(!other.exists());
}

#[tokio::test]
async fn resolving_other_conflicts_never_rebases_a_baseline() {
    let mut fixture = Fixture::new();
    let listing = format!("\tsource.rs\t100644\t{}\n", "a".repeat(64));
    fixture.baseline = SourceBaseline::new(Some(listing.clone())).unwrap();
    let state = fixture.targets().state_file().to_path_buf();
    fs::create_dir_all(state.parent().unwrap()).unwrap();
    fs::write(&state, "conflict").unwrap();
    let batch = PublicationBatch::rebuild(
        GlobalSeqNr::ZERO,
        GlobalSeqNr::ZERO,
        vec![
            PublicationFile::creation(&fixture.path(), &listing),
            PublicationFile::replacement(&state, "before", "after"),
        ],
    )
    .for_targets(&fixture.targets())
    .unwrap();
    let mut reader = fixture.reader();
    assert!(
        reader
            .publish(&Fixture::projection(), &batch, &Fixture::tables())
            .await
            .is_err()
    );
    fs::write(&state, "before").unwrap();
    fs::write(fixture.path(), format!("{listing}changed")).unwrap();
    assert!(
        reader
            .resolve_publication(&Fixture::projection(), &fixture.targets())
            .is_err()
    );
    assert_eq!(
        fs::read_to_string(fixture.path()).unwrap(),
        format!("{listing}changed")
    );
    assert_eq!(fs::read_to_string(state).unwrap(), "before");
}
