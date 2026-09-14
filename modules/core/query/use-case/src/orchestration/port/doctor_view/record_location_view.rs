//! 作業記録の置き場と選択 — 記録木の相対パスと、どの記録が選ばれたか。

/// パスはワークスペース根からの相対 (原因の表示に使う)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordLocationView {
    intents_relative: String,
    store_relative: String,
    records: Vec<String>,
    selected: Option<String>,
    cursor_target: Option<String>,
}

impl RecordLocationView {
    /// 観測を束ねる。`selected` は本家 `activeIntent` と同じ規則で選ばれた記録、`cursor_target`
    /// は `active-intent` が名指す実在ディレクトリ (状態ファイルを失って選ばれない記録を
    /// 名指すために持つ)。
    #[must_use]
    pub const fn new(
        intents_relative: String,
        store_relative: String,
        records: Vec<String>,
        selected: Option<String>,
        cursor_target: Option<String>,
    ) -> Self {
        Self {
            intents_relative,
            store_relative,
            records,
            selected,
            cursor_target,
        }
    }

    /// `aidlc/spaces/<space>/intents` の相対パス。
    #[must_use]
    pub fn intents_relative(&self) -> &str {
        &self.intents_relative
    }

    /// イベントストアの相対パス。
    #[must_use]
    pub fn store_relative(&self) -> &str {
        &self.store_relative
    }

    /// 記録ディレクトリ名の一覧 (名前順)。
    #[must_use]
    pub fn records(&self) -> &[String] {
        &self.records
    }

    /// 選択された記録ディレクトリ名 (一意に決まらなければ `None`)。
    #[must_use]
    pub fn selected(&self) -> Option<&str> {
        self.selected.as_deref()
    }

    /// `active-intent` が名指す実在ディレクトリ名 (無ければ `None`)。
    #[must_use]
    pub fn cursor_target(&self) -> Option<&str> {
        self.cursor_target.as_deref()
    }

    /// 観測の対象になった記録 — 選ばれた記録、さもなくばカーソルが名指す記録。
    #[must_use]
    pub fn observed(&self) -> Option<&str> {
        self.selected.as_deref().or(self.cursor_target.as_deref())
    }
}
