//! `WorkflowExecution` 集約 — 1 つの Intent の実行状態 (10 §2.1) をイベントソーシング形の
//! FSM として持つ集約ルート (ADR-001 / ADR-002)。
//!
//! **状態としてのデータ**(カーソル・CheckboxState・`Status` と直交する park マーカー・recompose
//! オーバレイ・AutonomyMode・ゲート承認履歴・差し戻し回数)、**状態遷移**(12 の decide コマンド)、
//! **判断**(`next_decision` / `jump_resolve` / `stale_report`) を 1 つの型に閉じ込める。
//!
//! - **1 コマンド 1 イベント** (BR1.1): 各 decide はガードを全て通してからイベントを 1 つ構築し、
//!   `apply_event` で自身に適用して返す。ガード不成立の `Err` では `self` に触れない。
//! - **通常実行とリプレイは同一経路** (BR2.3): 状態を動かすのは `apply_event` だけであり、decide は
//!   「どのイベントを起こすか」を決めるだけである。
//! - **ゲート判定はフェーズ**で決まる (BR1.3): `gated(s) = stages[s].phase != initialization`。
//!   索引 0 の特別扱いはしない (実グラフの initialization は 3 ステージある)。Quint slice-1 の
//!   `gated(s) = s != 0` は initialization 1 ステージの合成計画に対する抽象で、ITF 準拠テストは
//!   その合成計画で駆動する (BR2.5)。
//! - **時計を持たない** (NFR3.1): `occurred_at` は呼出側 (ユースケース) が Clock から渡す。
//! - **楽観 version は持たない** (ADR-010 / B7): 本家 event-store-adapter-rs v3.0.0 で
//!   `Aggregate` trait が廃れ、楽観ロックの版数は `SnapshotEnvelope::version()` (ストアの列) が
//!   正本になった。集約が持つ順序番号は `seq_nr` **だけ**であり、ストアの採番トークンとは混ざらない。
//!   serde 境界はスナップショットの直列化に要るが、**復号は状態の写し (memento) を経由する** —
//!   `into` / `try_from` で [`WorkflowExecutionState`] に委ね、`from_state()` の検査点を必ず通す
//!   (オーナー裁定 2026-08-27 (A))。したがって「不変条件を満たす集約しか存在しない」という
//!   保証は serde 経路でも破れない (security-design §2 の検査点 3)。
//! - **panic しない** (NFR4.3): ステージ位置は `StageIndex` で型保証し、範囲外は `Option::None` /
//!   `Err` で表す。`# Panics` を持つ公開 API は無い。
//!
//! 意味論の形式的正本は `formal/orchestration/engine_loop.qnt` (slice 1 v2)。ITF 準拠テスト
//! (`tests/engine_loop_conformance.rs`) がモデルトレースを再生して射影を突き合わせる。

use std::collections::BTreeSet;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::apply_error::ApplyError;
use super::autonomy_mode::AutonomyMode;
use super::command_error::CommandError;
use super::intent_id::IntentId;
use super::jump_direction::JumpDirection;
use super::next_decision::{NextDecision, NextRequest};
use super::phase_boundary::PhaseBoundary;
use super::stage_entry::StageEntry;
use super::stage_index::StageIndex;
use super::start_error::StartError;
use super::start_request::StartRequest;
use super::state_error::StateError;
use super::status::Status;
use super::workflow_execution_event::{
    AutonomyModeSet, GateApproved, GateOpened, GateRejected, Jumped, Parked, Recomposed,
    StageCompleted, StageRevised, StageSkipped, Started, WorkflowExecutionEvent,
};
use super::workflow_execution_state::WorkflowExecutionState;
use crate::workflow_definition::{
    DefinitionRevision, ExecutionKind, PhaseId, PlanAction, StageSlug, UnknownScope,
    WorkflowDefinition, WorkflowDefinitionId,
};
use crate::workspace::CheckboxState;

/// 前進 (`complete_stage` / `approve_gate`) と差し戻し (`reject_gate`) が受理する checkbox 集合。
///
/// これは**本集約が所有する遷移の前提集合** (I7 ゲート前提) であって、`CheckboxState` の一般分類
/// (in-flight / finished / active) ではない (tell-dont-ask.md「集約所有の前提集合」)。
// amadeus-lint: allow(checkbox-vocabulary) — I7: 集約が所有するゲート遷移の前提集合
const GATE_ADVANCE_PRECONDITION: [CheckboxState; 2] =
    [CheckboxState::InProgress, CheckboxState::AwaitingApproval];

/// `skip_stage` が受理する checkbox 集合 (I13 skipped 受理前提)。
///
/// 同じ集合が「実効 SKIP のカーソルから自力で復旧できるか」の判定にもなる — 復旧手段が
/// `skip_stage` そのものだからである (BR3.1 (5) の `RecoverSkipInconsistency`)。
// amadeus-lint: allow(checkbox-vocabulary) — I13: 集約が所有する skip 受理の前提集合
const SKIP_PRECONDITION: [CheckboxState; 2] = [CheckboxState::InProgress, CheckboxState::Revising];

/// エンジンループの状態機械 (集約ルート)。
///
/// serde は状態の写し ([`WorkflowExecutionState`]) を経由する — 直列化は [`WorkflowExecution::state`]、
/// 復号は [`WorkflowExecution::from_state`] であり、復号側の検査点が 1 か所に保たれる。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(into = "WorkflowExecutionState", try_from = "WorkflowExecutionState")]
pub struct WorkflowExecution {
    intent_id: IntentId,
    definition_id: WorkflowDefinitionId,
    definition_revision: DefinitionRevision,
    /// 文書順の解決済み計画。`stages` / `plan` / `conditional` の 3 属性をこの 1 列が担う
    /// (`plan` = `plan_action()`、`conditional` = `is_conditional()`) — 状態の写し (memento) は
    /// C6 の列構成に合わせて 3 列へ展開し、`from_state` が整合を検査する。
    stages: Vec<StageEntry>,
    overlay: Vec<PlanAction>,
    checkbox: Vec<CheckboxState>,
    cursor: StageIndex,
    status: Status,
    parked_at: Option<StageIndex>,
    autonomy: AutonomyMode,
    approved: Vec<bool>,
    revision_count: Vec<u32>,
    seq_nr: usize,
    /// 最後に適用したイベントの発生時刻。集約は時計を持たないので、この値は常に適用した
    /// イベントから来る (NFR3.1)。Repository はこれをイベント封筒の `occurred_at` に使う。
    last_updated_at: DateTime<Utc>,
}

impl WorkflowExecution {
    // ---- W1: 生成 (BR2.2 / BR2.6) ----

    /// 定義と呼出側の要求から解決済み計画を組み立てて実行を開始する。
    ///
    /// `def.id()` / `def.revision()` は**無条件に `Started` へ記録する** — 比較対象となる既存状態が
    /// 無い静的コンストラクタなので検査はしない (BR2.6)。以後の定義照合は `next_decision` が行う。
    /// [`StartRequest`] の `depth` / `test_strategy` は集約状態にはならず、`Started` へ素通しで
    /// 載るだけである (U4 の `Scope Configuration` 投影材料 — C5)。
    ///
    /// # Errors
    ///
    /// 未知スコープ (`UnknownScope`)、ステージ 0 件 (`Empty`)、initialization ステージが SKIP に
    /// 畳まれた / 先頭ステージがスコープ外 (`InitializationMustExecute`)、initialization ステージが
    /// CONDITIONAL (`InitializationMustBeUnconditional`) を拒否する。
    pub fn start(
        intent_id: IntentId,
        definition: &WorkflowDefinition,
        request: &StartRequest,
        occurred_at: DateTime<Utc>,
    ) -> Result<(WorkflowExecution, WorkflowExecutionEvent), StartError> {
        let scope = request.scope();
        if !definition.is_valid_scope(scope) {
            let valid = definition
                .valid_scopes()
                .into_iter()
                .map(str::to_string)
                .collect();
            return Err(StartError::UnknownScope(UnknownScope::new(scope, valid)));
        }
        let nodes = definition.graph().nodes();
        let stages = definition
            .stages_in_scope(scope)
            .into_iter()
            .enumerate()
            .map(|(index, (slug, phase, action))| {
                // `stages_in_scope` は execution を返さないので、同じ文書順のノード列から索引一致で
                // CONDITIONAL を拾う (BR2.2)。グリッド列が無いステージは `None → SKIP` に畳む。
                let conditional = nodes
                    .get(index)
                    .is_some_and(|node| node.execution() == ExecutionKind::Conditional);
                StageEntry::new(
                    slug.clone(),
                    phase,
                    action.unwrap_or(PlanAction::Skip),
                    conditional,
                )
            })
            .collect();
        WorkflowExecution::start_from_plan_unchecked(
            intent_id,
            definition.id().clone(),
            definition.revision().clone(),
            request,
            stages,
            occurred_at,
        )
    }

    /// 解決済み計画を直接与えて実行を開始する ([`WorkflowExecution::start`] の委譲先)。
    ///
    /// 定義を組み立てずに合成計画で駆動する ITF 準拠テストの入口でもある (BR2.5)。
    ///
    /// **`start` と違い [`StartError::UnknownScope`] を返せない** — 照合すべき
    /// [`WorkflowDefinition`] を受け取らないため、スコープ名が定義にあるかどうかを検査する材料が
    /// そもそも無い。名前の `_unchecked` はこの検査の欠落を指す。定義を持っている呼出側は
    /// [`WorkflowExecution::start`] を使うこと。
    ///
    /// # Errors
    ///
    /// ステージ 0 件、initialization ステージの SKIP / CONDITIONAL、先頭ステージのスコープ外を
    /// 拒否する (先頭はカーソルの初期位置なので実効 EXECUTE でなければ `cursor_in_scope` を破る)。
    /// スコープ名の妥当性 (`UnknownScope`) は上記のとおり検査しない。
    pub fn start_from_plan_unchecked(
        intent_id: IntentId,
        definition_id: WorkflowDefinitionId,
        definition_revision: DefinitionRevision,
        request: &StartRequest,
        stages: Vec<StageEntry>,
        occurred_at: DateTime<Utc>,
    ) -> Result<(WorkflowExecution, WorkflowExecutionEvent), StartError> {
        match stages.first() {
            None => return Err(StartError::Empty),
            Some(first) if first.plan_action() != PlanAction::Execute => {
                return Err(StartError::InitializationMustExecute);
            }
            Some(_) => {}
        }
        for entry in &stages {
            if entry.phase() != PhaseId::Initialization {
                continue;
            }
            if entry.plan_action() != PlanAction::Execute {
                return Err(StartError::InitializationMustExecute);
            }
            if entry.is_conditional() {
                return Err(StartError::InitializationMustBeUnconditional);
            }
        }

        let count = stages.len();
        let overlay: Vec<PlanAction> = stages.iter().map(StageEntry::plan_action).collect();
        let mut checkbox = vec![CheckboxState::Pending; count];
        if let Some(first) = checkbox.first_mut() {
            *first = CheckboxState::InProgress;
        }
        let event = WorkflowExecutionEvent::Started(Started::new(
            definition_id.clone(),
            definition_revision.clone(),
            request,
            stages.clone(),
        ));
        let execution = WorkflowExecution {
            intent_id,
            definition_id,
            definition_revision,
            stages,
            overlay,
            checkbox,
            cursor: StageIndex::new(0),
            status: Status::Running,
            parked_at: None,
            autonomy: AutonomyMode::Gated,
            approved: vec![false; count],
            revision_count: vec![0; count],
            seq_nr: 1,
            last_updated_at: occurred_at,
        };
        Ok((execution, event))
    }

    // ---- 観測 (read model) ----

    /// この実行が属する Intent の識別子 (以後不変)。
    #[must_use]
    pub const fn intent_id(&self) -> &IntentId {
        &self.intent_id
    }

    /// 適用済みイベント数と一致する順序番号 (`Started` = 1 — BR2.1)。
    ///
    /// 次のイベントの通番は `seq_nr + 1` であり、`commit` を通ったあとの値は**そのイベントの
    /// 通番そのもの**である。封筒を組む Repository はこの値を使う。
    #[must_use]
    pub const fn seq_nr(&self) -> usize {
        self.seq_nr
    }

    /// 最後に適用したイベントの発生時刻 (集約は時計を持たない — NFR3.1)。
    ///
    /// `commit` を通ったあとの値は**そのイベントの発生時刻**であり、封筒の `occurred_at` になる。
    #[must_use]
    pub const fn last_updated_at(&self) -> &DateTime<Utc> {
        &self.last_updated_at
    }

    /// `Started` に記録した定義の系譜 ID (以後不変 — BR2.6)。
    #[must_use]
    pub const fn definition_id(&self) -> &WorkflowDefinitionId {
        &self.definition_id
    }

    /// `Started` 時点の定義の内容版 (来歴。定義側が進んでも Err にはしない — BR2.6)。
    #[must_use]
    pub const fn definition_revision(&self) -> &DefinitionRevision {
        &self.definition_revision
    }

    /// 文書順の解決済み計画 (`Started` が確定させて以後不変)。
    #[must_use]
    pub fn stages(&self) -> &[StageEntry] {
        &self.stages
    }

    /// コンパイル済みグラフのステージ総数 (スコープ外のステージも含む)。
    #[must_use]
    pub const fn stage_count(&self) -> usize {
        self.stages.len()
    }

    /// 生の位置から `StageIndex` を作る唯一の公開経路。範囲外は `None` (BR5.1)。
    #[must_use]
    pub fn stage_index(&self, value: usize) -> Option<StageIndex> {
        (value < self.stage_count()).then(|| StageIndex::new(value))
    }

    /// `Current Stage` の位置。
    #[must_use]
    pub const fn cursor(&self) -> StageIndex {
        self.cursor
    }

    /// `Status` 行の現在値 (park マーカーとは直交)。
    #[must_use]
    pub const fn status(&self) -> Status {
        self.status
    }

    /// park マーカーが記録している位置 (`None` は未 park)。
    #[must_use]
    pub const fn parked_at(&self) -> Option<StageIndex> {
        self.parked_at
    }

    /// 現在の `Construction Autonomy Mode`。
    #[must_use]
    pub const fn autonomy(&self) -> AutonomyMode {
        self.autonomy
    }

    /// 名指しステージの checkbox マーカー。範囲外は `None`。
    #[must_use]
    pub fn checkbox(&self, stage: StageIndex) -> Option<CheckboxState> {
        self.checkbox.get(stage.to_usize()).copied()
    }

    /// 名指しステージのゲート承認履歴。範囲外は `None`。
    #[must_use]
    pub fn approved(&self, stage: StageIndex) -> Option<bool> {
        self.approved.get(stage.to_usize()).copied()
    }

    /// 名指しステージの差し戻し回数。範囲外は `None`。
    #[must_use]
    pub fn revision_count(&self, stage: StageIndex) -> Option<u32> {
        self.revision_count.get(stage.to_usize()).copied()
    }

    /// 実効プラン — オーバレイ (recompose) が静的グリッドに勝つ (BR4.2)。範囲外は `None`。
    #[must_use]
    pub fn effective_plan(&self, stage: StageIndex) -> Option<PlanAction> {
        self.overlay.get(stage.to_usize()).copied()
    }

    /// ゲート付きか — `phase != initialization` (BR1.3)。範囲外は `None`。
    #[must_use]
    pub fn gated(&self, stage: StageIndex) -> Option<bool> {
        self.stages.get(stage.to_usize()).map(StageEntry::is_gated)
    }

    /// parked 分岐の発火は導出述語 (マーカー有 ∧ 位置一致 — BR1.7)。
    #[must_use]
    pub fn parked_active(&self) -> bool {
        self.parked_at == Some(self.cursor)
    }

    /// コマンド受理述語 (BR1.0)。偽なら `unpark` 以外の decide は `NotRunning`。
    #[must_use]
    pub fn accepts_commands(&self) -> bool {
        self.status.is_running() && !self.parked_active()
    }

    // ---- 内部の索引ヘルパ (すべて `StageIndex` 経由 — 生の添字を使わない) ----

    fn entry(&self, stage: StageIndex) -> Option<&StageEntry> {
        self.stages.get(stage.to_usize())
    }

    fn is_gated(&self, stage: StageIndex) -> bool {
        self.entry(stage).is_some_and(StageEntry::is_gated)
    }

    fn in_scope(&self, stage: StageIndex) -> bool {
        self.effective_plan(stage) == Some(PlanAction::Execute)
    }

    fn next_in_scope(&self, after: StageIndex) -> Option<StageIndex> {
        ((after.to_usize() + 1)..self.stage_count())
            .map(StageIndex::new)
            .find(|&stage| self.in_scope(stage))
    }

    fn next_in_scope_slug(&self, after: StageIndex) -> Option<StageSlug> {
        self.next_in_scope(after)
            .and_then(|stage| self.entry(stage))
            .map(|entry| entry.slug().clone())
    }

    fn slug_of(&self, stage: StageIndex) -> Result<StageSlug, CommandError> {
        self.entry(stage)
            .map(|entry| entry.slug().clone())
            .ok_or(CommandError::InvalidTarget(stage))
    }

    fn resolve(&self, slug: &StageSlug) -> Result<StageIndex, ApplyError> {
        self.stages
            .iter()
            .position(|entry| entry.slug() == slug)
            .map(StageIndex::new)
            .ok_or_else(|| ApplyError::UnknownStage(slug.clone()))
    }

    /// ステージに状態の印を付ける (状態ファイルのチェックボックスがこの印の表現)。
    fn mark_stage(&mut self, stage: StageIndex, value: CheckboxState) {
        if let Some(slot) = self.checkbox.get_mut(stage.to_usize()) {
            *slot = value;
        }
    }

    /// ステージの承認を記録する (`GateApproved` の適用)。
    fn record_approval(&mut self, stage: StageIndex) {
        if let Some(slot) = self.approved.get_mut(stage.to_usize()) {
            *slot = true;
        }
    }

    /// ステージの承認履歴を無効化する (BR1.6 — jump が承認を巻き戻す)。
    fn invalidate_approval(&mut self, stage: StageIndex) {
        if let Some(slot) = self.approved.get_mut(stage.to_usize()) {
            *slot = false;
        }
    }

    // ---- ガード ----

    /// BR1.0 — コマンドを受理できる状態か検査し、カーソルを返す。
    fn guard_running(&self) -> Result<StageIndex, CommandError> {
        if self.accepts_commands() {
            Ok(self.cursor)
        } else {
            Err(CommandError::NotRunning)
        }
    }

    fn require_checkbox(
        &self,
        stage: StageIndex,
        allowed: &[CheckboxState],
    ) -> Result<CheckboxState, CommandError> {
        let actual = self
            .checkbox(stage)
            .ok_or(CommandError::InvalidTarget(stage))?;
        if allowed.contains(&actual) {
            Ok(actual)
        } else {
            Err(CommandError::CheckboxPrecondition { stage, actual })
        }
    }

    fn require_gated(&self, stage: StageIndex, gated: bool) -> Result<(), CommandError> {
        if self.is_gated(stage) == gated {
            Ok(())
        } else {
            Err(CommandError::InvalidTarget(stage))
        }
    }

    /// ガードを通過したイベントを自身に適用して返す (BR1.1)。
    ///
    /// 通番と発生時刻は**適用の引数**であって、イベントに載る材料ではない (ADR-010 / B7 —
    /// 輸送のメタデータは封筒が運ぶ)。適用が通れば `self.seq_nr` がそのイベントの通番、
    /// `self.last_updated_at` がその発生時刻になるので、封筒を組む Repository はそこから読む。
    ///
    /// `apply_event` は通番違反・未知 slug・不変条件違反を検査するが、ここへ来るイベントは通番を
    /// 自分で採番し slug を自分の `stages` から取り、遷移も不変条件を保つので、そのいずれにも
    /// 該当しない。到達不能な `Err` は状態を変えないまま `InvalidTarget(cursor)` として返す —
    /// panic しないことを優先する (NFR4.3)。通番枯渇だけは入口で明示に拒否する
    /// (`SequenceExhausted` — 飽和加算で seq_nr が停滞したまま成功を装わない)。
    fn commit(
        &mut self,
        event: WorkflowExecutionEvent,
        occurred_at: DateTime<Utc>,
    ) -> Result<WorkflowExecutionEvent, CommandError> {
        let Some(seq_nr) = self.seq_nr.checked_add(1) else {
            return Err(CommandError::SequenceExhausted);
        };
        match self.apply_event(seq_nr, occurred_at, &event) {
            Ok(()) => Ok(event),
            Err(_) => Err(CommandError::InvalidTarget(self.cursor)),
        }
    }

    // ---- W2: decide (12 コマンド、1 コマンド 1 イベント) ----

    /// 非ゲート (initialization フェーズ) ステージの完了 — `StageCompleted`。
    ///
    /// # Errors
    ///
    /// 非受理 (`NotRunning`)、ゲート付きステージでの呼出 (`InvalidTarget`)、checkbox 前提違反
    /// (`CheckboxPrecondition`) を拒否する。
    pub fn complete_stage(
        &mut self,
        occurred_at: DateTime<Utc>,
    ) -> Result<WorkflowExecutionEvent, CommandError> {
        let stage = self.guard_running()?;
        self.require_gated(stage, false)?;
        self.require_checkbox(stage, &GATE_ADVANCE_PRECONDITION)?;
        let material = StageCompleted::new(self.slug_of(stage)?, self.next_in_scope_slug(stage));
        self.commit(
            WorkflowExecutionEvent::StageCompleted(material),
            occurred_at,
        )
    }

    /// 承認ゲートの開放 — `GateOpened`。`artifacts` は呼出側が渡す投影材料 (C5)。
    ///
    /// # Errors
    ///
    /// 非受理、非ゲートステージ (`InvalidTarget`)、in-progress 以外 (`CheckboxPrecondition`) を
    /// 拒否する (「only an in-progress stage can open a gate」)。
    pub fn open_gate(
        &mut self,
        artifacts: Vec<String>,
        occurred_at: DateTime<Utc>,
    ) -> Result<WorkflowExecutionEvent, CommandError> {
        let stage = self.guard_running()?;
        self.require_gated(stage, true)?;
        self.require_checkbox(stage, &[CheckboxState::InProgress])?;
        let material = GateOpened::new(self.slug_of(stage)?, artifacts);
        self.commit(WorkflowExecutionEvent::GateOpened(material), occurred_at)
    }

    /// 承認ゲートの通過 — `GateApproved`。`phase_boundary` は呼出側が導出して渡す投影材料 (C5)。
    ///
    /// `open_gate` を省いた in-progress からの承認も受理する (BR1.3)。
    ///
    /// # Errors
    ///
    /// 非受理、非ゲートステージ (`InvalidTarget`)、checkbox 前提違反を拒否する。
    pub fn approve_gate(
        &mut self,
        user_input: Option<String>,
        phase_boundary: Option<PhaseBoundary>,
        occurred_at: DateTime<Utc>,
    ) -> Result<WorkflowExecutionEvent, CommandError> {
        let stage = self.guard_running()?;
        self.require_gated(stage, true)?;
        self.require_checkbox(stage, &GATE_ADVANCE_PRECONDITION)?;
        let material = GateApproved::new(
            self.slug_of(stage)?,
            user_input,
            self.next_in_scope_slug(stage),
            phase_boundary,
        );
        self.commit(WorkflowExecutionEvent::GateApproved(material), occurred_at)
    }

    /// 承認ゲートでの差し戻し — `GateRejected`。改訂回数を +1 してイベントに載せる (BR1.4)。
    ///
    /// # Errors
    ///
    /// 非受理、非ゲートステージ (`InvalidTarget`)、checkbox 前提違反を拒否する。
    pub fn reject_gate(
        &mut self,
        feedback: Option<String>,
        occurred_at: DateTime<Utc>,
    ) -> Result<WorkflowExecutionEvent, CommandError> {
        let stage = self.guard_running()?;
        self.require_gated(stage, true)?;
        self.require_checkbox(stage, &GATE_ADVANCE_PRECONDITION)?;
        let next = self
            .revision_count(stage)
            .ok_or(CommandError::InvalidTarget(stage))?
            .saturating_add(1);
        let material = GateRejected::new(self.slug_of(stage)?, feedback, next);
        self.commit(WorkflowExecutionEvent::GateRejected(material), occurred_at)
    }

    /// 差し戻し後のゲート再入 — `StageRevised`。
    ///
    /// # Errors
    ///
    /// 非受理、revising 以外 (`CheckboxPrecondition`) を拒否する
    /// (「only a revising stage can re-enter its gate」)。
    pub fn revise_stage(
        &mut self,
        occurred_at: DateTime<Utc>,
    ) -> Result<WorkflowExecutionEvent, CommandError> {
        let stage = self.guard_running()?;
        self.require_checkbox(stage, &[CheckboxState::Revising])?;
        let material = StageRevised::new(self.slug_of(stage)?);
        self.commit(WorkflowExecutionEvent::StageRevised(material), occurred_at)
    }

    /// ステージの読み飛ばし — `StageSkipped` (CONDITIONAL または実効 SKIP のみ — BR1.5)。
    ///
    /// # Errors
    ///
    /// 非受理、checkbox 前提違反、CONDITIONAL でも実効 SKIP でもない場合 (`NotSkippable`) を
    /// 拒否する。
    pub fn skip_stage(
        &mut self,
        reason: String,
        occurred_at: DateTime<Utc>,
    ) -> Result<WorkflowExecutionEvent, CommandError> {
        let stage = self.guard_running()?;
        self.require_checkbox(stage, &SKIP_PRECONDITION)?;
        let conditional = self.entry(stage).is_some_and(StageEntry::is_conditional);
        if !(conditional || self.effective_plan(stage) == Some(PlanAction::Skip)) {
            return Err(CommandError::NotSkippable(stage));
        }
        let material =
            StageSkipped::new(self.slug_of(stage)?, reason, self.next_in_scope_slug(stage));
        self.commit(WorkflowExecutionEvent::StageSkipped(material), occurred_at)
    }

    /// カーソルの移動 — `Jumped` (BR1.6)。差分集合をイベントに載せ、承認の消去は適用側が
    /// `direction` と `target` から決定的に導出する。
    ///
    /// # Errors
    ///
    /// [`WorkflowExecution::jump_resolve`] と同じ (`NotRunning` / `InvalidTarget`)。
    pub fn jump(
        &mut self,
        target: StageIndex,
        occurred_at: DateTime<Utc>,
    ) -> Result<WorkflowExecutionEvent, CommandError> {
        let direction = self.jump_resolve(target)?;
        let source = self.cursor;
        let mut stages_reset = Vec::new();
        let mut stages_skipped = Vec::new();
        match direction {
            JumpDirection::Forward => {
                for value in source.to_usize()..target.to_usize() {
                    let stage = StageIndex::new(value);
                    let Some(marker) = self.checkbox(stage) else {
                        continue;
                    };
                    let skip_current = value == source.to_usize() && marker.is_active();
                    let skip_between = value > source.to_usize() && marker.is_in_flight();
                    if skip_current || skip_between {
                        stages_skipped.push(self.slug_of(stage)?);
                    }
                }
            }
            JumpDirection::Backward => {
                for value in (target.to_usize() + 1)..self.stage_count() {
                    let stage = StageIndex::new(value);
                    if self.in_scope(stage) && self.checkbox(stage) != Some(CheckboxState::Pending)
                    {
                        stages_reset.push(self.slug_of(stage)?);
                    }
                }
            }
            JumpDirection::Redo => {}
        }
        let material = Jumped::new(
            direction,
            self.slug_of(source)?,
            self.slug_of(target)?,
            stages_reset,
            stages_skipped,
        );
        self.commit(WorkflowExecutionEvent::Jumped(material), occurred_at)
    }

    /// park マーカーの設置 — `Parked` (autonomous 下は拒否 — BR1.7)。
    ///
    /// # Errors
    ///
    /// 非受理 (`NotRunning`)、autonomous 中 (`RefusedUnderAutonomy`)。
    pub fn park(
        &mut self,
        occurred_at: DateTime<Utc>,
    ) -> Result<WorkflowExecutionEvent, CommandError> {
        let stage = self.guard_running()?;
        if self.autonomy.is_autonomous() {
            return Err(CommandError::RefusedUnderAutonomy);
        }
        let material = Parked::new(self.slug_of(stage)?);
        self.commit(WorkflowExecutionEvent::Parked(material), occurred_at)
    }

    /// park マーカーの除去 — `Unparked`。位置は `parked_at` から復元される (BR1.7)。
    ///
    /// # Errors
    ///
    /// park が活性でなければ `NotRunning`。
    pub fn unpark(
        &mut self,
        occurred_at: DateTime<Utc>,
    ) -> Result<WorkflowExecutionEvent, CommandError> {
        if !self.parked_active() {
            return Err(CommandError::NotRunning);
        }
        self.commit(WorkflowExecutionEvent::Unparked, occurred_at)
    }

    /// 実効プランの再形成 — `Recomposed` (BR1.8)。反転対象は 1 件以上で、いずれかが不正なら
    /// 全体を `Err` にする (部分適用しない)。
    ///
    /// # Errors
    ///
    /// 非受理、autonomous 中 (`RefusedUnderAutonomy`)、対象が空・カーソル以前・範囲外
    /// (`InvalidTarget`)、pending 以外 (`CheckboxPrecondition`)。
    pub fn recompose(
        &mut self,
        flips: &[StageIndex],
        occurred_at: DateTime<Utc>,
    ) -> Result<WorkflowExecutionEvent, CommandError> {
        let cursor = self.guard_running()?;
        if self.autonomy.is_autonomous() {
            return Err(CommandError::RefusedUnderAutonomy);
        }
        let targets: BTreeSet<usize> = flips.iter().map(|stage| stage.to_usize()).collect();
        if targets.is_empty() {
            return Err(CommandError::InvalidTarget(cursor));
        }
        for &value in &targets {
            let stage = StageIndex::new(value);
            if value <= cursor.to_usize() || value >= self.stage_count() {
                return Err(CommandError::InvalidTarget(stage));
            }
            self.require_checkbox(stage, &[CheckboxState::Pending])?;
        }

        let mut projected = self.overlay.clone();
        let mut skipped = Vec::new();
        let mut added = Vec::new();
        for &value in &targets {
            let stage = StageIndex::new(value);
            let slug = self.slug_of(stage)?;
            match self.effective_plan(stage) {
                Some(PlanAction::Execute) => skipped.push(slug),
                Some(PlanAction::Skip) => added.push(slug),
                None => return Err(CommandError::InvalidTarget(stage)),
            }
            if let Some(slot) = projected.get_mut(value) {
                *slot = slot.flipped();
            }
        }
        let stages_in_scope = projected
            .iter()
            .enumerate()
            .filter(|(_, action)| **action == PlanAction::Execute)
            .filter_map(|(index, _)| self.stages.get(index).map(|entry| entry.slug().clone()))
            .collect();
        let material = Recomposed::new(skipped, added, stages_in_scope);
        self.commit(WorkflowExecutionEvent::Recomposed(material), occurred_at)
    }

    /// 自律モードを切り替える — `AutonomyModeSet` (BR1.8)。
    ///
    /// 方向は 2 つあり、仕様はそれぞれを**昇格**(gated → autonomous) と**降格**(その逆) と呼ぶ。
    /// 本メソッドは両方向を受ける。**昇格だけが human presence を要する** (I11) が、その
    /// ガードはユースケース層にある (監査台帳の射影が要る) — ここは状態変更のみ。
    ///
    /// 発する監査イベント文字列 `AUTONOMY_MODE_SET` と CLI 動詞 `set-autonomy` は upstream の
    /// Published Language なので逐語で維持するが、**本メソッド名は外に出ない**のでドメインの語
    /// を使う (coding-rules/ubiquitous-language.md)。
    ///
    /// # Errors
    ///
    /// 非受理なら `NotRunning`。
    pub fn switch_autonomy(
        &mut self,
        mode: AutonomyMode,
        occurred_at: DateTime<Utc>,
    ) -> Result<WorkflowExecutionEvent, CommandError> {
        self.guard_running()?;
        self.commit(
            WorkflowExecutionEvent::AutonomyModeSet(AutonomyModeSet::new(mode)),
            occurred_at,
        )
    }

    // ---- W3: apply (リプレイと通常実行の同一経路 — BR2.1 / BR2.3) ----

    /// イベントを 1 つ適用する。通常実行 (decide 経由) とリプレイの唯一の状態遷移経路。
    ///
    /// `seq_nr` と `occurred_at` は封筒が運ぶ材料なので**引数で受け取る** (ADR-010 / B7)。
    /// 採番と連続性の検査は依然としてドメインの責務である — 本家 v3 は「採番・連続性は
    /// 利用側 (ドメイン) の責務」と明文化しており、ライブラリは飛び番を検出しない。
    ///
    /// 検査は一時コピーに対して行い、成功した場合だけ差し替える — `Err` では状態が変わらない。
    ///
    /// # Errors
    ///
    /// `seq_nr` が現在値 + 1 でない (`SequenceGap`)、イベントのステージ slug が `stages` に
    /// 無い (`UnknownStage`)、適用後に集約不変条件が破れる (`InvariantViolation`)、現在値が
    /// `usize::MAX` で後続を数えられない (`SequenceExhausted`) を拒否する。
    pub fn apply_event(
        &mut self,
        seq_nr: usize,
        occurred_at: DateTime<Utc>,
        event: &WorkflowExecutionEvent,
    ) -> Result<(), ApplyError> {
        let Some(expected) = self.seq_nr.checked_add(1) else {
            return Err(ApplyError::SequenceExhausted);
        };
        if seq_nr != expected {
            return Err(ApplyError::SequenceGap {
                expected,
                actual: seq_nr,
            });
        }
        let mut next = self.clone();
        next.mutate(event)?;
        next.seq_nr = seq_nr;
        next.last_updated_at = occurred_at;
        next.check_invariants()
            .map_err(ApplyError::InvariantViolation)?;
        *self = next;
        Ok(())
    }

    /// 12 変種の網羅 match (NFR1.3)。`#[non_exhaustive]` を付けないので腕の欠落はビルドで落ちる。
    fn mutate(&mut self, event: &WorkflowExecutionEvent) -> Result<(), ApplyError> {
        match event {
            WorkflowExecutionEvent::Started(_) => {
                // `Started` は genesis 専用 — 既存の集約には適用できない (BR2.2)。
                return Err(ApplyError::InvariantViolation(
                    "Started applies only at genesis".to_string(),
                ));
            }
            WorkflowExecutionEvent::StageCompleted(completed) => {
                let stage = self.resolve(completed.stage())?;
                self.mark_stage(stage, CheckboxState::Completed);
                self.advance(completed.next_stage())?;
            }
            WorkflowExecutionEvent::GateOpened(opened) => {
                let stage = self.resolve(opened.stage())?;
                self.mark_stage(stage, CheckboxState::AwaitingApproval);
            }
            WorkflowExecutionEvent::GateApproved(approved) => {
                let stage = self.resolve(approved.stage())?;
                self.record_approval(stage);
                self.mark_stage(stage, CheckboxState::Completed);
                self.advance(approved.next_stage())?;
            }
            WorkflowExecutionEvent::GateRejected(rejected) => {
                let stage = self.resolve(rejected.stage())?;
                self.mark_stage(stage, CheckboxState::Revising);
                if let Some(slot) = self.revision_count.get_mut(stage.to_usize()) {
                    *slot = rejected.revision_count();
                }
            }
            WorkflowExecutionEvent::StageRevised(revised) => {
                let stage = self.resolve(revised.stage())?;
                self.mark_stage(stage, CheckboxState::AwaitingApproval);
            }
            WorkflowExecutionEvent::StageSkipped(skipped) => {
                let stage = self.resolve(skipped.stage())?;
                self.mark_stage(stage, CheckboxState::Skipped);
                self.advance(skipped.next_stage())?;
            }
            WorkflowExecutionEvent::Jumped(jumped) => {
                self.apply_jump(jumped)?;
            }
            WorkflowExecutionEvent::Parked(parked) => {
                let stage = self.resolve(parked.stage())?;
                self.parked_at = Some(stage);
            }
            WorkflowExecutionEvent::Unparked => {
                self.parked_at = None;
            }
            WorkflowExecutionEvent::Recomposed(recomposed) => {
                for slug in recomposed.skipped() {
                    let stage = self.resolve(slug)?;
                    if let Some(slot) = self.overlay.get_mut(stage.to_usize()) {
                        *slot = PlanAction::Skip;
                    }
                }
                for slug in recomposed.added() {
                    let stage = self.resolve(slug)?;
                    if let Some(slot) = self.overlay.get_mut(stage.to_usize()) {
                        *slot = PlanAction::Execute;
                    }
                }
            }
            WorkflowExecutionEvent::AutonomyModeSet(set) => {
                self.autonomy = set.mode();
            }
        }
        Ok(())
    }

    fn apply_jump(&mut self, jumped: &Jumped) -> Result<(), ApplyError> {
        let source = self.resolve(jumped.source())?;
        let target = self.resolve(jumped.target())?;
        for slug in jumped.stages_reset() {
            let stage = self.resolve(slug)?;
            self.mark_stage(stage, CheckboxState::Pending);
        }
        for slug in jumped.stages_skipped() {
            let stage = self.resolve(slug)?;
            self.mark_stage(stage, CheckboxState::Skipped);
        }
        match jumped.direction() {
            // backward は target 以降の承認履歴を、redo は出発点の承認履歴を無効化する (BR1.6)。
            JumpDirection::Backward => {
                for value in target.to_usize()..self.stage_count() {
                    self.invalidate_approval(StageIndex::new(value));
                }
            }
            JumpDirection::Redo => self.invalidate_approval(source),
            JumpDirection::Forward => {}
        }
        self.mark_stage(target, CheckboxState::InProgress);
        self.cursor = target;
        Ok(())
    }

    /// 完了・スキップの後段 — 次の in-scope ステージへ進むか、無ければ完了する (BR1.5)。
    fn advance(&mut self, next_stage: Option<&StageSlug>) -> Result<(), ApplyError> {
        match next_stage {
            Some(slug) => {
                let stage = self.resolve(slug)?;
                self.mark_stage(stage, CheckboxState::InProgress);
                self.cursor = stage;
            }
            None => self.status = Status::Completed,
        }
        Ok(())
    }

    /// 集約不変条件 (Quint の cursor_in_scope / at_most_one_active / no_gate_bypass /
    /// parked_position と長さ整合)。材料は不変条件名で、文言はアダプタ層の責務。
    fn check_invariants(&self) -> Result<(), String> {
        let count = self.stage_count();
        if count == 0 {
            return Err("stage count is zero".to_string());
        }
        for (name, actual) in [
            ("overlay", self.overlay.len()),
            ("checkbox", self.checkbox.len()),
            ("approved", self.approved.len()),
            ("revision_count", self.revision_count.len()),
        ] {
            if actual != count {
                return Err(format!("length mismatch: {name}"));
            }
        }
        if self.seq_nr == 0 {
            return Err("seq_nr is zero".to_string());
        }
        if self.cursor.to_usize() >= count {
            return Err("cursor out of range".to_string());
        }
        if let Some(parked) = self.parked_at {
            if parked.to_usize() >= count {
                return Err("parked_at out of range".to_string());
            }
            if parked != self.cursor {
                return Err("parked_position".to_string());
            }
        }
        if self.accepts_commands() && !self.in_scope(self.cursor) {
            return Err("cursor_in_scope".to_string());
        }
        let mut active = 0_usize;
        for value in 0..count {
            let stage = StageIndex::new(value);
            let Some(marker) = self.checkbox(stage) else {
                continue;
            };
            if marker.is_active() {
                active += 1;
            }
            if self.is_gated(stage)
                && marker == CheckboxState::Completed
                && self.approved(stage) != Some(true)
            {
                return Err(format!("no_gate_bypass at stage {value}"));
            }
        }
        if active > 1 {
            return Err(format!("at_most_one_active: {active}"));
        }
        Ok(())
    }

    // ---- W3: 状態の写し (memento) (BR5.2 / BR5.3) ----

    /// 全状態を値オブジェクトへ写す。`plan` / `conditional` は解決済み計画からの展開 (C6 の列構成)。
    #[must_use]
    pub fn state(&self) -> WorkflowExecutionState {
        WorkflowExecutionState {
            intent_id: self.intent_id.clone(),
            definition_id: self.definition_id.clone(),
            definition_revision: self.definition_revision.clone(),
            plan: self.stages.iter().map(StageEntry::plan_action).collect(),
            conditional: self.stages.iter().map(StageEntry::is_conditional).collect(),
            stages: self.stages.clone(),
            overlay: self.overlay.clone(),
            checkbox: self.checkbox.clone(),
            cursor: self.cursor.to_usize(),
            status: self.status,
            parked_at: self.parked_at.map(StageIndex::to_usize),
            autonomy: self.autonomy,
            approved: self.approved.clone(),
            revision_count: self.revision_count.clone(),
            seq_nr: self.seq_nr,
            last_updated_at: self.last_updated_at,
        }
    }

    /// 状態の写し (memento) から集約を復元する (不変条件を検査する唯一の再水和経路)。
    ///
    /// # Errors
    ///
    /// 長さ不一致・`plan` / `conditional` と解決済み計画の食い違い・範囲外カーソル・
    /// `cursor_in_scope` / `at_most_one_active` / `no_gate_bypass` / `parked_position` の違反・
    /// `seq_nr` = 0 を `InvariantViolation` で拒否する。
    pub fn from_state(state: WorkflowExecutionState) -> Result<WorkflowExecution, StateError> {
        let count = state.stages.len();
        if state.plan.len() != count {
            return Err(StateError::InvariantViolation(
                "length mismatch: plan".to_string(),
            ));
        }
        if state.conditional.len() != count {
            return Err(StateError::InvariantViolation(
                "length mismatch: conditional".to_string(),
            ));
        }
        for (index, entry) in state.stages.iter().enumerate() {
            if state.plan.get(index).copied() != Some(entry.plan_action()) {
                return Err(StateError::InvariantViolation(format!(
                    "plan disagrees with stages at {index}"
                )));
            }
            if state.conditional.get(index).copied() != Some(entry.is_conditional()) {
                return Err(StateError::InvariantViolation(format!(
                    "conditional disagrees with stages at {index}"
                )));
            }
        }
        let execution = WorkflowExecution {
            intent_id: state.intent_id,
            definition_id: state.definition_id,
            definition_revision: state.definition_revision,
            stages: state.stages,
            overlay: state.overlay,
            checkbox: state.checkbox,
            cursor: StageIndex::new(state.cursor),
            status: state.status,
            parked_at: state.parked_at.map(StageIndex::new),
            autonomy: state.autonomy,
            approved: state.approved,
            revision_count: state.revision_count,
            seq_nr: state.seq_nr,
            last_updated_at: state.last_updated_at,
        };
        execution
            .check_invariants()
            .map_err(StateError::InvariantViolation)?;
        Ok(execution)
    }

    // ---- W4 / W5: クエリ (書込なし) ----

    /// 現状態から次に何をすべきかを 1 つ決める (BR3.1 の優先順)。書込なし。
    ///
    /// 第 2 引数の定義は id の照合にだけ使う — 計画は `Started` で自己完結しているため、現時点の
    /// 分岐は定義の内容を参照しない (将来の分岐のための契約上の引数)。
    ///
    /// # Errors
    ///
    /// 引数の定義 id が `definition_id` と異なれば `DefinitionMismatch` (revision の差は Ok — BR2.6)。
    pub fn next_decision(
        &self,
        definition: &WorkflowDefinition,
        request: &NextRequest,
    ) -> Result<NextDecision, CommandError> {
        if definition.id() != &self.definition_id {
            return Err(CommandError::DefinitionMismatch {
                expected: self.definition_id.clone(),
                actual: definition.id().clone(),
            });
        }
        if self.parked_active() && !request.is_reentry() {
            return Ok(if request.is_resume() {
                NextDecision::UnparkThenResume
            } else {
                NextDecision::Parked { stage: self.cursor }
            });
        }
        if request.is_resume() {
            return Ok(NextDecision::ResumeMenu);
        }
        if request.is_free_text() {
            return Ok(NextDecision::NewWorkRouting);
        }
        if !self.status.is_running() {
            return Ok(NextDecision::Done);
        }
        let cursor = self.cursor;
        if let Some(marker) = self.checkbox(cursor)
            && marker.is_in_flight()
        {
            if self.effective_plan(cursor) == Some(PlanAction::Skip) {
                // 実効 SKIP のステージに run-stage は出さない。自力で `skip_stage` を呼べる
                // 前提集合 (SKIP_PRECONDITION) にいるときだけ復旧可能と報告する。
                return Ok(if SKIP_PRECONDITION.contains(&marker) {
                    NextDecision::RecoverSkipInconsistency {
                        stage: cursor,
                        checkbox: marker,
                    }
                } else {
                    NextDecision::InconsistentSkip {
                        stage: cursor,
                        checkbox: marker,
                    }
                });
            }
            return Ok(NextDecision::RunStage {
                stage: cursor,
                gate: self.is_gated(cursor),
            });
        }
        Ok(match self.next_in_scope(cursor) {
            Some(stage) => NextDecision::RunStage {
                stage,
                gate: self.is_gated(stage),
            },
            None => NextDecision::Done,
        })
    }

    /// jump の検証と方向の導出 (書込なし — `aidlc-jump resolve` に対応、BR3.3)。
    ///
    /// # Errors
    ///
    /// 非受理 (`NotRunning`)、範囲外・initialization・スコープ外ターゲット、initialization カーソルの
    /// redo (`InvalidTarget`) を拒否する。
    pub fn jump_resolve(&self, target: StageIndex) -> Result<JumpDirection, CommandError> {
        if !self.accepts_commands() {
            return Err(CommandError::NotRunning);
        }
        if target.to_usize() >= self.stage_count() {
            return Err(CommandError::InvalidTarget(target));
        }
        let direction = JumpDirection::of(self.cursor.to_usize(), target.to_usize());
        match direction {
            // INIT_JUMP_ERROR: initialization フェーズのステージへは跳べない。scope 外も不可。
            JumpDirection::Forward | JumpDirection::Backward => {
                if !self.is_gated(target) || !self.in_scope(target) {
                    return Err(CommandError::InvalidTarget(target));
                }
            }
            JumpDirection::Redo => {
                if !self.is_gated(self.cursor) {
                    return Err(CommandError::InvalidTarget(target));
                }
            }
        }
        Ok(direction)
    }

    /// カーソル通過済み completed への再報告は**何もコミットせず**冪等 done (BR1.9)。
    ///
    /// # Errors
    ///
    /// 非受理 (`NotRunning`)、カーソル通過済み completed でない対象 (`NotStale`)。
    pub fn stale_report(&self, stage: StageIndex) -> Result<NextDecision, CommandError> {
        if !self.accepts_commands() {
            return Err(CommandError::NotRunning);
        }
        if stage.to_usize() >= self.cursor.to_usize()
            || self.checkbox(stage) != Some(CheckboxState::Completed)
        {
            return Err(CommandError::NotStale(stage));
        }
        Ok(NextDecision::Done)
    }
}

/// 直列化の入口 (serde の `into`)。中身は [`WorkflowExecution::state`] そのものである。
impl From<WorkflowExecution> for WorkflowExecutionState {
    fn from(execution: WorkflowExecution) -> WorkflowExecutionState {
        execution.state()
    }
}

/// 復号の入口 (serde の `try_from`)。[`WorkflowExecution::from_state`] の検査点を通る。
impl TryFrom<WorkflowExecutionState> for WorkflowExecution {
    type Error = StateError;

    fn try_from(state: WorkflowExecutionState) -> Result<WorkflowExecution, StateError> {
        WorkflowExecution::from_state(state)
    }
}

#[cfg(test)]
mod tests {
    // テストは固定長フィクスチャの添字参照を許容 (clippy.toml に相当設定が無いため file 単位で
    // allow)。panic! は想定外バリアントの即時失敗という検証用途で使っており、テスト失敗の
    // シグナルとして妥当なため同様に許容する。
    #![allow(clippy::indexing_slicing, clippy::panic)]

    use super::*;
    use crate::orchestration::{
        ApplyError, AutonomyMode, CommandError, EngineSignal, IntentId, JumpDirection,
        NextDecision, NextRequest, PhaseBoundary, StageCompleted, StageEntry, StageIndex,
        StartError, StartRequest, Started, StateError, Status, WorkflowExecutionEvent,
        WorkflowExecutionStateBuilder,
    };
    use crate::workflow_definition::{
        DefinitionRevision, ExecutionKind, PhaseId, PlanAction, ScopeGrid, ScopeMetadata,
        StageGraph, StageMode, StageNode, StageNodeBuilder, StageNumber, StageSlug,
        WorkflowDefinition, WorkflowDefinitionId,
    };
    use crate::workspace::CheckboxState;
    use std::collections::BTreeMap;

    use CheckboxState::{AwaitingApproval, Completed, InProgress, Pending, Revising, Skipped};
    use PlanAction::{Execute, Skip};

    /// ITF 再生も含め、集約は `occurred_at` を素通しするだけなので固定値でよい。
    const AT_TEXT: &str = "2026-08-23T00:00:00Z";

    fn occurred() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(AT_TEXT)
            .unwrap()
            .with_timezone(&Utc)
    }

    fn slug(i: usize) -> StageSlug {
        StageSlug::parse(&format!("stage-{i}")).unwrap()
    }

    fn intent() -> IntentId {
        IntentId::parse("01a02785-1bd8-76eb-aeea-5aa303ebd5b6").unwrap()
    }

    fn def_id(value: &str) -> WorkflowDefinitionId {
        WorkflowDefinitionId::parse(value).unwrap()
    }

    fn revision(fill: char) -> DefinitionRevision {
        DefinitionRevision::parse(&format!("sha256:{}", fill.to_string().repeat(64))).unwrap()
    }

    /// 索引 < `init` を initialization、残りを inception にした合成計画。
    fn entries(init: usize, actions: &[PlanAction], conditional: &[bool]) -> Vec<StageEntry> {
        actions
            .iter()
            .zip(conditional.iter())
            .enumerate()
            .map(|(i, (action, cond))| {
                let phase = if i < init {
                    PhaseId::Initialization
                } else {
                    PhaseId::Inception
                };
                StageEntry::new(slug(i), phase, *action, *cond)
            })
            .collect()
    }

    fn start_request() -> StartRequest {
        StartRequest::new("classic", "build it")
    }

    fn start_with(init: usize, actions: &[PlanAction], conditional: &[bool]) -> WorkflowExecution {
        WorkflowExecution::start_from_plan_unchecked(
            intent(),
            def_id("claude"),
            revision('0'),
            &start_request(),
            entries(init, actions, conditional),
            occurred(),
        )
        .unwrap()
        .0
    }

    fn all_exec(n: usize) -> WorkflowExecution {
        start_with(1, &vec![Execute; n], &vec![false; n])
    }

    fn at(w: &WorkflowExecution, i: usize) -> StageIndex {
        w.stage_index(i).unwrap()
    }

    /// `next_decision` の第 2 引数用の最小定義 (id の照合にしか使われない — BR3.1)。
    fn bare_definition(id: &str) -> WorkflowDefinition {
        WorkflowDefinition::new(
            def_id(id),
            revision('0'),
            StageGraph::new(Vec::new()).unwrap(),
            ScopeGrid::new(BTreeMap::new()),
            BTreeMap::new(),
        )
    }

    fn node(name: &str, number: &str, phase: PhaseId, execution: ExecutionKind) -> StageNode {
        StageNodeBuilder::new(
            StageSlug::parse(name).unwrap(),
            StageNumber::parse(number).unwrap(),
            name.to_string(),
            phase,
            execution,
            StageMode::Inline,
        )
        .scopes(vec!["classic".to_string()])
        .build()
    }

    /// 文書順 = 数値順の小さな出荷グラフ相当 (initialization 1 + ideation 2)。
    fn shipped_definition(grid: ScopeGrid) -> WorkflowDefinition {
        let graph = StageGraph::new(vec![
            node(
                "state-init",
                "0.1",
                PhaseId::Initialization,
                ExecutionKind::Always,
            ),
            node(
                "intent-capture",
                "1.1",
                PhaseId::Ideation,
                ExecutionKind::Always,
            ),
            node(
                "market-research",
                "1.2",
                PhaseId::Ideation,
                ExecutionKind::Conditional,
            ),
        ])
        .unwrap();
        let scopes: BTreeMap<String, ScopeMetadata> = [(
            "classic".to_string(),
            ScopeMetadata::new("classic").unwrap(),
        )]
        .into_iter()
        .collect();
        WorkflowDefinition::new(def_id("claude"), revision('a'), graph, grid, scopes)
    }

    fn full_grid() -> ScopeGrid {
        let column: BTreeMap<StageSlug, PlanAction> = [
            (StageSlug::parse("state-init").unwrap(), Execute),
            (StageSlug::parse("intent-capture").unwrap(), Execute),
            (StageSlug::parse("market-research").unwrap(), Execute),
        ]
        .into_iter()
        .collect();
        ScopeGrid::new([("classic".to_string(), column)].into_iter().collect())
    }

    // ---- W1: start (BR2.2 / BR2.6) ----

    #[test]
    fn start_records_the_definition_identity_and_the_resolved_plan() {
        let definition = shipped_definition(full_grid());
        let (w, event) =
            WorkflowExecution::start(intent(), &definition, &start_request(), occurred()).unwrap();

        // 通番・発生時刻・識別子は封筒 (アダプタ層) の材料であり、イベント自身は持たない。
        // genesis 直後の集約がその 3 点を保持している (B7)。
        assert_eq!(w.seq_nr(), 1);
        assert_eq!(w.last_updated_at(), &occurred());
        assert_eq!(w.intent_id(), &intent());
        let WorkflowExecutionEvent::Started(started) = &event else {
            panic!("start must emit Started");
        };
        assert_eq!(started.definition_id(), definition.id());
        assert_eq!(started.definition_revision(), definition.revision());
        assert_eq!(started.scope(), "classic");
        assert_eq!(started.request(), "build it");
        assert_eq!(started.stages().len(), 3);
        assert_eq!(started.stages()[0].phase(), PhaseId::Initialization);
        assert!(!started.stages()[1].is_conditional());
        assert!(started.stages()[2].is_conditional());

        assert_eq!(w.stage_count(), 3);
        assert_eq!(w.cursor(), at(&w, 0));
        assert_eq!(w.checkbox(at(&w, 0)), Some(InProgress));
        assert_eq!(w.checkbox(at(&w, 1)), Some(Pending));
        assert_eq!(w.status(), Status::Running);
        assert_eq!(w.autonomy(), AutonomyMode::Gated);
        assert_eq!(w.parked_at(), None);
        assert_eq!(w.definition_id(), definition.id());
        assert_eq!(w.definition_revision(), definition.revision());
        assert_eq!(w.revision_count(at(&w, 0)), Some(0));
    }

    #[test]
    fn start_carries_the_depth_and_test_strategy_the_caller_resolved() {
        // C5 の Started payload は depth / test_strategy を持つ (U4 が Scope Configuration を
        // 描く材料)。集約はこの 2 値に意味論を持たず、素通しでイベントに載せるだけである。
        let definition = shipped_definition(full_grid());
        let request = StartRequest::new("classic", "build it")
            .with_depth("standard")
            .with_test_strategy("comprehensive");
        let (_, event) =
            WorkflowExecution::start(intent(), &definition, &request, occurred()).unwrap();
        let WorkflowExecutionEvent::Started(started) = &event else {
            panic!("start must emit Started");
        };
        assert_eq!(started.scope(), "classic");
        assert_eq!(started.request(), "build it");
        assert_eq!(started.depth(), Some("standard"));
        assert_eq!(started.test_strategy(), Some("comprehensive"));

        // 省略時は None のまま載る (フラグ未指定 = 既定の解決は呼出側の責務)。
        let bare = StartRequest::new("classic", "build it");
        let (_, plain) =
            WorkflowExecution::start(intent(), &definition, &bare, occurred()).unwrap();
        let WorkflowExecutionEvent::Started(started) = &plain else {
            panic!("start must emit Started");
        };
        assert_eq!(started.depth(), None);
        assert_eq!(started.test_strategy(), None);
    }

    #[test]
    fn an_unknown_scope_is_refused_with_the_definition_material() {
        let definition = shipped_definition(full_grid());
        let unknown = StartRequest::new("nope", "build it");
        let err =
            WorkflowExecution::start(intent(), &definition, &unknown, occurred()).unwrap_err();
        let StartError::UnknownScope(scope) = err else {
            panic!("expected UnknownScope");
        };
        assert_eq!(scope.scope(), "nope");
        assert_eq!(scope.valid_scopes(), ["classic".to_string()]);
    }

    #[test]
    fn a_missing_grid_cell_folds_to_skip_outside_initialization() {
        // グリッドに列が無いステージは `None → SKIP` に畳む (BR2.2)。
        let column: BTreeMap<StageSlug, PlanAction> =
            [(StageSlug::parse("state-init").unwrap(), Execute)]
                .into_iter()
                .collect();
        let grid = ScopeGrid::new([("classic".to_string(), column)].into_iter().collect());
        let (w, _) = WorkflowExecution::start(
            intent(),
            &shipped_definition(grid),
            &start_request(),
            occurred(),
        )
        .unwrap();
        assert_eq!(w.effective_plan(at(&w, 1)), Some(Skip));
        assert_eq!(w.effective_plan(at(&w, 2)), Some(Skip));
    }

    #[test]
    fn an_empty_stage_list_is_refused() {
        let err = WorkflowExecution::start_from_plan_unchecked(
            intent(),
            def_id("claude"),
            revision('0'),
            &start_request(),
            Vec::new(),
            occurred(),
        )
        .unwrap_err();
        assert_eq!(err, StartError::Empty);
    }

    #[test]
    fn an_initialization_stage_that_folds_to_skip_is_refused() {
        let err = WorkflowExecution::start_from_plan_unchecked(
            intent(),
            def_id("claude"),
            revision('0'),
            &start_request(),
            entries(2, &[Execute, Skip, Execute], &[false, false, false]),
            occurred(),
        )
        .unwrap_err();
        assert_eq!(err, StartError::InitializationMustExecute);
    }

    #[test]
    fn a_conditional_initialization_stage_is_refused() {
        let err = WorkflowExecution::start_from_plan_unchecked(
            intent(),
            def_id("claude"),
            revision('0'),
            &start_request(),
            entries(1, &[Execute, Execute], &[true, false]),
            occurred(),
        )
        .unwrap_err();
        assert_eq!(err, StartError::InitializationMustBeUnconditional);
    }

    #[test]
    fn a_first_stage_outside_scope_is_refused_because_the_cursor_must_be_in_scope() {
        let err = WorkflowExecution::start_from_plan_unchecked(
            intent(),
            def_id("claude"),
            revision('0'),
            &start_request(),
            entries(0, &[Skip, Execute], &[false, false]),
            occurred(),
        )
        .unwrap_err();
        assert_eq!(err, StartError::InitializationMustExecute);
    }

    // ---- W2: 12 コマンド (BR1.0〜BR1.9) ----

    #[test]
    fn a_gated_stage_cannot_complete_without_passing_through_approval() {
        let mut w = all_exec(3);
        w.complete_stage(occurred()).unwrap();
        assert_eq!(w.cursor(), at(&w, 1));
        w.open_gate(Vec::new(), occurred()).unwrap();
        w.approve_gate(None, None, occurred()).unwrap();
        assert_eq!(w.approved(at(&w, 1)), Some(true));
        assert_eq!(w.checkbox(at(&w, 1)), Some(Completed));
    }

    #[test]
    fn complete_stage_is_refused_on_a_gated_stage() {
        let mut w = all_exec(3);
        w.complete_stage(occurred()).unwrap();
        let target = at(&w, 1);
        assert_eq!(
            w.complete_stage(occurred()),
            Err(CommandError::InvalidTarget(target))
        );
    }

    #[test]
    fn approve_gate_and_the_gate_openers_are_refused_on_a_non_gated_stage() {
        let mut w = all_exec(3);
        let target = at(&w, 0);
        assert_eq!(
            w.approve_gate(None, None, occurred()),
            Err(CommandError::InvalidTarget(target))
        );
        assert_eq!(
            w.open_gate(Vec::new(), occurred()),
            Err(CommandError::InvalidTarget(target))
        );
        assert_eq!(
            w.reject_gate(None, occurred()),
            Err(CommandError::InvalidTarget(target))
        );
    }

    #[test]
    fn approve_gate_accepts_the_open_gate_shortcut() {
        let mut w = all_exec(3);
        w.complete_stage(occurred()).unwrap();
        // open_gate を省いた in-progress からの承認も受理する (BR1.3)。
        let event = w
            .approve_gate(Some("ok".to_string()), None, occurred())
            .unwrap();
        let WorkflowExecutionEvent::GateApproved(approved) = &event else {
            panic!("expected GateApproved");
        };
        assert_eq!(approved.user_input(), Some("ok"));
        assert_eq!(approved.stage(), &slug(1));
        assert_eq!(approved.next_stage(), Some(&slug(2)));
        assert_eq!(w.checkbox(at(&w, 1)), Some(Completed));
    }

    #[test]
    fn gate_lifecycle_preconditions_are_strict() {
        let mut w = all_exec(3);
        w.complete_stage(occurred()).unwrap();
        assert!(matches!(
            w.revise_stage(occurred()),
            Err(CommandError::CheckboxPrecondition { .. })
        ));
        w.open_gate(Vec::new(), occurred()).unwrap();
        assert!(matches!(
            w.open_gate(Vec::new(), occurred()),
            Err(CommandError::CheckboxPrecondition { .. })
        ));
        w.reject_gate(None, occurred()).unwrap();
        assert_eq!(w.checkbox(at(&w, 1)), Some(Revising));
        w.revise_stage(occurred()).unwrap();
        assert_eq!(w.checkbox(at(&w, 1)), Some(AwaitingApproval));
    }

    #[test]
    fn reject_gate_increments_the_revision_count_and_carries_it() {
        let mut w = all_exec(3);
        w.complete_stage(occurred()).unwrap();
        let first = w.reject_gate(Some("redo".to_string()), occurred()).unwrap();
        let WorkflowExecutionEvent::GateRejected(rejected) = &first else {
            panic!("expected GateRejected");
        };
        assert_eq!(rejected.revision_count(), 1);
        assert_eq!(rejected.feedback(), Some("redo"));
        assert_eq!(w.revision_count(at(&w, 1)), Some(1));
        w.revise_stage(occurred()).unwrap();
        w.reject_gate(None, occurred()).unwrap();
        assert_eq!(w.revision_count(at(&w, 1)), Some(2));
    }

    #[test]
    fn skipped_is_refused_unless_conditional_or_plan_skip() {
        let mut w = start_with(1, &[Execute, Execute, Execute], &[false, false, true]);
        w.complete_stage(occurred()).unwrap();
        let cursor = at(&w, 1);
        assert_eq!(
            w.skip_stage("no".to_string(), occurred()),
            Err(CommandError::NotSkippable(cursor))
        );
        w.approve_gate(None, None, occurred()).unwrap();
        let event = w.skip_stage("conditional".to_string(), occurred()).unwrap();
        let WorkflowExecutionEvent::StageSkipped(skipped) = &event else {
            panic!("expected StageSkipped");
        };
        assert_eq!(skipped.reason(), "conditional");
        assert_eq!(skipped.next_stage(), None);
        assert_eq!(w.status(), Status::Completed);
        assert_eq!(w.checkbox(at(&w, 2)), Some(Skipped));
    }

    #[test]
    fn forward_jump_skips_intervening_in_flight_stages() {
        let mut w = all_exec(5);
        w.complete_stage(occurred()).unwrap();
        let target = at(&w, 3);
        let event = w.jump(target, occurred()).unwrap();
        let WorkflowExecutionEvent::Jumped(jumped) = &event else {
            panic!("expected Jumped");
        };
        assert_eq!(jumped.direction(), JumpDirection::Forward);
        assert_eq!(jumped.source(), &slug(1));
        assert_eq!(jumped.target(), &slug(3));
        assert_eq!(jumped.stages_skipped(), [slug(1), slug(2)]);
        assert!(jumped.stages_reset().is_empty());
        assert_eq!(w.checkbox(at(&w, 1)), Some(Skipped));
        assert_eq!(w.checkbox(at(&w, 2)), Some(Skipped));
        assert_eq!(w.checkbox(at(&w, 3)), Some(InProgress));
        assert_eq!(w.cursor(), target);
    }

    #[test]
    fn backward_jump_resets_downstream_and_invalidates_approvals() {
        let mut w = all_exec(4);
        w.complete_stage(occurred()).unwrap();
        w.open_gate(Vec::new(), occurred()).unwrap();
        w.approve_gate(None, None, occurred()).unwrap();
        let target = at(&w, 1);
        let event = w.jump(target, occurred()).unwrap();
        let WorkflowExecutionEvent::Jumped(jumped) = &event else {
            panic!("expected Jumped");
        };
        assert_eq!(jumped.direction(), JumpDirection::Backward);
        assert_eq!(jumped.stages_reset(), [slug(2)]);
        assert_eq!(w.checkbox(at(&w, 1)), Some(InProgress));
        assert_eq!(w.checkbox(at(&w, 2)), Some(Pending));
        assert_eq!(w.approved(at(&w, 1)), Some(false));
    }

    #[test]
    fn jump_to_an_initialization_stage_is_refused() {
        let mut w = all_exec(3);
        w.complete_stage(occurred()).unwrap();
        let target = at(&w, 0);
        assert_eq!(
            w.jump(target, occurred()),
            Err(CommandError::InvalidTarget(target))
        );
        assert_eq!(
            w.jump_resolve(target),
            Err(CommandError::InvalidTarget(target))
        );
    }

    #[test]
    fn redo_reopens_the_cursor_and_drops_its_approval() {
        let mut w = all_exec(3);
        w.complete_stage(occurred()).unwrap();
        w.open_gate(Vec::new(), occurred()).unwrap();
        w.reject_gate(None, occurred()).unwrap();
        let cursor = w.cursor();
        assert_eq!(w.jump_resolve(cursor), Ok(JumpDirection::Redo));
        w.jump(cursor, occurred()).unwrap();
        assert_eq!(w.checkbox(cursor), Some(InProgress));
        assert_eq!(w.approved(cursor), Some(false));
    }

    #[test]
    fn a_redo_on_an_initialization_cursor_is_refused() {
        let w = all_exec(3);
        let cursor = w.cursor();
        assert_eq!(
            w.jump_resolve(cursor),
            Err(CommandError::InvalidTarget(cursor))
        );
    }

    #[test]
    fn park_preserves_position_and_autonomous_park_is_refused() {
        let mut w = all_exec(3);
        w.complete_stage(occurred()).unwrap();
        let event = w.park(occurred()).unwrap();
        let WorkflowExecutionEvent::Parked(parked) = &event else {
            panic!("expected Parked");
        };
        assert_eq!(parked.stage(), &slug(1));
        assert!(w.parked_active());
        assert!(!w.accepts_commands());
        w.unpark(occurred()).unwrap();
        assert_eq!(w.cursor(), at(&w, 1));
        assert_eq!(w.parked_at(), None);
        w.switch_autonomy(AutonomyMode::Autonomous, occurred())
            .unwrap();
        assert_eq!(w.park(occurred()), Err(CommandError::RefusedUnderAutonomy));
    }

    #[test]
    fn every_command_but_unpark_is_refused_while_parked() {
        let mut w = all_exec(4);
        w.complete_stage(occurred()).unwrap();
        w.park(occurred()).unwrap();
        let target = at(&w, 2);
        assert_eq!(w.complete_stage(occurred()), Err(CommandError::NotRunning));
        assert_eq!(
            w.open_gate(Vec::new(), occurred()),
            Err(CommandError::NotRunning)
        );
        assert_eq!(
            w.approve_gate(None, None, occurred()),
            Err(CommandError::NotRunning)
        );
        assert_eq!(
            w.reject_gate(None, occurred()),
            Err(CommandError::NotRunning)
        );
        assert_eq!(w.revise_stage(occurred()), Err(CommandError::NotRunning));
        assert_eq!(
            w.skip_stage("x".to_string(), occurred()),
            Err(CommandError::NotRunning)
        );
        assert_eq!(w.jump(target, occurred()), Err(CommandError::NotRunning));
        assert_eq!(w.park(occurred()), Err(CommandError::NotRunning));
        assert_eq!(
            w.recompose(&[target], occurred()),
            Err(CommandError::NotRunning)
        );
        assert_eq!(
            w.switch_autonomy(AutonomyMode::Autonomous, occurred()),
            Err(CommandError::NotRunning)
        );
        assert_eq!(w.stale_report(at(&w, 0)), Err(CommandError::NotRunning));
        w.unpark(occurred()).unwrap();
        assert!(w.accepts_commands());
    }

    #[test]
    fn unpark_is_refused_when_the_marker_is_not_active() {
        let mut w = all_exec(3);
        assert_eq!(w.unpark(occurred()), Err(CommandError::NotRunning));
    }

    #[test]
    fn recompose_flips_only_pending_stages_ahead_of_the_cursor() {
        let mut w = all_exec(4);
        w.complete_stage(occurred()).unwrap();
        let cursor = w.cursor();
        assert_eq!(
            w.recompose(&[cursor], occurred()),
            Err(CommandError::InvalidTarget(cursor))
        );
        let event = w.recompose(&[at(&w, 2), at(&w, 3)], occurred()).unwrap();
        let WorkflowExecutionEvent::Recomposed(recomposed) = &event else {
            panic!("expected Recomposed");
        };
        assert_eq!(recomposed.skipped(), [slug(2), slug(3)]);
        assert!(recomposed.added().is_empty());
        assert_eq!(recomposed.stages_in_scope(), [slug(0), slug(1)]);
        assert_eq!(w.effective_plan(at(&w, 2)), Some(Skip));
        assert_eq!(w.effective_plan(at(&w, 3)), Some(Skip));
        // plan (静的グリッド) は不変 — オーバレイだけが動く。
        assert_eq!(w.stages()[2].plan_action(), Execute);
        w.switch_autonomy(AutonomyMode::Autonomous, occurred())
            .unwrap();
        assert_eq!(
            w.recompose(&[at(&w, 2)], occurred()),
            Err(CommandError::RefusedUnderAutonomy)
        );
    }

    #[test]
    fn recompose_rejects_the_whole_set_when_one_target_is_invalid() {
        let mut w = all_exec(4);
        w.complete_stage(occurred()).unwrap();
        let cursor = w.cursor();
        assert_eq!(
            w.recompose(&[at(&w, 2), cursor], occurred()),
            Err(CommandError::InvalidTarget(cursor))
        );
        // 部分適用しない (BR1.8)。
        assert_eq!(w.effective_plan(at(&w, 2)), Some(Execute));
        assert_eq!(
            w.recompose(&[], occurred()),
            Err(CommandError::InvalidTarget(cursor))
        );
    }

    #[test]
    fn set_autonomy_replaces_the_mode() {
        let mut w = all_exec(3);
        let event = w
            .switch_autonomy(AutonomyMode::Autonomous, occurred())
            .unwrap();
        let WorkflowExecutionEvent::AutonomyModeSet(set) = &event else {
            panic!("expected AutonomyModeSet");
        };
        assert_eq!(set.mode(), AutonomyMode::Autonomous);
        assert_eq!(w.autonomy(), AutonomyMode::Autonomous);
    }

    #[test]
    fn a_refused_command_leaves_the_state_and_the_sequence_untouched() {
        let mut w = all_exec(3);
        let before = w.clone();
        assert!(w.revise_stage(occurred()).is_err());
        assert_eq!(w, before);
        assert_eq!(w.seq_nr(), before.seq_nr());
    }

    #[test]
    fn a_completed_workflow_refuses_every_command() {
        let mut w = all_exec(2);
        w.complete_stage(occurred()).unwrap();
        w.approve_gate(None, None, occurred()).unwrap();
        assert_eq!(w.status(), Status::Completed);
        assert!(!w.accepts_commands());
        assert_eq!(w.complete_stage(occurred()), Err(CommandError::NotRunning));
    }

    // ---- BR1.9: stale_report ----

    #[test]
    fn stale_rereport_yields_done_and_commits_nothing() {
        let mut w = all_exec(3);
        w.complete_stage(occurred()).unwrap();
        let before = w.clone();
        assert_eq!(w.stale_report(at(&w, 0)), Ok(NextDecision::Done));
        assert_eq!(w, before);
        let cursor = at(&w, 1);
        assert_eq!(w.stale_report(cursor), Err(CommandError::NotStale(cursor)));
    }

    // ---- W3: apply_event (BR2.1) ----

    #[test]
    fn apply_event_refuses_a_sequence_gap() {
        let mut w = all_exec(3);
        let event =
            WorkflowExecutionEvent::StageCompleted(StageCompleted::new(slug(0), Some(slug(1))));
        assert_eq!(
            w.apply_event(9, occurred(), &event),
            Err(ApplyError::SequenceGap {
                expected: 2,
                actual: 9
            })
        );
        assert_eq!(w.seq_nr(), 1);
    }

    #[test]
    fn apply_event_refuses_at_sequence_exhaustion() {
        // memento 経由で通番を末端に据える (実運用では到達しない規模の境界)。
        let mut state = all_exec(3).state();
        state.seq_nr = usize::MAX;
        let mut w = WorkflowExecution::from_state(state).unwrap();
        let event =
            WorkflowExecutionEvent::StageCompleted(StageCompleted::new(slug(0), Some(slug(1))));
        assert_eq!(
            w.apply_event(1, occurred(), &event),
            Err(ApplyError::SequenceExhausted)
        );
        assert_eq!(w.seq_nr(), usize::MAX, "状態は変わらない");
    }

    #[test]
    fn a_command_at_sequence_exhaustion_is_refused() {
        let mut state = all_exec(3).state();
        state.seq_nr = usize::MAX;
        let mut w = WorkflowExecution::from_state(state).unwrap();
        assert_eq!(
            w.complete_stage(occurred()),
            Err(CommandError::SequenceExhausted)
        );
        assert_eq!(w.seq_nr(), usize::MAX, "状態は変わらない");
    }

    #[test]
    fn apply_event_refuses_an_unknown_stage() {
        let mut w = all_exec(3);
        let unknown = StageSlug::parse("no-such-stage").unwrap();
        let event =
            WorkflowExecutionEvent::StageCompleted(StageCompleted::new(unknown.clone(), None));
        assert_eq!(
            w.apply_event(2, occurred(), &event),
            Err(ApplyError::UnknownStage(unknown))
        );
    }

    #[test]
    fn apply_event_refuses_an_event_that_breaks_an_invariant() {
        let mut w = all_exec(3);
        let before = w.clone();
        // ゲート付きステージを承認なしで completed にすると no_gate_bypass が破れる。
        let event =
            WorkflowExecutionEvent::StageCompleted(StageCompleted::new(slug(1), Some(slug(2))));
        assert!(matches!(
            w.apply_event(2, occurred(), &event),
            Err(ApplyError::InvariantViolation(_))
        ));
        assert_eq!(w, before);
    }

    #[test]
    fn apply_event_refuses_a_started_outside_genesis() {
        let mut w = all_exec(3);
        let event = WorkflowExecutionEvent::Started(Started::new(
            def_id("claude"),
            revision('0'),
            &StartRequest::new("classic", "again"),
            entries(1, &[Execute], &[false]),
        ));
        assert!(matches!(
            w.apply_event(2, occurred(), &event),
            Err(ApplyError::InvariantViolation(_))
        ));
    }

    #[test]
    fn a_command_equals_the_old_state_plus_its_event() {
        let mut w = all_exec(4);
        let before = w.clone();
        let event = w.complete_stage(occurred()).unwrap();
        let mut replayed = before;
        replayed
            .apply_event(w.seq_nr(), *w.last_updated_at(), &event)
            .unwrap();
        assert_eq!(replayed, w);
    }

    #[test]
    fn a_gate_approval_carries_the_caller_supplied_phase_boundary() {
        let mut w = all_exec(3);
        w.complete_stage(occurred()).unwrap();
        let boundary = PhaseBoundary::new(PhaseId::Ideation, PhaseId::Inception);
        let event = w.approve_gate(None, Some(boundary), occurred()).unwrap();
        let WorkflowExecutionEvent::GateApproved(approved) = &event else {
            panic!("expected GateApproved");
        };
        assert_eq!(approved.phase_boundary(), Some(boundary));
    }

    // ---- W3: state / from_state (BR5.2 / BR5.3) ----

    #[test]
    fn the_state_carries_every_attribute_and_round_trips() {
        let mut w = all_exec(4);
        w.complete_stage(occurred()).unwrap();
        w.open_gate(Vec::new(), occurred()).unwrap();
        w.reject_gate(None, occurred()).unwrap();
        let state = w.state();
        assert_eq!(state.intent_id(), w.intent_id());
        assert_eq!(state.definition_id(), w.definition_id());
        assert_eq!(state.definition_revision(), w.definition_revision());
        assert_eq!(state.stages(), w.stages());
        assert_eq!(state.plan(), [Execute, Execute, Execute, Execute]);
        assert_eq!(state.overlay(), [Execute, Execute, Execute, Execute]);
        assert_eq!(state.conditional(), [false, false, false, false]);
        assert_eq!(state.checkbox()[0], Completed);
        assert_eq!(state.cursor(), w.cursor());
        assert_eq!(state.status(), Status::Running);
        assert_eq!(state.parked_at(), None);
        assert_eq!(state.autonomy(), AutonomyMode::Gated);
        assert_eq!(state.approved(), [false, false, false, false]);
        assert_eq!(state.revision_count(), [0, 1, 0, 0]);
        assert_eq!(state.seq_nr(), w.seq_nr());
        assert_eq!(state.last_updated_at(), *w.last_updated_at());
        assert_eq!(WorkflowExecution::from_state(state).unwrap(), w);
    }

    #[test]
    fn the_aggregate_round_trips_through_serde() {
        // スナップショットの直列化はこの経路を通る (本家 v3 のシリアライザは
        // `Serialize` / `DeserializeOwned` だけを要求する)。SQLite バックエンドはこの形で
        // payload 列を書く。
        let mut w = all_exec(3);
        w.complete_stage(occurred()).unwrap();
        // スナップショット payload の往復確認であり、契約 JSON (BR1.7) の直列化経路では
        // ないため、canon-json を経ない素の serde_json を使う。
        #[allow(
            clippy::disallowed_methods,
            reason = "契約 JSON ではなく serde 境界そのものの往復確認 (BR1.7 の射程外)"
        )]
        let json = serde_json::to_string(&w).unwrap();
        let decoded: WorkflowExecution = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, w);
        assert_eq!(decoded.seq_nr(), w.seq_nr());
        assert_eq!(decoded.last_updated_at(), w.last_updated_at());
        assert!(
            !json.contains("version"),
            "楽観 version は payload に載らない (B7): {json}"
        );
    }

    #[test]
    fn a_tampered_serialised_aggregate_is_refused() {
        // serde は memento (`WorkflowExecutionState`) 経由なので、復号は `from_state` の
        // 検査点をそのまま通る (オーナー裁定 2026-08-27 (A))。行を手で書き換えた JSON —
        // ここでは範囲外カーソル — が黙って通らないことを固定する。
        let w = all_exec(3);
        #[allow(
            clippy::disallowed_methods,
            reason = "契約 JSON ではなく serde 境界そのものの検査 (BR1.7 の射程外)"
        )]
        let json = serde_json::to_string(&w).unwrap();
        assert!(json.contains(r#""cursor":0"#), "{json}");
        let tampered = json.replace(r#""cursor":0"#, r#""cursor":99"#);
        let error = serde_json::from_str::<WorkflowExecution>(&tampered)
            .expect_err("不変条件を破る写しは復号できない");
        assert!(
            error.to_string().contains("invariant violation"),
            "実際: {error}"
        );
    }

    #[test]
    fn from_state_rejects_a_broken_invariant() {
        let w = all_exec(3);
        let base = w.state();

        let empty = WorkflowExecutionStateBuilder::new(
            intent(),
            def_id("claude"),
            revision('0'),
            Vec::new(),
        )
        .build();
        assert!(matches!(
            WorkflowExecution::from_state(empty),
            Err(StateError::InvariantViolation(_))
        ));

        for broken in [
            WorkflowExecutionStateBuilder::new(
                intent(),
                def_id("claude"),
                revision('0'),
                w.stages().to_vec(),
            )
            .checkbox(vec![InProgress])
            .build(),
            WorkflowExecutionStateBuilder::new(
                intent(),
                def_id("claude"),
                revision('0'),
                w.stages().to_vec(),
            )
            .cursor(9)
            .build(),
            WorkflowExecutionStateBuilder::new(
                intent(),
                def_id("claude"),
                revision('0'),
                w.stages().to_vec(),
            )
            .overlay(vec![Skip, Execute, Execute])
            .build(),
            WorkflowExecutionStateBuilder::new(
                intent(),
                def_id("claude"),
                revision('0'),
                w.stages().to_vec(),
            )
            .checkbox(vec![InProgress, InProgress, Pending])
            .build(),
            WorkflowExecutionStateBuilder::new(
                intent(),
                def_id("claude"),
                revision('0'),
                w.stages().to_vec(),
            )
            .checkbox(vec![InProgress, Completed, Pending])
            .build(),
            WorkflowExecutionStateBuilder::new(
                intent(),
                def_id("claude"),
                revision('0'),
                w.stages().to_vec(),
            )
            .parked_at(Some(2))
            .build(),
            WorkflowExecutionStateBuilder::new(
                intent(),
                def_id("claude"),
                revision('0'),
                w.stages().to_vec(),
            )
            .parked_at(Some(9))
            .build(),
            WorkflowExecutionStateBuilder::new(
                intent(),
                def_id("claude"),
                revision('0'),
                w.stages().to_vec(),
            )
            .approved(vec![false])
            .build(),
            WorkflowExecutionStateBuilder::new(
                intent(),
                def_id("claude"),
                revision('0'),
                w.stages().to_vec(),
            )
            .revision_count(vec![0, 0])
            .build(),
            WorkflowExecutionStateBuilder::new(
                intent(),
                def_id("claude"),
                revision('0'),
                w.stages().to_vec(),
            )
            .seq_nr(0)
            .build(),
            WorkflowExecutionStateBuilder::new(
                intent(),
                def_id("claude"),
                revision('0'),
                w.stages().to_vec(),
            )
            .plan(vec![Skip, Execute, Execute])
            .build(),
            WorkflowExecutionStateBuilder::new(
                intent(),
                def_id("claude"),
                revision('0'),
                w.stages().to_vec(),
            )
            .conditional(vec![true, false, false])
            .build(),
        ] {
            assert!(
                matches!(
                    WorkflowExecution::from_state(broken),
                    Err(StateError::InvariantViolation(_))
                ),
                "a broken state must be refused"
            );
        }

        assert!(WorkflowExecution::from_state(base).is_ok());
    }

    // ---- W4: next_decision (BR3.1 / BR2.6) ----

    #[test]
    fn next_decision_refuses_a_different_definition() {
        let w = all_exec(3);
        let other = bare_definition("kiro");
        assert_eq!(
            w.next_decision(&other, &NextRequest::default()),
            Err(CommandError::DefinitionMismatch {
                expected: def_id("claude"),
                actual: def_id("kiro"),
            })
        );
    }

    #[test]
    fn a_newer_revision_of_the_same_definition_is_accepted() {
        let w = all_exec(3);
        let drifted = WorkflowDefinition::new(
            def_id("claude"),
            revision('f'),
            StageGraph::new(Vec::new()).unwrap(),
            ScopeGrid::new(BTreeMap::new()),
            BTreeMap::new(),
        );
        assert!(w.next_decision(&drifted, &NextRequest::default()).is_ok());
    }

    #[test]
    fn next_decision_walks_the_branches_in_priority_order() {
        let definition = bare_definition("claude");
        let mut w = all_exec(3);

        // (6) cursor が in-flight
        assert_eq!(
            w.next_decision(&definition, &NextRequest::default()),
            Ok(NextDecision::RunStage {
                stage: at(&w, 0),
                gate: false
            })
        );
        // (1) park 中
        w.park(occurred()).unwrap();
        assert_eq!(
            w.next_decision(&definition, &NextRequest::default()),
            Ok(NextDecision::Parked { stage: at(&w, 0) })
        );
        assert_eq!(
            w.next_decision(&definition, &NextRequest::new(true, false, false)),
            Ok(NextDecision::UnparkThenResume)
        );
        // 再入フラグは park ガードを外す
        assert_eq!(
            w.next_decision(&definition, &NextRequest::new(false, true, false)),
            Ok(NextDecision::RunStage {
                stage: at(&w, 0),
                gate: false
            })
        );
        w.unpark(occurred()).unwrap();
        // (2) resume
        assert_eq!(
            w.next_decision(&definition, &NextRequest::new(true, false, false)),
            Ok(NextDecision::ResumeMenu)
        );
        // (3) 自由記述
        assert_eq!(
            w.next_decision(&definition, &NextRequest::new(false, false, true)),
            Ok(NextDecision::NewWorkRouting)
        );
        // (7) 次の in-scope / gate = true
        w.complete_stage(occurred()).unwrap();
        assert_eq!(
            w.next_decision(&definition, &NextRequest::default()),
            Ok(NextDecision::RunStage {
                stage: at(&w, 1),
                gate: true
            })
        );
        // (4) completed
        w.approve_gate(None, None, occurred()).unwrap();
        w.approve_gate(None, None, occurred()).unwrap();
        assert_eq!(w.status(), Status::Completed);
        assert_eq!(
            w.next_decision(&definition, &NextRequest::default()),
            Ok(NextDecision::Done)
        );
    }

    #[test]
    fn next_decision_reports_the_two_skip_inconsistencies() {
        // 実効 SKIP のカーソルは `cursor_in_scope` が禁じるので、集約のコマンド経由では作れない。
        // 唯一到達しうるのは「park 中 (受理述語が偽なので cursor_in_scope を検査しない) の状態を
        // 再水和し、再入フラグで park 分岐を外して問い合わせる」経路である (BR3.1 (5) の防御腕)。
        let definition = bare_definition("claude");
        let stages = all_exec(3).stages().to_vec();
        let reentry = NextRequest::new(false, true, false);

        for (marker, expected_recoverable) in [
            (InProgress, true),
            (Revising, true),
            (Pending, false),
            (AwaitingApproval, false),
        ] {
            let state = WorkflowExecutionStateBuilder::new(
                intent(),
                def_id("claude"),
                revision('0'),
                stages.clone(),
            )
            .overlay(vec![Execute, Skip, Execute])
            .checkbox(vec![Completed, marker, Pending])
            .cursor(1)
            .parked_at(Some(1))
            .seq_nr(4)
            .build();
            let w = WorkflowExecution::from_state(state).unwrap();
            let stage = at(&w, 1);
            let expected = if expected_recoverable {
                NextDecision::RecoverSkipInconsistency {
                    stage,
                    checkbox: marker,
                }
            } else {
                NextDecision::InconsistentSkip {
                    stage,
                    checkbox: marker,
                }
            };
            assert_eq!(w.next_decision(&definition, &reentry), Ok(expected));
            // どちらの不整合も Quint の DError に写る (BR3.1)。
            assert_eq!(
                EngineSignal::from(&w.next_decision(&definition, &reentry).unwrap()),
                EngineSignal::EngineError
            );
        }
    }

    #[test]
    fn jump_resolve_is_a_read_only_query() {
        let mut w = all_exec(4);
        w.complete_stage(occurred()).unwrap();
        let before = w.clone();
        assert_eq!(w.jump_resolve(at(&w, 3)), Ok(JumpDirection::Forward));
        assert_eq!(w, before);
        let out_of_scope = at(&w, 2);
        w.recompose(&[out_of_scope], occurred()).unwrap();
        assert_eq!(
            w.jump_resolve(out_of_scope),
            Err(CommandError::InvalidTarget(out_of_scope))
        );
    }

    // ---- 実グラフの索引 (NFR1.2) ----

    #[test]
    fn every_initialization_stage_is_non_gated_and_the_rest_are_gated() {
        let mut w = start_with(3, &[Execute; 6], &[false; 6]);
        for i in 0..3 {
            assert_eq!(w.gated(at(&w, i)), Some(false), "stage {i}");
        }
        for i in 3..6 {
            assert_eq!(w.gated(at(&w, i)), Some(true), "stage {i}");
        }
        // 索引 0〜2 は complete_stage で進み、open_gate は拒否される。
        for i in 0..3 {
            let cursor = at(&w, i);
            assert_eq!(w.cursor(), cursor);
            assert_eq!(
                w.open_gate(Vec::new(), occurred()),
                Err(CommandError::InvalidTarget(cursor))
            );
            w.complete_stage(occurred()).unwrap();
            assert_eq!(w.approved(cursor), Some(false));
        }
        // 索引 3 以降はゲート — complete_stage は拒否される。
        let cursor = at(&w, 3);
        assert_eq!(w.cursor(), cursor);
        assert_eq!(
            w.complete_stage(occurred()),
            Err(CommandError::InvalidTarget(cursor))
        );
        let init_target = at(&w, 1);
        assert_eq!(
            w.jump(init_target, occurred()),
            Err(CommandError::InvalidTarget(init_target))
        );
    }

    #[test]
    fn stage_index_is_only_constructed_within_range() {
        let w = all_exec(3);
        assert_eq!(w.stage_index(2).map(StageIndex::to_usize), Some(2));
        assert_eq!(w.stage_index(3), None);
        assert_eq!(w.stage_index(usize::MAX), None);
    }

    #[test]
    fn queries_about_a_foreign_stage_index_answer_none_instead_of_panicking() {
        let wide = all_exec(5);
        let narrow = all_exec(2);
        let foreign = at(&wide, 4);
        assert_eq!(narrow.checkbox(foreign), None);
        assert_eq!(narrow.approved(foreign), None);
        assert_eq!(narrow.effective_plan(foreign), None);
        assert_eq!(narrow.gated(foreign), None);
        assert_eq!(narrow.revision_count(foreign), None);
    }

    #[test]
    fn the_signal_projection_of_a_decision_matches_the_model_vocabulary() {
        let definition = bare_definition("claude");
        let w = all_exec(3);
        let decision = w
            .next_decision(&definition, &NextRequest::default())
            .unwrap();
        assert_eq!(
            EngineSignal::from(&decision),
            EngineSignal::RunStage(at(&w, 0))
        );
    }

    // ---- PBT (NFR2.2): 6 性質 + 定義側から移設した 2 性質 ----
    //
    // 生成器は合成定義 (stage_count 2〜8、initialization 1〜3 ステージ) とコマンド列 (≤ 60)。
    // シードは `PROPTEST_RNG_SEED` で固定する (scripts/coverage.sh / CI と同値)。

    use proptest::prelude::*;

    #[derive(Debug, Clone)]
    enum Cmd {
        Complete,
        OpenGate,
        Approve,
        Reject,
        Revise,
        SkipStage,
        Jump(usize),
        Park,
        Unpark,
        Recompose(usize),
        SetAutonomy(bool),
        Next,
        Stale(usize),
    }

    fn cmd_strategy(n: usize) -> impl Strategy<Value = Cmd> {
        prop_oneof![
            Just(Cmd::Complete),
            Just(Cmd::OpenGate),
            Just(Cmd::Approve),
            Just(Cmd::Reject),
            Just(Cmd::Revise),
            Just(Cmd::SkipStage),
            (0..n).prop_map(Cmd::Jump),
            Just(Cmd::Park),
            Just(Cmd::Unpark),
            (0..n).prop_map(Cmd::Recompose),
            any::<bool>().prop_map(Cmd::SetAutonomy),
            Just(Cmd::Next),
            (0..n).prop_map(Cmd::Stale),
        ]
    }

    /// 合成計画 — 索引 0..init が initialization (常に EXECUTE・非 CONDITIONAL)、残りは inception。
    fn synthetic_stages() -> impl Strategy<Value = Vec<StageEntry>> {
        (2usize..=8)
            .prop_flat_map(|count| {
                let init_max = if count < 3 { count } else { 3 };
                (Just(count), 1usize..=init_max)
            })
            .prop_flat_map(|(count, init)| {
                (
                    Just(count),
                    Just(init),
                    proptest::collection::vec(any::<bool>(), count),
                    proptest::collection::vec(any::<bool>(), count),
                )
            })
            .prop_map(|(count, init, exec_bits, cond_bits)| {
                (0..count)
                    .map(|index| {
                        let initialization = index < init;
                        let phase = if initialization {
                            PhaseId::Initialization
                        } else {
                            PhaseId::Inception
                        };
                        let execute =
                            initialization || exec_bits.get(index).copied().unwrap_or(true);
                        let conditional =
                            !initialization && cond_bits.get(index).copied().unwrap_or(false);
                        StageEntry::new(
                            slug(index),
                            phase,
                            if execute { Execute } else { Skip },
                            conditional,
                        )
                    })
                    .collect()
            })
    }

    fn start_synthetic(stages: Vec<StageEntry>) -> WorkflowExecution {
        WorkflowExecution::start_from_plan_unchecked(
            intent(),
            def_id("claude"),
            revision('0'),
            &start_request(),
            stages,
            occurred(),
        )
        .unwrap()
        .0
    }

    /// 1 コマンドを駆動する。`Err` は「発火しないアクション」なので状態は一切動かない (BR1.1 (e))。
    fn drive(
        w: &mut WorkflowExecution,
        definition: &WorkflowDefinition,
        cmd: &Cmd,
    ) -> Option<WorkflowExecutionEvent> {
        let before = w.clone();
        let outcome = match cmd {
            Cmd::Complete => w.complete_stage(occurred()),
            Cmd::OpenGate => w.open_gate(Vec::new(), occurred()),
            Cmd::Approve => w.approve_gate(None, None, occurred()),
            Cmd::Reject => w.reject_gate(None, occurred()),
            Cmd::Revise => w.revise_stage(occurred()),
            Cmd::SkipStage => w.skip_stage("pbt".to_string(), occurred()),
            Cmd::Jump(target) => match w.stage_index(*target) {
                Some(stage) => w.jump(stage, occurred()),
                None => Err(CommandError::NotRunning),
            },
            Cmd::Park => w.park(occurred()),
            Cmd::Unpark => w.unpark(occurred()),
            Cmd::Recompose(target) => match w.stage_index(*target) {
                Some(stage) => w.recompose(&[stage], occurred()),
                None => Err(CommandError::NotRunning),
            },
            Cmd::SetAutonomy(autonomous) => w.switch_autonomy(
                if *autonomous {
                    AutonomyMode::Autonomous
                } else {
                    AutonomyMode::Gated
                },
                occurred(),
            ),
            Cmd::Next => {
                let _ = w.next_decision(definition, &NextRequest::default());
                assert_eq!(*w, before, "next_decision は書き込まない");
                return None;
            }
            Cmd::Stale(target) => {
                if let Some(stage) = w.stage_index(*target) {
                    let _ = w.stale_report(stage);
                }
                assert_eq!(*w, before, "stale_report は書き込まない");
                return None;
            }
        };
        match outcome {
            Ok(event) => Some(event),
            Err(_) => {
                assert_eq!(*w, before, "Err は状態を変えない (BR1.1)");
                None
            }
        }
    }

    fn assert_quint_invariants(w: &WorkflowExecution) {
        let count = w.stage_count();
        // cursor_in_scope: コマンドを受理できる間、カーソルは実効 EXECUTE 上にある。
        if w.accepts_commands() {
            assert_eq!(
                w.effective_plan(w.cursor()),
                Some(Execute),
                "cursor_in_scope"
            );
        }
        let mut active = 0_usize;
        for value in 0..count {
            let stage = w.stage_index(value).unwrap();
            let marker = w.checkbox(stage).unwrap();
            if marker.is_active() {
                active += 1;
            }
            // no_gate_bypass: ゲート付きステージの completed は必ず承認履歴を伴う。
            if w.gated(stage) == Some(true) && marker == Completed {
                assert_eq!(w.approved(stage), Some(true), "no_gate_bypass at {value}");
            }
        }
        assert!(active <= 1, "at_most_one_active: {active}");
        // parked_position: park マーカーが活性ならカーソル位置と一致する。
        if w.parked_active() {
            assert_eq!(w.parked_at(), Some(w.cursor()), "parked_position");
        }
    }

    proptest! {
        /// (a) decide 後の状態 == 旧状態 + apply_event、(d) Quint 不変条件、(e) Err 無副作用、
        /// (f) from_state(state()) == self を全ステップで固定する。
        #[test]
        fn every_command_equals_the_old_state_plus_its_event(
            stages in synthetic_stages(),
            cmds in proptest::collection::vec(cmd_strategy(8), 1..60),
        ) {
            let definition = bare_definition("claude");
            let mut w = start_synthetic(stages);
            assert_quint_invariants(&w);
            for cmd in &cmds {
                let before = w.clone();
                if let Some(event) = drive(&mut w, &definition, cmd) {
                    let mut replayed = before;
                    replayed
                        .apply_event(w.seq_nr(), *w.last_updated_at(), &event)
                        .unwrap();
                    prop_assert_eq!(&replayed, &w);
                }
                assert_quint_invariants(&w);
                let restored = WorkflowExecution::from_state(w.state()).unwrap();
                prop_assert_eq!(&restored, &w);
            }
        }

        /// (b) リプレイの決定性 — 状態の写し (memento) + 以降のイベント列 == 通常実行 (BR2.3)。
        /// (c) seq_nr は 1 イベントにつき 1 だけ増え、順序違反は SequenceGap で拒否される (BR2.1)。
        #[test]
        fn replaying_the_event_stream_reproduces_the_executed_aggregate(
            stages in synthetic_stages(),
            cmds in proptest::collection::vec(cmd_strategy(8), 1..60),
        ) {
            let definition = bare_definition("claude");
            let mut w = start_synthetic(stages);
            let genesis = w.state();
            // 封筒の材料 (通番・発生時刻) は commit を通った集約から採る (B7 — Repository も同じ)。
            let mut events: Vec<(usize, DateTime<Utc>, WorkflowExecutionEvent)> = Vec::new();
            let mut expected_seq = w.seq_nr();
            for cmd in &cmds {
                if let Some(event) = drive(&mut w, &definition, cmd) {
                    expected_seq += 1;
                    prop_assert_eq!(w.seq_nr(), expected_seq);
                    events.push((w.seq_nr(), *w.last_updated_at(), event));
                }
            }

            let mut replayed = WorkflowExecution::from_state(genesis).unwrap();
            for (seq_nr, occurred_at, event) in &events {
                replayed.apply_event(*seq_nr, *occurred_at, event).unwrap();
            }
            prop_assert_eq!(&replayed, &w);

            // 順序違反は拒否され、状態も動かない。
            if let Some((seq_nr, occurred_at, event)) = events.first() {
                let mut fresh = replayed.clone();
                let gap = fresh.apply_event(*seq_nr, *occurred_at, event);
                let is_gap = matches!(gap, Err(ApplyError::SequenceGap { .. }));
                prop_assert!(is_gap, "順序違反は SequenceGap で拒否される");
                prop_assert_eq!(&fresh, &replayed);
            }
        }

        /// 定義側から移設した性質 (1): 実効プランはグリッドに recompose のサフィックスを重ねた値で
        /// あり、静的な `plan` は決して動かない (BR4.2)。
        #[test]
        fn the_recompose_suffix_beats_the_grid_and_the_static_plan_never_moves(
            stages in synthetic_stages(),
            cmds in proptest::collection::vec(cmd_strategy(8), 1..60),
        ) {
            let definition = bare_definition("claude");
            let grid: Vec<PlanAction> = stages.iter().map(StageEntry::plan_action).collect();
            let mut w = start_synthetic(stages);
            let mut expected = grid.clone();
            for cmd in &cmds {
                if let Some(event) = drive(&mut w, &definition, cmd)
                    && let WorkflowExecutionEvent::Recomposed(recomposed) = &event
                {
                    for slug in recomposed.skipped() {
                        let index = w.stages().iter().position(|e| e.slug() == slug).unwrap();
                        expected[index] = Skip;
                    }
                    for slug in recomposed.added() {
                        let index = w.stages().iter().position(|e| e.slug() == slug).unwrap();
                        expected[index] = Execute;
                    }
                }
                for value in 0..w.stage_count() {
                    let stage = w.stage_index(value).unwrap();
                    prop_assert_eq!(w.effective_plan(stage), Some(expected[value]));
                    prop_assert_eq!(w.stages()[value].plan_action(), grid[value]);
                }
            }
        }

        /// 定義側から移設した性質 (2): `next_decision` が名指しする先読みステージは、カーソルより
        /// 後ろで**最初**の in-scope ステージである (読み飛ばしの最小性)。無ければ `Done`。
        #[test]
        fn the_lookahead_target_is_the_first_in_scope_stage_in_document_order(
            stages in synthetic_stages(),
            cmds in proptest::collection::vec(cmd_strategy(8), 1..60),
        ) {
            let definition = bare_definition("claude");
            let mut w = start_synthetic(stages);
            for cmd in &cmds {
                drive(&mut w, &definition, cmd);
                let Ok(decision) = w.next_decision(&definition, &NextRequest::default()) else {
                    continue;
                };
                let cursor = w.cursor().to_usize();
                let cursor_in_flight = w
                    .checkbox(w.cursor())
                    .is_some_and(CheckboxState::is_in_flight);
                match decision {
                    NextDecision::RunStage { stage, gate } if stage.to_usize() != cursor => {
                        prop_assert!(stage.to_usize() > cursor);
                        for value in (cursor + 1)..stage.to_usize() {
                            let earlier = w.stage_index(value).unwrap();
                            prop_assert_ne!(w.effective_plan(earlier), Some(Execute));
                        }
                        prop_assert_eq!(w.effective_plan(stage), Some(Execute));
                        prop_assert_eq!(gate, w.gated(stage).unwrap());
                    }
                    NextDecision::Done if w.accepts_commands() && !cursor_in_flight => {
                        for value in (cursor + 1)..w.stage_count() {
                            let later = w.stage_index(value).unwrap();
                            prop_assert_ne!(w.effective_plan(later), Some(Execute));
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}
