//! `Intent` とその部品の永続化 DTO — `Started` が運ぶ静的材料のバイト形 (**読む側**)。

use core_command_domain::orchestration::{
    Intent, IntentId, StageDisplay, StageEntry, StartRequest, WorkspaceScan,
};
use core_command_domain::workflow_definition::{
    DefinitionRevision, StageNumber, StageSlug, WorkflowDefinitionId,
};
use serde::{Deserialize, Serialize};

use super::wire_error::WireDecodeError;
use super::wire_vocabulary::{
    phase_of, phase_spelling, plan_action_of, plan_action_spelling, project_type_of,
    project_type_spelling,
};

/// 静的な intent の行の形。**フィールド名と並びが契約**である。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct WireIntent {
    id: String,
    definition_id: String,
    definition_revision: String,
    start_request: WireStartRequest,
    stages: Vec<WireStageEntry>,
    scan: WireWorkspaceScan,
}

/// 呼出側の要求の行の形。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct WireStartRequest {
    scope: String,
    request: String,
    depth: Option<String>,
    test_strategy: Option<String>,
}

/// 解決済み計画 1 要素の行の形。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct WireStageEntry {
    slug: String,
    phase: String,
    plan_action: String,
    conditional: bool,
    display: WireStageDisplay,
}

/// ステージの表示属性の行の形。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct WireStageDisplay {
    number: String,
    name: String,
    lead_agent: String,
}

/// ワークスペース走査結果の行の形。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct WireWorkspaceScan {
    project_type: String,
    languages: String,
    frameworks: String,
    build_system: String,
}

impl WireIntent {
    /// ドメインの公開アクセサだけを読んで DTO を組む (書き)。
    pub(crate) fn of(intent: &Intent) -> WireIntent {
        WireIntent {
            id: intent.id().as_str().to_string(),
            definition_id: intent.definition_id().as_str().to_string(),
            definition_revision: intent.definition_revision().as_str().to_string(),
            start_request: WireStartRequest {
                scope: intent.scope().to_string(),
                request: intent.request().to_string(),
                depth: intent.depth().map(str::to_string),
                test_strategy: intent.test_strategy().map(str::to_string),
            },
            stages: intent.stages().iter().map(WireStageEntry::of).collect(),
            scan: WireWorkspaceScan::of(intent.scan()),
        }
    }

    /// 検査付き再構成コンストラクタへ渡してドメインへ戻す (読み)。
    ///
    /// # Errors
    ///
    /// 閉集合外の綴り・文法外の識別子は `Malformed`、組み上げが Always Valid を破る場合は
    /// `InvariantViolation` を返す。
    pub(crate) fn to_domain(&self) -> Result<Intent, WireDecodeError> {
        let stages = self
            .stages
            .iter()
            .map(WireStageEntry::to_domain)
            .collect::<Result<Vec<StageEntry>, WireDecodeError>>()?;
        let mut request = StartRequest::new(
            self.start_request.scope.clone(),
            self.start_request.request.clone(),
        );
        if let Some(depth) = &self.start_request.depth {
            request = request.with_depth(depth.clone());
        }
        if let Some(strategy) = &self.start_request.test_strategy {
            request = request.with_test_strategy(strategy.clone());
        }
        Intent::from_material(
            IntentId::parse(&self.id)
                .map_err(|_| WireDecodeError::malformed("id", self.id.clone()))?,
            WorkflowDefinitionId::parse(&self.definition_id).map_err(|_| {
                WireDecodeError::malformed("definition_id", self.definition_id.clone())
            })?,
            DefinitionRevision::parse(&self.definition_revision).map_err(|_| {
                WireDecodeError::malformed("definition_revision", self.definition_revision.clone())
            })?,
            request,
            stages,
            self.scan.to_domain()?,
        )
        .map_err(|_| WireDecodeError::InvariantViolation)
    }
}

impl WireStageEntry {
    fn of(entry: &StageEntry) -> WireStageEntry {
        WireStageEntry {
            slug: entry.slug().as_str().to_string(),
            phase: phase_spelling(entry.phase()).to_string(),
            plan_action: plan_action_spelling(entry.plan_action()).to_string(),
            conditional: entry.is_conditional(),
            display: WireStageDisplay {
                number: entry.display().number().as_str().to_string(),
                name: entry.display().name().to_string(),
                lead_agent: entry.display().lead_agent().to_string(),
            },
        }
    }

    fn to_domain(&self) -> Result<StageEntry, WireDecodeError> {
        let number = StageNumber::parse(&self.display.number)
            .map_err(|_| WireDecodeError::malformed("number", self.display.number.clone()))?;
        let display = StageDisplay::new(number, &self.display.name, &self.display.lead_agent)
            .map_err(|_| WireDecodeError::malformed("display", self.display.name.clone()))?;
        Ok(StageEntry::new(
            StageSlug::parse(&self.slug)
                .map_err(|_| WireDecodeError::malformed("slug", self.slug.clone()))?,
            phase_of(&self.phase, "phase")?,
            plan_action_of(&self.plan_action, "plan_action")?,
            self.conditional,
            display,
        ))
    }
}

impl WireWorkspaceScan {
    fn of(scan: &WorkspaceScan) -> WireWorkspaceScan {
        WireWorkspaceScan {
            project_type: project_type_spelling(scan.project_kind()).to_string(),
            languages: scan.languages().to_string(),
            frameworks: scan.frameworks().to_string(),
            build_system: scan.build_system().to_string(),
        }
    }

    fn to_domain(&self) -> Result<WorkspaceScan, WireDecodeError> {
        WorkspaceScan::new(
            project_type_of(&self.project_type)?,
            &self.languages,
            &self.frameworks,
            &self.build_system,
        )
        .map_err(|_| WireDecodeError::malformed("scan", self.languages.clone()))
    }
}
