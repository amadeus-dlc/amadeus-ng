//! `ExecutionRow` — `read_execution` の 1 行 (実行 1 本の現在状態)。

/// `read_execution` の 1 行。主キーは 1 列 `id` = 実行の識別子 (集約そのものの表なので
/// 代理キーを作らない)。`intent_id` は `read_intent.id` を指す FK である。
///
/// 値はすべて再生した [`IntentExecution`] のクエリの答えの写しである。`parked_active` と
/// `accepts_commands` は集約の**導出述語**であり、読取側が `status` と `parked_at` から
/// 組み直さなくてよいように列にしてある (裁定 §10-1 の非正規化)。
///
/// 行は値を運ぶだけである。材料から行を組む投影は
/// [`crate::read_tables::ReadTables::project`] が持つ。
///
/// [`IntentExecution`]: core_command_domain::orchestration::IntentExecution
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionRow {
    id: String,
    first_substantive_run: bool,
    continuation_wait: Option<String>,
    intent_id: String,
    scope: String,
    status: String,
    cursor_index: Option<usize>,
    cursor_slug: Option<String>,
    parked_at_index: Option<usize>,
    parked_at_slug: Option<String>,
    parked_active: bool,
    accepts_commands: bool,
    autonomy: String,
    skeleton_stance: Option<String>,
    seq_nr: usize,
    last_updated_at: String,
    state_binding: String,
}

impl ExecutionRow {
    /// 行の値を束ねる (**この型の唯一の構築経路**)。
    #[must_use]
    #[allow(
        clippy::too_many_arguments,
        reason = "表の 1 行の全列を唯一の構築口へ渡す — 列と引数の対応を一覧で読めることを優先する"
    )]
    pub const fn new(
        id: String,
        first_substantive_run: bool,
        continuation_wait: Option<String>,
        intent_id: String,
        scope: String,
        status: String,
        cursor_index: Option<usize>,
        cursor_slug: Option<String>,
        parked_at_index: Option<usize>,
        parked_at_slug: Option<String>,
        parked_active: bool,
        accepts_commands: bool,
        autonomy: String,
        skeleton_stance: Option<String>,
        seq_nr: usize,
        last_updated_at: String,
        state_binding: String,
    ) -> Self {
        Self {
            id,
            first_substantive_run,
            continuation_wait,
            intent_id,
            scope,
            status,
            cursor_index,
            cursor_slug,
            parked_at_index,
            parked_at_slug,
            parked_active,
            accepts_commands,
            autonomy,
            skeleton_stance,
            seq_nr,
            last_updated_at,
            state_binding,
        }
    }

    /// 集約が判定した人間待ち。
    #[must_use]
    pub fn continuation_wait(&self) -> Option<&str> {
        self.continuation_wait.as_deref()
    }

    /// 最初の実作業か（集約の判断結果）。
    #[must_use]
    pub const fn first_substantive_run(&self) -> bool {
        self.first_substantive_run
    }

    /// 主キー — 実行の識別子 (UUIDv7)。
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// この実行が対象にしている intent の識別子。
    #[must_use]
    pub fn intent_id(&self) -> &str {
        &self.intent_id
    }

    /// 選ばれたスコープ名 (intent から非正規化)。
    #[must_use]
    pub fn scope(&self) -> &str {
        &self.scope
    }

    /// ワークフロー全体の 2 値 (`running` / `completed`)。
    #[must_use]
    pub fn status(&self) -> &str {
        &self.status
    }

    /// カーソル位置 (文書順の索引)。
    #[must_use]
    pub const fn cursor_index(&self) -> Option<usize> {
        self.cursor_index
    }

    /// カーソル位置の slug。
    #[must_use]
    pub fn cursor_slug(&self) -> Option<&str> {
        self.cursor_slug.as_deref()
    }

    /// park マーカーの位置 (無ければ NULL)。
    #[must_use]
    pub const fn parked_at_index(&self) -> Option<usize> {
        self.parked_at_index
    }

    /// park マーカー位置の slug (無ければ NULL)。
    #[must_use]
    pub fn parked_at_slug(&self) -> Option<&str> {
        self.parked_at_slug.as_deref()
    }

    /// park 分岐が発火する状態か (マーカー有 ∧ 位置一致)。
    #[must_use]
    pub const fn parked_active(&self) -> bool {
        self.parked_active
    }

    /// 状態遷移コマンドを受理する状態か。
    #[must_use]
    pub const fn accepts_commands(&self) -> bool {
        self.accepts_commands
    }

    /// 自律モードの綴り (`autonomous` / `gated`)。
    #[must_use]
    pub fn autonomy(&self) -> &str {
        &self.autonomy
    }

    /// 記録済みの walking-skeleton stance (未記録は NULL)。
    ///
    /// 綴りはドメインの [`SkeletonStance::as_str`] — 状態ファイルの `Skeleton Stance` 欄と
    /// **同じ面**の値なので、綴りもそちらに揃える (`on` / `off` / `scope-dependent`)。
    ///
    /// [`SkeletonStance::as_str`]: core_command_domain::orchestration::SkeletonStance::as_str
    #[must_use]
    pub fn skeleton_stance(&self) -> Option<&str> {
        self.skeleton_stance.as_deref()
    }

    /// 集約内の通番 (歴史がどこまで進んだか)。
    #[must_use]
    pub const fn seq_nr(&self) -> usize {
        self.seq_nr
    }

    /// 最終更新時刻 (RFC3339 / 秒精度 / `Z`)。
    #[must_use]
    pub fn last_updated_at(&self) -> &str {
        &self.last_updated_at
    }

    /// 実行状態の束縛ダイジェスト (`h`)。
    #[must_use]
    pub fn state_binding(&self) -> &str {
        &self.state_binding
    }
}
