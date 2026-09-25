//! 公開するファイルの不変な書込前後。再開時は保存済みのバイトと現物を照合する。

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::ReadModelUpdateError;

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
    /// 存在しない投影ファイルを作る。既存ファイルの上書き許可にはしない。
    #[must_use]
    pub fn creation(path: &Path, after: &str) -> Self {
        Self::restored(
            path.to_path_buf(),
            None,
            after.as_bytes().to_vec(),
            false,
            false,
        )
    }

    /// 読み取った状態・規則の変更前後から置換計画を作る。
    #[must_use]
    pub fn replacement(path: &Path, before: &str, after: &str) -> PublicationFile {
        Self::restored(
            path.to_path_buf(),
            Some(before.as_bytes().to_vec()),
            after.as_bytes().to_vec(),
            false,
            false,
        )
    }

    /// 監査の現在内容を取得し、ヘッダを含む追記後の内容を固定する。
    ///
    /// # Errors
    /// 対象を安全に読めない場合。
    pub fn audit(path: &Path, blocks: &str) -> Result<PublicationFile, ReadModelUpdateError> {
        let before = read_regular(path)?;
        let mut after = before.clone().unwrap_or_default();
        if after.is_empty() && !blocks.is_empty() {
            after.extend_from_slice(crate::workspace::SHARD_HEADER.as_bytes());
        }
        after.extend_from_slice(blocks.as_bytes());
        Ok(Self::restored(
            path.to_path_buf(),
            before,
            after,
            true,
            false,
        ))
    }

    /// 利用者が編集する規則ファイルの置換計画。既存の失敗文言を保持する。
    #[must_use]
    pub fn memory(path: &Path, before: &str, after: &str) -> PublicationFile {
        Self::restored(
            path.to_path_buf(),
            Some(before.as_bytes().to_vec()),
            after.as_bytes().to_vec(),
            false,
            true,
        )
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
    pub(super) fn restore_missing(
        &self,
    ) -> Result<Option<(PublicationFile, bool)>, ReadModelUpdateError> {
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
    pub(super) fn rebase(&self) -> Result<PublicationFile, ReadModelUpdateError> {
        let current = read_regular(&self.path)?;
        if current == self.before || current.as_deref() == Some(self.after.as_slice()) {
            return Ok(self.clone());
        }
        let bytes =
            current
                .as_deref()
                .ok_or_else(|| ReadModelUpdateError::PublicationConflict {
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
            return Err(ReadModelUpdateError::PublicationConflict {
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
    pub fn apply(&self) -> Result<(), ReadModelUpdateError> {
        let current = read_regular(&self.path)?;
        if current.as_deref() == Some(self.after.as_slice()) {
            return sync_output(&self.path);
        }
        if self.append {
            let bytes = current.as_deref().unwrap_or_default();
            let before = self.before.as_deref().unwrap_or_default();
            if !bytes.starts_with(before) || (current.is_none() && self.before.is_some()) {
                return Err(ReadModelUpdateError::PublicationConflict {
                    path: self.path.clone(),
                });
            }
            let Some(remainder) = self.after.strip_prefix(bytes) else {
                return Err(ReadModelUpdateError::PublicationConflict {
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
                return Err(ReadModelUpdateError::PublicationConflict {
                    path: self.path.clone(),
                });
            }
            let text = std::str::from_utf8(&self.after).map_err(|_| {
                ReadModelUpdateError::PublicationConflict {
                    path: self.path.clone(),
                }
            })?;
            crate::workspace::write_state_file(&self.path, text).map_err(|error| {
                if self.memory {
                    ReadModelUpdateError::MemoryFileWrite {
                        path: self.path.display().to_string(),
                        detail: match error {
                            crate::workspace::StateFileWriteError::ReadOnlyTarget { .. } => {
                                "read-only target".to_string()
                            }
                            crate::workspace::StateFileWriteError::Io { message } => message,
                        },
                    }
                } else {
                    ReadModelUpdateError::StateFileWrite(error)
                }
            })?;
        }
        sync_output(&self.path)
    }
}

/// 公開先のI/O結果を、操作対象のパスとOSの分類を保持する失敗契約へ写す。
trait PublicationIoResultExt<T> {
    fn at_output(self, path: &Path) -> Result<T, ReadModelUpdateError>;
}

impl<T> PublicationIoResultExt<T> for io::Result<T> {
    fn at_output(self, path: &Path) -> Result<T, ReadModelUpdateError> {
        self.map_err(|error| ReadModelUpdateError::PublicationIo {
            path: path.to_path_buf(),
            kind: error.kind(),
        })
    }
}

fn sync_output(path: &Path) -> Result<(), ReadModelUpdateError> {
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

fn read_regular(path: &Path) -> Result<Option<Vec<u8>>, ReadModelUpdateError> {
    match fs::symlink_metadata(path) {
        Ok(meta) if !meta.file_type().is_file() => Err(ReadModelUpdateError::PublicationConflict {
            path: path.to_path_buf(),
        }),
        Ok(_) => fs::read(path).map(Some).at_output(path),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error).at_output(path),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 復号した計画の置換本文がUTF-8でなければ、元ファイルを壊さず拒否する。
    #[test]
    fn a_decoded_replacement_with_invalid_utf8_preserves_the_original() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state.md");
        fs::write(&path, "original").unwrap();
        let plan = PublicationFile::restored(
            path.clone(),
            Some(b"original".to_vec()),
            vec![0xff],
            false,
            false,
        );
        assert_eq!(
            plan.apply(),
            Err(ReadModelUpdateError::PublicationConflict { path: path.clone() })
        );
        assert_eq!(fs::read(&path).unwrap(), b"original");
    }
}
