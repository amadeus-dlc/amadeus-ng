//! **純粋投影核** — ドメインイベントの列をリードモデルへ写す（C5 の投影規則）。
//!
//! ここが知っているのはドメインイベント・解決済み計画・リードモデルの 3 つだけである。
//! `JournalReader`・SQLite 接続・チェックポイントは**署名にも本体にも現れない**
//! （`coding-rules/cqrs-boundaries.md` の禁止パターン「純粋投影核が取得の都合を知る」）。
//! 取得ループと投影核の二層を潰さないのは、投影の規則だけを単体でテストできるようにするため
//! である。
//!
//! # 計画が引数なのはなぜか
//!
//! 表示属性（ステージ番号・表題・担当エージェント）と走査結果は `Started` だけが運ぶ
//! （オーナー裁定 2026-08-29）。差分投影のバッチに `Started` が入っているとは限らないので、
//! 計画は [`ResolvedPlan`] として**渡される** — リードモデルと同じ「渡されるデータ」である。
//! 取ってくるのは取得ループの仕事であり、二層は保たれる。
//!
//! # 冪等（NFR3）
//!
//! 同じ入力からは常に同じバイトが出る — 壁時計を読まず（監査行の時刻はイベントの発生時刻）、
//! 乱数も環境変数も見ず、ワークフロー定義も引かない（引くと過去のイベントを今の定義で描くこと
//! になり、再構成が当時と一致しない）。二度描かない保証はチェックポイントが与えるので、投影核
//! 自身は「渡された列を順に写す」だけでよい。

use core_command_domain::orchestration::{
    AutonomyMode, GateApproved, GateOpened, GateRejected, JumpDirection, Jumped, Parked,
    PhaseBoundary, Recomposed, StageCompleted, StageRevised, StageSkipped, WorkflowExecutionEvent,
};
use core_command_domain::workflow_definition::{PhaseId, PlanAction, StageSlug};
use core_command_domain::workspace::{
    AuditFieldKey, AuditFieldKeyError, AuditFields, CheckboxState, CheckboxUpdateError,
    count_completed, with_checkbox_marker, with_checkbox_suffix,
};

use chrono::{DateTime, Utc};
use core_command_domain::workspace::EventType;

use super::audit_block::render_audit_block;
use super::read_model::ReadModel;
use super::resolved_plan::{PlannedStage, ResolvedPlan};
use super::state_writers::{FieldNotFound, find_field, with_field};
use crate::orchestration::JournalEntry;

// ---------------------------------------------------------------------------
// 逐語の綴り（upstream 実バイト — `tests/golden/` が正本）
// ---------------------------------------------------------------------------

/// 監査行のフィールドキー。
mod key {
    /// `**Stage**:`。
    pub(super) const STAGE: &str = "Stage";
    /// `**Agent**:`。
    pub(super) const AGENT: &str = "Agent";
    /// `**Details**:`。
    pub(super) const DETAILS: &str = "Details";
    /// `**Scope**:`。
    pub(super) const SCOPE: &str = "Scope";
    /// `**Request**:`。
    pub(super) const REQUEST: &str = "Request";
    /// `**Phase**:`。
    pub(super) const PHASE: &str = "Phase";
    /// `**Stage count**:`（genesis の `PHASE_STARTED` だけが持つ）。
    pub(super) const STAGE_COUNT: &str = "Stage count";
    /// `**Reason**:`。
    pub(super) const REASON: &str = "Reason";
    /// `**From phase**:`。
    pub(super) const FROM_PHASE: &str = "From phase";
    /// `**To phase**:`。
    pub(super) const TO_PHASE: &str = "To phase";
    /// `**Stages completed**:`。
    pub(super) const STAGES_COMPLETED: &str = "Stages completed";
    /// `**Phase boundary**:`。
    pub(super) const PHASE_BOUNDARY: &str = "Phase boundary";
    /// `**Project Type**:`。
    pub(super) const PROJECT_TYPE: &str = "Project Type";
    /// `**Languages**:`。
    pub(super) const LANGUAGES: &str = "Languages";
    /// `**Frameworks**:`。
    pub(super) const FRAMEWORKS: &str = "Frameworks";
    /// `**Build System**:`。
    pub(super) const BUILD_SYSTEM: &str = "Build System";
    /// `**User Input**:`。
    pub(super) const USER_INPUT: &str = "User Input";
    /// `**Direction**:`。
    pub(super) const DIRECTION: &str = "Direction";
    /// `**Source**:`。
    pub(super) const SOURCE: &str = "Source";
    /// `**Target**:`。
    pub(super) const TARGET: &str = "Target";
    /// `**Revision count**:`（状態ファイル側は大文字 C の `Revision Count` — upstream の非対称）。
    pub(super) const REVISION_COUNT: &str = "Revision count";
    /// `**Feedback**:`。
    pub(super) const FEEDBACK: &str = "Feedback";
    /// `**Stages skipped**:`。
    pub(super) const STAGES_SKIPPED: &str = "Stages skipped";
    /// `**Stages added**:`。
    pub(super) const STAGES_ADDED: &str = "Stages added";
    /// `**Stages in Scope**:`。
    pub(super) const STAGES_IN_SCOPE: &str = "Stages in Scope";
    /// `**Mode**:`。**upstream ゴールデン未採取**（`cli/set-autonomy` は失敗経路しか捉えて
    /// いない）。状態ファイル側の綴りは失敗文言が逐語で固定しているが、この行のキーは
    /// U1 の追加採取待ちである。
    pub(super) const MODE: &str = "Mode";
}

/// 状態ファイルの bullet ラベル。
mod field {
    /// `- **Active Agent**:`。
    pub(super) const ACTIVE_AGENT: &str = "Active Agent";
    /// `- **Completed**:`。
    pub(super) const COMPLETED: &str = "Completed";
    /// `- **In Progress**:`。
    pub(super) const IN_PROGRESS: &str = "In Progress";
    /// `- **Current Stage**:`。
    pub(super) const CURRENT_STAGE: &str = "Current Stage";
    /// `- **Next Stage**:`。
    pub(super) const NEXT_STAGE: &str = "Next Stage";
    /// `- **Last Completed Stage**:`。
    pub(super) const LAST_COMPLETED_STAGE: &str = "Last Completed Stage";
    /// `- **Next Action**:`。
    pub(super) const NEXT_ACTION: &str = "Next Action";
    /// `- **Revision Count**:`。
    pub(super) const REVISION_COUNT: &str = "Revision Count";
    /// `- **Total Stages**:`。
    pub(super) const TOTAL_STAGES: &str = "Total Stages";
    /// `- **Construction Autonomy Mode**:`。
    pub(super) const AUTONOMY_MODE: &str = "Construction Autonomy Mode";
    /// `- **Stages to Execute**:`。
    pub(super) const STAGES_TO_EXECUTE: &str = "Stages to Execute";
    /// `- **Stages to Skip**:`。
    pub(super) const STAGES_TO_SKIP: &str = "Stages to Skip";
}

/// 空のステージ集合を描く逐語（`**Stages added**: none`）。
const NONE_LITERAL: &str = "none";
/// 再入時の逐語（`report --result revised`）。
const REENTRY_DETAILS: &str = "Re-entering gate after revision";
/// フェーズ境界の区切り（U+2192）。
const BOUNDARY_ARROW: &str = " → ";
/// 一覧の区切り。
const LIST_SEPARATOR: &str = ", ";

/// 初期化 3 ステージが描く固有行の対応（upstream の出荷グラフに固定）。
const INITIALIZATION_ROWS: [(&str, EventType); 3] = [
    ("workspace-scaffold", EventType::WorkspaceScaffolded),
    ("workspace-detection", EventType::WorkspaceScanned),
    ("state-init", EventType::WorkspaceInitialised),
];

// ---------------------------------------------------------------------------
// 失敗
// ---------------------------------------------------------------------------

/// 投影の失敗（材料のみ — 文言はアダプタ層）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectionError {
    /// 状態ファイルに書き換え先のフィールド行が無い。
    ///
    /// 無言 no-op は検出不能なドリフトなので、upstream 逐語の拒否文言を添えて止める。
    StateField(FieldNotFound),
    /// 状態ファイルに対象ステージのチェックボックス行が無い、または行末トークンが無い。
    Checkbox(CheckboxUpdateError),
    /// 監査行のフィールドキーが文法外だった（材料の綴りの誤り）。
    AuditFieldKey(AuditFieldKeyError),
    /// イベントが名指したステージが解決済み計画に無い。
    ///
    /// 計画とジャーナルが同じワークフローのものでなければ起きる — 読み替えずに止める。
    UnknownStage {
        /// 計画に無かったステージ。
        stage: String,
    },
    /// 状態ファイルに park マーカーの置き場（`## Runtime State`）が無い。
    ParkSectionMissing,
    /// 状態ファイルを**ゼロから起こす**ことは未実装である。
    ///
    /// `Started` は本文が既にある状態ファイルへならフィールドを書ける（値はどれも計画から
    /// 導ける）。だが本文そのもの — 9 セクションの骨格と 31 のフィールド行 — を起こすには
    /// upstream の `state-template.md` の実バイトが要る。ゴールデンには差分（`state.diff`）
    /// しか無く、テンプレート本体は未採取である（U1 の追加採取待ち）。骨格を推測して書くと
    /// 0a 逐語契約を静かに破るので、そのときだけここで止める。
    ScaffoldTemplateUnavailable,
}

impl core::fmt::Display for ProjectionError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ProjectionError::StateField(inner) => write!(f, "state field: {}", inner.message()),
            ProjectionError::Checkbox(CheckboxUpdateError::MissingStage(slug)) => {
                write!(f, "checkbox: missing stage {slug}")
            }
            ProjectionError::Checkbox(CheckboxUpdateError::MissingSuffix(slug)) => {
                write!(f, "checkbox: missing suffix {slug}")
            }
            ProjectionError::AuditFieldKey(inner) => write!(f, "audit field key: {inner}"),
            ProjectionError::UnknownStage { stage } => write!(f, "unknown stage: {stage}"),
            ProjectionError::ParkSectionMissing => f.write_str("park section missing"),
            ProjectionError::ScaffoldTemplateUnavailable => {
                f.write_str("scaffold template unavailable")
            }
        }
    }
}

impl std::error::Error for ProjectionError {}

impl From<FieldNotFound> for ProjectionError {
    fn from(inner: FieldNotFound) -> ProjectionError {
        ProjectionError::StateField(inner)
    }
}

impl From<CheckboxUpdateError> for ProjectionError {
    fn from(inner: CheckboxUpdateError) -> ProjectionError {
        ProjectionError::Checkbox(inner)
    }
}

impl From<AuditFieldKeyError> for ProjectionError {
    fn from(inner: AuditFieldKeyError) -> ProjectionError {
        ProjectionError::AuditFieldKey(inner)
    }
}

/// 監査行のフィールドキーを組む（綴りはこのファイルの `key` モジュールが正本）。
fn key(raw: &str) -> Result<AuditFieldKey, ProjectionError> {
    AuditFieldKey::parse(raw).map_err(ProjectionError::from)
}

// ---------------------------------------------------------------------------
// 投影核
// ---------------------------------------------------------------------------

/// 純粋投影核 — 差分のジャーナル行をリードモデルへ写す。
///
/// 入口はドメインイベントと解決済み計画である。集約も Repository もストアのエラーも
/// ここには現れない。
///
/// # Errors
///
/// 状態ファイルに書き換え先が無い（`StateField` / `Checkbox`）、計画に無いステージを
/// 名指された（`UnknownStage`）、park マーカーの置き場が無い（`ParkSectionMissing`）、
/// `Started` の状態面を求められた（`ScaffoldTemplateUnavailable`）を返す。
pub fn project(
    entries: &[JournalEntry],
    plan: &ResolvedPlan,
    read_model: &mut ReadModel,
) -> Result<(), ProjectionError> {
    for entry in entries {
        project_one(entry.event(), entry.occurred_at(), plan, read_model)?;
    }
    Ok(())
}

fn project_one(
    event: &WorkflowExecutionEvent,
    at: &DateTime<Utc>,
    plan: &ResolvedPlan,
    read_model: &mut ReadModel,
) -> Result<(), ProjectionError> {
    match event {
        WorkflowExecutionEvent::Started(_) => started(at, plan, read_model),
        WorkflowExecutionEvent::StageCompleted(completed) => {
            stage_completed(completed, at, plan, read_model)
        }
        WorkflowExecutionEvent::GateOpened(opened) => gate_opened(opened, at, read_model),
        WorkflowExecutionEvent::GateApproved(approved) => {
            gate_approved(approved, at, plan, read_model)
        }
        WorkflowExecutionEvent::GateRejected(rejected) => gate_rejected(rejected, at, read_model),
        WorkflowExecutionEvent::StageRevised(revised) => stage_revised(revised, at, read_model),
        WorkflowExecutionEvent::StageSkipped(skipped) => {
            stage_skipped(skipped, at, plan, read_model)
        }
        WorkflowExecutionEvent::Jumped(jumped) => jumped_event(jumped, at, plan, read_model),
        WorkflowExecutionEvent::Parked(parked) => parked_event(parked, at, read_model),
        WorkflowExecutionEvent::Unparked => {
            unparked(at, read_model);
            Ok(())
        }
        WorkflowExecutionEvent::Recomposed(recomposed) => {
            recomposed_event(recomposed, at, plan, read_model)
        }
        WorkflowExecutionEvent::AutonomyModeSet(mode) => {
            autonomy_mode_set(mode.mode(), at, read_model)
        }
    }
}

// ---------------------------------------------------------------------------
// `Started` — 初期化 3 ステージの 16 行（`cli/intent-create/classic-scope`）
// ---------------------------------------------------------------------------

/// `Started` → 監査行 16 本と、状態ファイルの初期化。
///
/// 状態ファイルの**骨格が無ければ**（本文が空）`ScaffoldTemplateUnavailable` で止まる。
/// 骨格があるなら、初期化 3 ステージの完了・最初のゲート付きステージへの着地・総数を書く —
/// いずれも他のイベントで逐語検収済みの writer と導出をそのまま使う。
///
/// **書かないもの**: `- **Stages to Execute**: ` / `- **Stages to Skip**: ` の 2 つ。
/// ゴールデンの実バイトは `2.1 (reverse-engineering — greenfield)` のように**畳まれた理由**を
/// 括弧内に持つが、その理由は計画からは導けない（`PlanAction` は EXECUTE / SKIP の 2 値しか
/// 持たない）。推測で書かず、触らないままにする。
fn started(
    at: &DateTime<Utc>,
    plan: &ResolvedPlan,
    read_model: &mut ReadModel,
) -> Result<(), ProjectionError> {
    append_started_rows(at, plan, read_model)?;
    if read_model.state().trim().is_empty() {
        return Err(ProjectionError::ScaffoldTemplateUnavailable);
    }
    for stage in plan
        .stages()
        .iter()
        .filter(|stage| stage.is_in_scope() && stage.phase() == PhaseId::Initialization)
    {
        set_checkbox(read_model, stage.slug().as_str(), CheckboxState::Completed)?;
    }
    let completed = count_completed(read_model.state()).to_string();
    set_field(read_model, field::COMPLETED, &completed)?;
    if let Some(last) = plan
        .stages()
        .iter()
        .rfind(|stage| stage.is_in_scope() && stage.phase() == PhaseId::Initialization)
    {
        set_field(
            read_model,
            field::LAST_COMPLETED_STAGE,
            last.slug().as_str(),
        )?;
    }
    set_field(
        read_model,
        field::TOTAL_STAGES,
        &plan.in_scope_count().to_string(),
    )?;
    match first_gated_in_scope(plan).map(|stage| stage.slug().clone()) {
        Some(slug) => enter_stage_without_row(read_model, plan, &slug),
        None => Ok(()),
    }
}

/// `Started` の監査行 16 本（順序は upstream の emit 順）。
fn append_started_rows(
    at: &DateTime<Utc>,
    plan: &ResolvedPlan,
    read_model: &mut ReadModel,
) -> Result<(), ProjectionError> {
    let scope = plan.scope();

    read_model.append_audit(&render_audit_block(
        EventType::WorkflowStarted,
        at,
        &AuditFields::new()
            .with(key(key::SCOPE)?, scope)
            .with(key(key::REQUEST)?, plan.request()),
    ));

    read_model.append_audit(&render_audit_block(
        EventType::PhaseStarted,
        at,
        &AuditFields::new()
            .with(key(key::PHASE)?, PhaseId::Initialization.as_str())
            .with(
                key(key::STAGE_COUNT)?,
                &plan.in_scope_count_of(PhaseId::Initialization).to_string(),
            )
            .with(key(key::SCOPE)?, scope),
    ));

    for phase in plan.phases_out_of_scope() {
        read_model.append_audit(&render_audit_block(
            EventType::PhaseSkipped,
            at,
            &AuditFields::new()
                .with(key(key::PHASE)?, phase.as_str())
                .with(key(key::SCOPE)?, scope)
                .with(
                    key(key::REASON)?,
                    &format!("scope {scope} excludes {}", phase.as_str()),
                ),
        ));
    }

    let routing_to = first_gated_in_scope(plan);
    for stage in plan
        .stages()
        .iter()
        .filter(|stage| stage.is_in_scope() && stage.phase() == PhaseId::Initialization)
    {
        read_model.append_audit(&stage_started_row(stage, at)?);
        if let Some(row) = initialization_row(stage, at, plan, routing_to)? {
            read_model.append_audit(&row);
        }
        read_model.append_audit(&render_audit_block(
            EventType::StageCompleted,
            at,
            &AuditFields::new()
                .with(key(key::STAGE)?, stage.slug().as_str())
                .with(
                    key(key::DETAILS)?,
                    &initialization_completion_details(stage, plan, routing_to),
                ),
        ));
    }

    let phases = plan.phases_in_scope();
    if let Some(to_phase) = phases.get(1).copied() {
        append_phase_boundary(
            read_model,
            at,
            plan,
            PhaseBoundary::new(PhaseId::Initialization, to_phase),
        )?;
    }

    if let Some(stage) = routing_to {
        read_model.append_audit(&stage_started_row(stage, at)?);
    }
    Ok(())
}

/// 最初のゲート付きスコープ内ステージ（`routing to …` の材料）。
fn first_gated_in_scope(plan: &ResolvedPlan) -> Option<&PlannedStage> {
    plan.stages()
        .iter()
        .find(|stage| stage.is_in_scope() && stage.phase() != PhaseId::Initialization)
}

/// initialization ステージ固有の行（`WORKSPACE_*`）。
fn initialization_row(
    stage: &PlannedStage,
    at: &DateTime<Utc>,
    plan: &ResolvedPlan,
    routing_to: Option<&PlannedStage>,
) -> Result<Option<String>, ProjectionError> {
    let Some((_, event)) = INITIALIZATION_ROWS
        .iter()
        .find(|(slug, _)| *slug == stage.slug().as_str())
    else {
        return Ok(None);
    };
    let scan = plan.scan();
    let fields = match *event {
        EventType::WorkspaceScaffolded => AuditFields::new()
            .with(key(key::REQUEST)?, plan.request())
            .with(
                key(key::DETAILS)?,
                &format!(
                    "{} in-scope phase dirs + verification/ + space-level knowledge/ ensured \
                     (shell shipped by SEED)",
                    plan.phases_in_scope().len()
                ),
            ),
        EventType::WorkspaceScanned => AuditFields::new()
            .with(key(key::PROJECT_TYPE)?, scan.project_type())
            .with(key(key::LANGUAGES)?, scan.languages())
            .with(key(key::FRAMEWORKS)?, scan.frameworks())
            .with(key(key::BUILD_SYSTEM)?, scan.build_system())
            .with(key(key::DETAILS)?, "Deterministic rule-based scan"),
        _ => AuditFields::new()
            .with(key(key::REQUEST)?, plan.request())
            .with(key(key::PROJECT_TYPE)?, scan.project_type())
            .with(key(key::SCOPE)?, plan.scope())
            .with(key(key::LANGUAGES)?, scan.languages())
            .with(key(key::FRAMEWORKS)?, scan.frameworks())
            .with(key(key::BUILD_SYSTEM)?, scan.build_system())
            .with(
                key(key::DETAILS)?,
                &format!(
                    "{} stages in scope, routing to {}",
                    plan.in_scope_count(),
                    routing_to.map_or("-", |stage| stage.slug().as_str())
                ),
            ),
    };
    Ok(Some(render_audit_block(*event, at, &fields)))
}

/// initialization ステージの `STAGE_COMPLETED` の `**Details**:`（ステージごとに逐語が違う）。
fn initialization_completion_details(
    stage: &PlannedStage,
    plan: &ResolvedPlan,
    routing_to: Option<&PlannedStage>,
) -> String {
    let scan = plan.scan();
    let routing = routing_to.map_or("-", |stage| stage.slug().as_str());
    match stage.slug().as_str() {
        "workspace-scaffold" => format!(
            "{} in-scope phase dirs + verification/ + space-level knowledge/ ensured",
            plan.phases_in_scope().len()
        ),
        "workspace-detection" => format!(
            "Classified {}; languages={}; frameworks={}",
            scan.project_type(),
            scan.languages(),
            scan.frameworks()
        ),
        _ => format!(
            "State initialized: {} scope, {} stages, routing to {routing}",
            plan.scope(),
            plan.in_scope_count()
        ),
    }
}

// ---------------------------------------------------------------------------
// ゲートまわり
// ---------------------------------------------------------------------------

/// `GateOpened` → `STAGE_AWAITING_APPROVAL`、チェックボックス `[-]` → `[?]`。
fn gate_opened(
    opened: &GateOpened,
    at: &DateTime<Utc>,
    read_model: &mut ReadModel,
) -> Result<(), ProjectionError> {
    let fields = AuditFields::new().with(key(key::STAGE)?, opened.stage().as_str());
    read_model.append_audit(&render_audit_block(
        EventType::StageAwaitingApproval,
        at,
        &fields,
    ));
    set_checkbox(
        read_model,
        opened.stage().as_str(),
        CheckboxState::AwaitingApproval,
    )
}

/// `GateRejected` → `GATE_REJECTED` + `STAGE_REVISING`、`[?]` → `[R]`、`Revision Count`。
fn gate_rejected(
    rejected: &GateRejected,
    at: &DateTime<Utc>,
    read_model: &mut ReadModel,
) -> Result<(), ProjectionError> {
    let stage = rejected.stage().as_str();
    let feedback = rejected.feedback().unwrap_or_default();
    let revisions = rejected.revision_count().to_string();

    let mut gate = AuditFields::new().with(key(key::STAGE)?, stage);
    if rejected.feedback().is_some() {
        gate = gate.with(key(key::FEEDBACK)?, feedback);
    }
    read_model.append_audit(&render_audit_block(EventType::GateRejected, at, &gate));

    let mut revising = AuditFields::new()
        .with(key(key::STAGE)?, stage)
        .with(key(key::REVISION_COUNT)?, &revisions);
    if rejected.feedback().is_some() {
        revising = revising.with(key(key::FEEDBACK)?, feedback);
    }
    read_model.append_audit(&render_audit_block(EventType::StageRevising, at, &revising));

    set_checkbox(read_model, stage, CheckboxState::Revising)?;
    set_field(read_model, field::REVISION_COUNT, &revisions)
}

/// `StageRevised` → `STAGE_AWAITING_APPROVAL`（再入の逐語つき）、`[R]` → `[?]`。
fn stage_revised(
    revised: &StageRevised,
    at: &DateTime<Utc>,
    read_model: &mut ReadModel,
) -> Result<(), ProjectionError> {
    let fields = AuditFields::new()
        .with(key(key::STAGE)?, revised.stage().as_str())
        .with(key(key::DETAILS)?, REENTRY_DETAILS);
    read_model.append_audit(&render_audit_block(
        EventType::StageAwaitingApproval,
        at,
        &fields,
    ));
    set_checkbox(
        read_model,
        revised.stage().as_str(),
        CheckboxState::AwaitingApproval,
    )
}

/// `GateApproved` → `GATE_APPROVED` + `STAGE_COMPLETED` + (フェーズ境界) + 次ステージの開始。
fn gate_approved(
    approved: &GateApproved,
    at: &DateTime<Utc>,
    plan: &ResolvedPlan,
    read_model: &mut ReadModel,
) -> Result<(), ProjectionError> {
    let stage = approved.stage();
    let title = title_of(plan, stage)?;

    let mut gate = AuditFields::new().with(key(key::STAGE)?, stage.as_str());
    if let Some(input) = approved.user_input() {
        gate = gate.with(key(key::USER_INPUT)?, input);
    }
    read_model.append_audit(&render_audit_block(EventType::GateApproved, at, &gate));
    read_model.append_audit(&render_audit_block(
        EventType::StageCompleted,
        at,
        &AuditFields::new()
            .with(key(key::STAGE)?, stage.as_str())
            .with(
                key(key::DETAILS)?,
                &format!("Stage {title} approved by gate"),
            ),
    ));
    if let Some(boundary) = approved.phase_boundary() {
        append_phase_boundary(read_model, at, plan, boundary)?;
    }

    complete_stage(read_model, stage)?;
    leave_for(read_model, at, plan, approved.next_stage())
}

// ---------------------------------------------------------------------------
// 進行
// ---------------------------------------------------------------------------

/// `StageCompleted`（非ゲートの完了）→ `STAGE_COMPLETED` + 次ステージの開始。
///
/// **ゴールデン未採取**である。出荷グラフで非ゲートなのは initialization の 3 ステージだけで、
/// その 3 本は `Started` の投影が描く（`cli/intent-create` が実バイトを固定している）。単独の
/// `complete_stage` が打たれる経路の実バイトは採取されていないので、行の形は `GateApproved`
/// の完了部と同型に置いた（U1 の追加採取待ち）。
fn stage_completed(
    completed: &StageCompleted,
    at: &DateTime<Utc>,
    plan: &ResolvedPlan,
    read_model: &mut ReadModel,
) -> Result<(), ProjectionError> {
    let stage = completed.stage();
    let title = title_of(plan, stage)?;
    read_model.append_audit(&render_audit_block(
        EventType::StageCompleted,
        at,
        &AuditFields::new()
            .with(key(key::STAGE)?, stage.as_str())
            .with(key(key::DETAILS)?, &format!("Stage {title} completed")),
    ));
    complete_stage(read_model, stage)?;
    leave_for(read_model, at, plan, completed.next_stage())
}

/// `StageSkipped` → `STAGE_SKIPPED` + 次ステージの開始。完了数と最終完了ステージは動かさない。
fn stage_skipped(
    skipped: &StageSkipped,
    at: &DateTime<Utc>,
    plan: &ResolvedPlan,
    read_model: &mut ReadModel,
) -> Result<(), ProjectionError> {
    read_model.append_audit(&render_audit_block(
        EventType::StageSkipped,
        at,
        &AuditFields::new()
            .with(key(key::STAGE)?, skipped.stage().as_str())
            .with(key(key::REASON)?, skipped.reason()),
    ));
    set_checkbox(read_model, skipped.stage().as_str(), CheckboxState::Skipped)?;
    leave_for(read_model, at, plan, skipped.next_stage())
}

/// `Jumped` → 読み飛ばした各ステージの `STAGE_SKIPPED` + `STAGE_JUMPED` + 目標の開始。
fn jumped_event(
    jumped: &Jumped,
    at: &DateTime<Utc>,
    plan: &ResolvedPlan,
    read_model: &mut ReadModel,
) -> Result<(), ProjectionError> {
    let target = jumped.target();
    let wire = direction_wire(jumped.direction());
    let lowered = wire.to_lowercase();

    for slug in jumped.stages_skipped() {
        read_model.append_audit(&render_audit_block(
            EventType::StageSkipped,
            at,
            &AuditFields::new()
                .with(key(key::STAGE)?, slug.as_str())
                .with(
                    key(key::REASON)?,
                    &format!("Skipped by jump to {} ({lowered})", target.as_str()),
                ),
        ));
        set_checkbox(read_model, slug.as_str(), CheckboxState::Skipped)?;
    }
    for slug in jumped.stages_reset() {
        set_checkbox(read_model, slug.as_str(), CheckboxState::Pending)?;
    }

    let number = number_of(plan, target)?;
    read_model.append_audit(&render_audit_block(
        EventType::StageJumped,
        at,
        &AuditFields::new()
            .with(key(key::DIRECTION)?, wire)
            .with(key(key::SOURCE)?, jumped.source().as_str())
            .with(key(key::TARGET)?, target.as_str())
            .with(key(key::SCOPE)?, plan.scope())
            .with(
                key(key::DETAILS)?,
                &format!(
                    "{wire} jump from {} to {} ({number}). Scope: {}.",
                    jumped.source().as_str(),
                    target.as_str(),
                    plan.scope()
                ),
            ),
    ));
    enter_stage(read_model, at, plan, target)
}

/// `Recomposed` → `RECOMPOSED`、計画一覧・総数・行末トークンの更新。
fn recomposed_event(
    recomposed: &Recomposed,
    at: &DateTime<Utc>,
    plan: &ResolvedPlan,
    read_model: &mut ReadModel,
) -> Result<(), ProjectionError> {
    let in_scope = recomposed.stages_in_scope().len().to_string();
    read_model.append_audit(&render_audit_block(
        EventType::Recomposed,
        at,
        &AuditFields::new()
            .with(key(key::SCOPE)?, plan.scope())
            .with(key(key::STAGES_SKIPPED)?, &stage_list(recomposed.skipped()))
            .with(key(key::STAGES_ADDED)?, &stage_list(recomposed.added()))
            .with(key(key::STAGES_IN_SCOPE)?, &in_scope),
    ));

    for slug in recomposed.skipped() {
        let number = number_of(plan, slug)?;
        remove_from_list(read_model, field::STAGES_TO_EXECUTE, &number)?;
        append_to_list(
            read_model,
            field::STAGES_TO_SKIP,
            &format!("{number} ({})", slug.as_str()),
        )?;
        set_suffix(read_model, slug.as_str(), PlanAction::Skip)?;
    }
    for slug in recomposed.added() {
        let number = number_of(plan, slug)?;
        remove_from_list(
            read_model,
            field::STAGES_TO_SKIP,
            &format!("{number} ({})", slug.as_str()),
        )?;
        append_to_list(read_model, field::STAGES_TO_EXECUTE, &number)?;
        set_suffix(read_model, slug.as_str(), PlanAction::Execute)?;
    }
    set_field(read_model, field::TOTAL_STAGES, &in_scope)
}

// ---------------------------------------------------------------------------
// park / autonomy
// ---------------------------------------------------------------------------

/// `Parked` → `WORKFLOW_PARKED`、park マーカーの設置。
fn parked_event(
    parked: &Parked,
    at: &DateTime<Utc>,
    read_model: &mut ReadModel,
) -> Result<(), ProjectionError> {
    let fields = AuditFields::new().with(key(key::STAGE)?, parked.stage().as_str());
    read_model.append_audit(&render_audit_block(EventType::WorkflowParked, at, &fields));
    park_marker::set(read_model, parked.stage().as_str(), at)
}

/// `Unparked` → `WORKFLOW_UNPARKED`（フィールド無し）、park マーカーの除去。
fn unparked(at: &DateTime<Utc>, read_model: &mut ReadModel) {
    read_model.append_audit(&render_audit_block(
        EventType::WorkflowUnparked,
        at,
        &AuditFields::new(),
    ));
    park_marker::clear(read_model);
}

/// `AutonomyModeSet` → `AUTONOMY_MODE_SET`、`Construction Autonomy Mode`。
fn autonomy_mode_set(
    mode: AutonomyMode,
    at: &DateTime<Utc>,
    read_model: &mut ReadModel,
) -> Result<(), ProjectionError> {
    let fields = AuditFields::new().with(key(key::MODE)?, mode.as_state_field());
    read_model.append_audit(&render_audit_block(EventType::AutonomyModeSet, at, &fields));
    set_field(read_model, field::AUTONOMY_MODE, mode.as_state_field())
}

// ---------------------------------------------------------------------------
// 共有の断片
// ---------------------------------------------------------------------------

/// フェーズ境界の 3 行（完了 → 検証 → 次フェーズ開始）。
fn append_phase_boundary(
    read_model: &mut ReadModel,
    at: &DateTime<Utc>,
    plan: &ResolvedPlan,
    boundary: PhaseBoundary,
) -> Result<(), ProjectionError> {
    let from = boundary.from_phase();
    let to = boundary.to_phase();
    read_model.append_audit(&render_audit_block(
        EventType::PhaseCompleted,
        at,
        &AuditFields::new()
            .with(key(key::FROM_PHASE)?, from.as_str())
            .with(key(key::TO_PHASE)?, to.as_str())
            .with(
                key(key::STAGES_COMPLETED)?,
                &plan.in_scope_count_of(from).to_string(),
            ),
    ));
    read_model.append_audit(&render_audit_block(
        EventType::PhaseVerified,
        at,
        &AuditFields::new().with(
            key(key::PHASE_BOUNDARY)?,
            &format!("{}{BOUNDARY_ARROW}{}", from.as_str(), to.as_str()),
        ),
    ));
    read_model.append_audit(&render_audit_block(
        EventType::PhaseStarted,
        at,
        &AuditFields::new()
            .with(key(key::PHASE)?, to.as_str())
            .with(key(key::SCOPE)?, plan.scope()),
    ));
    Ok(())
}

/// `STAGE_STARTED` 行 1 本。
fn stage_started_row(stage: &PlannedStage, at: &DateTime<Utc>) -> Result<String, ProjectionError> {
    Ok(render_audit_block(
        EventType::StageStarted,
        at,
        &AuditFields::new()
            .with(key(key::STAGE)?, stage.slug().as_str())
            .with(key(key::AGENT)?, stage.display().lead_agent()),
    ))
}

/// 次ステージへ移る（次が無ければワークフロー完了）。
fn leave_for(
    read_model: &mut ReadModel,
    at: &DateTime<Utc>,
    plan: &ResolvedPlan,
    next: Option<&StageSlug>,
) -> Result<(), ProjectionError> {
    match next {
        Some(slug) => enter_stage(read_model, at, plan, slug),
        None => {
            read_model.append_audit(&render_audit_block(
                EventType::WorkflowCompleted,
                at,
                &AuditFields::new(),
            ));
            Ok(())
        }
    }
}

/// ステージを開始する — `STAGE_STARTED` 行と、現在位置まわりの状態フィールド。
fn enter_stage(
    read_model: &mut ReadModel,
    at: &DateTime<Utc>,
    plan: &ResolvedPlan,
    slug: &StageSlug,
) -> Result<(), ProjectionError> {
    let stage = plan.find(slug).ok_or_else(|| unknown(slug))?;
    read_model.append_audit(&stage_started_row(stage, at)?);
    enter_stage_without_row(read_model, plan, slug)
}

/// 現在位置まわりの状態フィールドだけを書く（`STAGE_STARTED` 行は描かない）。
///
/// `Started` は初期化 3 ステージの行を先に描き終えており、最初のゲート付きステージの
/// `STAGE_STARTED` もその列の最後に既に入っている。ここで二度描かないための分割である。
fn enter_stage_without_row(
    read_model: &mut ReadModel,
    plan: &ResolvedPlan,
    slug: &StageSlug,
) -> Result<(), ProjectionError> {
    let stage = plan.find(slug).ok_or_else(|| unknown(slug))?;
    set_checkbox(read_model, slug.as_str(), CheckboxState::InProgress)?;
    set_field(
        read_model,
        field::ACTIVE_AGENT,
        stage.display().lead_agent(),
    )?;
    set_field(read_model, field::IN_PROGRESS, slug.as_str())?;
    set_field(read_model, field::CURRENT_STAGE, slug.as_str())?;
    set_field(
        read_model,
        field::NEXT_STAGE,
        plan.next_in_scope_after(slug)
            .map_or("", |next| next.slug().as_str()),
    )?;
    set_field(
        read_model,
        field::NEXT_ACTION,
        &format!("Execute {}", stage.display().name()),
    )
}

/// ステージを完了させる — チェックボックス `[x]`、完了数の同期、最終完了ステージ。
fn complete_stage(read_model: &mut ReadModel, slug: &StageSlug) -> Result<(), ProjectionError> {
    set_checkbox(read_model, slug.as_str(), CheckboxState::Completed)?;
    let completed = count_completed(read_model.state()).to_string();
    set_field(read_model, field::COMPLETED, &completed)?;
    set_field(read_model, field::LAST_COMPLETED_STAGE, slug.as_str())
}

/// 計画上の表題を引く。
fn title_of(plan: &ResolvedPlan, slug: &StageSlug) -> Result<String, ProjectionError> {
    plan.display_of(slug)
        .map(|display| display.name().to_string())
        .ok_or_else(|| unknown(slug))
}

/// 計画上のステージ番号を引く。
fn number_of(plan: &ResolvedPlan, slug: &StageSlug) -> Result<String, ProjectionError> {
    plan.display_of(slug)
        .map(|display| display.number().as_str().to_string())
        .ok_or_else(|| unknown(slug))
}

fn unknown(slug: &StageSlug) -> ProjectionError {
    ProjectionError::UnknownStage {
        stage: slug.as_str().to_string(),
    }
}

/// `JumpDirection` のワイヤ綴り（`**Direction**:` は大文字）。
const fn direction_wire(direction: JumpDirection) -> &'static str {
    match direction {
        JumpDirection::Forward => "FORWARD",
        JumpDirection::Backward => "BACKWARD",
        JumpDirection::Redo => "REDO",
    }
}

/// ステージ集合を `**Stages ...**:` の値へ描く（空は逐語 `none`）。
fn stage_list(stages: &[StageSlug]) -> String {
    if stages.is_empty() {
        return NONE_LITERAL.to_string();
    }
    stages
        .iter()
        .map(|slug| slug.as_str().to_string())
        .collect::<Vec<_>>()
        .join(LIST_SEPARATOR)
}

/// チェックボックスのマーカーだけを書き換える（接尾辞には触れない）。
fn set_checkbox(
    read_model: &mut ReadModel,
    slug: &str,
    state: CheckboxState,
) -> Result<(), ProjectionError> {
    let next = with_checkbox_marker(read_model.state(), slug, state)?;
    read_model.replace_state(next);
    Ok(())
}

/// チェックボックスの行末トークンだけを書き換える（マーカーには触れない）。
fn set_suffix(
    read_model: &mut ReadModel,
    slug: &str,
    action: PlanAction,
) -> Result<(), ProjectionError> {
    let next = with_checkbox_suffix(read_model.state(), slug, action)?;
    read_model.replace_state(next);
    Ok(())
}

/// 状態ファイルのフィールド行を書き換える（不在は拒否 — 無言 no-op は検出不能なドリフト）。
fn set_field(read_model: &mut ReadModel, field: &str, value: &str) -> Result<(), ProjectionError> {
    let next = with_field(read_model.state(), field, value)?;
    read_model.replace_state(next);
    Ok(())
}

/// カンマ区切り一覧フィールドから 1 項目を落とす（不在なら何も変えない）。
fn remove_from_list(
    read_model: &mut ReadModel,
    field: &str,
    item: &str,
) -> Result<(), ProjectionError> {
    let current = list_of(read_model, field)?;
    let kept: Vec<&str> = current
        .iter()
        .map(String::as_str)
        .filter(|entry| *entry != item)
        .collect();
    set_field(read_model, field, &kept.join(LIST_SEPARATOR))
}

/// カンマ区切り一覧フィールドへ 1 項目を末尾に足す（重複は足さない）。
fn append_to_list(
    read_model: &mut ReadModel,
    field: &str,
    item: &str,
) -> Result<(), ProjectionError> {
    let mut current = list_of(read_model, field)?;
    if current.iter().any(|entry| entry == item) {
        return Ok(());
    }
    current.push(item.to_string());
    set_field(read_model, field, &current.join(LIST_SEPARATOR))
}

/// 一覧フィールドの現在値を項目へ割る（空は 0 項目）。
fn list_of(read_model: &ReadModel, field: &str) -> Result<Vec<String>, ProjectionError> {
    let raw = find_field(read_model.state(), field).ok_or_else(|| {
        ProjectionError::StateField(FieldNotFound::new(super::wording::field_not_found_message(
            field,
        )))
    })?;
    Ok(if raw.is_empty() {
        Vec::new()
    } else {
        raw.split(LIST_SEPARATOR)
            .map(|entry| entry.trim().to_string())
            .collect()
    })
}

/// park マーカー — `## Runtime State` セクションの**末尾**（次の見出しの直前）に 2 行、
/// unpark で 2 行とも消す。
///
/// # なぜ `with_field_or_insert` を使わないのか
///
/// あちらは挿入位置をセクション末尾の**空行より前**へ巻き戻す（upstream `setOrInsertField`
/// の挙動）。park マーカーはそうならない — ゴールデン `cli/park/park/state.diff` の実バイトは
/// 空行の**あと**、次の `## ` 見出しの直前に 2 行が入っている。別の書き手なので別に実装する。
mod park_marker {
    use super::{ProjectionError, ReadModel};
    use chrono::{DateTime, SecondsFormat, Utc};

    /// マーカーを差し込むセクション見出し。
    const HEADING: &str = "## Runtime State";
    /// 停止時刻のフィールド行の接頭辞。
    const PARKED_PREFIX: &str = "- **Parked**:";
    /// 停止したステージのフィールド行の接頭辞。
    const PARKED_AT_STAGE_PREFIX: &str = "- **Parked At Stage**:";

    /// park マーカーを設置する（既にあれば置き直す — 冪等）。
    pub(super) fn set(
        read_model: &mut ReadModel,
        stage: &str,
        at: &DateTime<Utc>,
    ) -> Result<(), ProjectionError> {
        let cleared = removed(read_model.state());
        let lines: Vec<&str> = cleared.lines().collect();
        let start = lines
            .iter()
            .position(|line| line.trim_end() == HEADING)
            .ok_or(ProjectionError::ParkSectionMissing)?;
        // セクション末尾 = 次の `## ` 見出しの直前（末尾の空行は**そのまま**残す）。
        let end = lines
            .iter()
            .enumerate()
            .skip(start.saturating_add(1))
            .find(|(_, line)| line.starts_with("## "))
            .map_or(lines.len(), |(index, _)| index);

        let stamp = at.to_rfc3339_opts(SecondsFormat::Secs, true);
        let mut out: Vec<String> = lines.iter().map(|line| (*line).to_string()).collect();
        out.splice(
            end..end,
            [
                format!("{PARKED_PREFIX} {stamp}"),
                format!("{PARKED_AT_STAGE_PREFIX} {stage}"),
            ],
        );
        read_model.replace_state(rejoin(&out, &cleared));
        Ok(())
    }

    /// park マーカーを除去する（不在は no-op — 二重 unpark で落ちない）。
    pub(super) fn clear(read_model: &mut ReadModel) {
        let next = removed(read_model.state());
        read_model.replace_state(next);
    }

    /// マーカー 2 行を落とした本文。
    fn removed(content: &str) -> String {
        let out: Vec<String> = content
            .lines()
            .filter(|line| {
                !line.starts_with(PARKED_PREFIX) && !line.starts_with(PARKED_AT_STAGE_PREFIX)
            })
            .map(|line| line.to_string())
            .collect();
        rejoin(&out, content)
    }

    /// 末尾改行の有無を元の本文に合わせて行を綴じ直す。
    fn rejoin(lines: &[String], original: &str) -> String {
        let mut joined = lines.join("\n");
        if original.ends_with('\n') {
            joined.push('\n');
        }
        joined
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core_command_domain::orchestration::{
        AutonomyModeSet, IntentId, StageDisplay, StageEntry, StartRequest, Started, WorkspaceScan,
    };
    use core_command_domain::workflow_definition::{
        BrownfieldGreenfield, DefinitionRevision, StageNumber, WorkflowDefinitionId,
    };

    fn at() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-08-21T09:14:07Z")
            .expect("固定の ISO 8601")
            .with_timezone(&Utc)
    }

    fn slug(value: &str) -> StageSlug {
        StageSlug::parse(value).expect("テストの slug は文法内")
    }

    fn stage(name: &str, number: &str, phase: PhaseId, action: PlanAction) -> StageEntry {
        StageEntry::new(
            slug(name),
            phase,
            action,
            false,
            StageDisplay::new(
                StageNumber::parse(number).expect("番号"),
                "Some Title",
                "orchestrator",
            )
            .expect("単一行"),
        )
    }

    /// initialization 1 + inception 2 + operation 1 の合成計画。
    fn started() -> Started {
        Started::new(
            WorkflowDefinitionId::parse("claude").expect("定義 id"),
            DefinitionRevision::parse(&format!("sha256:{}", "0".repeat(64))).expect("revision"),
            &StartRequest::new("classic", "build it"),
            vec![
                stage(
                    "state-init",
                    "0.1",
                    PhaseId::Initialization,
                    PlanAction::Execute,
                ),
                stage("first", "2.1", PhaseId::Inception, PlanAction::Execute),
                stage("second", "2.2", PhaseId::Inception, PlanAction::Execute),
                stage("late", "4.1", PhaseId::Operation, PlanAction::Skip),
            ],
            WorkspaceScan::new(
                BrownfieldGreenfield::Greenfield,
                "Unknown",
                "Unknown",
                "Unknown",
            )
            .expect("単一行"),
        )
    }

    fn plan() -> ResolvedPlan {
        ResolvedPlan::of(&started())
    }

    const SKELETON: &str = "\
## Project Information
- **Active Agent**: orchestrator

## Scope Configuration
- **Stages to Execute**: 0.1, 2.1
- **Stages to Skip**: 4.1 (late)

## Execution Plan Summary
- **Total Stages**: 3
- **Completed**: 0
- **In Progress**: state-init

## Runtime State
- **Revision Count**: 0
- **Construction Autonomy Mode**: gated

## Stage Progress
- [-] state-init — EXECUTE
- [ ] first — EXECUTE
- [ ] second — EXECUTE
- [ ] late — SKIP

## Current Status
- **Current Stage**: state-init
- **Next Stage**: first

## Session Resume Point
- **Last Completed Stage**: 
- **Next Action**: Execute Stage
";

    fn model() -> ReadModel {
        ReadModel::new(SKELETON)
    }

    fn entry(event: WorkflowExecutionEvent) -> JournalEntry {
        JournalEntry::new(
            crate::orchestration::GlobalSeqNr::new(1),
            IntentId::parse("01a02785-1bd8-76eb-aeea-5aa303ebd5b6").expect("UUIDv7"),
            1,
            at(),
            event,
        )
    }

    fn run(event: WorkflowExecutionEvent) -> ReadModel {
        let mut read_model = model();
        project(&[entry(event)], &plan(), &mut read_model).expect("投影");
        read_model
    }

    #[test]
    fn the_genesis_lands_on_the_first_gated_stage_when_the_skeleton_exists() {
        let read_model = run(WorkflowExecutionEvent::Started(started()));
        // initialization は完了、最初のゲート付きステージが in-flight。
        assert!(read_model.state().contains("- [x] state-init — EXECUTE"));
        assert!(read_model.state().contains("- [-] first — EXECUTE"));
        assert!(read_model.state().contains("- **Completed**: 1\n"));
        assert!(
            read_model
                .state()
                .contains("- **Last Completed Stage**: state-init\n")
        );
        // スコープ内は state-init / first / second の 3 つ。
        assert!(read_model.state().contains("- **Total Stages**: 3\n"));
        assert!(read_model.state().contains("- **Next Stage**: second\n"));
        assert!(
            read_model
                .state()
                .contains("- **Next Action**: Execute Some Title\n")
        );
        // 計画一覧は触らない（畳まれた理由を導けないため）。
        assert!(
            read_model
                .state()
                .contains("- **Stages to Skip**: 4.1 (late)\n")
        );
        // `STAGE_STARTED` は 1 本だけ（行を二度描かない）。
        assert_eq!(
            read_model
                .appended_audit()
                .matches("**Event**: STAGE_STARTED")
                .count(),
            2,
            "初期化 1 本 + 最初のゲート付き 1 本"
        );
    }

    #[test]
    fn a_genesis_without_a_skeleton_stops_instead_of_inventing_one() {
        let mut read_model = ReadModel::new("   \n");
        let error = project(
            &[entry(WorkflowExecutionEvent::Started(started()))],
            &plan(),
            &mut read_model,
        )
        .expect_err("骨格が無い");
        assert_eq!(error, ProjectionError::ScaffoldTemplateUnavailable);
        assert_eq!(error.to_string(), "scaffold template unavailable");
        // 監査行だけは描けている（骨格が無くても台帳は書ける）。
        assert!(
            read_model
                .appended_audit()
                .contains("**Event**: WORKFLOW_STARTED")
        );
    }

    #[test]
    fn switching_the_autonomy_mode_writes_the_row_and_the_field() {
        let read_model = run(WorkflowExecutionEvent::AutonomyModeSet(
            AutonomyModeSet::new(AutonomyMode::Autonomous),
        ));
        assert!(
            read_model
                .appended_audit()
                .contains("**Event**: AUTONOMY_MODE_SET")
        );
        assert!(
            read_model
                .appended_audit()
                .contains("**Mode**: autonomous\n")
        );
        assert!(
            read_model
                .state()
                .contains("- **Construction Autonomy Mode**: autonomous\n")
        );
    }

    #[test]
    fn approving_the_last_stage_completes_the_workflow_instead_of_starting_one() {
        let read_model = run(WorkflowExecutionEvent::GateApproved(GateApproved::new(
            slug("second"),
            None,
            None,
            None,
        )));
        assert!(
            read_model
                .appended_audit()
                .contains("**Event**: WORKFLOW_COMPLETED")
        );
        assert!(
            !read_model
                .appended_audit()
                .contains("**Event**: STAGE_STARTED"),
            "次が無ければステージは始まらない"
        );
        // `User Input` が無ければ行も出ない。
        assert!(!read_model.appended_audit().contains("**User Input**"));
    }

    #[test]
    fn a_phase_boundary_adds_the_three_boundary_rows_in_order() {
        let read_model = run(WorkflowExecutionEvent::GateApproved(GateApproved::new(
            slug("first"),
            Some("A".to_string()),
            Some(slug("second")),
            Some(PhaseBoundary::new(
                PhaseId::Inception,
                PhaseId::Construction,
            )),
        )));
        let events: Vec<&str> = read_model
            .appended_audit()
            .lines()
            .filter_map(|line| line.strip_prefix("**Event**: "))
            .collect();
        assert_eq!(
            events,
            [
                "GATE_APPROVED",
                "STAGE_COMPLETED",
                "PHASE_COMPLETED",
                "PHASE_VERIFIED",
                "PHASE_STARTED",
                "STAGE_STARTED",
            ]
        );
        assert!(
            read_model
                .appended_audit()
                .contains("**Phase boundary**: inception → construction\n")
        );
        assert!(
            read_model
                .appended_audit()
                .contains("**Stages completed**: 2\n")
        );
    }

    #[test]
    fn completing_a_non_gated_stage_uses_the_completed_wording() {
        let read_model = run(WorkflowExecutionEvent::StageCompleted(StageCompleted::new(
            slug("state-init"),
            Some(slug("first")),
        )));
        assert!(
            read_model
                .appended_audit()
                .contains("**Details**: Stage Some Title completed\n")
        );
        assert!(read_model.state().contains("- [x] state-init — EXECUTE"));
    }

    #[test]
    fn a_backward_jump_resets_the_downstream_checkboxes() {
        let read_model = run(WorkflowExecutionEvent::Jumped(Jumped::new(
            JumpDirection::Backward,
            slug("second"),
            slug("first"),
            vec![slug("second")],
            Vec::new(),
        )));
        assert!(
            read_model
                .appended_audit()
                .contains("**Direction**: BACKWARD\n")
        );
        assert!(
            read_model.appended_audit().contains(
                "**Details**: BACKWARD jump from second to first (2.1). Scope: classic.\n"
            )
        );
        assert!(
            read_model.state().contains("- [ ] second — EXECUTE"),
            "下流は pending へ"
        );
        assert!(read_model.state().contains("- [-] first — EXECUTE"));
    }

    #[test]
    fn every_jump_direction_has_a_wire_spelling() {
        assert_eq!(direction_wire(JumpDirection::Forward), "FORWARD");
        assert_eq!(direction_wire(JumpDirection::Backward), "BACKWARD");
        assert_eq!(direction_wire(JumpDirection::Redo), "REDO");
    }

    #[test]
    fn recomposing_back_into_scope_moves_the_entry_the_other_way() {
        let read_model = run(WorkflowExecutionEvent::Recomposed(Recomposed::new(
            Vec::new(),
            vec![slug("late")],
            vec![
                slug("state-init"),
                slug("first"),
                slug("second"),
                slug("late"),
            ],
        )));
        assert!(
            read_model
                .state()
                .contains("- **Stages to Execute**: 0.1, 2.1, 4.1\n"),
            "実際: {}",
            read_model.state()
        );
        assert!(read_model.state().contains("- **Stages to Skip**: \n"));
        assert!(read_model.state().contains("- [ ] late — EXECUTE"));
        assert!(read_model.state().contains("- **Total Stages**: 4\n"));
        assert!(
            read_model
                .appended_audit()
                .contains("**Stages skipped**: none\n")
        );
    }

    #[test]
    fn appending_an_entry_that_is_already_listed_changes_nothing() {
        let mut read_model = model();
        append_to_list(&mut read_model, field::STAGES_TO_EXECUTE, "0.1").expect("追加");
        assert!(
            read_model
                .state()
                .contains("- **Stages to Execute**: 0.1, 2.1\n")
        );
    }

    #[test]
    fn a_missing_list_field_is_refused_with_the_verbatim_wording() {
        let mut read_model = ReadModel::new("## Empty\n");
        let error = append_to_list(&mut read_model, field::STAGES_TO_EXECUTE, "0.1")
            .expect_err("一覧フィールドが無い");
        assert_eq!(
            error.to_string(),
            "state field: Field not found in state file: \"Stages to Execute\". \
             Cannot update — refusing to silently no-op."
        );
    }

    #[test]
    fn every_projection_refusal_renders_its_material() {
        assert_eq!(
            ProjectionError::UnknownStage {
                stage: "ghost".to_string()
            }
            .to_string(),
            "unknown stage: ghost"
        );
        assert_eq!(
            ProjectionError::from(CheckboxUpdateError::MissingSuffix("s".to_string())).to_string(),
            "checkbox: missing suffix s"
        );
        assert_eq!(
            ProjectionError::from(CheckboxUpdateError::MissingStage("s".to_string())).to_string(),
            "checkbox: missing stage s"
        );
        // 綴りを取り違えたキーは投影の材料として拒否される（本体の定数は全て文法内なので、
        // この写像は直接固定しておく）。
        let malformed = AuditFieldKey::parse("1x").expect_err("文法外");
        assert_eq!(
            ProjectionError::from(malformed).to_string(),
            "audit field key: malformed audit field key: 1x"
        );
        let boxed: Box<dyn std::error::Error> = Box::new(ProjectionError::ParkSectionMissing);
        assert_eq!(boxed.to_string(), "park section missing");
    }

    #[test]
    fn a_stage_the_plan_does_not_know_is_refused_everywhere_it_is_named() {
        let ghost = slug("ghost");
        assert_eq!(
            title_of(&plan(), &ghost),
            Err(ProjectionError::UnknownStage {
                stage: "ghost".to_string()
            })
        );
        assert_eq!(
            number_of(&plan(), &ghost),
            Err(ProjectionError::UnknownStage {
                stage: "ghost".to_string()
            })
        );
    }
}
