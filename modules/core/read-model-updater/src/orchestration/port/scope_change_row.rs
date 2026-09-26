//! `ScopeChangeRow` — `read_scope_change` の 1 行 (要求 scope と state の scope の照合)。

/// `read_scope_change` の 1 行。主キーは 1 列 `id` (自然キー
/// (`execution_id`, `scope`) から導いた代理キー)。`execution_id` は `read_execution.id` を
/// 指す FK である。
///
/// 読取コマンドは `--scope <名前>` の値でこの表を引く。**行が返らなければ無効な scope**で
/// あり (有効な scope にしか行が無い)、返れば `kind` が「state の scope と違うので
/// scope-change を出す」か「同じなので通常どおり進む」かを言う。
///
/// upstream は現在値を見ない config-change (depth / test_strategy / review) と違い、
/// scope だけは現在値との比較で分岐する — だから scope だけが表になる (設計 §0)。
///
/// 行は値を運ぶだけである。材料から行を組む投影は
/// [`crate::read_tables::ReadTables::project`] が持つ。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopeChangeRow {
    id: String,
    execution_id: String,
    scope: String,
    kind: String,
}

impl ScopeChangeRow {
    /// 行の値を束ねる (**この型の唯一の構築経路**)。
    #[must_use]
    pub const fn new(id: String, execution_id: String, scope: String, kind: String) -> Self {
        Self {
            id,
            execution_id,
            scope,
            kind,
        }
    }

    /// 主キー — 自然キー (`execution_id`, `scope`) から導いた代理キー。
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// `read_execution.id` を指す FK。
    #[must_use]
    pub fn execution_id(&self) -> &str {
        &self.execution_id
    }

    /// 要求されうる scope 名 (有効な scope だけが行になる)。
    #[must_use]
    pub fn scope(&self) -> &str {
        &self.scope
    }

    /// 照合の答え (`scope-change` / `same-as-state`)。
    #[must_use]
    pub fn kind(&self) -> &str {
        &self.kind
    }
}
