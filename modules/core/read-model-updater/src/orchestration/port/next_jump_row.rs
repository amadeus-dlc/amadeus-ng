//! `NextJumpRow` — `read_next_jump` の 1 行 (ジャンプ先ごとの受理判定と方向)。

/// `read_next_jump` の 1 行。主キーは 1 列 `id` (自然キー
/// (`execution_id`, `target_index`) から導いた代理キー)。`execution_id` は
/// `read_execution.id` を指す FK である。
///
/// 値は集約のクエリ [`IntentExecution::jump_resolve`] の答えである。行は**全 target を
/// 網羅する** — 読取側が「跳べるか」を自分で判定しないための非正規化であり、拒否も
/// 1 つの答えとして行になる (裁定 §10-1)。
///
/// 行は値を運ぶだけである。材料から行を組む投影は
/// [`crate::read_tables::ReadTables::project`] が持つ。
///
/// [`IntentExecution::jump_resolve`]: core_command_domain::orchestration::IntentExecution::jump_resolve
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NextJumpRow {
    id: String,
    execution_id: String,
    target_index: usize,
    target_slug: String,
    outcome: String,
    refusal: Option<String>,
    resolution: Option<String>,
}

impl NextJumpRow {
    /// 行の値を束ねる (**この型の唯一の構築経路**)。
    #[must_use]
    pub const fn new(
        id: String,
        execution_id: String,
        target_index: usize,
        target_slug: String,
        outcome: String,
        refusal: Option<String>,
        resolution: Option<String>,
    ) -> Self {
        Self {
            id,
            execution_id,
            target_index,
            target_slug,
            outcome,
            refusal,
            resolution,
        }
    }

    /// resolveの公開結果。判断は投影時に確定する。
    #[must_use]
    pub fn resolution(&self) -> Option<&str> {
        self.resolution.as_deref()
    }

    /// 主キー — 自然キー (`execution_id`, `target_index`) から導いた代理キー。
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// `read_execution.id` を指す FK。
    #[must_use]
    pub fn execution_id(&self) -> &str {
        &self.execution_id
    }

    /// ジャンプ先の位置 (文書順の索引)。
    #[must_use]
    pub const fn target_index(&self) -> usize {
        self.target_index
    }

    /// ジャンプ先の slug。
    #[must_use]
    pub fn target_slug(&self) -> &str {
        &self.target_slug
    }

    /// 受理なら方向 (`forward` / `backward` / `redo`)、非受理なら `refused`。
    #[must_use]
    pub fn outcome(&self) -> &str {
        &self.outcome
    }

    /// 非受理のときだけ在る拒否理由 (受理は NULL)。
    #[must_use]
    pub fn refusal(&self) -> Option<&str> {
        self.refusal.as_deref()
    }
}
