//! 公開するファイルの不変な書込前後。再開時は保存済みのバイトと現物を照合する。

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::CatchUpError;

/// ファイル1本の公開計画。監査は追記、その他は原子的な置換を行う。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicationFile {
    path: PathBuf,
    before: Option<Vec<u8>>,
    after: Vec<u8>,
    append: bool,
    memory: bool,
}

impl PublicationFile {
    /// 読み取った状態・規則の変更前後から置換計画を作る。
    #[must_use]
    pub fn replacement(path: &Path, before: &str, after: &str) -> PublicationFile {
        PublicationFile {
            path: path.to_path_buf(),
            before: Some(before.as_bytes().to_vec()),
            after: after.as_bytes().to_vec(),
            append: false,
            memory: false,
        }
    }

    /// 監査の現在内容を取得し、ヘッダを含む追記後の内容を固定する。
    ///
    /// # Errors
    /// 対象を安全に読めない場合。
    pub fn audit(path: &Path, blocks: &str) -> Result<PublicationFile, CatchUpError> {
        let before = read_regular(path)?;
        let mut after = before.clone().unwrap_or_default();
        if after.is_empty() && !blocks.is_empty() {
            after.extend_from_slice(crate::workspace::SHARD_HEADER.as_bytes());
        }
        after.extend_from_slice(blocks.as_bytes());
        Ok(PublicationFile {
            path: path.to_path_buf(),
            before,
            after,
            append: true,
            memory: false,
        })
    }

    /// 利用者が編集する規則ファイルの置換計画。既存の失敗文言を保持する。
    #[must_use]
    pub fn memory(path: &Path, before: &str, after: &str) -> PublicationFile {
        let mut file = Self::replacement(path, before, after);
        file.memory = true;
        file
    }

    /// 計画が所有する書込先。
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub(super) fn before(&self) -> Option<&[u8]> {
        self.before.as_deref()
    }
    pub(super) fn after(&self) -> &[u8] {
        &self.after
    }
    pub(super) const fn is_append(&self) -> bool {
        self.append
    }
    pub(super) const fn is_memory(&self) -> bool {
        self.memory
    }

    /// 明示的な復元操作用。存在する本文は利用者の変更も含めて保持する。
    pub(super) fn restore_missing(&self) -> Result<Option<(PublicationFile, bool)>, CatchUpError> {
        let current = read_regular(&self.path)?;
        // memoryは利用者所有。意図して削除した原本をキャッシュから復活させない。
        if self.memory && current.is_none() {
            return Ok(None);
        }
        let missing = current.is_none();
        let after = current.clone().unwrap_or_else(|| self.after.clone());
        Ok(Some((
            Self::restored(self.path.clone(), current, after, self.append, self.memory),
            missing,
        )))
    }

    /// 確認できる前後への追加だけを保持し、既に反映された変更を二重適用しない。
    /// 対応が曖昧な本文は変更せず競合として返す。
    pub(super) fn rebase(&self) -> Result<PublicationFile, CatchUpError> {
        let current = read_regular(&self.path)?;
        if current == self.before || current.as_deref() == Some(self.after.as_slice()) {
            return Ok(self.clone());
        }
        let bytes = current
            .as_deref()
            .ok_or_else(|| CatchUpError::PublicationConflict {
                path: self.path.clone(),
            })?;
        let before = self.before.as_deref().unwrap_or_default();
        let after = if self.append && bytes.starts_with(before) && self.after.starts_with(bytes) {
            self.after.clone()
        } else if !self.after.is_empty()
            && (bytes.starts_with(&self.after) || (!self.append && bytes.ends_with(&self.after)))
        {
            bytes.to_vec()
        } else if !self.append
            && !before.is_empty()
            && let Some(suffix) = bytes.strip_prefix(before)
        {
            let mut merged = self.after.clone();
            merged.extend_from_slice(suffix);
            merged
        } else if !self.append
            && !before.is_empty()
            && let Some(prefix) = bytes.strip_suffix(before)
        {
            let mut merged = prefix.to_vec();
            merged.extend_from_slice(&self.after);
            merged
        } else {
            return Err(CatchUpError::PublicationConflict {
                path: self.path.clone(),
            });
        };
        Ok(Self::restored(
            self.path.clone(),
            current,
            after,
            self.append,
            self.memory,
        ))
    }

    pub(super) const fn restored(
        path: PathBuf,
        before: Option<Vec<u8>>,
        after: Vec<u8>,
        append: bool,
        memory: bool,
    ) -> PublicationFile {
        PublicationFile {
            path,
            before,
            after,
            append,
            memory,
        }
    }

    /// 未反映部分だけを書き、反映済みなら何もしない。
    ///
    /// # Errors
    /// 保存済みの前後いずれとも異なる内容、またはファイルI/Oの失敗。
    pub fn apply(&self) -> Result<(), CatchUpError> {
        let current = read_regular(&self.path)?;
        if current.as_deref() == Some(self.after.as_slice()) {
            return sync_output(&self.path);
        }
        if self.append {
            let bytes = current.as_deref().unwrap_or_default();
            let before = self.before.as_deref().unwrap_or_default();
            if !bytes.starts_with(before) || (current.is_none() && self.before.is_some()) {
                return Err(CatchUpError::PublicationConflict {
                    path: self.path.clone(),
                });
            }
            let Some(remainder) = self.after.strip_prefix(bytes) else {
                return Err(CatchUpError::PublicationConflict {
                    path: self.path.clone(),
                });
            };
            if remainder.is_empty() {
                return Ok(());
            }
            if let Some(parent) = self.path.parent() {
                fs::create_dir_all(parent).at_output(&self.path)?;
            }
            let mut file = core_infrastructure::append_only::open_append_only(&self.path)
                .at_output(&self.path)?;
            core_infrastructure::append_only::append_all(&mut file, remainder)
                .at_output(&self.path)?;
            file.sync_all().at_output(&self.path)?;
        } else {
            if current != self.before {
                return Err(CatchUpError::PublicationConflict {
                    path: self.path.clone(),
                });
            }
            let text = std::str::from_utf8(&self.after).map_err(|_| {
                CatchUpError::PublicationConflict {
                    path: self.path.clone(),
                }
            })?;
            crate::workspace::write_state_file(&self.path, text).map_err(|error| {
                if self.memory {
                    CatchUpError::MemoryFileWrite {
                        path: self.path.display().to_string(),
                        detail: match error {
                            crate::workspace::StateFileWriteError::ReadOnlyTarget { .. } => {
                                "read-only target".to_string()
                            }
                            crate::workspace::StateFileWriteError::Io { message } => message,
                        },
                    }
                } else {
                    CatchUpError::StateFileWrite(error)
                }
            })?;
        }
        sync_output(&self.path)
    }
}

/// 公開先のI/O結果を、操作対象のパスとOSの分類を保持する失敗契約へ写す。
trait PublicationIoResultExt<T> {
    fn at_output(self, path: &Path) -> Result<T, CatchUpError>;
}

impl<T> PublicationIoResultExt<T> for io::Result<T> {
    fn at_output(self, path: &Path) -> Result<T, CatchUpError> {
        self.map_err(|error| CatchUpError::PublicationIo {
            path: path.to_path_buf(),
            kind: error.kind(),
        })
    }
}

fn sync_output(path: &Path) -> Result<(), CatchUpError> {
    fs::File::open(path)
        .and_then(|file| file.sync_all())
        .at_output(path)?;
    if let Some(parent) = path.parent() {
        fs::File::open(parent)
            .and_then(|directory| directory.sync_all())
            .at_output(parent)?;
    }
    Ok(())
}

fn read_regular(path: &Path) -> Result<Option<Vec<u8>>, CatchUpError> {
    match fs::symlink_metadata(path) {
        Ok(meta) if !meta.file_type().is_file() => Err(CatchUpError::PublicationConflict {
            path: path.to_path_buf(),
        }),
        Ok(_) => fs::read(path).map(Some).at_output(path),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error).at_output(path),
    }
}
