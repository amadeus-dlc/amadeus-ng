//! D4 / D5 の観測 — 作業記録がある場合だけ取れる事実。

use super::{
    ExecutionCursorView, ObservationFailure, ProjectionObservationView, RecordLocationView,
    StateFileObservationView, StoreObservationView,
};

/// 置き場と選択・状態・カーソル・登録簿・ストア・投影。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordObservationView {
    location: RecordLocationView,
    state: StateFileObservationView,
    cursor: Result<Option<ExecutionCursorView>, ObservationFailure>,
    registry_directory: Result<Option<String>, ObservationFailure>,
    store: StoreObservationView,
    projection: Result<ProjectionObservationView, ObservationFailure>,
}

impl RecordObservationView {
    /// 観測を束ねる。
    #[must_use]
    pub const fn new(
        location: RecordLocationView,
        state: StateFileObservationView,
        cursor: Result<Option<ExecutionCursorView>, ObservationFailure>,
        registry_directory: Result<Option<String>, ObservationFailure>,
        store: StoreObservationView,
        projection: Result<ProjectionObservationView, ObservationFailure>,
    ) -> Self {
        Self {
            location,
            state,
            cursor,
            registry_directory,
            store,
            projection,
        }
    }

    /// 記録の置き場と選択。
    #[must_use]
    pub const fn location(&self) -> &RecordLocationView {
        &self.location
    }

    /// 状態ファイルの観測。
    #[must_use]
    pub const fn state(&self) -> &StateFileObservationView {
        &self.state
    }

    /// 実行カーソル (無ければ `Ok(None)`、読めなければ原因)。
    pub const fn cursor(&self) -> &Result<Option<ExecutionCursorView>, ObservationFailure> {
        &self.cursor
    }

    /// 登録簿がカーソルの intent に対応付ける記録ディレクトリ名。
    pub const fn registry_directory(&self) -> &Result<Option<String>, ObservationFailure> {
        &self.registry_directory
    }

    /// ストアの観測。
    #[must_use]
    pub const fn store(&self) -> &StoreObservationView {
        &self.store
    }

    /// 投影の観測 (ストアを開けなければ原因)。
    pub const fn projection(&self) -> &Result<ProjectionObservationView, ObservationFailure> {
        &self.projection
    }
}
