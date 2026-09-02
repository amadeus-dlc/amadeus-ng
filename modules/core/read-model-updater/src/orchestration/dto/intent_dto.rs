//! `Intent` とその部品の永続化 DTO — intent ジャーナル面のバイト形 (**読む側**)。
//!
//! 解決済み計画 1 要素の綴り (`StageEntryDto`) は `Started` 面と共有する — 実行の誕生記録も
//! 同じ計画の写しを運ぶからである (共有 private 型は主たる従属先に置く —
//! `coding-rules/abstract-data-type.md`)。

use core_command_domain::orchestration::{
    Created, Intent, IntentId, StageDisplay, StageEntry, StartRequest, WorkspaceScan,
};
use core_command_domain::workflow_definition::{
    DefinitionRevision, StageNumber, StageSlug, WorkflowDefinitionId,
};
use serde::{Deserialize, Serialize};

use super::dto_decode_error::DtoDecodeError;
use super::dto_vocabulary::{
    phase_of, phase_spelling, plan_action_of, plan_action_spelling, project_type_of,
    project_type_spelling,
};

/// 静的な intent の行の形。**フィールド名と並びが契約**である。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntentDto {
    id: String,
    definition_id: String,
    definition_revision: String,
    start_request: StartRequestDto,
    stages: Vec<StageEntryDto>,
    scan: WorkspaceScanDto,
    /// 鋳造の発生時刻 (書き手の `IntentDto` と同じワイヤ位置 — 側ごと専用化した写し)。
    created_at: chrono::DateTime<chrono::Utc>,
}

/// 呼出側の要求の行の形。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct StartRequestDto {
    scope: String,
    request: String,
    depth: Option<String>,
    test_strategy: Option<String>,
    review: Option<String>,
}

/// 解決済み計画 1 要素の行の形 (intent 面と `Started` 面が共有する)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct StageEntryDto {
    slug: String,
    phase: String,
    plan_action: String,
    conditional: bool,
    display: StageDisplayDto,
}

/// ステージの表示属性の行の形。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct StageDisplayDto {
    number: String,
    name: String,
    lead_agent: String,
}

/// ワークスペース走査結果の行の形。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct WorkspaceScanDto {
    project_type: String,
    languages: String,
    frameworks: String,
    build_system: String,
}

impl IntentDto {
    /// ドメインの公開アクセサだけを読んで DTO を組む (書き)。
    pub(crate) fn of(intent: &Intent) -> IntentDto {
        IntentDto {
            id: intent.id().as_str().to_string(),
            definition_id: intent.definition_id().as_str().to_string(),
            definition_revision: intent.definition_revision().as_str().to_string(),
            start_request: StartRequestDto {
                scope: intent.scope().to_string(),
                request: intent.request().to_string(),
                depth: intent.depth().map(str::to_string),
                test_strategy: intent.test_strategy().map(str::to_string),
                review: intent.review().map(str::to_string),
            },
            stages: intent.stages().iter().map(StageEntryDto::of).collect(),
            scan: WorkspaceScanDto::of(intent.scan()),
            created_at: *intent.created_at(),
        }
    }

    /// 検査付き再構成コンストラクタへ渡してドメインへ戻す (読み)。
    ///
    /// # Errors
    ///
    /// 閉集合外の綴り・文法外の識別子は `Malformed` を返す。組み上げ (誕生記録の変換) が
    /// Always Valid を破る場合は回復せずクラッシュする — 再構成は失敗を返さない
    /// (オーナー裁定 2026-08-30)。
    pub(crate) fn to_domain(&self) -> Result<Intent, DtoDecodeError> {
        let stages = self
            .stages
            .iter()
            .map(StageEntryDto::to_domain)
            .collect::<Result<Vec<StageEntry>, DtoDecodeError>>()?;
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
        if let Some(review) = &self.start_request.review {
            request = request.with_review(review.clone());
        }
        Ok(Intent::from((
            Created::new(
                IntentId::parse(&self.id)
                    .map_err(|_| DtoDecodeError::malformed("id", self.id.clone()))?,
                WorkflowDefinitionId::parse(&self.definition_id).map_err(|_| {
                    DtoDecodeError::malformed("definition_id", self.definition_id.clone())
                })?,
                DefinitionRevision::parse(&self.definition_revision).map_err(|_| {
                    DtoDecodeError::malformed(
                        "definition_revision",
                        self.definition_revision.clone(),
                    )
                })?,
                request,
                stages,
                self.scan.to_domain()?,
            ),
            self.created_at,
        )))
    }
}

impl StageEntryDto {
    /// ドメインの公開アクセサだけを読んで DTO を組む (書き)。
    pub(super) fn of(entry: &StageEntry) -> StageEntryDto {
        StageEntryDto {
            slug: entry.slug().as_str().to_string(),
            phase: phase_spelling(entry.phase()).to_string(),
            plan_action: plan_action_spelling(entry.plan_action()).to_string(),
            conditional: entry.is_conditional(),
            display: StageDisplayDto {
                number: entry.display().number().as_str().to_string(),
                name: entry.display().name().to_string(),
                lead_agent: entry.display().lead_agent().to_string(),
            },
        }
    }

    /// 検査付き再構成コンストラクタへ渡してドメインへ戻す (読み)。
    pub(super) fn to_domain(&self) -> Result<StageEntry, DtoDecodeError> {
        let number = StageNumber::parse(&self.display.number)
            .map_err(|_| DtoDecodeError::malformed("number", self.display.number.clone()))?;
        let display = StageDisplay::new(number, &self.display.name, &self.display.lead_agent)
            .map_err(|_| DtoDecodeError::malformed("display", self.display.name.clone()))?;
        Ok(StageEntry::new(
            StageSlug::parse(&self.slug)
                .map_err(|_| DtoDecodeError::malformed("slug", self.slug.clone()))?,
            phase_of(&self.phase, "phase")?,
            plan_action_of(&self.plan_action, "plan_action")?,
            self.conditional,
            display,
        ))
    }
}

impl WorkspaceScanDto {
    fn of(scan: &WorkspaceScan) -> WorkspaceScanDto {
        WorkspaceScanDto {
            project_type: project_type_spelling(scan.project_kind()).to_string(),
            languages: scan.languages().to_string(),
            frameworks: scan.frameworks().to_string(),
            build_system: scan.build_system().to_string(),
        }
    }

    fn to_domain(&self) -> Result<WorkspaceScan, DtoDecodeError> {
        WorkspaceScan::new(
            project_type_of(&self.project_type)?,
            &self.languages,
            &self.frameworks,
            &self.build_system,
        )
        .map_err(|_| DtoDecodeError::malformed("scan", self.languages.clone()))
    }
}
