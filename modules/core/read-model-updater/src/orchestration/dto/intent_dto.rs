//! `IntentEvent::Created` とその部品の永続化 DTO — intent ジャーナル面のバイト形 (**読む側**)。
//!
//! 先頭 2 つは `id` (イベント自身の識別子) と `aggregate_id` (どの集約の事実か) —
//! ドメインイベントはエンティティの一種だからである (オーナー裁定 2026-09-02)。書き手
//! (コマンド側の `CreatedDto`) とバイトが一致していることは横断適合テストが固定する。
//!
//! 解決済み計画 1 要素の綴り (`StageEntryDto`) は `Started` 面と共有する — 実行の誕生記録も
//! 同じ計画の写しを運ぶからである (共有 private 型は主たる従属先に置く —
//! `coding-rules/abstract-data-type.md`)。

use core_command_domain::orchestration::{
    Created, Intent, IntentEventId, IntentId, StageDisplay, StageEntries, StageEntry, StartRequest,
    WorkspaceScan,
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

/// 誕生記録の行の形。**フィールド名と並びが契約**である。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntentDto {
    id: String,
    aggregate_id: String,
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
    pub(crate) fn of(created: &Created, occurred_at: chrono::DateTime<chrono::Utc>) -> IntentDto {
        // 誕生の材料は集約の全状態と同一なので、内容の綴りは集約の読取面から組む。
        let intent = Intent::from((created.clone(), occurred_at));
        IntentDto {
            id: created.id().as_str().to_string(),
            aggregate_id: created.aggregate_id().as_str().to_string(),
            definition_id: intent.definition_id().as_str().to_string(),
            definition_revision: intent.definition_revision().as_str().to_string(),
            start_request: StartRequestDto {
                scope: intent.scope().to_string(),
                request: intent.request().to_string(),
                depth: intent.depth().map(str::to_string),
                test_strategy: intent.test_strategy().map(str::to_string),
                review: intent.review().map(str::to_string),
            },
            stages: intent.stages().fold_left(Vec::new(), |mut rows, entry| {
                rows.push(StageEntryDto::of(entry));
                rows
            }),
            scan: WorkspaceScanDto::of(intent.scan()),
            created_at: occurred_at,
        }
    }

    /// 検査付き再構成コンストラクタへ渡してドメインへ戻す (読み)。
    ///
    /// # Errors
    ///
    /// 閉集合外の綴り・文法外の識別子は `Malformed`、計画の不変条件違反は
    /// `InvariantViolation` を返す。後者を**復号の境界で**止めるのは `Started` 面と同じ
    /// 規律である (b40 で intent 面にも揃えた) — 通すと集約の再構成まで届いてクラッシュ
    /// する (再構成は失敗を返さない — オーナー裁定 2026-08-30)。復号境界で拒めない破損
    /// (通番の飛びなど) は従来どおりクラッシュが正である。
    pub(crate) fn to_domain(&self) -> Result<Intent, DtoDecodeError> {
        let stages = self
            .stages
            .iter()
            .map(StageEntryDto::to_domain)
            .collect::<Result<Vec<StageEntry>, DtoDecodeError>>()?;
        // 計画そのものの不変条件はドメインが持つ (`StageEntries::new` の構築検査) — 判断を
        // DTO に複製せず、値を組む口を通すだけにする。
        let stages = StageEntries::new(stages).map_err(|_| DtoDecodeError::InvariantViolation)?;
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
                IntentEventId::parse(&self.id)
                    .map_err(|_| DtoDecodeError::malformed("id", self.id.clone()))?,
                IntentId::parse(&self.aggregate_id).map_err(|_| {
                    DtoDecodeError::malformed("aggregate_id", self.aggregate_id.clone())
                })?,
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
