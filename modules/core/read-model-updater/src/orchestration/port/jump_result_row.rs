//! `JumpResultRow` — 移動イベントとその前後から組んだ移動結果の 1 行。

/// 呼出側のイベント識別子で取得する過去の移動結果。
///
/// 行は値を運ぶだけである。材料から行を組む投影は
/// [`crate::read_tables::ReadTables::project`] が持つ。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JumpResultRow {
    id: String,
    payload: String,
}

impl JumpResultRow {
    /// 行の値を束ねる (**この型の唯一の構築経路**)。
    #[must_use]
    pub const fn new(id: String, payload: String) -> Self {
        Self { id, payload }
    }

    /// SQLの主キー。
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// 公開言語で投影した結果。Queryは現在状態から再計算しない。
    #[must_use]
    pub fn payload(&self) -> &str {
        &self.payload
    }
}
