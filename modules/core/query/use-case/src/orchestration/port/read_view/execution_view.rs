//! `ExecutionView` — `read_execution` 1 行の写し (実行 1 本の現在地)。

/// `read_execution` の 1 行 (主キーは実行の識別子)。
///
/// state の**有無**そのものは引当の `Option` が言う — 行が無ければ「まだ実行が無い」で
/// あり、この型は「在ったときの中身」だけを持つ。
///
/// 定義の識別子はここに無い。それは intent の持ち物なので、要るときは
/// [`ExecutionView::intent_id`] の FK をたどる (オーナー裁定 2026-09-03 — 関連行は FK 列で
/// 指す)。`scope` だけは RMU が intent から非正規化して載せた**答え**なのでこの行にある。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionView {
    execution_id: String,
    intent_id: String,
    scope: String,
    status: String,
    cursor_slug: Option<String>,
    parked_at_slug: Option<String>,
    parked_active: bool,
    state_binding: String,
}

impl ExecutionView {
    /// 8 列をそのまま束ねる (**この型の唯一の構築経路**)。
    #[expect(
        clippy::too_many_arguments,
        reason = "行の写しの完全コンストラクタ — 全列が必須なので引数がそのまま列になる"
    )]
    #[must_use]
    pub const fn new(
        execution_id: String,
        intent_id: String,
        scope: String,
        status: String,
        cursor_slug: Option<String>,
        parked_at_slug: Option<String>,
        parked_active: bool,
        state_binding: String,
    ) -> ExecutionView {
        ExecutionView {
            execution_id,
            intent_id,
            scope,
            status,
            cursor_slug,
            parked_at_slug,
            parked_active,
            state_binding,
        }
    }

    /// 実行の識別子 (主キー)。
    #[must_use]
    pub fn execution_id(&self) -> &str {
        &self.execution_id
    }

    /// この実行が進める intent を指す FK。
    #[must_use]
    pub fn intent_id(&self) -> &str {
        &self.intent_id
    }

    /// 実行が進んでいるスコープ名。
    #[must_use]
    pub fn scope(&self) -> &str {
        &self.scope
    }

    /// 実行の状態の綴り (`running` / `completed`)。
    #[must_use]
    pub fn status(&self) -> &str {
        &self.status
    }

    /// カーソルが指すステージの slug。
    #[must_use]
    pub fn cursor_slug(&self) -> Option<&str> {
        self.cursor_slug.as_deref()
    }

    /// park した位置の slug。
    #[must_use]
    pub fn parked_at_slug(&self) -> Option<&str> {
        self.parked_at_slug.as_deref()
    }

    /// いま park 中か。
    #[must_use]
    pub const fn parked_active(&self) -> bool {
        self.parked_active
    }

    /// 状態の束縛ダイジェスト (token に封じる値)。
    #[must_use]
    pub fn state_binding(&self) -> &str {
        &self.state_binding
    }
}
