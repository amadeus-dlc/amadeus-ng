//! 同じ履歴断面から計算し、耐久的に保存してから適用する公開計画。

use super::{CatchUpError, GlobalSeqNr, PublicationFile};

/// ファイル公開と確定位置を結ぶ不変な計画。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicationBatch {
    request_id: String,
    generation: u64,
    rebuild: bool,
    predecessor: Option<String>,
    target_binding: Option<String>,
    transform_revision: String,
    from: GlobalSeqNr,
    to: GlobalSeqNr,
    files: Vec<PublicationFile>,
}

impl PublicationBatch {
    /// 計算済みのファイル出力を一つの履歴範囲へ束ねる。
    #[must_use]
    pub fn new(
        from: GlobalSeqNr,
        to: GlobalSeqNr,
        files: Vec<PublicationFile>,
    ) -> PublicationBatch {
        let paths = files
            .iter()
            .map(|file| file.path().to_str())
            .collect::<Option<Vec<_>>>();
        let target_binding = paths.filter(|paths| !paths.is_empty()).map(|paths| {
            let material = core_infrastructure::canon_json::JsonValue::Array(
                paths
                    .into_iter()
                    .map(|path| {
                        core_infrastructure::canon_json::JsonValue::String(path.to_string())
                    })
                    .collect(),
            );
            core_infrastructure::canon_json::hash_compact(&material).rendered()
        });
        PublicationBatch {
            request_id: uuid::Uuid::now_v7().to_string(),
            generation: 0,
            rebuild: false,
            predecessor: None,
            target_binding,
            transform_revision: Self::current_transform_revision(),
            from,
            to,
            files,
        }
    }

    /// 同位置を含む明示的な再生成要求を作る。
    #[must_use]
    pub fn rebuild(
        from: GlobalSeqNr,
        to: GlobalSeqNr,
        files: Vec<PublicationFile>,
    ) -> PublicationBatch {
        let mut batch = Self::new(from, to, files);
        batch.rebuild = true;
        batch
    }

    /// 同じ要求の再送を識別するID。
    #[must_use]
    pub fn request_id(&self) -> &str {
        &self.request_id
    }

    /// 保存時に採番された世代。未受理の候補は0。
    #[must_use]
    pub const fn generation(&self) -> u64 {
        self.generation
    }

    /// ファイルを変更しないバッチも含め、同じ所有対象へ計画を束縛する。
    ///
    /// # Errors
    /// 対象のパスを損失なく記録できない場合。
    pub fn for_targets(
        mut self,
        targets: &super::ProjectionTargets,
    ) -> Result<PublicationBatch, CatchUpError> {
        self.target_binding = Some(targets.binding()?);
        Ok(self)
    }

    /// 変換規約が現在の実装と同じか。規約を変える変更ではこの版を更新する。
    #[must_use]
    pub fn uses_current_transform(&self) -> bool {
        self.transform_revision == Self::current_transform_revision()
    }

    pub(super) fn current_transform_revision() -> String {
        format!(
            "publication-1/read-{}",
            crate::read_tables::READ_SCHEMA_VERSION
        )
    }
    pub(super) fn target_binding(&self) -> Option<&str> {
        self.target_binding.as_deref()
    }
    pub(super) fn transform_revision(&self) -> &str {
        &self.transform_revision
    }
    pub(super) fn bound(mut self, binding: Option<String>, revision: String) -> PublicationBatch {
        self.target_binding = binding;
        self.transform_revision = revision;
        self
    }

    pub(super) const fn is_rebuild(&self) -> bool {
        self.rebuild
    }
    pub(super) fn predecessor(&self) -> Option<&str> {
        self.predecessor.as_deref()
    }
    pub(super) fn replacing(mut self, previous: &str) -> PublicationBatch {
        self.predecessor = Some(previous.to_string());
        self
    }
    pub(super) fn with_files(mut self, files: Vec<PublicationFile>) -> PublicationBatch {
        self.files = files;
        self
    }
    pub(super) fn accepted(
        mut self,
        request_id: String,
        generation: u64,
        rebuild: bool,
    ) -> PublicationBatch {
        self.request_id = request_id;
        self.generation = generation;
        self.rebuild = rebuild;
        self.predecessor = None;
        self
    }

    pub(super) fn restored(
        from: GlobalSeqNr,
        to: GlobalSeqNr,
        files: Vec<PublicationFile>,
        request_id: String,
        generation: u64,
        rebuild: bool,
    ) -> PublicationBatch {
        PublicationBatch {
            from,
            to,
            files,
            request_id,
            generation,
            rebuild,
            predecessor: None,
            target_binding: None,
            transform_revision: Self::current_transform_revision(),
        }
    }

    /// 全対象の束縛と、個々の出力先の所有が呼出元に一致するかを検査する。
    #[must_use]
    pub fn matches_targets(&self, targets: &super::ProjectionTargets) -> bool {
        targets
            .binding()
            .is_ok_and(|binding| self.target_binding.as_deref() == Some(binding.as_str()))
            && self
                .files
                .iter()
                .all(|file| targets.owned_paths().contains(&file.path()))
    }

    /// 入力を読み始めた確定位置。
    #[must_use]
    pub const fn from(&self) -> GlobalSeqNr {
        self.from
    }

    /// 全出力が表す履歴の末尾。
    #[must_use]
    pub const fn to(&self) -> GlobalSeqNr {
        self.to
    }

    /// 保存対象のファイル計画。
    #[must_use]
    pub fn files(&self) -> &[PublicationFile] {
        &self.files
    }

    /// 保存済み計画を順番に適用する。排他は呼出側が保持する。
    ///
    /// # Errors
    /// いずれかのファイルの競合またはI/O失敗。
    pub fn apply(&self) -> Result<(), CatchUpError> {
        for file in &self.files {
            file.apply()?;
        }
        Ok(())
    }
}
