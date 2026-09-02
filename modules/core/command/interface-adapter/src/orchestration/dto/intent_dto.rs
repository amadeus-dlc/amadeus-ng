//! `Intent` とその部品の永続化 DTO — `Created` が運ぶ誕生の材料のバイト形。
//!
//! この 1 つの型が **2 つの面**で同じバイトを張る: (a) intent 自身のジャーナルの `Created`
//! ペイロード、(b) intent 集約のスナップショット行。どちらも運ぶ内容は「誕生の材料 = 集約の
//! 全状態」で完全に同一なので、綴りを 1 か所に束ねて面ごとの乖離を構造的に不能にする
//! (issue #50)。実行ジャーナルの `Started` は intent 全体を埋め込まない (issue #56) が、
//! 誕生状態の導出に要る**計画の写し**は運ぶ — その 1 要素の綴りは本モジュールの
//! `StageEntryDto` を共有するので、intent 面と `Started` 面で同じバイトになる
//! (共有 private 型は主たる従属先に置く — `coding-rules/abstract-data-type.md`)。

use core_command_domain::orchestration::{
    Created, Intent, IntentEventId, IntentId, StageDisplay, StageEntry, StartRequest, WorkspaceScan,
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
    /// 鋳造の発生時刻 (集約 `Intent::created_at` の写し — 封筒とスナップショットの両方が
    /// 同じ値の出所を持つ)。
    created_at: chrono::DateTime<chrono::Utc>,
}

/// 呼出側の要求の行の形 (intent 面と `Created` 面が共有する)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct StartRequestDto {
    pub(super) scope: String,
    pub(super) request: String,
    pub(super) depth: Option<String>,
    pub(super) test_strategy: Option<String>,
    pub(super) review: Option<String>,
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

/// ワークスペース走査結果の行の形 (intent 面と `Created` 面が共有する)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct WorkspaceScanDto {
    pub(super) project_type: String,
    pub(super) languages: String,
    pub(super) frameworks: String,
    pub(super) build_system: String,
}

impl IntentDto {
    /// ドメインの公開アクセサだけを読んで DTO を組む (書き)。
    #[must_use]
    pub fn of(intent: &Intent) -> IntentDto {
        IntentDto {
            id: intent.id().as_str().to_string(),
            definition_id: intent.definition_id().as_str().to_string(),
            definition_revision: intent.definition_revision().as_str().to_string(),
            start_request: StartRequestDto::of(intent),
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
    pub fn to_domain(&self) -> Result<Intent, DtoDecodeError> {
        Ok(Intent::from((self.to_created()?, self.created_at)))
    }

    /// スナップショット行を集約へ写すための誕生記録を組む (読み — スナップショット面)。
    ///
    /// `Intent` の唯一の再構成経路が `From<(Created, occurred_at)>` なので、スナップショット
    /// からの復元もここを通す。**イベント識別子はここで捨てられる**値である — スナップ
    /// ショット行はイベントではないので同一性を持たず、[`Intent`] も `Created` の `id` を
    /// 保持しない。ジャーナル面の復号は [`CreatedDto`](super::CreatedDto) が行い、そちらは
    /// 行に書かれた本物の識別子を運ぶ。
    fn to_created(&self) -> Result<Created, DtoDecodeError> {
        let stages = self
            .stages
            .iter()
            .map(StageEntryDto::to_domain)
            .collect::<Result<Vec<StageEntry>, DtoDecodeError>>()?;
        let request = self.start_request.to_domain();
        Ok(Created::new(
            IntentEventId::generate(),
            IntentId::parse(&self.id)
                .map_err(|_| DtoDecodeError::malformed("id", self.id.clone()))?,
            WorkflowDefinitionId::parse(&self.definition_id).map_err(|_| {
                DtoDecodeError::malformed("definition_id", self.definition_id.clone())
            })?,
            DefinitionRevision::parse(&self.definition_revision).map_err(|_| {
                DtoDecodeError::malformed("definition_revision", self.definition_revision.clone())
            })?,
            request,
            stages,
            self.scan.to_domain()?,
        ))
    }
}

impl StartRequestDto {
    /// ドメインの公開アクセサだけを読んで DTO を組む (書き)。
    pub(super) fn of(intent: &Intent) -> StartRequestDto {
        StartRequestDto {
            scope: intent.scope().to_string(),
            request: intent.request().to_string(),
            depth: intent.depth().map(str::to_string),
            test_strategy: intent.test_strategy().map(str::to_string),
            review: intent.review().map(str::to_string),
        }
    }

    /// ドメインへ戻す (読み — 任意項目はビルダーの `with_*` で足す)。
    pub(super) fn to_domain(&self) -> StartRequest {
        let mut request = StartRequest::new(self.scope.clone(), self.request.clone());
        if let Some(depth) = &self.depth {
            request = request.with_depth(depth.clone());
        }
        if let Some(strategy) = &self.test_strategy {
            request = request.with_test_strategy(strategy.clone());
        }
        if let Some(review) = &self.review {
            request = request.with_review(review.clone());
        }
        request
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
    ///
    /// # Errors
    ///
    /// 閉集合外の綴り・文法外の識別子・単一行でない表示属性は `Malformed` を返す。
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
    pub(super) fn of(scan: &WorkspaceScan) -> WorkspaceScanDto {
        WorkspaceScanDto {
            project_type: project_type_spelling(scan.project_kind()).to_string(),
            languages: scan.languages().to_string(),
            frameworks: scan.frameworks().to_string(),
            build_system: scan.build_system().to_string(),
        }
    }

    pub(super) fn to_domain(&self) -> Result<WorkspaceScan, DtoDecodeError> {
        WorkspaceScan::new(
            project_type_of(&self.project_type)?,
            &self.languages,
            &self.frameworks,
            &self.build_system,
        )
        .map_err(|_| DtoDecodeError::malformed("scan", self.languages.clone()))
    }
}
