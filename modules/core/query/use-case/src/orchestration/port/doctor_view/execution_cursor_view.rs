//! `<record>/.aidlc-execution` の写し。

/// 記録が指す実行と、その実行が属する intent。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionCursorView {
    execution_id: String,
    intent_id: String,
}

impl ExecutionCursorView {
    /// 観測を束ねる。
    #[must_use]
    pub const fn new(execution_id: String, intent_id: String) -> Self {
        Self {
            execution_id,
            intent_id,
        }
    }

    /// 実行の識別子。
    #[must_use]
    pub fn execution_id(&self) -> &str {
        &self.execution_id
    }

    /// intent の識別子。
    #[must_use]
    pub fn intent_id(&self) -> &str {
        &self.intent_id
    }
}
