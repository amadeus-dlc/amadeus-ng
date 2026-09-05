//! ファイル公開境界の契約。利用者の編集・削除・アクセス拒否を上書きしない。
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::io::ErrorKind;
use std::os::unix::ffi::OsStringExt;
use std::os::unix::fs::{PermissionsExt, symlink};

use core_read_model_updater::orchestration::{
    CatchUpError, GlobalSeqNr, ProjectionTargets, PublicationBatch, PublicationFile,
};
use core_read_model_updater::workspace::StateFileWriteError;

#[test]
fn an_empty_audit_does_not_create_an_output() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("absent/audit.md");
    PublicationFile::audit(&path, "").unwrap().apply().unwrap();
    assert!(!path.exists());
    assert!(!path.parent().unwrap().exists());
}

#[test]
fn an_audit_removed_or_rewritten_after_planning_is_not_recreated() {
    for text in [None, Some("user replacement\n")] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("audit.md");
        fs::write(&path, "original\n").unwrap();
        let plan = PublicationFile::audit(&path, "next\n").unwrap();
        if let Some(text) = text {
            fs::write(&path, text).unwrap();
        } else {
            fs::remove_file(&path).unwrap();
        }

        let error = plan.apply().unwrap_err();
        assert_eq!(
            error,
            CatchUpError::PublicationConflict { path: path.clone() }
        );
        assert_eq!(
            error.to_string(),
            format!("publication conflict: {}", path.display())
        );
        assert!(std::error::Error::source(&error).is_none());
        assert_eq!(fs::read_to_string(&path).ok().as_deref(), text);
    }
}

#[test]
fn replacement_refuses_a_deleted_or_changed_original() {
    for text in [None, Some("user replacement\n")] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state.md");
        if let Some(text) = text {
            fs::write(&path, text).unwrap();
        }
        let plan = PublicationFile::replacement(&path, "original\n", "projected\n");

        let error = plan.apply().unwrap_err();
        assert_eq!(
            error,
            CatchUpError::PublicationConflict { path: path.clone() }
        );
        assert_eq!(
            error.to_string(),
            format!("publication conflict: {}", path.display())
        );
        assert!(std::error::Error::source(&error).is_none());
        assert_eq!(fs::read_to_string(&path).ok().as_deref(), text);
    }
}

#[test]
fn directories_and_symlinks_are_never_followed_as_publication_files() {
    let dir = tempfile::tempdir().unwrap();
    let original = dir.path().join("user.md");
    fs::write(&original, "user text").unwrap();
    let link = dir.path().join("link.md");
    symlink(&original, &link).unwrap();
    let directory = dir.path().join("directory.md");
    fs::create_dir(&directory).unwrap();
    for path in [link, directory] {
        assert!(matches!(
            PublicationFile::audit(&path, "event"),
            Err(CatchUpError::PublicationConflict { .. })
        ));
        assert!(matches!(
            PublicationFile::replacement(&path, "user text", "changed").apply(),
            Err(CatchUpError::PublicationConflict { .. })
        ));
    }
    assert_eq!(fs::read_to_string(&original).unwrap(), "user text");
}

#[test]
fn read_only_memory_and_state_report_their_distinct_write_errors() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("rules.md");
    fs::write(&path, "original").unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o444)).unwrap();
    let memory = PublicationFile::memory(&path, "original", "changed");
    let state = PublicationFile::replacement(&path, "original", "changed");

    let memory_error = memory.apply().unwrap_err();
    let state_error = state.apply().unwrap_err();

    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
    assert_eq!(
        memory_error,
        CatchUpError::MemoryFileWrite {
            path: path.display().to_string(),
            detail: "read-only target".to_string(),
        }
    );
    assert!(matches!(
        state_error,
        CatchUpError::StateFileWrite(StateFileWriteError::ReadOnlyTarget { .. })
    ));
    assert_eq!(fs::read_to_string(&path).unwrap(), "original");
    memory.apply().unwrap();
    assert_eq!(fs::read_to_string(&path).unwrap(), "changed");
}

#[test]
fn an_unwritable_parent_preserves_memory_and_reports_the_io_reason() {
    let dir = tempfile::tempdir().unwrap();
    let folder = dir.path().join("memory");
    fs::create_dir(&folder).unwrap();
    let path = folder.join("team.md");
    fs::write(&path, "original").unwrap();
    fs::set_permissions(&folder, fs::Permissions::from_mode(0o555)).unwrap();

    let error = PublicationFile::memory(&path, "original", "changed")
        .apply()
        .unwrap_err();

    fs::set_permissions(&folder, fs::Permissions::from_mode(0o755)).unwrap();
    assert!(
        matches!(error, CatchUpError::MemoryFileWrite { path: got, detail } if got == path.display().to_string() && !detail.is_empty() && detail != "read-only target")
    );
    assert_eq!(fs::read_to_string(&path).unwrap(), "original");
}

#[test]
fn unreadable_files_and_uncreatable_audits_return_io_errors_with_the_target() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("audit.md");
    fs::write(&path, "original").unwrap();
    let plan = PublicationFile::audit(&path, "next").unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o000)).unwrap();
    let unreadable = plan.apply().unwrap_err();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o444)).unwrap();
    let unwritable = plan.apply().unwrap_err();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
    for error in [unreadable, unwritable] {
        assert_eq!(
            error.to_string(),
            format!("publication io: PermissionDenied at {}", path.display())
        );
        assert!(std::error::Error::source(&error).is_none());
        assert_eq!(
            error,
            CatchUpError::PublicationIo {
                path: path.clone(),
                kind: ErrorKind::PermissionDenied
            }
        );
    }
    assert_eq!(fs::read_to_string(&path).unwrap(), "original");
    let folder = dir.path().join("output");
    fs::create_dir(&folder).unwrap();
    let missing = folder.join("nested/audit.md");
    let plan = PublicationFile::audit(&missing, "event").unwrap();
    fs::set_permissions(&folder, fs::Permissions::from_mode(0o555)).unwrap();
    let error = plan.apply().unwrap_err();
    fs::set_permissions(&folder, fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(
        error,
        CatchUpError::PublicationIo {
            path: missing.clone(),
            kind: ErrorKind::PermissionDenied
        }
    );
    assert!(!missing.exists());
}

#[test]
fn a_parent_symlink_loop_is_reported_without_treating_the_file_as_absent() {
    let dir = tempfile::tempdir().unwrap();
    let parent = dir.path().join("loop");
    symlink(&parent, &parent).unwrap();
    let path = parent.join("audit.md");
    assert!(matches!(PublicationFile::audit(&path, "event"),
        Err(CatchUpError::PublicationIo { path: got, .. }) if got == path));
}

#[test]
fn targets_without_lossless_path_representation_are_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let invalid = dir.path().join(std::ffi::OsString::from_vec(vec![0xff]));
    let targets = ProjectionTargets::new(
        &invalid,
        dir.path().join("audit.md"),
        dir.path().join("memory"),
    );
    let batch = PublicationBatch::new(GlobalSeqNr::ZERO, GlobalSeqNr::ZERO, vec![]);
    assert!(
        matches!(batch.for_targets(&targets), Err(CatchUpError::PublicationConflict { path }) if path == invalid)
    );
}

#[test]
fn unexpected_audit_suffix_is_preserved_as_a_conflict() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("audit.md");
    fs::write(&path, "original\n").unwrap();
    let plan = PublicationFile::audit(&path, "expected\n").unwrap();
    fs::write(&path, "original\nunrelated user text\n").unwrap();
    assert_eq!(
        plan.apply(),
        Err(CatchUpError::PublicationConflict { path: path.clone() })
    );
    assert_eq!(
        fs::read_to_string(&path).unwrap(),
        "original\nunrelated user text\n"
    );
}

#[test]
fn target_binding_keeps_roles_and_file_ownership_together() {
    let dir = tempfile::tempdir().unwrap();
    let state = dir.path().join("state.md");
    let audit = dir.path().join("audit.md");
    let memory = dir.path().join("memory");
    let targets = ProjectionTargets::new(&state, &audit, &memory);
    let plan = PublicationBatch::rebuild(GlobalSeqNr::ZERO, GlobalSeqNr::ZERO, vec![])
        .for_targets(&targets)
        .unwrap();
    assert!(plan.matches_targets(&targets));
    assert!(
        !plan.matches_targets(&ProjectionTargets::new(&audit, &state, &memory)),
        "同じパス集合でも役割の入替えを拒否する"
    );
    let foreign = PublicationBatch::rebuild(
        GlobalSeqNr::ZERO,
        GlobalSeqNr::ZERO,
        vec![PublicationFile::replacement(
            &dir.path().join("foreign.md"),
            "before",
            "after",
        )],
    )
    .for_targets(&targets)
    .unwrap();
    assert!(
        !foreign.matches_targets(&targets),
        "束縛だけが正しくても所有外のファイルを拒否する"
    );
    let invalid = dir.path().join(std::ffi::OsString::from_vec(vec![0xff]));
    assert!(
        !plan.matches_targets(&ProjectionTargets::new(invalid, &audit, &memory)),
        "損失なく記録できない対象に一致しない"
    );
}
