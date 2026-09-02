//! `ScopeCost` — スコープ 1 列の費用 (upstream `gridCostSummary` `aidlc-lib.ts:9836-9854`)。
//!
//! compose 提案と `intent-create` 命令の費用節 (「n of m stages, k approval gates」) の材料。
//! 判断は定義集約が持つ ([`WorkflowDefinition::scope_cost`](super::WorkflowDefinition::scope_cost))
//! — RMU が `read_definition_scope.cost` へ投影し、クエリ側はそれを読んで描くだけである。

/// スコープ 1 列の費用の内訳。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScopeCost {
    total: usize,
    execute: usize,
    gates: usize,
    per_unit_stages: usize,
}

impl ScopeCost {
    /// 内訳をそのまま束ねる。
    #[must_use]
    pub const fn new(
        total: usize,
        execute: usize,
        gates: usize,
        per_unit_stages: usize,
    ) -> ScopeCost {
        ScopeCost {
            total,
            execute,
            gates,
            per_unit_stages,
        }
    }

    /// グリッド列に載るステージ数 (EXECUTE + SKIP)。
    #[must_use]
    pub const fn total(&self) -> usize {
        self.total
    }

    /// EXECUTE のステージ数。
    #[must_use]
    pub const fn execute(&self) -> usize {
        self.execute
    }

    /// 承認ゲート数 (EXECUTE かつ `phase != initialization`)。
    #[must_use]
    pub const fn gates(&self) -> usize {
        self.gates
    }

    /// unit ごとに反復するステージ数 (EXECUTE かつ per-unit)。
    #[must_use]
    pub const fn per_unit_stages(&self) -> usize {
        self.per_unit_stages
    }
}
