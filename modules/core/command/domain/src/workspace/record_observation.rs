//! D4 / D5 の観測 — 作業記録がある場合だけ取れる事実。

use super::{
    ExecutionCursorObservation, ObservationFailure, ProjectionObservation, RecordLocation,
    StateFileObservation, StoreObservation,
};

/// 置き場と選択・状態・カーソル・登録簿・ストア・投影。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordObservation {
    location: RecordLocation,
    state: StateFileObservation,
    cursor: Result<Option<ExecutionCursorObservation>, ObservationFailure>,
    registry_directory: Result<Option<String>, ObservationFailure>,
    store: StoreObservation,
    projection: Result<ProjectionObservation, ObservationFailure>,
}

impl RecordObservation {
    /// 観測を束ねる。
    #[must_use]
    pub const fn new(
        location: RecordLocation,
        state: StateFileObservation,
        cursor: Result<Option<ExecutionCursorObservation>, ObservationFailure>,
        registry_directory: Result<Option<String>, ObservationFailure>,
        store: StoreObservation,
        projection: Result<ProjectionObservation, ObservationFailure>,
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
    pub const fn location(&self) -> &RecordLocation {
        &self.location
    }

    /// 状態ファイルの観測。
    #[must_use]
    pub const fn state(&self) -> &StateFileObservation {
        &self.state
    }

    /// 実行カーソル (無ければ `Ok(None)`、読めなければ原因)。
    pub const fn cursor(&self) -> &Result<Option<ExecutionCursorObservation>, ObservationFailure> {
        &self.cursor
    }

    /// 登録簿がカーソルの intent に対応付ける記録ディレクトリ名。
    pub const fn registry_directory(&self) -> &Result<Option<String>, ObservationFailure> {
        &self.registry_directory
    }

    /// ストアの観測。
    #[must_use]
    pub const fn store(&self) -> &StoreObservation {
        &self.store
    }

    /// 投影の観測 (ストアを開けなければ原因)。
    pub const fn projection(&self) -> &Result<ProjectionObservation, ObservationFailure> {
        &self.projection
    }
}
