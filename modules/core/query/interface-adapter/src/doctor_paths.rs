//! 自己診断が読む場所 — ワークスペース根から導いた配置 (合成ルートが決めて渡す)。

use std::path::{Path, PathBuf};

/// 診断の観測先。**媒体の場所は実装詳細**であり、ポート面には現れない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoctorPaths {
    project_dir: PathBuf,
    space: String,
    record_dir: Option<PathBuf>,
    store_path: PathBuf,
}

impl DoctorPaths {
    /// ワークスペース根・space 名・選択された記録 (無ければ `None`)・ストアの実体を束ねる。
    #[must_use]
    pub const fn new(
        project_dir: PathBuf,
        space: String,
        record_dir: Option<PathBuf>,
        store_path: PathBuf,
    ) -> Self {
        Self {
            project_dir,
            space,
            record_dir,
            store_path,
        }
    }

    /// ワークスペース根。
    #[must_use]
    pub fn project_dir(&self) -> &Path {
        &self.project_dir
    }

    /// `.claude/`。
    #[must_use]
    pub fn harness_dir(&self) -> PathBuf {
        self.project_dir.join(".claude")
    }

    /// `aidlc/spaces/default/memory/` (配布シェルの基準 — active space に追従しない)。
    #[must_use]
    pub fn default_memory_dir(&self) -> PathBuf {
        self.project_dir.join("aidlc/spaces/default/memory")
    }

    /// `aidlc/spaces/<space>/intents/`。
    #[must_use]
    pub fn intents_dir(&self) -> PathBuf {
        self.project_dir
            .join("aidlc/spaces")
            .join(&self.space)
            .join("intents")
    }

    /// 選択された記録ディレクトリ。
    #[must_use]
    pub fn record_dir(&self) -> Option<&Path> {
        self.record_dir.as_deref()
    }

    /// 記録木の根 (記録が選ばれていれば記録、さもなくば intents ディレクトリ —
    /// 本家 `docsRoot`)。
    #[must_use]
    pub fn docs_root(&self) -> PathBuf {
        self.record_dir
            .clone()
            .unwrap_or_else(|| self.intents_dir())
    }

    /// イベントストアの実体。
    #[must_use]
    pub fn store_path(&self) -> &Path {
        &self.store_path
    }

    /// ワークスペース根からの相対表記 (原因の表示用)。
    #[must_use]
    pub fn relative(&self, path: &Path) -> String {
        path.strip_prefix(&self.project_dir)
            .unwrap_or(path)
            .to_string_lossy()
            .into_owned()
    }
}
