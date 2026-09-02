//! `LineageMismatch` — 配布束と定義の系譜が食い違う (`WorkflowDefinition::define` / `redefine` のガード)。

use std::fmt;

use super::compiled_definition_id::CompiledDefinitionId;
use super::workflow_definition_id::WorkflowDefinitionId;

/// 定義に渡された配布束が、その定義の系譜のものではない。
///
/// 集約は他集約を `&` 参照で受けるとき取り違えをガードする
/// (`coding-rules/aggregate-references.md`)。系譜の同一性は識別子の名前の一致
/// (`CompiledDefinitionId == WorkflowDefinitionId` — 同じ `harness.json` の `name`)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineageMismatch {
    definition: WorkflowDefinitionId,
    bundle: CompiledDefinitionId,
}

impl LineageMismatch {
    /// 材料をそのまま束ねる。
    #[must_use]
    pub const fn new(
        definition: WorkflowDefinitionId,
        bundle: CompiledDefinitionId,
    ) -> LineageMismatch {
        LineageMismatch { definition, bundle }
    }

    /// 受け手の定義の系譜 ID。
    #[must_use]
    pub const fn definition(&self) -> &WorkflowDefinitionId {
        &self.definition
    }

    /// 渡された配布束の識別子。
    #[must_use]
    pub const fn bundle(&self) -> &CompiledDefinitionId {
        &self.bundle
    }
}

impl fmt::Display for LineageMismatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "lineage mismatch: definition {} was handed the bundle {}",
            self.definition.as_str(),
            self.bundle.as_str()
        )
    }
}

impl std::error::Error for LineageMismatch {}
