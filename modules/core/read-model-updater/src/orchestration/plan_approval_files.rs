//! 承認用の機械ローカル投影を、所有するディレクトリ内で再生成する。
use super::JournalReadError;
use crate::read_tables::PlanApprovalFile;
use std::{fs, io, path::Path};
fn failure(path: &Path, error: &io::Error) -> JournalReadError {
    JournalReadError::Io {
        kind: error.kind(),
        path: Some(path.to_path_buf()),
    }
}
fn reject(path: &Path) -> JournalReadError {
    failure(
        path,
        &io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid approval projection target",
        ),
    )
}
pub(super) fn publish(
    root: &Path,
    files: &[PlanApprovalFile],
    directory_present: bool,
) -> Result<(), JournalReadError> {
    let sessions = root.join(".aidlc-sessions");
    let directory = sessions.join("plan-approval");
    for path in [root, sessions.as_path(), directory.as_path()] {
        match fs::symlink_metadata(path) {
            Ok(metadata) if !metadata.is_dir() || metadata.file_type().is_symlink() => {
                return Err(reject(path));
            }
            Ok(_) => (),
            Err(error) if error.kind() == io::ErrorKind::NotFound => (),
            Err(error) => return Err(failure(path, &error)),
        }
    }
    if directory.exists() {
        for entry in fs::read_dir(&directory).map_err(|error| failure(&directory, &error))? {
            let entry = entry.map_err(|error| failure(&directory, &error))?;
            let path = entry.path();
            let metadata = fs::symlink_metadata(&path).map_err(|error| failure(&path, &error))?;
            if !metadata.is_file() || metadata.file_type().is_symlink() {
                return Err(reject(&path));
            }
            if !files.iter().any(|file| entry.file_name() == file.name()) {
                fs::remove_file(&path).map_err(|error| failure(&path, &error))?;
            }
        }
    }
    if files.is_empty() && !directory_present {
        if directory.exists() {
            fs::remove_dir(&directory).map_err(|error| failure(&directory, &error))?;
        }
        return Ok(());
    }
    fs::create_dir_all(&directory).map_err(|error| failure(&directory, &error))?;
    for file in files {
        let path = directory.join(file.name());
        if !matches!(
            Path::new(file.name())
                .components()
                .collect::<Vec<_>>()
                .as_slice(),
            [std::path::Component::Normal(_)]
        ) {
            return Err(reject(&path));
        }
        match fs::read(&path) {
            Ok(current) if current == file.content().as_bytes() => continue,
            Ok(_) => (),
            Err(error) if error.kind() == io::ErrorKind::NotFound => (),
            Err(error) => return Err(failure(&path, &error)),
        }
        core_infrastructure::atomic::write_file_atomic(&path, file.content().as_bytes())
            .map_err(|error| failure(&path, &error))?;
    }
    Ok(())
}
