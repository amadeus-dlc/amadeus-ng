// engine_loop.qnt — orchestration コンテキスト第一陣モデル (slice 1, v2)
//
// 対象: エンジンループの核 — カーソル×CheckboxState×PlanAction(overlay 合成)×ゲート×
//       park/jump/recompose×stale re-report。upstream aidlc-workflows v2 @ 3c3146cf の
//       02-orchestration-engine.md を抽象化 (根拠: docs/specs/research/orchestration-*.md)。
// v2.2 (2026-09-01): 誕生 = 初期化完了済み (issue #76 / オーナー裁定) — init が stage 0 を
//   CompletedBox で生み、カーソルは最初の in-scope 実ステージに立つ。書き面 (集約) と
//   読み面 (RMU 誕生投影) の「いまどこ」を一致させ、誕生直後の report が initialization を
//   再完了して STAGE_COMPLETED を二重記録する経路を構成不能にした。
// v2.1 (2026-08-30): actJumpForward の介在条件へ inScope(u) を追加 — upstream 実バイト
//   (cli/jump/execute-forward-across-phases: 実効 SKIP の介在行は `[ ]` のまま・
//   STAGE_SKIPPED 行も出ない) が正本であり、観測互換はモデル記述に常に勝つ。
//   イベント痩身 (issue #56: 事実のみ — 適用側が読み飛ばし列を導出) で乖離が顕在化した。
// v2 (2026-08-22): 敵対的レビュー + mutation テストの結果を反映 —
//   - actJumpForward の忠実性修正 (介在 Pending も skipped、非 Pending 条件は現ステージ専用)
//   - 空回り不変条件の是正: cursor_in_scope (状態レベル) を追加、stale re-report は
//     前状態スナップショット (prev*) によるフレーム条件で検査
//   - actReportSkipped は done を返す (upstream: spawn 成功のエピローグは done)
//   - init の conditional から stage 0 (initialization は ALWAYS) を除外
//   - gate-lifecycle の checkbox 前提を named invariant 化 (mutation 検出可能)
//
// モデル型 ↔ Rust Domain Primitive 対応表 (ADR 0003 決定 6):
//   CheckboxState  ↔ CheckboxState (6 値 enum、01 §3.3 E1)
//   PlanAction     ↔ PlanAction (EXECUTE/SKIP、01 §3.1)
//   overlay        ↔ recompose のステージ別サフィックス (effectivePlanAction、裁定 B1)
//   WorkflowStatus ↔ 【簡約】upstream は Status(Running/Completed) × Parked マーカーの直交
//                    2 フィールド。本モデルの 3 値は「マーカー有 ∧ Parked At Stage == Current
//                    Stage」の合成状態のみを表す。stale-by-progress parked は slice 2
//   DirectiveKind  ↔ Directive enum の観測射影 (run-stage/done/parked/error のみ、slice 1)
//   approved       ↔ ゲート承認履歴 (GATE_APPROVED 監査行の射影)
//   lastAction     ↔ ITF リプレイ用アクション記録 (ADR 0003 決定 5 の lastAction 規約)
//   prev*          ↔ (モデル専用) フレーム条件検査用の前状態スナップショット。Rust 対応物なし
//
// slice 1 の抽象化と既知の恒偽枝 (第二陣以降で精密化):
//   - stage 0 = initialization (非ゲート・常時 EXECUTE・非 CONDITIONAL)。skeleton stance /
//     gate:"unresolved" は未モデル化
//   - per-unit 反復・swarm・load-steering/continue_token・human-presence・Build-and-Test
//     loop-back は未モデル化
//   - recompose の 8 ガードのうち 4 (Running/非autonomous/pending/前方) のみ
//   - 到達不能枝 (意図どおり): actNext の DError 枝、actReportSkipped の
//     effectivePlan == SkipPlan 腕、actReportForward の非 gated 腕 (v2.2 — 誕生 =
//     初期化完了済みにより cursor が stage 0 の in-flight に乗る歴史が構成不能になった。
//     upstream 自身も一般分岐として持つ実行時到達不能コードの鏡として存置)
//   - actNext の完了済みカーソル枝は v2.2 で到達可能になった (縮退誕生 — 実ステージが
//     全て実効 SKIP のとき cursor 0 が CompletedBox のまま Running で生まれる)
//   - skipped 受理条件 e の [S] 再報告 (upstream は冪等再 skip を受理) は省略 —
//     slice 1 では Running 中に cursor が SkippedBox に乗らないため
//
// 検査力の確認 (mutation テスト、v2 で実施):
//   - recompose 前方ガード除去 → cursor_in_scope が検出
//   - actStaleReport への状態コミット注入 → stale_rereport_frame が検出
//
// E4 トレーサビリティ (10-orchestration.md §6 から参照される定義名):
//   engine_loop::no_run_stage_for_skip / cursor_in_scope / no_gate_bypass /
//   gate_lifecycle_preconditions / parked_position / unpark_restores_position /
//   stale_rereport_yields_done / stale_rereport_frame / at_most_one_active

module engine_loop {
  pure val N = 5
  pure val STAGES = 0.to(N - 1)

  type CheckboxState = Pending | InProgress | AwaitingApproval | Revising | CompletedBox | SkippedBox
  type PlanAction = Execute | SkipPlan
  type WorkflowStatus = Running | WorkflowCompleted | WorkflowParked
  type DirectiveKind = DRunStage(int) | DDone | DParked | DError | DNone

  var plan: int -> PlanAction        // 静的 scope grid (immutable — 裁定 B1)
  var overlay: int -> PlanAction     // recompose オーバレイ (実効プランの源)
  var conditional: int -> bool       // execution: CONDITIONAL 宣言 (stage 0 は常に false)
  var checkbox: int -> CheckboxState
  var cursor: int
  var status: WorkflowStatus
  var parkedAt: int                  // -1 = 非 park
  var autonomous: bool               // Construction Autonomy Mode
  var approved: int -> bool          // ゲート承認履歴
  var lastDirective: DirectiveKind
  var lastAction: str                // ITF リプレイ駆動用

  // 前状態スナップショット (フレーム条件検査用。全アクションが snapshot と合成される)
  var prevCheckbox: int -> CheckboxState
  var prevCursor: int
  var prevOverlay: int -> PlanAction
  var prevApproved: int -> bool
  var prevStatus: WorkflowStatus
  var prevParkedAt: int
  var prevAutonomous: bool

  def gated(s: int): bool = s != 0
  def effectivePlan(s: int): PlanAction = overlay.get(s)
  def inScope(s: int): bool = effectivePlan(s) == Execute
  def isInFlight(cb: CheckboxState): bool = or {
    cb == Pending, cb == InProgress, cb == AwaitingApproval, cb == Revising
  }
  def nextInScope(after: int): int =
    STAGES.filter(s => s > after and inScope(s))
      .fold(-1, (acc, s) => if (acc == -1 or s < acc) s else acc)

  // 誕生 = 初期化完了済み (issue #76 / オーナー裁定 2026-09-01、v2.2): upstream の意味論では
  // initialization (stage 0) は鋳造の一部として完了し、カーソルは最初の in-scope 実ステージに
  // 立つ。実ステージが 1 つも in-scope に無い縮退形はカーソル 0 (CompletedBox) のまま Running —
  // actNext の完了済みカーソル枝 (branch 10) がこの形を受ける。
  action init = {
    nondet execSet = STAGES.powerset().oneOf()
    nondet condSet = STAGES.powerset().oneOf()
    val p = STAGES.mapBy(s => if (s == 0 or execSet.contains(s)) Execute else SkipPlan)
    val firstReal = STAGES.filter(s => s > 0 and p.get(s) == Execute)
      .fold(-1, (acc, s) => if (acc == -1 or s < acc) s else acc)
    val cb0 = STAGES.mapBy(s =>
      if (s == 0) CompletedBox
      else if (s == firstReal) InProgress
      else Pending)
    val cur0 = if (firstReal == -1) 0 else firstReal
    val ap0 = STAGES.mapBy(s => false)
    all {
      plan' = p,
      overlay' = p,
      conditional' = STAGES.mapBy(s => s != 0 and condSet.contains(s)),
      checkbox' = cb0,
      cursor' = cur0,
      status' = Running,
      parkedAt' = -1,
      autonomous' = false,
      approved' = ap0,
      lastDirective' = DNone,
      lastAction' = "init",
      prevCheckbox' = cb0,
      prevCursor' = cur0,
      prevOverlay' = p,
      prevApproved' = ap0,
      prevStatus' = Running,
      prevParkedAt' = -1,
      prevAutonomous' = false,
    }
  }

  action snapshot = all {
    prevCheckbox' = checkbox,
    prevCursor' = cursor,
    prevOverlay' = overlay,
    prevApproved' = approved,
    prevStatus' = status,
    prevParkedAt' = parkedAt,
    prevAutonomous' = autonomous,
  }

  // next: 読み取り専用。in-flight なら run-stage (実効 SKIP なら error)、
  // 完了済みなら次の in-scope、無ければ done (branch 10)
  action actNext = all {
    status == Running,
    plan' = plan, overlay' = overlay, conditional' = conditional,
    checkbox' = checkbox, cursor' = cursor, status' = status,
    parkedAt' = parkedAt, autonomous' = autonomous, approved' = approved,
    lastDirective' =
      if (isInFlight(checkbox.get(cursor)))
        (if (effectivePlan(cursor) == SkipPlan) DError else DRunStage(cursor))
      else
        (if (nextInScope(cursor) == -1) DDone else DRunStage(nextInScope(cursor))),
    lastAction' = "next",
  }

  // parked ワークフローへの next は parked directive (branch 2.5)
  action actNextParked = all {
    status == WorkflowParked,
    plan' = plan, overlay' = overlay, conditional' = conditional,
    checkbox' = checkbox, cursor' = cursor, status' = status,
    parkedAt' = parkedAt, autonomous' = autonomous, approved' = approved,
    lastDirective' = DParked,
    lastAction' = "next_parked",
  }

  // report forward verdict: gated は approve 経由 (in-progress は gate-start --recovered + approve)、
  // 非 gated は advance。final なら complete-workflow (ディスパッチ表 3.2)
  action actReportForward = {
    val s = cursor
    val nxt = nextInScope(s)
    all {
      status == Running,
      or { checkbox.get(s) == InProgress, checkbox.get(s) == AwaitingApproval },
      approved' = if (gated(s)) approved.set(s, true) else approved,
      checkbox' =
        if (nxt == -1) checkbox.set(s, CompletedBox)
        else checkbox.set(s, CompletedBox).set(nxt, InProgress),
      cursor' = if (nxt == -1) s else nxt,
      status' = if (nxt == -1) WorkflowCompleted else Running,
      plan' = plan, overlay' = overlay, conditional' = conditional,
      parkedAt' = parkedAt, autonomous' = autonomous,
      lastDirective' = DDone,
      lastAction' = "report_forward",
    }
  }

  // gate-lifecycle 3 verdict (checkbox 前提は厳密 — ガード表 3.1)
  action actGateStart = all {
    status == Running,
    gated(cursor),
    checkbox.get(cursor) == InProgress,
    checkbox' = checkbox.set(cursor, AwaitingApproval),
    plan' = plan, overlay' = overlay, conditional' = conditional,
    cursor' = cursor, status' = status, parkedAt' = parkedAt,
    autonomous' = autonomous, approved' = approved,
    lastDirective' = DNone,
    lastAction' = "report_awaiting_approval",
  }

  action actReject = all {
    status == Running,
    gated(cursor),
    or { checkbox.get(cursor) == InProgress, checkbox.get(cursor) == AwaitingApproval },
    checkbox' = checkbox.set(cursor, Revising),
    plan' = plan, overlay' = overlay, conditional' = conditional,
    cursor' = cursor, status' = status, parkedAt' = parkedAt,
    autonomous' = autonomous, approved' = approved,
    lastDirective' = DNone,
    lastAction' = "report_rejected",
  }

  action actRevise = all {
    status == Running,
    checkbox.get(cursor) == Revising,
    checkbox' = checkbox.set(cursor, AwaitingApproval),
    plan' = plan, overlay' = overlay, conditional' = conditional,
    cursor' = cursor, status' = status, parkedAt' = parkedAt,
    autonomous' = autonomous, approved' = approved,
    lastDirective' = DNone,
    lastAction' = "report_revised",
  }

  // skipped: routed lifecycle outcome。dispatch 成功のエピローグは done (02 §7.3)
  action actReportSkipped = {
    val s = cursor
    val nxt = nextInScope(s)
    all {
      status == Running,
      or { checkbox.get(s) == InProgress, checkbox.get(s) == Revising },
      or { conditional.get(s), effectivePlan(s) == SkipPlan },
      checkbox' =
        if (nxt == -1) checkbox.set(s, SkippedBox)
        else checkbox.set(s, SkippedBox).set(nxt, InProgress),
      cursor' = if (nxt == -1) s else nxt,
      status' = if (nxt == -1) WorkflowCompleted else Running,
      plan' = plan, overlay' = overlay, conditional' = conditional,
      parkedAt' = parkedAt, autonomous' = autonomous, approved' = approved,
      lastDirective' = DDone,
      lastAction' = "report_skipped",
    }
  }

  // stale re-report guard: カーソル通過済み completed への再 report は
  // 何もコミットせず冪等 done (ガード表 3.2)。フレーム条件は stale_rereport_frame が検査
  action actStaleReport = {
    nondet s = STAGES.oneOf()
    all {
      status == Running,
      s < cursor,
      checkbox.get(s) == CompletedBox,
      plan' = plan, overlay' = overlay, conditional' = conditional,
      checkbox' = checkbox, cursor' = cursor, status' = status,
      parkedAt' = parkedAt, autonomous' = autonomous, approved' = approved,
      lastDirective' = DDone,
      lastAction' = "report_stale",
    }
  }

  // jump forward: 介在 (cursor < u < t) の in-scope in-flight は Pending 含め skipped、
  // 現ステージは in-flight かつ非 Pending のときのみ skipped (02 §8)。実効 SKIP の介在は
  // 触らない (upstream 実バイト cli/jump/execute-forward-across-phases — v2.1)
  action actJumpForward = {
    nondet t = STAGES.oneOf()
    all {
      status == Running,
      t > cursor, t != 0, inScope(t),
      checkbox' = STAGES.mapBy(u =>
        if (u == t) InProgress
        else if (u == cursor and isInFlight(checkbox.get(u)) and checkbox.get(u) != Pending) SkippedBox
        else if (u > cursor and u < t and inScope(u) and isInFlight(checkbox.get(u))) SkippedBox
        else checkbox.get(u)),
      cursor' = t,
      plan' = plan, overlay' = overlay, conditional' = conditional,
      status' = status, parkedAt' = parkedAt, autonomous' = autonomous,
      approved' = approved,
      lastDirective' = DNone,
      lastAction' = "jump_forward",
    }
  }

  // jump backward: ターゲット + 下流の EXECUTE ステージを pending へ戻し、承認履歴を無効化
  action actJumpBackward = {
    nondet t = STAGES.oneOf()
    all {
      status == Running,
      t < cursor, t != 0, inScope(t),
      checkbox' = STAGES.mapBy(u =>
        if (u == t) InProgress
        else if (u > t and inScope(u) and checkbox.get(u) != Pending) Pending
        else checkbox.get(u)),
      approved' = STAGES.mapBy(u => if (u >= t) false else approved.get(u)),
      cursor' = t,
      plan' = plan, overlay' = overlay, conditional' = conditional,
      status' = status, parkedAt' = parkedAt, autonomous' = autonomous,
      lastDirective' = DNone,
      lastAction' = "jump_backward",
    }
  }

  action actJumpRedo = all {
    status == Running,
    cursor != 0,
    checkbox' = checkbox.set(cursor, InProgress),
    approved' = approved.set(cursor, false),
    plan' = plan, overlay' = overlay, conditional' = conditional,
    cursor' = cursor, status' = status, parkedAt' = parkedAt,
    autonomous' = autonomous,
    lastDirective' = DNone,
    lastAction' = "jump_redo",
  }

  // park: autonomous 中は拒否 (unattended run has no human to resume it)
  action actPark = all {
    status == Running,
    not(autonomous),
    status' = WorkflowParked,
    parkedAt' = cursor,
    plan' = plan, overlay' = overlay, conditional' = conditional,
    checkbox' = checkbox, cursor' = cursor, autonomous' = autonomous,
    approved' = approved,
    lastDirective' = DParked,
    lastAction' = "park",
  }

  action actUnpark = all {
    status == WorkflowParked,
    status' = Running,
    parkedAt' = -1,
    plan' = plan, overlay' = overlay, conditional' = conditional,
    checkbox' = checkbox, cursor' = cursor, autonomous' = autonomous,
    approved' = approved,
    lastDirective' = DNone,
    lastAction' = "unpark",
  }

  // recompose: Running・非 autonomous・pending・カーソル前方のみ flip (slice 1 は 8 ガード中 4)
  action actRecompose = {
    nondet s = STAGES.oneOf()
    all {
      status == Running,
      not(autonomous),
      s > cursor,
      checkbox.get(s) == Pending,
      overlay' = overlay.set(s, if (overlay.get(s) == Execute) SkipPlan else Execute),
      plan' = plan, conditional' = conditional,
      checkbox' = checkbox, cursor' = cursor, status' = status,
      parkedAt' = parkedAt, autonomous' = autonomous, approved' = approved,
      lastDirective' = DNone,
      lastAction' = "recompose",
    }
  }

  action actSetAutonomy = all {
    status == Running,
    autonomous' = not(autonomous),
    plan' = plan, overlay' = overlay, conditional' = conditional,
    checkbox' = checkbox, cursor' = cursor, status' = status,
    parkedAt' = parkedAt, approved' = approved,
    lastDirective' = DNone,
    lastAction' = "set_autonomy",
  }

  // 完了後のスタッタ (デッドロック回避)
  action actDoneStutter = all {
    status == WorkflowCompleted,
    plan' = plan, overlay' = overlay, conditional' = conditional,
    checkbox' = checkbox, cursor' = cursor, status' = status,
    parkedAt' = parkedAt, autonomous' = autonomous, approved' = approved,
    lastDirective' = DDone,
    lastAction' = "done_stutter",
  }

  action step = any {
    all { snapshot, actNext },
    all { snapshot, actNextParked },
    all { snapshot, actReportForward },
    all { snapshot, actGateStart },
    all { snapshot, actReject },
    all { snapshot, actRevise },
    all { snapshot, actReportSkipped },
    all { snapshot, actStaleReport },
    all { snapshot, actJumpForward },
    all { snapshot, actJumpBackward },
    all { snapshot, actJumpRedo },
    all { snapshot, actPark },
    all { snapshot, actUnpark },
    all { snapshot, actRecompose },
    all { snapshot, actSetAutonomy },
    all { snapshot, actDoneStutter },
  }

  // === 不変条件 (DoD — ADR 0003 決定 7) ===

  // 有効プランが SKIP のステージに run-stage を放出しない (観測射影)
  val no_run_stage_for_skip = match lastDirective {
    | DRunStage(s) => effectivePlan(s) == Execute
    | _ => true
  }

  // 状態レベルの守り: Running 中のカーソルは常に実効 EXECUTE 上にある
  // (単一ガード破壊を直接検出する — mutation テストで確認済み)
  val cursor_in_scope = (status == Running) implies (effectivePlan(cursor) == Execute)

  // ゲート迂回トレースが存在しない: gated ステージが completed なら必ず承認履歴を持つ
  val no_gate_bypass = STAGES.forall(s =>
    (checkbox.get(s) == CompletedBox and gated(s)) implies approved.get(s))

  // gate-lifecycle の checkbox 前提 (ガード除去 mutation を prev* 経由で検出)
  val gate_lifecycle_preconditions = and {
    (lastAction == "report_awaiting_approval") implies (prevCheckbox.get(cursor) == InProgress),
    (lastAction == "report_rejected") implies (or {
      prevCheckbox.get(cursor) == InProgress,
      prevCheckbox.get(cursor) == AwaitingApproval,
    }),
    (lastAction == "report_revised") implies (prevCheckbox.get(cursor) == Revising),
  }

  // park 中は位置が固定されている
  val parked_position = (status == WorkflowParked) implies (parkedAt == cursor)

  // resume (unpark) 時、カーソルは park 時の位置のまま
  val unpark_restores_position = (lastAction == "unpark") implies (cursor == prevParkedAt)

  // stale re-report は冪等 done になる
  val stale_rereport_yields_done =
    (lastAction == "report_stale") implies (lastDirective == DDone)

  // stale re-report は何もコミットしない (フレーム条件)
  val stale_rereport_frame = (lastAction == "report_stale") implies and {
    checkbox == prevCheckbox,
    cursor == prevCursor,
    overlay == prevOverlay,
    approved == prevApproved,
    status == prevStatus,
    parkedAt == prevParkedAt,
    autonomous == prevAutonomous,
  }

  // アクティブ (in-progress / awaiting / revising) なステージは高々 1 つ
  // 【暫定規範】upstream 未明文化の派生不変条件。A7 追従で反例が出たら降格
  val at_most_one_active = STAGES.filter(s => or {
    checkbox.get(s) == InProgress,
    checkbox.get(s) == AwaitingApproval,
    checkbox.get(s) == Revising,
  }).size() <= 1
}
