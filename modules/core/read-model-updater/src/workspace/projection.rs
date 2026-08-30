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
    AutonomyMode, GateApproved, GateOpened, GateRejected, IntentExecutionEvent, JumpDirection,
    Jumped, Parked, PhaseBoundary, Recomposed, StageCompleted, StageRevised, StageSkipped,
};
use core_command_domain::workflow_definition::{PhaseId, PlanAction, StageSlug};
use core_command_domain::workspace::{
    AuditFieldKey, AuditFieldKeyError, AuditFields, CheckboxState, CheckboxUpdateError, Checkboxes,
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
    /// `**Mode**:`。**upstream の実行出力としては採れない**（`cli/set-autonomy` は失敗経路
    /// しか捉えていない）。ピン `3c3146cf` の配布シェルには
    /// `- **Construction Autonomy Mode**:` 行を状態ファイルへ書き込む経路が 1 つも無く、
    /// `set-autonomy` は行の不在を検出して終了コード 1 で止まるため、成功経路そのものが
    /// 到達不能である（全数走査の根拠は `tests/golden/upstream-3c3146cf/README.md` と
    /// `cli/cases-missing.json` の `set-autonomy/gated`）。このキーはピンの**ソース**
    /// （`aidlc-bolt.ts` の `emitAudit(pd, "AUTONOMY_MODE_SET", { Mode: … })`）から読んだ値で
    /// あり、実行バイトでの裏取りはピン更新待ちである。状態ファイル側の綴りは
    /// `cli/set-autonomy/state-field-absent` の失敗文言が逐語で固定している。
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
    /// `- **Lifecycle Phase**:`（値は**大文字**の フェーズ名 — `INCEPTION`）。
    pub(super) const LIFECYCLE_PHASE: &str = "Lifecycle Phase";
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

    /// `## Phase Progress` の 1 行のラベル（`- **Inception**:`）。
    ///
    /// upstream は `phase.charAt(0).toUpperCase() + phase.slice(1)` で作る — フェーズ slug の
    /// 先頭 1 文字だけを大文字にしたものである。5 つとも ASCII 小文字始まりなので、この
    /// 単純な変換で `Initialization` / `Ideation` / `Inception` / `Construction` / `Operation`
    /// になる。
    pub(super) fn phase_row(phase: super::PhaseId) -> String {
        let slug = phase.as_str();
        let mut label = String::with_capacity(slug.len());
        for (index, ch) in slug.chars().enumerate() {
            if index == 0 {
                label.extend(ch.to_uppercase());
            } else {
                label.push(ch);
            }
        }
        label
    }
}

/// 空のステージ集合を描く逐語（`**Stages added**: none`）。
const NONE_LITERAL: &str = "none";
/// 再入時の逐語（`report --result revised`）。
const REENTRY_DETAILS: &str = "Re-entering gate after revision";
/// フェーズ境界の区切り（U+2192）。
const BOUNDARY_ARROW: &str = " → ";
/// Skip 行の項目に付く注釈の区切り（U+2014）。`2.1 (reverse-engineering — greenfield)`。
const SKIP_ANNOTATION: &str = " — ";
/// 一覧の区切り。
const LIST_SEPARATOR: &str = ", ";

/// `## Phase Progress` の行がとる 4 値（`<!-- Status values: … -->` が正本）。
mod phase_status {
    /// まだ来ていない。
    pub(super) const PENDING: &str = "Pending";
    /// いま走っている。
    pub(super) const ACTIVE: &str = "Active";
    /// 通過して検証済み。
    pub(super) const VERIFIED: &str = "Verified";
    /// スコープ内ステージが 1 つも無い、または飛び越えた。
    pub(super) const SKIPPED: &str = "Skipped";
}

/// ジャンプがフェーズ境界をまたいだときの `PHASE_VERIFIED` の逐語
/// （`cli/jump/execute-backward` / `execute-forward-across-phases`）。
const JUMP_BOUNDARY_VERIFICATION: &str = "Traceability verification on jump";

/// 走査して 1 つも完了ステージが見つからなかったときの `- **Last Completed Stage**:`
/// （upstream がジャンプ経路にだけ置いている既定値）。
const NO_EARLIER_COMPLETION: &str = "state-init";

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
    /// 状態ファイルの**骨格が無い** — 投影の前提違反である。
    ///
    /// # 骨格を書くのは投影ではない（オーナー裁定 2026-08-29）
    ///
    /// 投影の責務は**既存本文への差分適用に徹する**ことであり、本文そのもの — 9 セクションの
    /// 骨格と 31 のフィールド行 — を起こすことは含まれない。骨格は intent-create の時点で
    /// **合成ルート**が書く（環境と両側を知ってよい唯一の場所。実装は U7）。
    ///
    /// これは「導出の工夫が足りない」のではなく、**構造から従う**裁定である。骨格には
    /// `- **Project Root**:` があり、これはワークツリーの絶対パス — すなわち**環境の値**で、
    /// ジャーナルに存在しない。投影がこれを書けるようになる道は「環境を読む」か「環境パスを
    /// ドメインイベントへ載せる」かの 2 つしかなく、前者は投影核の定義を壊し、後者は ADR-008
    /// と NFR3 の趣旨に反する。書けないのではなく、**書く場所がここではない**。
    ///
    /// # NFR3 の適用範囲
    ///
    /// 冪等な再構成が保証するのは**差分適用**である — 同じジャーナルを同じ本文へ当てれば
    /// 常に同じバイトが出る。骨格はその保証の対象ではなく**環境成果物**であり、全損したら
    /// 再生成ではなく upstream 同様 archive & recreate で復旧する運用に載る。
    ///
    /// # 骨格の実バイトはある
    ///
    /// `cli/intent-create/classic-scope/state-full.md` が全文（102 行）で、U7 が骨格を書く
    /// ときの正本になる。upstream 側の正本は `aidlc-utility.ts` の template literal である
    /// （`knowledge/aidlc-shared/state-template.md` は LLM 向けの契約文書でツールは読まない）。
    ScaffoldMissing,
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
            ProjectionError::ScaffoldMissing => f.write_str("scaffold missing"),
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
/// 骨格の無い状態ファイルへ `Started` の状態面を求められた（`ScaffoldMissing`）を返す。
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
    event: &IntentExecutionEvent,
    at: &DateTime<Utc>,
    plan: &ResolvedPlan,
    read_model: &mut ReadModel,
) -> Result<(), ProjectionError> {
    match event {
        IntentExecutionEvent::Started(_) => started(at, plan, read_model),
        IntentExecutionEvent::StageCompleted(completed) => {
            stage_completed(completed, at, plan, read_model)
        }
        IntentExecutionEvent::GateOpened(opened) => gate_opened(opened, at, read_model),
        IntentExecutionEvent::GateApproved(approved) => {
            gate_approved(approved, at, plan, read_model)
        }
        IntentExecutionEvent::GateRejected(rejected) => gate_rejected(rejected, at, read_model),
        IntentExecutionEvent::StageRevised(revised) => stage_revised(revised, at, read_model),
        IntentExecutionEvent::StageSkipped(skipped) => stage_skipped(skipped, at, plan, read_model),
        IntentExecutionEvent::Jumped(jumped) => jumped_event(jumped, at, plan, read_model),
        IntentExecutionEvent::Parked(parked) => parked_event(parked, at, read_model),
        IntentExecutionEvent::Unparked => {
            unparked(at, read_model);
            Ok(())
        }
        IntentExecutionEvent::Recomposed(recomposed) => {
            recomposed_event(recomposed, at, plan, read_model)
        }
        IntentExecutionEvent::AutonomyModeSet(mode) => {
            autonomy_mode_set(mode.mode(), at, read_model)
        }
    }
}

// ---------------------------------------------------------------------------
// `Started` — 初期化 3 ステージの 16 行（`cli/intent-create/classic-scope`）
// ---------------------------------------------------------------------------

/// `Started` → 監査行 16 本と、状態ファイルの初期化。
///
/// 状態ファイルの**骨格が無ければ**（本文が空）`ScaffoldMissing` で止まる — 骨格を書くのは
/// 合成ルートであって投影ではない（オーナー裁定 2026-08-29、[`ProjectionError::ScaffoldMissing`]）。
/// 骨格があるなら、初期化 3 ステージの完了・最初のゲート付きステージへの着地・総数を書く —
/// いずれも他のイベントで逐語検収済みの writer と導出をそのまま使う。
///
/// **書かないもの**: `- **Stages to Execute**: ` / `- **Stages to Skip**: ` の 2 つ。どちらも
/// 骨格の行であり、書くのは合成ルートである。`- **Stages to Skip**:` の実バイトは
/// `2.1 (reverse-engineering — greenfield)` のように**畳まれた理由**を括弧内に持つが、その理由は
/// 素のグリッド値と調整後の値の区別を要し、`Started` の材料からは導けない — これも骨格生成が
/// 投影の仕事ではないことの傍証である。
fn started(
    at: &DateTime<Utc>,
    plan: &ResolvedPlan,
    read_model: &mut ReadModel,
) -> Result<(), ProjectionError> {
    append_started_rows(at, plan, read_model)?;
    if read_model.state().trim().is_empty() {
        return Err(ProjectionError::ScaffoldMissing);
    }
    for stage in plan
        .stages()
        .iter()
        .filter(|stage| stage.is_in_scope() && stage.phase() == PhaseId::Initialization)
    {
        set_checkbox(read_model, stage.slug().as_str(), CheckboxState::Completed)?;
    }
    let completed = Checkboxes::parse(read_model.state())
        .count_completed()
        .to_string();
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
            &plan.in_scope_count_of(PhaseId::Initialization).to_string(),
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
    // 改訂回数はイベントに載らない — upstream `aidlc-state.ts` と同じく、リードモデルの
    // `Revision Count` を読んで +1 する (非数値・欠落は 0 に畳む — 正本互換の導出)。
    let prior = find_field(read_model.state(), field::REVISION_COUNT)
        .and_then(|value| value.trim().parse::<u32>().ok())
        .unwrap_or(0);
    let revisions = prior.saturating_add(1).to_string();

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
    // 境界行の `**Stages completed**:` は**倒したあとの**チェックボックスを数えた値なので、
    // 先に完了させる（`cli/report/approved-across-phases` は 2 — 計画上の inception 内
    // スコープ件数 8 とは一致しない）。監査行の順序はここでは動かない — `complete_stage` は
    // 状態面だけを触り、行を描かないからである。
    complete_stage(read_model, stage)?;
    // 次カーソルとフェーズ境界はイベントに載らない — リードモデルの実効プランと計画から
    // 導く (`Jumped` の境界導出と同じ理由 — 材料が足りているうちはイベントを太らせない)。
    let next = next_in_effective_scope(read_model, plan, stage);
    if let Some(next) = &next
        && let Some(boundary) = crossed_phase_boundary(plan, stage, next)?
    {
        let completed = completed_count(read_model);
        append_phase_boundary(read_model, at, plan, boundary, &completed)?;
        set_phase_progress_for_advance(read_model, boundary)?;
    }
    leave_for(read_model, at, plan, next.as_ref())
}

// ---------------------------------------------------------------------------
// 進行
// ---------------------------------------------------------------------------

/// `StageCompleted`（非ゲートの完了）→ `STAGE_COMPLETED` + 次ステージの開始。
///
/// `**Details**:` の逐語は `Stage <表示名> completed` である（`cli/report/completed-ungated`
/// が実バイトを固定している）。ゲート経由の完了は `Stage <表示名> approved by gate` で、
/// **同じ `STAGE_COMPLETED` でも文言が割れる** — 書き手が違うからで、片方に寄せてはならない。
///
/// 出荷グラフで非ゲートなのは initialization の 3 ステージだけであり、その 3 本は genesis で
/// 完了済みになる。この単独経路へ到達するには**後方ジャンプで initialization ステージを
/// `[-]` へ戻してから** `report --result completed` を打つ（採取手順は同ケースの `argv`）。
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
    let next = next_in_effective_scope(read_model, plan, stage);
    leave_for(read_model, at, plan, next.as_ref())
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
    let next = next_in_effective_scope(read_model, plan, skipped.stage());
    leave_for(read_model, at, plan, next.as_ref())
}

/// `Jumped` → 読み飛ばした各ステージの `STAGE_SKIPPED` + (フェーズ境界) + `STAGE_JUMPED`
/// + 目標の開始。
///
/// # フェーズ境界はイベントに載らず、計画から導く
///
/// `Jumped` は出発点と到達点の slug しか運ばない。だが計画は両方のフェーズを知っているので、
/// またいだかどうかは**渡された計画から導ける** — `GateApproved` のように `PhaseBoundary` を
/// イベントへ足す必要は無い。導出であって推測ではないので、材料が足りているうちはイベントを
/// 太らせない（`resolved_plan.rs` の「正本は 1 つでよい」と同じ理由）。
///
/// 境界 3 行はゲート経由のものと**同型ではない**。ジャンプ側だけが `**Details**:` を持ち
/// （`Phase boundary crossed via <方向> jump` / `Traceability verification on jump`）、
/// `**Stages completed**:` は計画上のフェーズ内件数ではなく**チェックボックスの数え直し**で
/// ある（`cli/jump/execute-backward` は 0、`execute-forward-across-phases` は 1 — どちらも
/// 直前の書き換え後に数えた値でしか説明が付かない）。
fn jumped_event(
    jumped: &Jumped,
    at: &DateTime<Utc>,
    plan: &ResolvedPlan,
    read_model: &mut ReadModel,
) -> Result<(), ProjectionError> {
    let target = jumped.target();
    // イベントは到達点しか運ばない — 出発点は自分の `Current Stage` 行、方向は計画上の
    // 位置の大小、読み飛ばし・巻き戻しの列は跳躍規則 (BR1.6) をリードモデルの行へ適用して
    // 導く (オーナー裁定 2026-08-30「イベントに状態は含めるな」)。
    let source_raw = find_field(read_model.state(), field::CURRENT_STAGE).unwrap_or_default();
    let source =
        StageSlug::parse(source_raw.trim()).map_err(|_| ProjectionError::UnknownStage {
            stage: source_raw.trim().to_string(),
        })?;
    let position = |slug: &StageSlug| -> Result<usize, ProjectionError> {
        plan.stages()
            .iter()
            .position(|stage| stage.slug() == slug)
            .ok_or_else(|| unknown(slug))
    };
    let (src_at, tgt_at) = (position(&source)?, position(target)?);
    let direction = JumpDirection::of(src_at, tgt_at);
    let wire = direction_wire(direction);
    let lowered = wire.to_lowercase();

    let checkboxes = Checkboxes::parse(read_model.state());
    let state_of = |slug: &StageSlug| {
        checkboxes
            .iter()
            .find(|entry| entry.slug() == slug.as_str())
            .map(core_command_domain::workspace::CheckboxEntry::state)
    };
    match direction {
        JumpDirection::Forward => {
            // 中間の未了 + 出発点で稼働中のものを skipped にする。行の並びは upstream の
            // emit 順 — **中間ステージを計画順に並べたあと、最後に出発点そのもの**が来る
            // (`jump/execute-forward-across-phases` の実バイト)。
            let mut skipped: Vec<&StageSlug> = plan
                .stages()
                .get(src_at + 1..tgt_at)
                .unwrap_or_default()
                .iter()
                .filter(|stage| {
                    // 実効 SKIP の中間は触らない (`SKIP` 行はそのまま — upstream 実バイト)。
                    effective_action(read_model, stage) == PlanAction::Execute
                        && state_of(stage.slug()).is_some_and(CheckboxState::is_in_flight)
                })
                .map(PlannedStage::slug)
                .collect();
            if state_of(&source).is_some_and(CheckboxState::is_active) {
                skipped.push(&source);
            }
            for slug in skipped {
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
        }
        JumpDirection::Backward => {
            // 到達点**以降**の in-scope 既着手を pending へ戻す — upstream は到達点自身も
            // 一度 pending へ戻してから開始し直す (`jump/execute-backward` の
            // `**Stages completed**: 0` は到達点の [x] を戻した後の数え直しでしか説明が
            // 付かない)。
            for stage in plan.stages().get(tgt_at..).unwrap_or_default() {
                let touched =
                    state_of(stage.slug()).is_some_and(|marker| marker != CheckboxState::Pending);
                if effective_action(read_model, stage) == PlanAction::Execute && touched {
                    set_checkbox(read_model, stage.slug().as_str(), CheckboxState::Pending)?;
                }
            }
        }
        JumpDirection::Redo => {}
    }
    // 完了数はジャンプでも**数え直す**。後方ジャンプは `[x]` を `[ ]` へ戻すので減る
    // （`cli/jump/execute-backward` は 4 → 0）。境界をまたがないジャンプでも upstream は
    // 同じ書き換えを打つ（値が動かないだけ）。
    set_field(read_model, field::COMPLETED, &completed_count(read_model))?;

    if let Some(boundary) = crossed_phase_boundary(plan, &source, target)? {
        append_jump_phase_boundary(read_model, at, plan, boundary, &lowered)?;
    }

    let number = number_of(plan, target)?;
    read_model.append_audit(&render_audit_block(
        EventType::StageJumped,
        at,
        &AuditFields::new()
            .with(key(key::DIRECTION)?, wire)
            .with(key(key::SOURCE)?, source.as_str())
            .with(key(key::TARGET)?, target.as_str())
            .with(key(key::SCOPE)?, plan.scope())
            .with(
                key(key::DETAILS)?,
                &format!(
                    "{wire} jump from {} to {} ({number}). Scope: {}.",
                    source.as_str(),
                    target.as_str(),
                    plan.scope()
                ),
            ),
    ));
    enter_stage(read_model, at, plan, target)?;
    set_field(
        read_model,
        field::LAST_COMPLETED_STAGE,
        &last_completion_before(read_model, plan, target),
    )
}

/// 出発点と到達点のフェーズが違えば境界を返す（どちらも計画に居ることが前提）。
fn crossed_phase_boundary(
    plan: &ResolvedPlan,
    source: &StageSlug,
    target: &StageSlug,
) -> Result<Option<PhaseBoundary>, ProjectionError> {
    let from = plan.find(source).ok_or_else(|| unknown(source))?.phase();
    let to = plan.find(target).ok_or_else(|| unknown(target))?.phase();
    Ok((from != to).then(|| PhaseBoundary::new(from, to)))
}

/// ジャンプの境界 3 行と、`## Phase Progress` の行の付け替え。
fn append_jump_phase_boundary(
    read_model: &mut ReadModel,
    at: &DateTime<Utc>,
    plan: &ResolvedPlan,
    boundary: PhaseBoundary,
    direction: &str,
) -> Result<(), ProjectionError> {
    let from = boundary.from_phase();
    let to = boundary.to_phase();
    read_model.append_audit(&render_audit_block(
        EventType::PhaseCompleted,
        at,
        &AuditFields::new()
            .with(key(key::FROM_PHASE)?, from.as_str())
            .with(key(key::TO_PHASE)?, to.as_str())
            .with(key(key::STAGES_COMPLETED)?, &completed_count(read_model))
            .with(
                key(key::DETAILS)?,
                &format!("Phase boundary crossed via {direction} jump"),
            ),
    ));
    read_model.append_audit(&render_audit_block(
        EventType::PhaseVerified,
        at,
        &AuditFields::new()
            .with(
                key(key::PHASE_BOUNDARY)?,
                &format!("{}{BOUNDARY_ARROW}{}", from.as_str(), to.as_str()),
            )
            .with(key(key::DETAILS)?, JUMP_BOUNDARY_VERIFICATION),
    ));
    read_model.append_audit(&render_audit_block(
        EventType::PhaseStarted,
        at,
        &AuditFields::new()
            .with(key(key::PHASE)?, to.as_str())
            .with(key(key::SCOPE)?, plan.scope()),
    ));
    set_phase_progress_for_jump(read_model, plan, from, to)
}

/// ジャンプ後の `## Phase Progress`。
///
/// 前方は「出発フェーズは通過したので `Verified`、飛び越えたフェーズは `Skipped`」、後方は
/// 「到達フェーズより後ろでスコープ内ステージを持つものは `Pending` へ戻す」。どちらも最後に
/// 到達フェーズを `Active` にする。スコープ内ステージが 1 つも無いフェーズには触れない —
/// genesis が置いた `Skipped` のままでよく、触ると `- **Ideation**: Skipped` が動いてしまう。
fn set_phase_progress_for_jump(
    read_model: &mut ReadModel,
    plan: &ResolvedPlan,
    from: PhaseId,
    to: PhaseId,
) -> Result<(), ProjectionError> {
    let (from_at, to_at) = (phase_order(from), phase_order(to));
    if from_at < to_at {
        set_field(read_model, &field::phase_row(from), phase_status::VERIFIED)?;
        for phase in PhaseId::ALL
            .iter()
            .copied()
            .skip(from_at.saturating_add(1))
            .take(to_at.saturating_sub(from_at).saturating_sub(1))
        {
            set_field(read_model, &field::phase_row(phase), phase_status::SKIPPED)?;
        }
    } else {
        for phase in PhaseId::ALL
            .iter()
            .copied()
            .skip(to_at.saturating_add(1))
            .filter(|phase| plan.in_scope_count_of(*phase) > 0)
        {
            set_field(read_model, &field::phase_row(phase), phase_status::PENDING)?;
        }
    }
    set_field(read_model, &field::phase_row(to), phase_status::ACTIVE)
}

/// ジャンプ後の `- **Last Completed Stage**:`。
///
/// 到達点より**手前**にある最後の `[x]` のステージを書く。1 つも無ければ upstream の既定値
/// `state-init` を書く（到達点が先頭ステージのときに起きる — `cli/jump/execute-backward` が
/// その実測である）。
///
/// 前から辿って**最後に当たったもの**を残すのは、後ろから探して最初に当たったものと同じで
/// ある。`take_while` は逆順に辿れないので、こちらの向きで書いた。到達点そのものは見ない。
fn last_completion_before(
    read_model: &ReadModel,
    plan: &ResolvedPlan,
    target: &StageSlug,
) -> String {
    let checkboxes = Checkboxes::parse(read_model.state());
    let mut found = NO_EARLIER_COMPLETION;
    for stage in plan
        .stages()
        .iter()
        .take_while(|stage| stage.slug() != target)
    {
        let slug = stage.slug().as_str();
        if checkboxes
            .iter()
            .any(|entry| entry.slug() == slug && entry.state() == CheckboxState::Completed)
        {
            found = slug;
        }
    }
    found.to_string()
}

/// `Recomposed` → `RECOMPOSED`、計画一覧・総数・行末トークンの更新。
fn recomposed_event(
    recomposed: &Recomposed,
    at: &DateTime<Utc>,
    plan: &ResolvedPlan,
    read_model: &mut ReadModel,
) -> Result<(), ProjectionError> {
    // 適用後の in-scope 数はイベントに載らない — 行末トークンを反転してから自分の行を
    // 数える (オーナー裁定 2026-08-30)。監査行の位置は従来と同じ (このイベントで 1 行)。
    for slug in recomposed.skipped() {
        set_suffix(read_model, slug.as_str(), PlanAction::Skip)?;
    }
    for slug in recomposed.added() {
        set_suffix(read_model, slug.as_str(), PlanAction::Execute)?;
    }
    let in_scope = plan
        .stages()
        .iter()
        .filter(|stage| effective_action(read_model, stage) == PlanAction::Execute)
        .count()
        .to_string();
    read_model.append_audit(&render_audit_block(
        EventType::Recomposed,
        at,
        &AuditFields::new()
            .with(key(key::SCOPE)?, plan.scope())
            .with(key(key::STAGES_SKIPPED)?, &stage_list(recomposed.skipped()))
            .with(key(key::STAGES_ADDED)?, &stage_list(recomposed.added()))
            .with(key(key::STAGES_IN_SCOPE)?, &in_scope),
    ));
    rebuild_plan_rows(read_model, plan)
}

/// `- **Stages to Execute**: ` / `- **Stages to Skip**: ` / `- **Total Stages**: ` を
/// 行末トークンから組み直す。
///
/// # 2 行は同じ規則で作られていない（upstream 実バイト）
///
/// **Execute 行は毎回 graph 順に組み直す**。`cli/recompose/add-restores-conditional` で
/// `2.1` が末尾ではなく `0.3` と `2.2` の**間**へ入るのがその実測である。
///
/// **Skip 行は既存項目をその位置のまま保つ**。項目は `<番号> (<slug>)` の形だが、genesis が
/// 書いた `2.1 (reverse-engineering — greenfield)` のように**注釈**を持つものがあり、slug から
/// 組み直すと注釈が消えてしまうためである。まだ skip のままの項目を逐語で残し、EXECUTE へ
/// 戻った項目を落とし、新しく skip になった項目を graph 順で末尾へ足す。
/// `cli/recompose/skip-two-appends-in-graph-order` が「既存の 4.5 の**後ろ**に 4.3, 4.7 が
/// 並ぶ」ことを、`add-restores-conditional` が「注釈ごと消える」ことを固定している。
///
/// # 実効計画をどこから読むか
///
/// upstream の `eff` は「recompose のオーバレイ ?? スコープグリッド」である。投影にとっての
/// オーバレイはチェックボックス行の行末トークンで、`set_suffix` がそれを保っている。行が
/// 無ければ（差分の断片しか無いテストなど）genesis の計画へ落とす — これは upstream の
/// `?? scopeDef.stages[slug]` と同じ既定である。
fn rebuild_plan_rows(
    read_model: &mut ReadModel,
    plan: &ResolvedPlan,
) -> Result<(), ProjectionError> {
    let checkboxes = Checkboxes::parse(read_model.state());
    let effective = |stage: &PlannedStage| -> PlanAction {
        checkboxes
            .iter()
            .find(|entry| entry.slug() == stage.slug().as_str())
            .and_then(|entry| match entry.rest().trim() {
                "EXECUTE" => Some(PlanAction::Execute),
                "SKIP" => Some(PlanAction::Skip),
                _ => None,
            })
            .unwrap_or_else(|| stage.plan_action())
    };

    let mut skips: Vec<String> = Vec::new();
    let mut preserved: Vec<String> = Vec::new();
    for token in list_of(read_model, field::STAGES_TO_SKIP)? {
        let slug = slug_of_skip_token(&token);
        let still_skipped = plan
            .stages()
            .iter()
            .find(|stage| stage.slug().as_str() == slug)
            .is_some_and(|stage| effective(stage) == PlanAction::Skip);
        if still_skipped {
            preserved.push(slug.to_string());
            skips.push(token);
        }
    }

    let mut executes: Vec<String> = Vec::new();
    for stage in plan.stages() {
        let slug = stage.slug().as_str();
        if effective(stage) == PlanAction::Execute {
            executes.push(stage.display().number().as_str().to_string());
        } else if !preserved.iter().any(|kept| kept == slug) {
            skips.push(format!("{} ({slug})", stage.display().number().as_str()));
        }
    }

    let total = executes.len().to_string();
    set_field(
        read_model,
        field::STAGES_TO_EXECUTE,
        &executes.join(LIST_SEPARATOR),
    )?;
    set_field(
        read_model,
        field::STAGES_TO_SKIP,
        &if skips.is_empty() {
            NONE_LITERAL.to_string()
        } else {
            skips.join(LIST_SEPARATOR)
        },
    )?;
    set_field(read_model, field::TOTAL_STAGES, &total)
}

/// Skip 行の 1 項目から slug を取り出す。
///
/// 項目は `<番号> (<slug>)`、注釈付きなら `<番号> (<slug> — <理由>)`。括弧の中を取り、
/// em dash があればその手前までが slug である（upstream の `slugOfSkipToken` と同じ規則）。
/// 形に合わない項目は丸ごと slug として扱い、既知の slug に一致しないので保存されない。
fn slug_of_skip_token(token: &str) -> &str {
    let inner = token
        .split_once(" (")
        .and_then(|(_, rest)| rest.strip_suffix(')'))
        .unwrap_or(token);
    inner.split(SKIP_ANNOTATION).next().unwrap_or(inner)
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
///
/// `**Stages completed**:` は**呼出側が決める**。genesis は計画上の initialization 件数
/// （まだ 1 つも倒れていない時点で描くため）、ゲート承認は倒したあとのチェックボックスの
/// 数え直しで、値が一致しない（`cli/report/approved-across-phases` は 2、計画上の inception 内
/// スコープ件数は 8）。ジャンプの境界 3 行は `**Details**:` を持つので別関数である。
fn append_phase_boundary(
    read_model: &mut ReadModel,
    at: &DateTime<Utc>,
    plan: &ResolvedPlan,
    boundary: PhaseBoundary,
    stages_completed: &str,
) -> Result<(), ProjectionError> {
    let from = boundary.from_phase();
    let to = boundary.to_phase();
    read_model.append_audit(&render_audit_block(
        EventType::PhaseCompleted,
        at,
        &AuditFields::new()
            .with(key(key::FROM_PHASE)?, from.as_str())
            .with(key(key::TO_PHASE)?, to.as_str())
            .with(key(key::STAGES_COMPLETED)?, stages_completed),
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

/// 通過したフェーズを `Verified`、入ったフェーズを `Active` にする
/// （`cli/report/approved-across-phases`）。
///
/// [`append_phase_boundary`] とは分けてある — あちらは genesis の**監査行だけを描く**段からも
/// 呼ばれ、その時点では状態ファイルの本文がまだ無いことがあるからである。genesis 自身は
/// 骨格が既にこの 2 行を正しい値で持っているので、書き換える必要がない。
fn set_phase_progress_for_advance(
    read_model: &mut ReadModel,
    boundary: PhaseBoundary,
) -> Result<(), ProjectionError> {
    set_field(
        read_model,
        &field::phase_row(boundary.from_phase()),
        phase_status::VERIFIED,
    )?;
    set_field(
        read_model,
        &field::phase_row(boundary.to_phase()),
        phase_status::ACTIVE,
    )
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
    // 値は大文字。同じフェーズへ移るケースでは書き換えても値が変わらないので、フェーズを
    // またぐジャンプ (`cli/jump/execute-backward` ほか) だけがこの行の差分を見せる。
    set_field(
        read_model,
        field::LIFECYCLE_PHASE,
        &stage.phase().as_str().to_uppercase(),
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
    set_field(read_model, field::COMPLETED, &completed_count(read_model))?;
    set_field(read_model, field::LAST_COMPLETED_STAGE, slug.as_str())
}

/// いま `[x]` のチェックボックスの数（`- **Completed**:` と境界行の材料）。
fn completed_count(read_model: &ReadModel) -> String {
    Checkboxes::parse(read_model.state())
        .count_completed()
        .to_string()
}

/// リードモデルの行末トークンから実効プランを読む (無ければ静的計画の値)。
///
/// イベントは事実だけを運ぶ (オーナー裁定 2026-08-30) ので、次カーソル・in-scope 数の材料は
/// **リードモデル自身**である — `rebuild_plan_rows` と同じ読み方 (状態の正本は自分の行)。
fn effective_action(read_model: &ReadModel, stage: &PlannedStage) -> PlanAction {
    Checkboxes::parse(read_model.state())
        .iter()
        .find(|entry| entry.slug() == stage.slug().as_str())
        .and_then(|entry| match entry.rest().trim() {
            "EXECUTE" => Some(PlanAction::Execute),
            "SKIP" => Some(PlanAction::Skip),
            _ => None,
        })
        .unwrap_or_else(|| stage.plan_action())
}

/// 名指しステージの後で実効 EXECUTE の最初のステージ (無ければ `None` = ワークフロー完了)。
fn next_in_effective_scope(
    read_model: &ReadModel,
    plan: &ResolvedPlan,
    after: &StageSlug,
) -> Option<StageSlug> {
    plan.stages()
        .iter()
        .skip_while(|stage| stage.slug() != after)
        .skip(1)
        .find(|stage| effective_action(read_model, stage) == PlanAction::Execute)
        .map(|stage| stage.slug().clone())
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

/// フェーズの文書順の位置。
///
/// `PhaseId::ALL` を `position` で引くと「見つからない」枝が生まれるが、閉集合なので実際には
/// 起きない。網羅 `match` にすればその枝が消え、フェーズを増やしたときは**コンパイルエラー**で
/// ここを直すよう強制できる（順序が `PhaseId::ALL` と一致することは単体テストが見張る）。
const fn phase_order(phase: PhaseId) -> usize {
    match phase {
        PhaseId::Initialization => 0,
        PhaseId::Ideation => 1,
        PhaseId::Inception => 2,
        PhaseId::Construction => 3,
        PhaseId::Operation => 4,
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
    let next = Checkboxes::with_marker(read_model.state(), slug, state)?;
    read_model.replace_state(next);
    Ok(())
}

/// チェックボックスの行末トークンだけを書き換える（マーカーには触れない）。
fn set_suffix(
    read_model: &mut ReadModel,
    slug: &str,
    action: PlanAction,
) -> Result<(), ProjectionError> {
    let next = Checkboxes::with_suffix(read_model.state(), slug, action)?;
    read_model.replace_state(next);
    Ok(())
}

/// 状態ファイルのフィールド行を書き換える（不在は拒否 — 無言 no-op は検出不能なドリフト）。
fn set_field(read_model: &mut ReadModel, field: &str, value: &str) -> Result<(), ProjectionError> {
    let next = with_field(read_model.state(), field, value)?;
    read_model.replace_state(next);
    Ok(())
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
    use core_command_domain::orchestration::Created;
    use core_command_domain::orchestration::{
        AutonomyModeSet, Intent, IntentExecutionId, IntentId, StageDisplay, StageEntry,
        StartRequest, Started, WorkspaceScan,
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
        Started::new(Intent::from(Created::new(
            IntentId::parse("01a02785-1bd8-76eb-aeea-5aa303ebd5b6").expect("UUIDv7"),
            WorkflowDefinitionId::parse("claude").expect("定義 id"),
            DefinitionRevision::parse(&format!("sha256:{}", "0".repeat(64))).expect("revision"),
            StartRequest::new("classic", "build it"),
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
        )))
    }

    fn plan() -> ResolvedPlan {
        ResolvedPlan::of(&started())
    }

    const SKELETON: &str = "\
## Project Information
- **Active Agent**: orchestrator

## Scope Configuration
- **Stages to Execute**: 0.1, 2.1, 2.2
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

## Phase Progress
- **Initialization**: Active
- **Ideation**: Pending
- **Inception**: Pending
- **Construction**: Pending
- **Operation**: Pending

## Current Status
- **Lifecycle Phase**: INITIALIZATION
- **Current Stage**: state-init
- **Next Stage**: first

## Session Resume Point
- **Last Completed Stage**: 
- **Next Action**: Execute Stage
";

    fn model() -> ReadModel {
        ReadModel::new(SKELETON)
    }

    fn entry(event: IntentExecutionEvent) -> JournalEntry {
        JournalEntry::new(
            crate::orchestration::GlobalSeqNr::new(1),
            IntentExecutionId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000").expect("UUIDv7"),
            1,
            at(),
            event,
        )
    }

    fn run(event: IntentExecutionEvent) -> ReadModel {
        let mut read_model = model();
        project(&[entry(event)], &plan(), &mut read_model).expect("投影");
        read_model
    }

    #[test]
    fn the_phase_order_agrees_with_the_declared_document_order() {
        // 網羅 match で書いた順序が `PhaseId::ALL` とずれていないことを見張る
        // （ずれると `## Phase Progress` の前方 / 後方の判定が逆になる）。
        for (index, phase) in PhaseId::ALL.into_iter().enumerate() {
            assert_eq!(phase_order(phase), index, "{}", phase.as_str());
        }
    }

    #[test]
    fn the_genesis_lands_on_the_first_gated_stage_when_the_skeleton_exists() {
        let read_model = run(IntentExecutionEvent::Started(started()));
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
            &[entry(IntentExecutionEvent::Started(started()))],
            &plan(),
            &mut read_model,
        )
        .expect_err("骨格が無い");
        assert_eq!(error, ProjectionError::ScaffoldMissing);
        assert_eq!(error.to_string(), "scaffold missing");
        // 監査行だけは描けている（骨格が無くても台帳は書ける）。
        assert!(
            read_model
                .appended_audit()
                .contains("**Event**: WORKFLOW_STARTED")
        );
    }

    #[test]
    fn switching_the_autonomy_mode_writes_the_row_and_the_field() {
        let read_model = run(IntentExecutionEvent::AutonomyModeSet(AutonomyModeSet::new(
            AutonomyMode::Autonomous,
        )));
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
        // 次は導出 — second の後の実効 EXECUTE は無い (late は SKIP) ので完了行になる。
        let read_model = run(IntentExecutionEvent::GateApproved(GateApproved::new(
            slug("second"),
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
        // 境界はイベントに載らない — state-init (initialization) の次の実効 EXECUTE が
        // first (inception) なので、計画からの導出で境界が立つ。
        let read_model = run(IntentExecutionEvent::GateApproved(GateApproved::new(
            slug("state-init"),
            Some("A".to_string()),
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
                .contains("**Phase boundary**: initialization → inception\n")
        );
        // 数え直しである — 承認で `state-init` が `[x]` になった時点の 1 本。
        assert!(
            read_model
                .appended_audit()
                .contains("**Stages completed**: 1\n")
        );
    }

    #[test]
    fn completing_a_non_gated_stage_uses_the_completed_wording() {
        let read_model = run(IntentExecutionEvent::StageCompleted(StageCompleted::new(
            slug("state-init"),
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
        // 出発点はイベントに載らない — 自分の `Current Stage` 行から導く。second で稼働中の
        // 状態を作り、first へ跳ぶ。
        let mut read_model = ReadModel::new(
            SKELETON
                .replace(
                    "- **Current Stage**: state-init",
                    "- **Current Stage**: second",
                )
                .replace("- [-] state-init — EXECUTE", "- [x] state-init — EXECUTE")
                .replace("- [ ] first — EXECUTE", "- [x] first — EXECUTE")
                .replace("- [ ] second — EXECUTE", "- [-] second — EXECUTE"),
        );
        project(
            &[entry(IntentExecutionEvent::Jumped(Jumped::new(slug(
                "first",
            ))))],
            &plan(),
            &mut read_model,
        )
        .expect("投影");
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
    fn skipping_the_last_effective_stage_completes_the_workflow_row() {
        // 導出 leave_for の None 腕 — second を skip すると次の実効 EXECUTE は無い
        // (late は SKIP) ので WORKFLOW_COMPLETED 行になる。
        let mut read_model = ReadModel::new(
            SKELETON
                .replace(
                    "- **Current Stage**: state-init",
                    "- **Current Stage**: second",
                )
                .replace("- [-] state-init — EXECUTE", "- [x] state-init — EXECUTE")
                .replace("- [ ] first — EXECUTE", "- [x] first — EXECUTE")
                .replace("- [ ] second — EXECUTE", "- [-] second — EXECUTE"),
        );
        project(
            &[entry(IntentExecutionEvent::StageSkipped(
                StageSkipped::new(slug("second"), "not needed".to_string()),
            ))],
            &plan(),
            &mut read_model,
        )
        .expect("投影");
        assert!(
            read_model
                .appended_audit()
                .contains("**Event**: WORKFLOW_COMPLETED")
        );
        assert!(read_model.state().contains("- [S] second — EXECUTE"));
    }

    #[test]
    fn a_forward_jump_emits_skips_in_plan_order_with_the_source_last() {
        // upstream の emit 順 — 中間 (計画順) → 最後に出発点。実効 SKIP の介在 (late) は
        // 触らない (実バイトが正本)。first で稼働中の状態から second を跨いで…は 4 ステージ
        // 構成では作れないので、state-init 稼働中から second へ跳ぶ。
        let mut read_model = ReadModel::new(SKELETON.to_string());
        project(
            &[entry(IntentExecutionEvent::Jumped(Jumped::new(slug(
                "second",
            ))))],
            &plan(),
            &mut read_model,
        )
        .expect("投影");
        let skipped: Vec<&str> = read_model
            .appended_audit()
            .lines()
            .filter_map(|line| line.strip_prefix("**Stage**: "))
            .collect();
        // STAGE_SKIPPED は first (中間)、state-init (出発点) の順。STAGE_STARTED の
        // second が最後に混ざるので先頭 2 つを見る。
        assert_eq!(skipped.first(), Some(&"first"), "中間が先");
        assert_eq!(skipped.get(1), Some(&"state-init"), "出発点が最後");
        assert!(read_model.state().contains("- [S] state-init — EXECUTE"));
        assert!(read_model.state().contains("- [S] first — EXECUTE"));
        assert!(read_model.state().contains("- [-] second — EXECUTE"));
        assert!(
            read_model.state().contains("- [ ] late — SKIP"),
            "実効 SKIP の行は触らない"
        );
    }

    #[test]
    fn a_non_numeric_revision_count_is_coerced_to_zero_before_the_bump() {
        // upstream getField + 1 と同じ防御 — 非数値 (手編集・欠落) は 0 に畳んでから +1。
        let mut read_model = ReadModel::new(
            SKELETON.replace("- **Revision Count**: 0", "- **Revision Count**: abc"),
        );
        project(
            &[entry(IntentExecutionEvent::GateRejected(
                GateRejected::new(slug("state-init"), None),
            ))],
            &plan(),
            &mut read_model,
        )
        .expect("投影");
        assert!(read_model.state().contains("- **Revision Count**: 1\n"));
    }

    #[test]
    fn a_second_rejection_bumps_the_read_model_counter_again() {
        // read-modify-write の連続性 — 現値 1 の状態からの差し戻しは 2 を書く
        // (upstream getField + 1 と同じ)。
        let mut read_model =
            ReadModel::new(SKELETON.replace("- **Revision Count**: 0", "- **Revision Count**: 1"));
        project(
            &[entry(IntentExecutionEvent::GateRejected(
                GateRejected::new(slug("state-init"), None),
            ))],
            &plan(),
            &mut read_model,
        )
        .expect("投影");
        assert!(read_model.state().contains("- **Revision Count**: 2\n"));
    }

    #[test]
    fn an_unknown_plan_suffix_token_falls_back_to_the_static_plan() {
        // 行末トークンが閉集合外なら静的計画の値で読む (次の導出が止まらない)。
        let mut read_model =
            ReadModel::new(SKELETON.replace("- [ ] first — EXECUTE", "- [ ] first — WHAT"));
        project(
            &[entry(IntentExecutionEvent::StageCompleted(
                StageCompleted::new(slug("state-init")),
            ))],
            &plan(),
            &mut read_model,
        )
        .expect("投影");
        // first の静的計画は EXECUTE — 次の開始先として選ばれる。
        assert!(read_model.state().contains("- [-] first — WHAT"));
    }

    #[test]
    fn a_redo_jump_reopens_the_current_stage_without_touching_neighbours() {
        // 到達点 = 現在地 (redo)。checkbox の書き換えは到達点の [-] 化だけで、隣は触らない。
        let read_model = run(IntentExecutionEvent::Jumped(Jumped::new(slug(
            "state-init",
        ))));
        assert!(
            read_model
                .appended_audit()
                .contains("**Direction**: REDO\n")
        );
        assert!(read_model.appended_audit().contains(
            "**Details**: REDO jump from state-init to state-init (0.1). Scope: classic.\n"
        ));
        assert!(read_model.state().contains("- [-] state-init — EXECUTE"));
        assert!(read_model.state().contains("- [ ] first — EXECUTE"));
    }

    #[test]
    fn a_jump_with_a_broken_current_stage_row_is_refused() {
        // 出発点の導出元 (`Current Stage` 行) が壊れていれば、読み替えずに止める (fail-closed)。
        let mut read_model = ReadModel::new(SKELETON.replace(
            "- **Current Stage**: state-init",
            "- **Current Stage**: NOT A SLUG",
        ));
        let error = project(
            &[entry(IntentExecutionEvent::Jumped(Jumped::new(slug(
                "first",
            ))))],
            &plan(),
            &mut read_model,
        )
        .expect_err("出発点が導けない");
        assert_eq!(error.to_string(), "unknown stage: NOT A SLUG");
    }

    #[test]
    fn a_rejection_without_feedback_omits_the_feedback_rows() {
        // feedback 無しの差し戻し — 行は出ず、Revision Count は現値 +1 (0 → 1)。
        let read_model = run(IntentExecutionEvent::GateRejected(GateRejected::new(
            slug("state-init"),
            None,
        )));
        assert!(!read_model.appended_audit().contains("**Feedback**"));
        assert!(
            read_model
                .appended_audit()
                .contains("**Revision count**: 1\n")
        );
        assert!(read_model.state().contains("- **Revision Count**: 1\n"));
        assert!(read_model.state().contains("- [R] state-init — EXECUTE"));
    }

    #[test]
    fn recomposing_back_into_scope_moves_the_entry_the_other_way() {
        // 適用後の in-scope 数は行末トークンの反転後に自分の行から導く (= 4)。
        let read_model = run(IntentExecutionEvent::Recomposed(Recomposed::new(
            Vec::new(),
            vec![slug("late")],
        )));
        // Execute 行は graph 順に組み直される（4.1 は末尾で、ここでは順序が変わらない）。
        assert!(
            read_model
                .state()
                .contains("- **Stages to Execute**: 0.1, 2.1, 2.2, 4.1\n"),
            "実際: {}",
            read_model.state()
        );
        // 空になった Skip 行は upstream と同じ逐語 `none` を書く。
        assert!(read_model.state().contains("- **Stages to Skip**: none\n"));
        assert!(read_model.state().contains("- [ ] late — EXECUTE"));
        assert!(read_model.state().contains("- **Total Stages**: 4\n"));
        assert!(
            read_model
                .appended_audit()
                .contains("**Stages skipped**: none\n")
        );
    }

    #[test]
    fn a_missing_list_field_is_refused_with_the_verbatim_wording() {
        let mut read_model = ReadModel::new("## Empty\n");
        let error = rebuild_plan_rows(&mut read_model, &plan()).expect_err("一覧フィールドが無い");
        assert_eq!(
            error.to_string(),
            "state field: Field not found in state file: \"Stages to Skip\". \
             Cannot update — refusing to silently no-op."
        );
    }

    #[test]
    fn the_skip_token_parser_keeps_the_slug_and_drops_the_annotation() {
        // 注釈付き項目から slug を取れないと、EXECUTE へ戻した段でその項目が落ちずに残る。
        assert_eq!(
            slug_of_skip_token("4.5 (incident-response)"),
            "incident-response"
        );
        assert_eq!(
            slug_of_skip_token("2.1 (reverse-engineering — greenfield)"),
            "reverse-engineering"
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
