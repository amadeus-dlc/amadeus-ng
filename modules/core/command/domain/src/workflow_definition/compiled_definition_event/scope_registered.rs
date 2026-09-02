//! `ScopeRegistered` — スコープが配布束に登記されたイベントのペイロード。

use std::collections::BTreeMap;

use crate::workflow_definition::compiled_definition_id::CompiledDefinitionId;
use crate::workflow_definition::plan_action::PlanAction;
use crate::workflow_definition::scope_metadata::ScopeMetadata;
use crate::workflow_definition::stage_slug::StageSlug;

/// 新しいスコープが登記された (identity + グリッド 1 列)、という事実の材料。
///
/// コンポーザ承認時の scope 登記 (`scopes/aidlc-<name>.md` の新設 + `scope-grid.json` への
/// 列追加) に対応する。列は空でもよい (zero-EXECUTE スコープ — 12 §4 #6)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopeRegistered {
    id: CompiledDefinitionId,
    metadata: ScopeMetadata,
    column: BTreeMap<StageSlug, PlanAction>,
}

impl ScopeRegistered {
    /// 材料をそのまま束ねる。
    #[must_use]
    pub const fn new(
        id: CompiledDefinitionId,
        metadata: ScopeMetadata,
        column: BTreeMap<StageSlug, PlanAction>,
    ) -> ScopeRegistered {
        ScopeRegistered {
            id,
            metadata,
            column,
        }
    }

    /// 配布束の識別子。
    #[must_use]
    pub const fn id(&self) -> &CompiledDefinitionId {
        &self.id
    }

    /// 登記されたスコープの identity (frontmatter)。
    #[must_use]
    pub const fn metadata(&self) -> &ScopeMetadata {
        &self.metadata
    }

    /// 登記されたグリッド列 (slug → EXECUTE / SKIP)。
    #[must_use]
    pub const fn column(&self) -> &BTreeMap<StageSlug, PlanAction> {
        &self.column
    }
}
