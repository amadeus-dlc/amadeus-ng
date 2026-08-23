//! `WorkflowExecution` 集約 — 1 つの Intent の実行状態 (10 §2.1)。カーソル・CheckboxState・
//! `Status`（Running / Completed）と**直交する park マーカー**・recompose オーバレイ・
//! AutonomyMode・ゲート承認履歴を内包し、状態遷移コマンドの唯一の所有者となる (S1)。
//!
//! 意味論の形式的正本は `formal/orchestration/engine_loop.qnt` (slice 1 v2 — green・
//! mutation 3/3)。本実装は同モデルの純粋ステップ関数であり、ITF 準拠テスト
//! (`tests/engine_loop_conformance.rs`) がモデルトレースを再生して突き合わせる (ADR 0003 決定 5)。
//!
//! ステージはコンパイル済みグラフ順のインデックスで扱う (slug → index の解決は
//! ユースケース層の責務)。stage 0 = initialization (非ゲート・常時 EXECUTE・非 CONDITIONAL)。

use super::autonomy_mode::AutonomyMode;
use super::jump_direction::JumpDirection;
use crate::workflow_definition::PlanAction;
use crate::workspace::CheckboxState;

/// エンジンが放出する信号の観測射影 (モデルの `DirectiveKind` サブセット)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineSignal {
    /// 名指しのステージを走らせる。引数は文書順のステージインデックス (slug 解決は
    /// ユースケース層)。実効プランが EXECUTE のステージにしか出ない (I2)。
    RunStage(usize),
    /// ループの停止。スコープ内に未実施ステージが残っていない完了に加え、report が
    /// コミットに成功したエピローグと、何もコミットしない冪等な終端 (stale re-report) も
    /// この 1 語に畳まれる。
    Done,
    /// 意図的に park された状態。`Done` とは別で、スコープ内にはまだ未実施ステージが残る。
    Parked,
    /// plan/cursor 不整合 (実効 SKIP のステージに run-stage を出さない — I2 の拒否腕)。
    EngineError,
}

/// 状態ファイル `Status` 行の 2 値。park マーカーとは**直交**するので、これだけでは
/// 「今コマンドを受け付けるか」は決まらない (10 §2.1 — 判定は `parked_active` と併せる)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// 進行中 — スコープ内に未決着のステージが残っている。
    Running,
    /// スコープ内の最後のステージまで決着済み。以後、状態遷移コマンドは `NotRunning` で
    /// 拒否される。
    Completed,
}

/// `start` が初期状態を組み立てられない理由 (集約の生成時不変条件の違反)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StartError {
    /// stage 0 (initialization) は常時 EXECUTE (転置の特例)。
    InitializationMustExecute,
    /// initialization は execution: ALWAYS (CONDITIONAL 不可)。
    InitializationMustBeUnconditional,
    /// `plan` と `conditional` の要素数が食い違う (同一ステージ列に対する 2 つの射影で
    /// あるべきなので、長さは一致していなければならない)。
    LengthMismatch,
    /// ステージ 0 件。コンパイル済みグラフは最低でも initialization を含むため空はありえない。
    Empty,
}

/// 状態遷移コマンドの拒否理由。ガード違反は「発火しないアクション」であって状態は一切
/// 動かない (モデルの enabled 条件と同型)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandError {
    /// ワークフローが Running でない (Completed または parked が活性)。
    NotRunning,
    /// checkbox 前提の不一致 (gate-lifecycle の厳密前提 — I7)。
    CheckboxPrecondition {
        /// 前提を満たさなかったステージの文書順インデックス。
        stage: usize,
        /// そのステージの実測 checkbox。受理される前提集合はコマンドごとに異なるため、
        /// ここは期待値ではなく**観測値**を運ぶ。
        actual: CheckboxState,
    },
    /// skipped 受理条件 (CONDITIONAL でも plan SKIP でもない — I13)。
    NotSkippable(usize),
    /// stale re-report の前提不一致。
    NotStale(usize),
    /// jump / recompose の対象不正。
    InvalidTarget(usize),
    /// autonomous 下で拒否されるコマンド (park / recompose)。
    RefusedUnderAutonomy,
}

/// engine_loop.qnt slice 1 の純粋ステップ関数 (I1〜I7 / I10 の内側)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowExecution {
    plan: Vec<PlanAction>,
    overlay: Vec<PlanAction>,
    conditional: Vec<bool>,
    checkbox: Vec<CheckboxState>,
    cursor: usize,
    status: Status,
    /// park マーカー (Status と直交 — 10 §2.1。stale-by-progress は slice 2)。
    parked_at: Option<usize>,
    autonomy: AutonomyMode,
    approved: Vec<bool>,
}

impl WorkflowExecution {
    /// # Errors
    ///
    /// 空プラン・長さ不一致・stage 0 (initialization) の EXECUTE / 非 CONDITIONAL 違反を拒否する。
    pub fn start(plan: Vec<PlanAction>, conditional: Vec<bool>) -> Result<Self, StartError> {
        if plan.is_empty() {
            return Err(StartError::Empty);
        }
        if plan.len() != conditional.len() {
            return Err(StartError::LengthMismatch);
        }
        if plan[0] != PlanAction::Execute {
            return Err(StartError::InitializationMustExecute);
        }
        if conditional[0] {
            return Err(StartError::InitializationMustBeUnconditional);
        }
        let n = plan.len();
        let mut checkbox = vec![CheckboxState::Pending; n];
        checkbox[0] = CheckboxState::InProgress;
        Ok(WorkflowExecution {
            overlay: plan.clone(),
            plan,
            conditional,
            checkbox,
            cursor: 0,
            status: Status::Running,
            parked_at: None,
            autonomy: AutonomyMode::Gated,
            approved: vec![false; n],
        })
    }

    // ---- 観測 (read model) ----

    /// コンパイル済みグラフのステージ総数。本集約が扱うインデックス空間は `0..stage_count`
    /// のみで、スコープ外 (実効 SKIP) のステージもこの数に含まれる。
    #[must_use]
    pub const fn stage_count(&self) -> usize {
        self.plan.len()
    }

    /// `Current Stage` の文書順インデックス (モデルの cursor)。Running かつ非 parked の
    /// 間は必ず実効 EXECUTE のステージを指す (`cursor_in_scope`)。
    #[must_use]
    pub const fn cursor(&self) -> usize {
        self.cursor
    }

    /// `Status` 行の現在値。park マーカーとは直交するので、`Running` でも parked が活性なら
    /// コマンドは受け付けない。
    #[must_use]
    pub const fn status(&self) -> Status {
        self.status
    }

    /// 名指しステージの checkbox マーカーの現在値。アクティブ (in-progress /
    /// awaiting-approval / revising) なステージは全体で高々 1 つ (I6)。
    #[must_use]
    pub fn checkbox(&self, stage: usize) -> CheckboxState {
        self.checkbox[stage]
    }

    /// 名指しステージのゲート承認履歴 (`GATE_APPROVED` 監査行の射影)。gated ステージの
    /// completed は必ずこれを伴い、backward jump / redo は取り消す (I3)。
    #[must_use]
    pub fn approved(&self, stage: usize) -> bool {
        self.approved[stage]
    }

    /// 現在の `Construction Autonomy Mode`。初期値は gated — 付与は明示のコマンドのみで
    /// 推論されない。
    #[must_use]
    pub const fn autonomy(&self) -> AutonomyMode {
        self.autonomy
    }

    /// park マーカーが記録している位置 (park した時点のカーソル)。`None` は未 park。
    /// カーソルが先へ進んでもマーカーは残るため、これ単体は発火条件ではない。
    #[must_use]
    pub const fn parked_at(&self) -> Option<usize> {
        self.parked_at
    }

    /// parked 分岐の発火は導出述語 (マーカー有 ∧ 位置一致 — 10 §2.1)。
    #[must_use]
    pub fn parked_active(&self) -> bool {
        self.parked_at == Some(self.cursor)
    }

    /// gated(s) — initialization (stage 0) のみ非ゲート。
    #[must_use]
    pub const fn gated(&self, stage: usize) -> bool {
        stage != 0
    }

    /// `effectivePlanAction` — オーバレイ (recompose) が grid に勝つ (裁定 B1)。
    #[must_use]
    pub fn effective_plan(&self, stage: usize) -> PlanAction {
        self.overlay[stage]
    }

    fn in_scope(&self, stage: usize) -> bool {
        self.effective_plan(stage) == PlanAction::Execute
    }

    fn next_in_scope(&self, after: usize) -> Option<usize> {
        ((after + 1)..self.stage_count()).find(|&s| self.in_scope(s))
    }

    fn running(&self) -> bool {
        self.status == Status::Running && !self.parked_active()
    }

    // ---- next (読み取り専用 — I8: &self が型レベルの保証) ----

    /// 現状態から放出すべき信号をちょうど 1 つ決める (`handleNext` ラダーの slice 1 射影)。
    /// 判定順は parked → completed → in-flight カーソル → 次の in-scope ステージ。
    #[must_use]
    pub fn next(&self) -> EngineSignal {
        if self.parked_active() {
            return EngineSignal::Parked;
        }
        if self.status == Status::Completed {
            return EngineSignal::Done;
        }
        let cb = self.checkbox[self.cursor];
        if cb.is_in_flight() {
            if self.effective_plan(self.cursor) == PlanAction::Skip {
                return EngineSignal::EngineError;
            }
            return EngineSignal::RunStage(self.cursor);
        }
        match self.next_in_scope(self.cursor) {
            None => EngineSignal::Done,
            Some(s) => EngineSignal::RunStage(s),
        }
    }

    // ---- report (書込半分 — ディスパッチ表 3.2 / gate-lifecycle 表 3.1) ----

    /// forward verdict: gated は承認経由、advance / complete-workflow はエンジンが選ぶ。
    /// # Errors
    ///
    /// 非 Running (`NotRunning`)、または checkbox が in-progress / awaiting-approval 以外
    /// (`CheckboxPrecondition`) を拒否する。
    pub fn report_forward(&mut self) -> Result<EngineSignal, CommandError> {
        if !self.running() {
            return Err(CommandError::NotRunning);
        }
        let s = self.cursor;
        let cb = self.checkbox[s];
        // I7 ゲート前提 — forward が受理する checkbox 集合は本集約が所有する遷移の前提であって、
        // CheckboxState の一般分類 (in-flight / finished / active) ではない。
        // amadeus-lint: allow(checkbox-vocabulary) — 上記により述語化せず前提集合を明示する
        if !matches!(
            cb,
            CheckboxState::InProgress | CheckboxState::AwaitingApproval
        ) {
            return Err(CommandError::CheckboxPrecondition {
                stage: s,
                actual: cb,
            });
        }
        if self.gated(s) {
            self.approved[s] = true;
        }
        self.checkbox[s] = CheckboxState::Completed;
        match self.next_in_scope(s) {
            None => {
                self.status = Status::Completed;
            }
            Some(nxt) => {
                self.checkbox[nxt] = CheckboxState::InProgress;
                self.cursor = nxt;
            }
        }
        Ok(EngineSignal::Done)
    }

    /// `awaiting-approval` — 「only an in-progress stage can open a gate」。
    /// # Errors
    ///
    /// 非 Running・非ゲート・in-progress 以外 (「only an in-progress stage can open a gate」) を拒否する。
    pub fn gate_start(&mut self) -> Result<(), CommandError> {
        if !self.running() {
            return Err(CommandError::NotRunning);
        }
        let s = self.cursor;
        if !self.gated(s) {
            return Err(CommandError::InvalidTarget(s));
        }
        if self.checkbox[s] != CheckboxState::InProgress {
            return Err(CommandError::CheckboxPrecondition {
                stage: s,
                actual: self.checkbox[s],
            });
        }
        self.checkbox[s] = CheckboxState::AwaitingApproval;
        Ok(())
    }

    /// `rejected` — in-progress | awaiting-approval から revising へ。
    /// # Errors
    ///
    /// 非 Running・非ゲート (initialization)・in-progress / awaiting-approval 以外を拒否する。
    pub fn reject(&mut self) -> Result<(), CommandError> {
        if !self.running() {
            return Err(CommandError::NotRunning);
        }
        let s = self.cursor;
        if !self.gated(s) {
            return Err(CommandError::InvalidTarget(s));
        }
        let cb = self.checkbox[s];
        // I7 ゲート前提 — reject が受理する checkbox 集合は本集約が所有する遷移の前提であって、
        // CheckboxState の一般分類 (in-flight / finished / active) ではない。
        // amadeus-lint: allow(checkbox-vocabulary) — 上記により述語化せず前提集合を明示する
        if !matches!(
            cb,
            CheckboxState::InProgress | CheckboxState::AwaitingApproval
        ) {
            return Err(CommandError::CheckboxPrecondition {
                stage: s,
                actual: cb,
            });
        }
        self.checkbox[s] = CheckboxState::Revising;
        Ok(())
    }

    /// `revised` — 「only a revising stage can re-enter its gate」。
    /// # Errors
    ///
    /// revising でなければ `CheckboxPrecondition` (「only a revising stage can re-enter its gate」)。
    pub fn revise(&mut self) -> Result<(), CommandError> {
        if !self.running() {
            return Err(CommandError::NotRunning);
        }
        let s = self.cursor;
        if self.checkbox[s] != CheckboxState::Revising {
            return Err(CommandError::CheckboxPrecondition {
                stage: s,
                actual: self.checkbox[s],
            });
        }
        self.checkbox[s] = CheckboxState::AwaitingApproval;
        Ok(())
    }

    /// `skipped` — routed lifecycle outcome (CONDITIONAL または実効 SKIP のみ — I13)。
    /// # Errors
    ///
    /// 非 Running・checkbox 前提違反・CONDITIONAL でも plan SKIP でもない場合 (`NotSkippable`) を拒否する。
    pub fn report_skipped(&mut self) -> Result<EngineSignal, CommandError> {
        if !self.running() {
            return Err(CommandError::NotRunning);
        }
        let s = self.cursor;
        let cb = self.checkbox[s];
        // I13 skipped 受理前提 — 本集約が所有する遷移の前提集合であって、
        // CheckboxState の一般分類 (in-flight / finished / active) ではない。
        // amadeus-lint: allow(checkbox-vocabulary) — 上記により述語化せず前提集合を明示する
        if !matches!(cb, CheckboxState::InProgress | CheckboxState::Revising) {
            return Err(CommandError::CheckboxPrecondition {
                stage: s,
                actual: cb,
            });
        }
        if !(self.conditional[s] || self.effective_plan(s) == PlanAction::Skip) {
            return Err(CommandError::NotSkippable(s));
        }
        self.checkbox[s] = CheckboxState::Skipped;
        match self.next_in_scope(s) {
            None => {
                self.status = Status::Completed;
            }
            Some(nxt) => {
                self.checkbox[nxt] = CheckboxState::InProgress;
                self.cursor = nxt;
            }
        }
        Ok(EngineSignal::Done)
    }

    /// stale re-report — カーソル通過済み completed への再報告は**何もコミットせず**冪等 done
    /// (I5。&self が「何もコミットしない」の型レベル保証)。
    /// # Errors
    ///
    /// 非 Running、またはカーソル通過済み completed でない対象は `NotStale`。
    pub fn stale_report(&self, stage: usize) -> Result<EngineSignal, CommandError> {
        if !self.running() {
            return Err(CommandError::NotRunning);
        }
        if stage >= self.cursor || self.checkbox[stage] != CheckboxState::Completed {
            return Err(CommandError::NotStale(stage));
        }
        Ok(EngineSignal::Done)
    }

    // ---- jump (resolve は導出、execute は効果 — 02 §8) ----

    /// # Errors
    ///
    /// 非 Running・範囲外/initialization/スコープ外ターゲット (`InvalidTarget`) を拒否する。
    ///
    /// # Panics
    ///
    /// 内部インデックスは `stage_count` 検証後のため panic しない (検証はガードが先行)。
    pub fn jump(&mut self, target: usize) -> Result<JumpDirection, CommandError> {
        if !self.running() {
            return Err(CommandError::NotRunning);
        }
        if target >= self.stage_count() {
            return Err(CommandError::InvalidTarget(target));
        }
        let direction = JumpDirection::derive(self.cursor, target);
        match direction {
            JumpDirection::Forward | JumpDirection::Backward => {
                // INIT_JUMP_ERROR: initialization へは跳べない。scope 外もエラー。
                if target == 0 || !self.in_scope(target) {
                    return Err(CommandError::InvalidTarget(target));
                }
            }
            JumpDirection::Redo => {
                if self.cursor == 0 {
                    return Err(CommandError::InvalidTarget(target));
                }
            }
        }
        match direction {
            JumpDirection::Forward => {
                let cur = self.cursor;
                for u in cur..target {
                    let cb = self.checkbox[u];
                    let skip_current = u == cur && cb.is_active();
                    let skip_between = u > cur && cb.is_in_flight();
                    if skip_current || skip_between {
                        self.checkbox[u] = CheckboxState::Skipped;
                    }
                }
            }
            JumpDirection::Backward => {
                for u in (target + 1)..self.stage_count() {
                    if self.in_scope(u) && self.checkbox[u] != CheckboxState::Pending {
                        self.checkbox[u] = CheckboxState::Pending;
                    }
                }
                // backward jump は承認履歴を無効化する (I3 の後段)
                for u in target..self.stage_count() {
                    self.approved[u] = false;
                }
            }
            JumpDirection::Redo => {
                self.approved[self.cursor] = false;
            }
        }
        self.checkbox[target] = CheckboxState::InProgress;
        self.cursor = target;
        Ok(direction)
    }

    // ---- park / unpark (02 §11) ----

    /// autonomous 中は拒否 —「無人実行に再開する人間はいない」。
    /// # Errors
    ///
    /// 非 Running は `NotRunning`、autonomous 中は `RefusedUnderAutonomy`。
    pub fn park(&mut self) -> Result<EngineSignal, CommandError> {
        if !self.running() {
            return Err(CommandError::NotRunning);
        }
        if self.autonomy.is_autonomous() {
            return Err(CommandError::RefusedUnderAutonomy);
        }
        self.parked_at = Some(self.cursor);
        Ok(EngineSignal::Parked)
    }

    /// # Errors
    ///
    /// parked が活性でなければ `NotRunning`。
    pub fn unpark(&mut self) -> Result<(), CommandError> {
        if !self.parked_active() {
            return Err(CommandError::NotRunning);
        }
        self.parked_at = None;
        Ok(())
    }

    // ---- recompose (in-flight の計画再形成 — slice 1 は 8 ガード中 4) ----

    /// # Errors
    ///
    /// 8 ガードのうち slice 1 実装分: 非 Running・autonomous 中 (`RefusedUnderAutonomy`)・
    /// カーソル以前や範囲外 (`InvalidTarget`)・pending でない (`CheckboxPrecondition`) を拒否する。
    pub fn recompose_flip(&mut self, stage: usize) -> Result<(), CommandError> {
        if !self.running() {
            return Err(CommandError::NotRunning);
        }
        if self.autonomy.is_autonomous() {
            return Err(CommandError::RefusedUnderAutonomy);
        }
        if stage <= self.cursor || stage >= self.stage_count() {
            return Err(CommandError::InvalidTarget(stage));
        }
        if self.checkbox[stage] != CheckboxState::Pending {
            return Err(CommandError::CheckboxPrecondition {
                stage,
                actual: self.checkbox[stage],
            });
        }
        self.overlay[stage] = self.overlay[stage].flipped();
        Ok(())
    }

    // ---- autonomy ----

    /// human-presence ガード (I11) はユースケース層 (監査台帳の射影が必要) — ここは状態変更のみ。
    /// # Errors
    ///
    /// Running (かつ非 parked) でなければ `NotRunning`。
    pub fn set_autonomy(&mut self, mode: AutonomyMode) -> Result<(), CommandError> {
        if !self.running() {
            return Err(CommandError::NotRunning);
        }
        self.autonomy = mode;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use PlanAction::Execute;

    fn wf(plan: &[PlanAction], conditional: &[bool]) -> WorkflowExecution {
        WorkflowExecution::start(plan.to_vec(), conditional.to_vec()).unwrap()
    }

    fn all_exec(n: usize) -> WorkflowExecution {
        wf(&vec![Execute; n], &vec![false; n])
    }

    #[test]
    fn a_gated_stage_cannot_complete_without_passing_through_approval() {
        let mut w = all_exec(3);
        w.report_forward().unwrap(); // stage 0 (initialization, 非ゲート)
        assert_eq!(w.cursor(), 1);
        // gated ステージの forward は承認履歴を残す (I3: completed ⇒ approved)
        w.gate_start().unwrap();
        w.report_forward().unwrap();
        assert!(w.approved(1));
        assert_eq!(w.checkbox(1), CheckboxState::Completed);
    }

    #[test]
    fn gate_lifecycle_preconditions_are_strict() {
        let mut w = all_exec(3);
        w.report_forward().unwrap();
        // revising でないステージの revise は拒否
        assert!(matches!(
            w.revise(),
            Err(CommandError::CheckboxPrecondition { .. })
        ));
        w.gate_start().unwrap();
        // awaiting-approval からの gate_start 再実行は拒否 (in-progress のみ開ける)
        assert!(matches!(
            w.gate_start(),
            Err(CommandError::CheckboxPrecondition { .. })
        ));
        w.reject().unwrap();
        assert_eq!(w.checkbox(1), CheckboxState::Revising);
        w.revise().unwrap();
        assert_eq!(w.checkbox(1), CheckboxState::AwaitingApproval);
    }

    #[test]
    fn skipped_is_refused_unless_conditional_or_plan_skip() {
        let mut w = wf(&[Execute, Execute, Execute], &[false, false, true]);
        w.report_forward().unwrap();
        assert_eq!(w.report_skipped(), Err(CommandError::NotSkippable(1)));
        w.report_forward().unwrap();
        // stage 2 は CONDITIONAL — 自己スキップ可、そして最終なので完了
        w.report_skipped().unwrap();
        assert_eq!(w.status(), Status::Completed);
        assert_eq!(w.checkbox(2), CheckboxState::Skipped);
    }

    #[test]
    fn stale_rereport_yields_done_and_commits_nothing() {
        let mut w = all_exec(3);
        w.report_forward().unwrap();
        let before = w.clone();
        assert_eq!(w.stale_report(0), Ok(EngineSignal::Done));
        assert_eq!(w, before); // フレーム条件 (I5) — &self なので構造的にも保証
        assert!(matches!(w.stale_report(1), Err(CommandError::NotStale(1))));
    }

    #[test]
    fn forward_jump_skips_intervening_in_flight_stages() {
        let mut w = all_exec(5);
        w.report_forward().unwrap(); // cursor → 1 (in-progress)
        let dir = w.jump(3).unwrap();
        assert_eq!(dir, JumpDirection::Forward);
        assert_eq!(w.checkbox(1), CheckboxState::Skipped); // 現ステージ (in-progress) は skip
        assert_eq!(w.checkbox(2), CheckboxState::Skipped); // 介在 Pending も skip (v2 の忠実性修正)
        assert_eq!(w.checkbox(3), CheckboxState::InProgress);
        assert_eq!(w.cursor(), 3);
    }

    #[test]
    fn backward_jump_resets_downstream_and_invalidates_approvals() {
        let mut w = all_exec(4);
        w.report_forward().unwrap();
        w.gate_start().unwrap();
        w.report_forward().unwrap(); // stage1 completed+approved, cursor → 2
        w.jump(1).unwrap();
        assert_eq!(w.checkbox(1), CheckboxState::InProgress);
        assert_eq!(w.checkbox(2), CheckboxState::Pending);
        assert!(!w.approved(1)); // 承認履歴の無効化 (no_gate_bypass の裏面)
    }

    #[test]
    fn jump_to_initialization_is_refused() {
        let mut w = all_exec(3);
        w.report_forward().unwrap();
        assert_eq!(w.jump(0), Err(CommandError::InvalidTarget(0)));
    }

    #[test]
    fn park_preserves_position_and_autonomous_park_is_refused() {
        let mut w = all_exec(3);
        w.report_forward().unwrap();
        assert_eq!(w.park(), Ok(EngineSignal::Parked));
        assert!(w.parked_active());
        assert_eq!(w.next(), EngineSignal::Parked);
        // parked 中のコマンドは拒否
        assert_eq!(w.report_forward(), Err(CommandError::NotRunning));
        w.unpark().unwrap();
        assert_eq!(w.cursor(), 1); // 位置保存 (I4)
        w.set_autonomy(AutonomyMode::Autonomous).unwrap();
        assert_eq!(w.park(), Err(CommandError::RefusedUnderAutonomy));
    }

    #[test]
    fn recompose_flips_only_pending_stages_ahead_of_the_cursor() {
        let mut w = all_exec(4);
        w.report_forward().unwrap(); // cursor 1
        assert_eq!(w.recompose_flip(1), Err(CommandError::InvalidTarget(1))); // カーソル位置は不可
        w.recompose_flip(2).unwrap();
        assert_eq!(w.effective_plan(2), PlanAction::Skip);
        assert_eq!(w.effective_plan(3), PlanAction::Execute); // grid は不変、オーバレイのみ
        // SKIP になったステージは next がルーティングから外す (I2)
        w.report_forward().unwrap(); // stage1 done → next in scope は 3
        assert_eq!(w.cursor(), 3);
        // autonomy 中の recompose は拒否
        w.set_autonomy(AutonomyMode::Autonomous).unwrap();
        assert_eq!(w.recompose_flip(3), Err(CommandError::RefusedUnderAutonomy));
    }

    // ---- PBT: ランダムコマンド列の下で Quint 不変条件の Rust 版が全ステップ成立する ----
    // (engine_loop.qnt の cursor_in_scope / no_gate_bypass / at_most_one_active /
    //  parked_position を実装レベルで再検査する。単体テストは基本 PBT — オーナー規約)

    use proptest::prelude::*;

    #[derive(Debug, Clone)]
    enum Cmd {
        Next,
        ReportForward,
        GateStart,
        Reject,
        Revise,
        ReportSkipped,
        Stale(usize),
        Jump(usize),
        Park,
        Unpark,
        RecomposeFlip(usize),
        SetAutonomy(bool),
    }

    fn cmd_strategy(n: usize) -> impl Strategy<Value = Cmd> {
        prop_oneof![
            Just(Cmd::Next),
            Just(Cmd::ReportForward),
            Just(Cmd::GateStart),
            Just(Cmd::Reject),
            Just(Cmd::Revise),
            Just(Cmd::ReportSkipped),
            (0..n).prop_map(Cmd::Stale),
            (0..n).prop_map(Cmd::Jump),
            Just(Cmd::Park),
            Just(Cmd::Unpark),
            (0..n).prop_map(Cmd::RecomposeFlip),
            any::<bool>().prop_map(Cmd::SetAutonomy),
        ]
    }

    fn apply(w: &mut WorkflowExecution, cmd: &Cmd) {
        // ガード違反の Err は「発火しないアクション」— モデルの enabled 条件と同型
        let _ = match cmd {
            Cmd::Next => {
                let _ = w.next();
                Ok(())
            }
            Cmd::ReportForward => w.report_forward().map(|_| ()),
            Cmd::GateStart => w.gate_start(),
            Cmd::Reject => w.reject(),
            Cmd::Revise => w.revise(),
            Cmd::ReportSkipped => w.report_skipped().map(|_| ()),
            Cmd::Stale(s) => w.stale_report(*s).map(|_| ()),
            Cmd::Jump(t) => w.jump(*t).map(|_| ()),
            Cmd::Park => w.park().map(|_| ()),
            Cmd::Unpark => w.unpark(),
            Cmd::RecomposeFlip(s) => w.recompose_flip(*s),
            Cmd::SetAutonomy(b) => w.set_autonomy(if *b {
                AutonomyMode::Autonomous
            } else {
                AutonomyMode::Gated
            }),
        };
    }

    fn assert_invariants(w: &WorkflowExecution) {
        // cursor_in_scope: Running 中のカーソルは常に実効 EXECUTE 上
        if w.status() == Status::Running && !w.parked_active() {
            assert_eq!(
                w.effective_plan(w.cursor()),
                PlanAction::Execute,
                "cursor_in_scope"
            );
        }
        // no_gate_bypass: gated ステージの completed は必ず承認履歴を持つ
        for s in 0..w.stage_count() {
            if w.gated(s) && w.checkbox(s) == CheckboxState::Completed {
                assert!(w.approved(s), "no_gate_bypass at stage {s}");
            }
        }
        // at_most_one_active
        let active = (0..w.stage_count())
            .filter(|&s| {
                matches!(
                    w.checkbox(s),
                    CheckboxState::InProgress
                        | CheckboxState::AwaitingApproval
                        | CheckboxState::Revising
                )
            })
            .count();
        assert!(active <= 1, "at_most_one_active: {active}");
        // parked_position: park マーカーが活性ならカーソル位置と一致
        if w.parked_active() {
            assert_eq!(w.parked_at(), Some(w.cursor()), "parked_position");
        }
    }

    proptest! {
        #[test]
        fn quint_invariants_hold_under_random_command_sequences(
            exec_bits in proptest::collection::vec(any::<bool>(), 4),
            cond_bits in proptest::collection::vec(any::<bool>(), 4),
            cmds in proptest::collection::vec(cmd_strategy(5), 1..60),
        ) {
            let mut plan = vec![Execute];
            plan.extend(exec_bits.iter().map(|&b| if b { Execute } else { PlanAction::Skip }));
            let mut conditional = vec![false];
            conditional.extend(cond_bits.iter().copied());
            let mut w = WorkflowExecution::start(plan, conditional).unwrap();
            assert_invariants(&w);
            for cmd in &cmds {
                apply(&mut w, cmd);
                assert_invariants(&w);
            }
        }

        /// stale re-report は状態を一切変えない (I5 のフレーム条件 — &self に加えて実測でも)
        #[test]
        fn stale_report_never_mutates(
            cmds in proptest::collection::vec(cmd_strategy(5), 0..40),
            probe in 0usize..5,
        ) {
            let mut w = all_exec(5);
            for cmd in &cmds {
                apply(&mut w, cmd);
            }
            let before = w.clone();
            let _ = w.stale_report(probe);
            assert_eq!(w, before);
        }
    }

    #[test]
    fn next_is_read_only_and_routes_by_effective_plan() {
        let w = all_exec(2);
        assert_eq!(w.next(), EngineSignal::RunStage(0));
        // &self — 呼んでも状態は変わらない (I8 の型レベル保証はシグネチャそのもの)
    }
}
