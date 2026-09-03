//! `ScopeChangeView` — `read_scope_change` 1 行の写し。

/// `read_scope_change` の 1 行 (主キー `execution_id` × `scope`)。
///
/// **行が返らなければ無効な scope** である (有効な scope にしか行が無い)。返れば `kind` が
/// 「state の scope と違う」か「同じ」かを言う — 比較はクエリ側に無い。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopeChangeView {
    kind: String,
}

impl ScopeChangeView {
    /// 照合結果の綴りを束ねる (**この型の唯一の構築経路**)。
    #[must_use]
    pub const fn new(kind: String) -> ScopeChangeView {
        ScopeChangeView { kind }
    }

    /// 要求 scope と state の scope の照合結果の綴り。
    #[must_use]
    pub fn kind(&self) -> &str {
        &self.kind
    }
}
