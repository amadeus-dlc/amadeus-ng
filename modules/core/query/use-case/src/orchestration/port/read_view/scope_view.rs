//! `ScopeView` — `read_definition_scope` 1 行の写し (scope カタログ 1 列)。

/// `read_definition_scope` の 1 行 (主キー `definition_id` × `scope`)。
///
/// **行が返ること自体が「その scope は有効」の答え**である — 有効性の判定はクエリ側に
/// 無い (投影が有効な scope にしか行を作らない)。
///
/// `cost_*` はグリッド列を持たない scope では `None` になる。upstream はその場合コストの
/// 括弧ごと落とすが、その描き分けはプレゼンタの仕事である。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopeView {
    scope: String,
    depth: Option<String>,
    keywords: String,
    skeleton: Option<String>,
    review_cap: Option<String>,
    freeform_default: bool,
    has_grid_column: bool,
    cost_total: Option<u32>,
    cost_execute: Option<u32>,
    cost_gates: Option<u32>,
    cost_per_unit_stages: Option<u32>,
}

impl ScopeView {
    /// 11 列をそのまま束ねる (**この型の唯一の構築経路**)。
    #[expect(
        clippy::too_many_arguments,
        reason = "行の写しの完全コンストラクタ — 全列が必須なので引数がそのまま列になる"
    )]
    #[must_use]
    pub const fn new(
        scope: String,
        depth: Option<String>,
        keywords: String,
        skeleton: Option<String>,
        review_cap: Option<String>,
        freeform_default: bool,
        has_grid_column: bool,
        cost_total: Option<u32>,
        cost_execute: Option<u32>,
        cost_gates: Option<u32>,
        cost_per_unit_stages: Option<u32>,
    ) -> ScopeView {
        ScopeView {
            scope,
            depth,
            keywords,
            skeleton,
            review_cap,
            freeform_default,
            has_grid_column,
            cost_total,
            cost_execute,
            cost_gates,
            cost_per_unit_stages,
        }
    }

    /// スコープ名。
    #[must_use]
    pub fn scope(&self) -> &str {
        &self.scope
    }

    /// scope ファイルが宣言する深さ。
    #[must_use]
    pub fn depth(&self) -> Option<&str> {
        self.depth.as_deref()
    }

    /// キーワードの 1 行 JSON 配列。
    #[must_use]
    pub fn keywords(&self) -> &str {
        &self.keywords
    }

    /// walking skeleton の既定 (`on` / `off`)。
    #[must_use]
    pub fn skeleton(&self) -> Option<&str> {
        self.skeleton.as_deref()
    }

    /// レビューの上限クラス。
    #[must_use]
    pub fn review_cap(&self) -> Option<&str> {
        self.review_cap.as_deref()
    }

    /// 自由記述をこの scope の既定として受けるか。
    #[must_use]
    pub const fn freeform_default(&self) -> bool {
        self.freeform_default
    }

    /// スコープグリッドに列を持つか。
    #[must_use]
    pub const fn has_grid_column(&self) -> bool {
        self.has_grid_column
    }

    /// グリッド列に載るステージ数 (EXECUTE + SKIP)。
    #[must_use]
    pub const fn cost_total(&self) -> Option<u32> {
        self.cost_total
    }

    /// EXECUTE 数。
    #[must_use]
    pub const fn cost_execute(&self) -> Option<u32> {
        self.cost_execute
    }

    /// ゲート付きステージ数。
    #[must_use]
    pub const fn cost_gates(&self) -> Option<u32> {
        self.cost_gates
    }

    /// ユニットごとに回るステージ数。
    #[must_use]
    pub const fn cost_per_unit_stages(&self) -> Option<u32> {
        self.cost_per_unit_stages
    }
}
