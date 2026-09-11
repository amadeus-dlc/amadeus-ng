// engine_loop.qnt — orchestration コンテキスト第一陣モデル (slice 1, v2.7)
//
// 対象: エンジンループの核 — カーソル×CheckboxState×PlanAction(overlay 合成)×ゲート×
//       park/jump/recompose×stale re-report。upstream aidlc-workflows v2 @ 3c3146cf の
//       02-orchestration-engine.md を抽象化 (根拠: ピン 3c3146cf の配布資産。旧契約マップ
//       docs/specs/research/ は 2026-09-07 に削除した)。
// v2.10 (2026-09-10): redo (別 scope を名指さない直接 execute) のガードを実装へ揃えた
//   (裁定 jump-redo-guard Q1 = A)。actJumpRedo は status == Running と cursor != 0 だけを
//   要求していたので、TaskUpdate 同期 (synced) や別 scope の跳躍 (foreign) が実効 SKIP へ
//   置いたカーソルへの redo を許していた。集約の jump_execute_guard は別 scope 無しの跳躍に
//   jump_plan(None) = in_scope を要求するため同じ redo を InvalidTarget で拒否する。
//   モデルが許し実装が拒否する向きの差であり、実装の振る舞いは変えない。
//   - actJumpRedo に inScope(cursor) を追加。
//   - 不変条件 jump_redo_on_plan (lastAction == "jump_redo" ⟹ effectivePlan(cursor) == Execute)
//     を追加 — 追加前の同じ seed で violation を採取済み (red)。
//   - witness w_jump_redo を追加 — ガードが redo 経路を死なせていないことを固定する。
// v2.9 (2026-09-10): 別 scope を名指した直接 execute (裁定 jump-contract Q1 = A の
//   foreign-scope 部分) をモデル化 — 本家 2.7.1 (a277af21 `aidlc-jump.ts:257-296, 308-375`)
//   は `--scope <名前>` の直接 execute を、その scope の静的な列で到達可否と読み飛ばしを
//   導いて受理する。到達点が自 scope の実効計画では EXECUTE でなくても届く。
//   - foreignPlan: int -> PlanAction (immutable — init で nondet に 1 度選ぶ。stage 0 は
//     Execute 固定 — plan と同じ規律)。名指された別 scope の静的な列の射影。
//   - foreign: bool — 別 scope 由来の跳躍がカーソルを自実効計画の外へ置いた印
//     (Rust 側 IntentExecution::cursor_foreign_scoped)。synced (TaskUpdate 同期由来) とは
//     由来が違うので別の印であり、cursor_in_scope は両方が立っていないときだけ適用する。
//   - actForeignJumpForward: actJumpForward と同じ枠組みで、到達可否・介在の読み飛ばしの
//     membership を inScope から foreignPlan へ置き換えたもの。
//   - witness w_foreign_jump (= foreign) を追加 — foreign-scope の跳躍経路が実在することを
//     固定する。
//
//   抽象化の対応表 (Rust 側 IntentExecution との射影):
//     foreignPlan ↔ JumpScope の静的な列 (名指された scope が EXECUTE と言う位置の集合)。
//                   scope の内容は定義の静的な材料なので、実行中は変わらない
//     foreign     ↔ IntentExecution::cursor_foreign_scoped() — エンジン遷移 (advance) や
//                   TaskUpdate 同期がカーソルを動かすと偽へ戻る
// v2.7 (2026-09-05): set-autonomy の受理集合を upstream に揃えた (#72 / b50、オーナー裁定 A) —
//   actSetAutonomy から status == Running を外した。upstream handleSetAutonomy
//   (3c3146cf aidlc-bolt.ts:799-859) は状態ファイルを受理条件として読まないので、park 中でも
//   Completed でも切替は成功する (--single / review / practices-promote と同じ受理集合)。
//   新しい不変条件は無い — 変わったのは 1 アクションの enabled 条件だけであり、既存の
//   不変条件 19 本 (とくに actPark の not(autonomous) ガードとの相互作用) が green のままで
//   あることが検査である。
//
//   human presence ガード (I11) は**モデル外**である。判定の材料は監査台帳の HUMAN_TURN 行
//   (フックが書く外部の入力) と集約の last_gate_resolution_at であり、どちらもこのモデルが
//   抽象化している「エンジンループの核」の外にある。仕様 10 §6 I11 の強制手段は E2+E3
//   (境界型 + 集約のガード + 単体 / end-to-end テスト) のままである。
// v2.6 (2026-09-05): 昇格受領証 (PRACTICES_AFFIRMED) と、practices-discovery の承認ガードを
//   モデル化 (#7 キュー 5 の残り / B10 / b49) — 状態変数 2 本 (practicesStage / affirmed) と
//   スナップショット 1 本 (prevAffirmed) を足し、アクション actPromotePractices を追加した。
//   actReportForward には段 12 のガード (cursor == practicesStage ⟹ affirmed) が付き、
//   フロア (受領証を落とす位置) を forward / skipped の新カーソル・reject のカーソル・
//   jump 3 種へ入れた (レビュー受領証と同じ FLOOR 集合 — upstream の
//   hasFreshPracticesAffirmationReceipt は STAGE_STARTED / GATE_REJECTED / STAGE_REVISING を
//   見るが、STAGE_REVISING を emit するのは reject の対と approve の revision backstop だけで、
//   本モデルの actReject がその対を 1 アクションで表す)。不変条件
//   approve_requires_practices_receipt / practices_receipt_floor / promote_frame と
//   witness w_practices_affirmed / w_approved_practices を追加。
//
//   抽象化の対応表 (Rust 側 IntentExecution との射影):
//     practicesStage ↔ IntentExecution::practices_stage() — 計画上の practices-discovery の
//                      位置。init で 1 度選んで以後凍結する静的材料であり、-1 は
//                      「計画にそのステージが無い」(段 12 が空回りする歴史) を表す。
//                      stage 0 (initialization) は非ゲートなので選ばない
//     affirmed       ↔ practices_affirmed[practicesStage] — 他のステージの受領証は
//                      決して立たないので 1 bit へ射影する。昇格の**内容**
//                      (置換する節・印付き規則行) は投影の材料であって状態ではないので
//                      モデルは持たない
// v2.5 (2026-09-04): レビュー受領証 (REVIEW_REQUESTED / REVIEW_COMPLETED) の会計と、
//   承認ゲートの受領証ガードをモデル化 (#51 / B10 / b48) — 状態変数 5 本
//   (reviewed / advisory / reqCount / pending / terminal) とスナップショット 3 本
//   (prevReqCount / prevPending / prevTerminal) を足し、アクション 3 本
//   (actRequestReview / actRetryReview / actRecordVerdict) を追加した。actReportForward には
//   受領証ガード (gated ∧ reviewed ⟹ terminal) が付き、フロア (試行を空に戻す位置) を
//   forward / skipped の新カーソル・reject のカーソル・jump 3 種の全ステージへ入れた。
//   不変条件 approve_requires_terminal_receipt / review_attempt_floor / review_budget /
//   review_frame と witness w_review_requested / w_verdict_terminal / w_approved_reviewed /
//   w_retry_review を追加。
//
//   抽象化の対応表 (Rust 側 IntentExecution / ReviewPolicy との射影):
//     reviewed(s)      ↔ そのステージがレビュアーを宣言し実効クラスが none でない
//                        (Rust: `policy.is_some() && policy.requires_receipt()`)。
//                        false は「宣言なし」と「実効クラス none」の両方を畳んだもので、
//                        どちらも依頼は budget 0 で拒まれ承認は受領証を要求しない
//     advisory(s)      ↔ 実効クラスが advisory (budget 1、1 パスで終端)。
//                        false かつ reviewed なら adversarial (budget = BUDGET)
//     reqCount(s)      ↔ ReviewAttempt::request_count()
//     pending(s)       ↔ ReviewAttempt::pending() (判定待ちの通し番号の集合)
//     terminal(s)      ↔ ReviewAttempt::has_terminal(policy) — 判定の値そのものは
//                        持たず「終端受領証が在るか」の 1 bit に射影する
//     retry-pending    ↔ actRetryReview (フレーム空 — 呼び直しは依頼に数えず、
//                        判定待ち集合も既にその番号を含む)
// v2.8 (2026-09-09): TaskUpdate 同期 (actTaskSync) と明示 direction の直接 execute に伴う
//   2 つの裁定 (U2 task-update-contract Q1 = A / jump-contract Q1 = A) を反映 —
//   - at_most_one_active を**降格** (削除)。本家 2.7.1 の実観測に反例が 2 つある: 明示
//     backward の直接 execute は出発点と到達点の両方を `[-]` に残し、別 stage への TaskUpdate
//     同期は旧 stage の checkbox を戻さない (jump-logs / runtime-sync-observations)。
//   - actTaskSync: `set-status --stage <slug>` の射影。任意の stage (実効 SKIP を含む) を
//     現在位置にし、その checkbox を InProgress にする。他の状態は動かさない。
//   - synced: 同期が実効 EXECUTE の外へカーソルを置いたか。cursor_in_scope はこの印が立って
//     いる間だけ適用を外す (エンジン遷移がカーソルを動かすと印は消える)。
//   - actRequestReview に「判定待ちが無い」ガードを追加 — 集約は `PendingIterations` で
//     同一試行内の重複依頼を拒否する (upstream "still waiting for a verdict")。
// v2.4 (2026-09-04): --single 隔離実行と walking-skeleton stance の分類往復をモデル化
//   (#73 / b47) — 状態変数 stanceRecorded を足し、フレーム空アクション actSingleRun
//   (全状態変数が不変) と actRecordSkeletonStance (stanceRecorded だけを立てる) を追加した。
//   フレーム条件 single_run_frame / stance_frame と到達性 witness w_single_run /
//   w_stance_recorded を追加。
//   **skeleton-gate ステージの抽象は skeletonGateStage = 静的計画 plan の最初の非 init
//   EXECUTE ステージ** (`gated(s) = s != 0` と同じ「非 init = 実ステージ」の簡約の上に、
//   Rust 側 IntentExecution::skeleton_gate_stage の「**静的**計画の Construction フェーズ
//   最初の EXECUTE ステージ」を重ねたもの)。本モデルは Construction フェーズを持たないので、
//   ITF 準拠テストは合成計画の索引 1 以降を Construction フェーズに割り当てて両者を
//   一致させる。設計 §3 の当初案は `cursor == 1` だったが、これは**不忠実**だった —
//   縮退誕生 (実ステージが全て静的 SKIP) から recompose でオーバレイを Execute に反転し
//   索引 1 へ jump すると `cursor == 1` かつ `plan.get(1) == SkipPlan` に到達でき、この形で
//   Rust の skeleton_gate_stage は None (または別索引) を返して InvalidTarget で拒む。
//   quint の一時プローブ不変条件で反例を実測 (5000 サンプル中 290 トレース目で違反) し、
//   ガードを静的計画由来へ改めた (b47 の実測裁定)。
//   追補 (PR #103 レビュー): actRecordSkeletonStance の `status == Running` を撤去して
//   実装の受理集合に揃え、actSingleRun の対象選択を `STAGES.oneOf()` + `gated(s)` の
//   一段に統一した (以前は filter で非 init に絞ったうえで gated も課す二重ガード)。
// v2.3 (2026-09-04): actPark のガードを `Running or WorkflowParked` へ緩和 (#74 / b45) —
//   upstream handlePark は park 済みでも成功し Parked / Parked At Stage を書き直して
//   WORKFLOW_PARKED を再 emit する (**再スタンプ**)。到達性 witness w_repark と
//   不変条件 parked_marker_status を追加。
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
//   foreignPlan    ↔ JumpScope の静的な列 (別 scope を名指した直接 execute が使う計画 — v2.9)
//   foreign        ↔ IntentExecution::cursor_foreign_scoped() (別 scope 由来の跳躍がカーソルを
//                    自実効計画の外へ置いた印 — v2.9。synced (同期由来) とは別の印)
//   stanceRecorded ↔ skeleton_stance: Option<SkeletonStance> の「記録済みか」射影。
//                    stance の 3 値 (on/off/scope-dependent) は未モデル化 — upstream の
//                    resolveSkeletonGate はどの値でも同じ答え (gated) を返すため、
//                    ゲート判定に効くのは「記録されたか」だけである
//   lastAction     ↔ ITF リプレイ用アクション記録 (ADR 0003 決定 5 の lastAction 規約)
//   prev*          ↔ (モデル専用) フレーム条件検査用の前状態スナップショット。Rust 対応物なし
//
// slice 1 の抽象化と既知の恒偽枝 (第二陣以降で精密化):
//   - stage 0 = initialization (非ゲート・常時 EXECUTE・非 CONDITIONAL)
//   - skeleton stance は v2.4 で「記録されたか」だけをモデル化した。`next` が返す
//     gate の 3 値 (gated / ungated / unresolved) 自体は依然として未モデル化 —
//     観測射影 DirectiveKind の DRunStage(int) が gate 欄を運ばないからである
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
// 検査力の確認 (mutation テスト、v2.3 = b45 の再スタンプ緩和で実施):
//   - actPark の parkedAt' を -1 に変異 (マーカー脱落) → parked_position と
//     unpark_restores_position が検出
//   - actPark の status' を Running に変異 → parked_marker_status が検出。この変異は
//     WorkflowParked を到達不能にするため parked_position / unpark_restores_position が
//     空虚に成立してしまい、b45 以前の Running 限定ガードでも検出できない既存の穴だった。
//     b45 で parked_marker_status を追加して塞いだ (統合判断 2026-09-04 — Fable 5 メインセッションの裁定、PR #7 キュー 4 の受入時にオーナー確認)。
// 検査力の確認 (mutation テスト、v2.4 = b47 の 2 アクション追加で実施):
//   - actSingleRun に cursor' = cursor + 1 を注入 (フレーム空の破壊) → single_run_frame が検出
//   - actRecordSkeletonStance に checkbox' = checkbox.set(cursor, CompletedBox) を注入
//     (stanceRecorded 以外への波及) → stance_frame が検出
// 検査力の確認 (mutation テスト、v2.5 = b48 の受領証会計で実施):
//   - actReportForward の受領証ガードを外す → approve_requires_terminal_receipt が検出
//   - actReject のリセットを外す → review_attempt_floor が検出
//   - actRequestReview の上限ガードを外す → review_budget が検出
//   - actRetryReview で reqCount を +1 する → review_frame が検出
// 検査力の確認 (mutation テスト、v2.6 = b49 の昇格受領証で実施。対照の無変異は [ok]):
//   - actReportForward の段 12 ガードを外す → approve_requires_practices_receipt が検出
//   - actReject の昇格受領証リセットを外す → practices_receipt_floor が検出
//   - actPromotePractices で cursor を +1 する → promote_frame が検出
//
// E4 トレーサビリティ (10-orchestration.md §6 から参照される定義名):
//   engine_loop::no_run_stage_for_skip / cursor_in_scope / no_gate_bypass /
//   gate_lifecycle_preconditions / parked_position / parked_marker_status /
//   unpark_restores_position / stale_rereport_yields_done / stale_rereport_frame /
//   single_run_frame / stance_frame / (at_most_one_active は v2.8 で降格) /
//   approve_requires_terminal_receipt / review_attempt_floor / review_budget / review_frame /
//   approve_requires_practices_receipt / practices_receipt_floor / promote_frame

module engine_loop {
  pure val N = 5
  pure val STAGES = 0.to(N - 1)

  type CheckboxState = Pending | InProgress | AwaitingApproval | Revising | CompletedBox | SkippedBox
  type PlanAction = Execute | SkipPlan
  type WorkflowStatus = Running | WorkflowCompleted | WorkflowParked
  type DirectiveKind = DRunStage(int) | DDone | DParked | DError | DNone

  var plan: int -> PlanAction        // 静的 scope grid (immutable — 裁定 B1)
  var foreignPlan: int -> PlanAction // 別 scope の静的な列 (immutable — v2.9。stage 0 は Execute 固定)
  var overlay: int -> PlanAction     // recompose オーバレイ (実効プランの源)
  var conditional: int -> bool       // execution: CONDITIONAL 宣言 (stage 0 は常に false)
  var checkbox: int -> CheckboxState
  var cursor: int
  var synced: bool                   // TaskUpdate 同期が実効 EXECUTE の外へカーソルを置いた印 (v2.8)
  var foreign: bool                  // 別 scope 由来の跳躍が自実効計画の外へカーソルを置いた印 (v2.9)
  var status: WorkflowStatus
  var parkedAt: int                  // -1 = 非 park
  var autonomous: bool               // Construction Autonomy Mode
  var approved: int -> bool          // ゲート承認履歴
  var lastDirective: DirectiveKind
  var lastAction: str                // ITF リプレイ駆動用
  var stanceRecorded: bool           // walking-skeleton stance が記録済みか (#73)
  // --- レビュー受領証 (b48 / B10)。reviewed / advisory は init で選んで以後凍結する静的材料
  var reviewed: int -> bool          // レビュアー宣言あり ∧ 実効クラス != none
  var advisory: int -> bool          // 実効クラスが advisory (budget 1)
  var reqCount: int -> int           // この試行で数え上げた依頼の回数
  var pending: int -> Set[int]       // 判定待ちの通し番号
  var terminal: int -> bool          // この試行に終端受領証が在るか
  // --- 昇格受領証 (b49 / B10)。practicesStage は init で選んで以後凍結する静的材料
  var practicesStage: int            // practices-discovery の位置 (-1 = 計画に無い)
  var affirmed: bool                 // その試行で昇格 (practices-promote) が成功したか

  // 前状態スナップショット (フレーム条件検査用。全アクションが snapshot と合成される)
  var prevCheckbox: int -> CheckboxState
  var prevCursor: int
  var prevOverlay: int -> PlanAction
  var prevApproved: int -> bool
  var prevStatus: WorkflowStatus
  var prevParkedAt: int
  var prevAutonomous: bool
  var prevStanceRecorded: bool
  var prevReqCount: int -> int
  var prevPending: int -> Set[int]
  var prevTerminal: int -> bool
  var prevAffirmed: bool

  // レビュー予算 — adversarial は reviewer_max_iterations (既定 2)、advisory は 1 パス
  pure val BUDGET = 2
  def budget(s: int): int = if (advisory.get(s)) 1 else BUDGET
  def attemptEmpty(s: int): bool = and {
    reqCount.get(s) == 0, pending.get(s) == Set(), not(terminal.get(s))
  }

  def gated(s: int): bool = s != 0
  def effectivePlan(s: int): PlanAction = overlay.get(s)
  def inScope(s: int): bool = effectivePlan(s) == Execute
  def isInFlight(cb: CheckboxState): bool = or {
    cb == Pending, cb == InProgress, cb == AwaitingApproval, cb == Revising
  }
  // skeleton-gate ステージ = **静的計画** plan の最初の非 init EXECUTE ステージ。
  // -1 = 該当なし (全ての実ステージが静的 SKIP = 縮退誕生)。overlay (recompose の結果) は
  // 見ない — upstream の isSkeletonGateStage は静的サブグラフを歩くからである
  // (Rust 側 IntentExecution::skeleton_gate_stage と同じ規則)。
  val skeletonGateStage: int = STAGES.filter(s => s > 0 and plan.get(s) == Execute)
    .fold(-1, (acc, s) => if (acc == -1 or s < acc) s else acc)

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
    // 別 scope の静的な列 (v2.9)。plan と同じ規律で 1 度だけ選び、以後どのアクションでも
    // 動かない (定義の静的な材料の射影)。stage 0 は Execute 固定。
    nondet foreignExecSet = STAGES.powerset().oneOf()
    // レビューの静的材料。init で 1 度だけ選び、以後どのアクションでも動かない
    // (定義 × scope × override から解決される値の射影 — 実行中は変わらない)。
    nondet reviewedSet = STAGES.powerset().oneOf()
    nondet advisorySet = STAGES.powerset().oneOf()
    // practices-discovery の位置。stage 0 (initialization) は非ゲートなので選ばない。
    // -1 = そのステージが計画に無い形 (段 12 が空回りする歴史)。
    nondet practicesPick = STAGES.filter(s => s > 0).union(Set(-1)).oneOf()
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
      foreignPlan' = STAGES.mapBy(s => if (s == 0 or foreignExecSet.contains(s)) Execute else SkipPlan),
      overlay' = p,
      conditional' = STAGES.mapBy(s => s != 0 and condSet.contains(s)),
      checkbox' = cb0,
      cursor' = cur0,
      synced' = false,
      foreign' = false,
      status' = Running,
      parkedAt' = -1,
      autonomous' = false,
      approved' = ap0,
      lastDirective' = DNone,
      lastAction' = "init",
      stanceRecorded' = false,
      // stage 0 (initialization) はレビュアーを宣言しない — 非ゲートだからである。
      reviewed' = STAGES.mapBy(s => s != 0 and reviewedSet.contains(s)),
      advisory' = STAGES.mapBy(s => advisorySet.contains(s)),
      reqCount' = STAGES.mapBy(s => 0),
      pending' = STAGES.mapBy(s => Set()),
      terminal' = STAGES.mapBy(s => false),
      practicesStage' = practicesPick,
      affirmed' = false,
      prevCheckbox' = cb0,
      prevCursor' = cur0,
      prevOverlay' = p,
      prevApproved' = ap0,
      prevStatus' = Running,
      prevParkedAt' = -1,
      prevAutonomous' = false,
      prevStanceRecorded' = false,
      prevReqCount' = STAGES.mapBy(s => 0),
      prevPending' = STAGES.mapBy(s => Set()),
      prevTerminal' = STAGES.mapBy(s => false),
      prevAffirmed' = false,
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
    prevStanceRecorded' = stanceRecorded,
    prevReqCount' = reqCount,
    prevPending' = pending,
    prevTerminal' = terminal,
    prevAffirmed' = affirmed,
  }

  // next: 読み取り専用。in-flight なら run-stage (実効 SKIP なら error)、
  // 完了済みなら次の in-scope、無ければ done (branch 10)
  action actNext = all {
    status == Running,
    plan' = plan, foreignPlan' = foreignPlan, overlay' = overlay, conditional' = conditional,
    checkbox' = checkbox, cursor' = cursor, status' = status,
    parkedAt' = parkedAt, autonomous' = autonomous, approved' = approved,
    lastDirective' =
      if (isInFlight(checkbox.get(cursor)))
        (if (effectivePlan(cursor) == SkipPlan) DError else DRunStage(cursor))
      else
        (if (nextInScope(cursor) == -1) DDone else DRunStage(nextInScope(cursor))),
    stanceRecorded' = stanceRecorded,
    reviewed' = reviewed, advisory' = advisory,
    practicesStage' = practicesStage, affirmed' = affirmed,
    reqCount' = reqCount, pending' = pending, terminal' = terminal,
    lastAction' = "next",
    synced' = synced,
    foreign' = foreign,
  }

  // parked ワークフローへの next は parked directive (branch 2.5)
  action actNextParked = all {
    status == WorkflowParked,
    plan' = plan, foreignPlan' = foreignPlan, overlay' = overlay, conditional' = conditional,
    checkbox' = checkbox, cursor' = cursor, status' = status,
    parkedAt' = parkedAt, autonomous' = autonomous, approved' = approved,
    lastDirective' = DParked,
    stanceRecorded' = stanceRecorded,
    reviewed' = reviewed, advisory' = advisory,
    practicesStage' = practicesStage, affirmed' = affirmed,
    reqCount' = reqCount, pending' = pending, terminal' = terminal,
    lastAction' = "next_parked",
    synced' = synced,
    foreign' = foreign,
  }

  // report forward verdict: gated は approve 経由 (in-progress は gate-start --recovered + approve)、
  // 非 gated は advance。final なら complete-workflow (ディスパッチ表 3.2)
  action actReportForward = {
    val s = cursor
    val nxt = nextInScope(s)
    all {
      status == Running,
      or { checkbox.get(s) == InProgress, checkbox.get(s) == AwaitingApproval },
      // 段 11 — レビュアーを宣言したゲートは現在の試行の終端受領証を要する (b48)。
      (gated(s) and reviewed.get(s)) implies terminal.get(s),
      // 段 12 — practices-discovery の承認は現在の試行の昇格受領証を要する (b49)。
      (s == practicesStage) implies affirmed,
      approved' = if (gated(s)) approved.set(s, true) else approved,
      checkbox' =
        if (nxt == -1) checkbox.set(s, CompletedBox)
        else checkbox.set(s, CompletedBox).set(nxt, InProgress),
      cursor' = if (nxt == -1) s else nxt,
      status' = if (nxt == -1) WorkflowCompleted else Running,
      plan' = plan, foreignPlan' = foreignPlan, overlay' = overlay, conditional' = conditional,
      parkedAt' = parkedAt, autonomous' = autonomous,
      lastDirective' = DDone,
      stanceRecorded' = stanceRecorded,
      reviewed' = reviewed, advisory' = advisory,
      practicesStage' = practicesStage,
      // フロア — 次ステージに立つ (STAGE_STARTED 相当) とその試行は空に戻る。
      affirmed' = if (nxt != -1 and nxt == practicesStage) false else affirmed,
      reqCount' = if (nxt == -1) reqCount else reqCount.set(nxt, 0),
      pending' = if (nxt == -1) pending else pending.set(nxt, Set()),
      terminal' = if (nxt == -1) terminal else terminal.set(nxt, false),
      lastAction' = "report_forward",
      synced' = false,
      foreign' = false,
    }
  }

  // gate-lifecycle 3 verdict (checkbox 前提は厳密 — ガード表 3.1)
  action actGateStart = all {
    status == Running,
    gated(cursor),
    checkbox.get(cursor) == InProgress,
    checkbox' = checkbox.set(cursor, AwaitingApproval),
    plan' = plan, foreignPlan' = foreignPlan, overlay' = overlay, conditional' = conditional,
    cursor' = cursor, status' = status, parkedAt' = parkedAt,
    autonomous' = autonomous, approved' = approved,
    lastDirective' = DNone,
    stanceRecorded' = stanceRecorded,
    reviewed' = reviewed, advisory' = advisory,
    practicesStage' = practicesStage, affirmed' = affirmed,
    reqCount' = reqCount, pending' = pending, terminal' = terminal,
    lastAction' = "report_awaiting_approval",
    synced' = synced,
    foreign' = foreign,
  }

  action actReject = all {
    status == Running,
    gated(cursor),
    or { checkbox.get(cursor) == InProgress, checkbox.get(cursor) == AwaitingApproval },
    checkbox' = checkbox.set(cursor, Revising),
    plan' = plan, foreignPlan' = foreignPlan, overlay' = overlay, conditional' = conditional,
    cursor' = cursor, status' = status, parkedAt' = parkedAt,
    autonomous' = autonomous, approved' = approved,
    lastDirective' = DNone,
    stanceRecorded' = stanceRecorded,
    reviewed' = reviewed, advisory' = advisory,
    practicesStage' = practicesStage,
    // フロア — 差し戻しは受領証の会計を数え直しにする (GATE_REJECTED)。
    affirmed' = if (cursor == practicesStage) false else affirmed,
    reqCount' = reqCount.set(cursor, 0),
    pending' = pending.set(cursor, Set()),
    terminal' = terminal.set(cursor, false),
    lastAction' = "report_rejected",
    synced' = synced,
    foreign' = foreign,
  }

  action actRevise = all {
    status == Running,
    checkbox.get(cursor) == Revising,
    checkbox' = checkbox.set(cursor, AwaitingApproval),
    plan' = plan, foreignPlan' = foreignPlan, overlay' = overlay, conditional' = conditional,
    cursor' = cursor, status' = status, parkedAt' = parkedAt,
    autonomous' = autonomous, approved' = approved,
    lastDirective' = DNone,
    stanceRecorded' = stanceRecorded,
    reviewed' = reviewed, advisory' = advisory,
    practicesStage' = practicesStage, affirmed' = affirmed,
    reqCount' = reqCount, pending' = pending, terminal' = terminal,
    lastAction' = "report_revised",
    synced' = synced,
    foreign' = foreign,
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
      plan' = plan, foreignPlan' = foreignPlan, overlay' = overlay, conditional' = conditional,
      parkedAt' = parkedAt, autonomous' = autonomous, approved' = approved,
      lastDirective' = DDone,
      stanceRecorded' = stanceRecorded,
      reviewed' = reviewed, advisory' = advisory,
      practicesStage' = practicesStage,
      // フロア — 読み飛ばしも次ステージを立てるので、その試行は空に戻る。
      affirmed' = if (nxt != -1 and nxt == practicesStage) false else affirmed,
      reqCount' = if (nxt == -1) reqCount else reqCount.set(nxt, 0),
      pending' = if (nxt == -1) pending else pending.set(nxt, Set()),
      terminal' = if (nxt == -1) terminal else terminal.set(nxt, false),
      lastAction' = "report_skipped",
      synced' = false,
      foreign' = false,
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
      plan' = plan, foreignPlan' = foreignPlan, overlay' = overlay, conditional' = conditional,
      checkbox' = checkbox, cursor' = cursor, status' = status,
      parkedAt' = parkedAt, autonomous' = autonomous, approved' = approved,
      lastDirective' = DDone,
      stanceRecorded' = stanceRecorded,
      reviewed' = reviewed, advisory' = advisory,
      practicesStage' = practicesStage, affirmed' = affirmed,
      reqCount' = reqCount, pending' = pending, terminal' = terminal,
      lastAction' = "report_stale",
      synced' = synced,
      foreign' = foreign,
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
      plan' = plan, foreignPlan' = foreignPlan, overlay' = overlay, conditional' = conditional,
      status' = status, parkedAt' = parkedAt, autonomous' = autonomous,
      approved' = approved,
      lastDirective' = DNone,
      stanceRecorded' = stanceRecorded,
      reviewed' = reviewed, advisory' = advisory,
      practicesStage' = practicesStage,
      // フロア — ジャーナルの STAGE_JUMPED はステージ非依存のフロアである
      // (fail-closed over precise)。全ステージの試行を空へ戻す。
      affirmed' = false,
      reqCount' = STAGES.mapBy(u => 0),
      pending' = STAGES.mapBy(u => Set()),
      terminal' = STAGES.mapBy(u => false),
      lastAction' = "jump_forward",
      synced' = false,
      foreign' = false,
    }
  }

  // foreign jump forward (v2.9): `--scope <名前>` を名指した直接 execute (裁定
  // jump-contract Q1 = A)。到達可否と介在の読み飛ばしは**別 scope の静的な列**
  // (foreignPlan) で導く — 自 scope の実効計画 (inScope) では EXECUTE でない到達点にも
  // 届き、その場合は foreign を立てて cursor_in_scope の適用を外す。それ以外の枠組み
  // (checkbox の導出・フロアの全リセット) は actJumpForward と同じである。
  action actForeignJumpForward = {
    nondet t = STAGES.oneOf()
    all {
      status == Running,
      t > cursor, t != 0, foreignPlan.get(t) == Execute,
      checkbox' = STAGES.mapBy(u =>
        if (u == t) InProgress
        else if (u == cursor and isInFlight(checkbox.get(u)) and checkbox.get(u) != Pending) SkippedBox
        else if (u > cursor and u < t and foreignPlan.get(u) == Execute and isInFlight(checkbox.get(u))) SkippedBox
        else checkbox.get(u)),
      cursor' = t,
      plan' = plan, foreignPlan' = foreignPlan, overlay' = overlay, conditional' = conditional,
      status' = status, parkedAt' = parkedAt, autonomous' = autonomous,
      approved' = approved,
      lastDirective' = DNone,
      stanceRecorded' = stanceRecorded,
      reviewed' = reviewed, advisory' = advisory,
      practicesStage' = practicesStage,
      // フロア — ジャーナルの STAGE_JUMPED はステージ非依存のフロアである
      // (fail-closed over precise)。全ステージの試行を空へ戻す。
      affirmed' = false,
      reqCount' = STAGES.mapBy(u => 0),
      pending' = STAGES.mapBy(u => Set()),
      terminal' = STAGES.mapBy(u => false),
      lastAction' = "foreign_jump",
      synced' = false,
      foreign' = not(inScope(t)),
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
      // 承認の無効化は「pending へ戻す stage + 到達点」に限る (v2.8) — 巻き戻さない実効 SKIP の
      // 完了 (TaskUpdate 同期の後に承認された stage) は checkbox と承認を保つ。
      approved' = STAGES.mapBy(u => if (u == t or (u > t and inScope(u))) false else approved.get(u)),
      cursor' = t,
      plan' = plan, foreignPlan' = foreignPlan, overlay' = overlay, conditional' = conditional,
      status' = status, parkedAt' = parkedAt, autonomous' = autonomous,
      lastDirective' = DNone,
      stanceRecorded' = stanceRecorded,
      reviewed' = reviewed, advisory' = advisory,
      practicesStage' = practicesStage,
      // フロア — ジャーナルの STAGE_JUMPED はステージ非依存のフロアである
      // (fail-closed over precise)。全ステージの試行を空へ戻す。
      affirmed' = false,
      reqCount' = STAGES.mapBy(u => 0),
      pending' = STAGES.mapBy(u => Set()),
      terminal' = STAGES.mapBy(u => false),
      lastAction' = "jump_backward",
      synced' = false,
      foreign' = false,
    }
  }

  action actJumpRedo = all {
    status == Running,
    cursor != 0,
    // 別 scope を名指さない跳躍は自実効計画の EXECUTE にしか届かない (集約の
    // jump_execute_guard / jump_plan(None) と同じ規則)。同期 (synced) や別 scope の跳躍
    // (foreign) が計画外へ置いたカーソルへの redo は本家でも InvalidTarget である —
    // 裁定 jump-redo-guard Q1 = A (モデルを実装へ揃える)。
    inScope(cursor),
    checkbox' = checkbox.set(cursor, InProgress),
    approved' = approved.set(cursor, false),
    plan' = plan, foreignPlan' = foreignPlan, overlay' = overlay, conditional' = conditional,
    cursor' = cursor, status' = status, parkedAt' = parkedAt,
    autonomous' = autonomous,
    lastDirective' = DNone,
    stanceRecorded' = stanceRecorded,
    reviewed' = reviewed, advisory' = advisory,
    practicesStage' = practicesStage,
    // フロア — ジャーナルの STAGE_JUMPED はステージ非依存のフロアである
    // (fail-closed over precise)。全ステージの試行を空へ戻す。
    affirmed' = false,
    reqCount' = STAGES.mapBy(u => 0),
    pending' = STAGES.mapBy(u => Set()),
    terminal' = STAGES.mapBy(u => false),
    lastAction' = "jump_redo",
    synced' = synced,
    foreign' = foreign,
  }

  // park: autonomous 中は拒否 (unattended run has no human to resume it)。
  // park 済みからの再 park も受理する — upstream の handlePark は park 済みを成功させ、
  // Parked 時刻を上書き・Parked At Stage を書き直し・WORKFLOW_PARKED を再 emit する
  // (**再スタンプ**。.claude/tools/aidlc-state.ts:1685-1760)。park 中はカーソルを動かす
  // アクションがすべて status == Running を要求するので、再 park の parkedAt' = cursor は
  // 常に同じ位置への再書き込みであり parked_position を保つ。
  action actPark = all {
    or { status == Running, status == WorkflowParked },
    not(autonomous),
    status' = WorkflowParked,
    parkedAt' = cursor,
    plan' = plan, foreignPlan' = foreignPlan, overlay' = overlay, conditional' = conditional,
    checkbox' = checkbox, cursor' = cursor, autonomous' = autonomous,
    approved' = approved,
    lastDirective' = DParked,
    stanceRecorded' = stanceRecorded,
    reviewed' = reviewed, advisory' = advisory,
    practicesStage' = practicesStage, affirmed' = affirmed,
    reqCount' = reqCount, pending' = pending, terminal' = terminal,
    lastAction' = "park",
    synced' = synced,
    foreign' = foreign,
  }

  action actUnpark = all {
    status == WorkflowParked,
    status' = Running,
    parkedAt' = -1,
    plan' = plan, foreignPlan' = foreignPlan, overlay' = overlay, conditional' = conditional,
    checkbox' = checkbox, cursor' = cursor, autonomous' = autonomous,
    approved' = approved,
    lastDirective' = DNone,
    stanceRecorded' = stanceRecorded,
    reviewed' = reviewed, advisory' = advisory,
    practicesStage' = practicesStage, affirmed' = affirmed,
    reqCount' = reqCount, pending' = pending, terminal' = terminal,
    lastAction' = "unpark",
    synced' = synced,
    foreign' = foreign,
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
      plan' = plan, foreignPlan' = foreignPlan, conditional' = conditional,
      checkbox' = checkbox, cursor' = cursor, status' = status,
      parkedAt' = parkedAt, autonomous' = autonomous, approved' = approved,
      lastDirective' = DNone,
      stanceRecorded' = stanceRecorded,
      reviewed' = reviewed, advisory' = advisory,
      practicesStage' = practicesStage, affirmed' = affirmed,
      reqCount' = reqCount, pending' = pending, terminal' = terminal,
      lastAction' = "recompose",
      synced' = synced,
      foreign' = foreign,
    }
  }

  // 受理集合は状態に依らない (v2.7 / b50) — upstream は状態ファイルを読まない。
  action actSetAutonomy = all {
    autonomous' = not(autonomous),
    plan' = plan, foreignPlan' = foreignPlan, overlay' = overlay, conditional' = conditional,
    checkbox' = checkbox, cursor' = cursor, status' = status,
    parkedAt' = parkedAt, approved' = approved,
    lastDirective' = DNone,
    stanceRecorded' = stanceRecorded,
    reviewed' = reviewed, advisory' = advisory,
    practicesStage' = practicesStage, affirmed' = affirmed,
    reqCount' = reqCount, pending' = pending, terminal' = terminal,
    lastAction' = "set_autonomy",
    synced' = synced,
    foreign' = foreign,
  }

  // 完了後のスタッタ (デッドロック回避)
  action actDoneStutter = all {
    status == WorkflowCompleted,
    plan' = plan, foreignPlan' = foreignPlan, overlay' = overlay, conditional' = conditional,
    checkbox' = checkbox, cursor' = cursor, status' = status,
    parkedAt' = parkedAt, autonomous' = autonomous, approved' = approved,
    lastDirective' = DDone,
    stanceRecorded' = stanceRecorded,
    reviewed' = reviewed, advisory' = advisory,
    practicesStage' = practicesStage, affirmed' = affirmed,
    reqCount' = reqCount, pending' = pending, terminal' = terminal,
    lastAction' = "done_stutter",
    synced' = synced,
    foreign' = foreign,
  }

  // record-skeleton-stance: conductor が分類した walking-skeleton stance の記録 (#73)。
  // 受理できるのはカーソルが skeleton-gate ステージに立っているときだけ (upstream の
  // handleSkeletonStanceReport の防御)。skeleton-gate ステージの抽象は skeletonGateStage
  // (静的計画の最初の非 init EXECUTE ステージ) — ヘッダ v2.4 の簡約注記を参照。
  // 再記録は受理する (上書き — 冪等に true のまま)。
  // 動かすのは stanceRecorded だけであり、それを stance_frame が検査する。
  // ガードは skeleton-gate 位置だけである — Rust の record_skeleton_stance も取り違えと
  // この位置しか見ず、park 中・完了後を拒まない (ピンにもそのガードは無い)。v2.4 当初は
  // `status == Running` で狭めていたが、モデルを実装の受理集合に揃えて撤去した
  // (PR #103 のレビュー指摘)。
  action actRecordSkeletonStance = all {
    cursor == skeletonGateStage,
    stanceRecorded' = true,
    reviewed' = reviewed, advisory' = advisory,
    practicesStage' = practicesStage, affirmed' = affirmed,
    reqCount' = reqCount, pending' = pending, terminal' = terminal,
    plan' = plan, foreignPlan' = foreignPlan, overlay' = overlay, conditional' = conditional,
    checkbox' = checkbox, cursor' = cursor, status' = status,
    parkedAt' = parkedAt, autonomous' = autonomous, approved' = approved,
    lastDirective' = DNone,
    lastAction' = "record_skeleton_stance",
    synced' = synced,
    foreign' = foreign,
  }

  // single-stage run: 隔離実行 (`report --single`) の疑似ワークフロー ID 付き対の記録 (#73)。
  // 本流の状態 (Completed / park 中 / autonomous) に依らず受理され、**状態変数を 1 つも
  // 動かさない** — これが仕様 I10 (--single は本流を進められない) の強制手段であり、
  // single_run_frame が検査するフレーム条件そのものである (設計 §0 の裁定 B)。
  // 対象ステージは全ステージから nondet に選び、initialization の拒否は `gated(s)` が
  // 単独で担う — これは Rust の record_single_stage_run が持つ唯一のガード
  // (require_gated = 非 initialization) の写しである。選ばれた値は状態に現れない。
  action actSingleRun = {
    nondet s = STAGES.oneOf()
    all {
      gated(s),
      plan' = plan, foreignPlan' = foreignPlan, overlay' = overlay, conditional' = conditional,
      checkbox' = checkbox, cursor' = cursor, status' = status,
      parkedAt' = parkedAt, autonomous' = autonomous, approved' = approved,
      stanceRecorded' = stanceRecorded,
      reviewed' = reviewed, advisory' = advisory,
      practicesStage' = practicesStage, affirmed' = affirmed,
      reqCount' = reqCount, pending' = pending, terminal' = terminal,
      lastDirective' = DNone,
      lastAction' = "single_run",
      synced' = synced,
      foreign' = foreign,
    }
  }

  // --- レビュー受領証の 3 アクション (b48 / B10) ---
  //
  // 本流の状態 (カーソル・checkbox・Status・park・overlay・autonomy・承認履歴・stance) は
  // どれも動かさない — upstream の `aidlc-log review` は監査行を足すだけで状態ファイルを
  // 書かないからである。それを review_frame が検査する。

  // 依頼 (REVIEW_REQUESTED): レビュアーを宣言したステージで、予算の残りがあるときだけ通る。
  // 通し番号は「数え上げ済みの依頼数 + 1」に固定されている (upstream の順序ガード)。
  action actRequestReview = {
    nondet s = STAGES.oneOf()
    all {
      reviewed.get(s),
      reqCount.get(s) + 1 <= budget(s),
      // 判定待ちの依頼が残る間は次の依頼を受けない (集約の PendingIterations — v2.8)。
      pending.get(s) == Set(),
      reqCount' = reqCount.set(s, reqCount.get(s) + 1),
      pending' = pending.set(s, pending.get(s).union(Set(reqCount.get(s) + 1))),
      terminal' = terminal,
      reviewed' = reviewed, advisory' = advisory,
      practicesStage' = practicesStage, affirmed' = affirmed,
      plan' = plan, foreignPlan' = foreignPlan, overlay' = overlay, conditional' = conditional,
      checkbox' = checkbox, cursor' = cursor, status' = status,
      parkedAt' = parkedAt, autonomous' = autonomous, approved' = approved,
      stanceRecorded' = stanceRecorded,
      lastDirective' = DNone,
      lastAction' = "request_review",
      synced' = synced,
      foreign' = foreign,
    }
  }

  // 呼び直し (Retry: pending-request): 判定待ちの依頼をもう一度差し向けるだけで、
  // 依頼の回数には数えない (upstream `:810-812`)。**全状態変数が不変**である。
  action actRetryReview = {
    nondet s = STAGES.oneOf()
    nondet i = pending.get(s).oneOf()
    all {
      pending.get(s) != Set(),
      i >= 1,
      reqCount' = reqCount, pending' = pending, terminal' = terminal,
      reviewed' = reviewed, advisory' = advisory,
      practicesStage' = practicesStage, affirmed' = affirmed,
      plan' = plan, foreignPlan' = foreignPlan, overlay' = overlay, conditional' = conditional,
      checkbox' = checkbox, cursor' = cursor, status' = status,
      parkedAt' = parkedAt, autonomous' = autonomous, approved' = approved,
      stanceRecorded' = stanceRecorded,
      lastDirective' = DNone,
      lastAction' = "retry_review",
      synced' = synced,
      foreign' = foreign,
    }
  }

  // 判定 (REVIEW_COMPLETED): 開いている依頼にだけ対応する。判定の値そのものは持たず、
  // 「終端受領証が在るか」の 1 bit へ射影する — READY、advisory の 1 パス、または
  // 上限到達の NOT-READY が終端である (upstream `terminalReviewVerdict`)。
  action actRecordVerdict = {
    nondet s = STAGES.oneOf()
    nondet i = pending.get(s).oneOf()
    nondet ready = Bool.oneOf()
    all {
      pending.get(s) != Set(),
      pending' = pending.set(s, pending.get(s).exclude(Set(i))),
      terminal' = terminal.set(s, terminal.get(s) or ready or advisory.get(s) or i >= BUDGET),
      reqCount' = reqCount,
      reviewed' = reviewed, advisory' = advisory,
      practicesStage' = practicesStage, affirmed' = affirmed,
      plan' = plan, foreignPlan' = foreignPlan, overlay' = overlay, conditional' = conditional,
      checkbox' = checkbox, cursor' = cursor, status' = status,
      parkedAt' = parkedAt, autonomous' = autonomous, approved' = approved,
      stanceRecorded' = stanceRecorded,
      lastDirective' = DNone,
      lastAction' = "record_verdict",
      synced' = synced,
      foreign' = foreign,
    }
  }

  // --- 昇格受領証のアクション (b49 / B10) ---
  //
  // 昇格 (`aidlc-state practices-promote`) は memory 層の 2 ファイルを書いて監査行を足すだけで、
  // 本流の状態 (カーソル・checkbox・Status・park・overlay・autonomy・承認履歴・stance) も
  // レビュー会計も動かさない — それを promote_frame が検査する。受理に本流の状態を見ない
  // のは upstream の handlePracticesPromote に status ガードが 1 つも無いからである。
  // 再昇格は受理する (上書き — 冪等に true のまま)。
  action actPromotePractices = all {
    practicesStage != -1,
    affirmed' = true,
    practicesStage' = practicesStage,
    reviewed' = reviewed, advisory' = advisory,
    reqCount' = reqCount, pending' = pending, terminal' = terminal,
    plan' = plan, foreignPlan' = foreignPlan, overlay' = overlay, conditional' = conditional,
    checkbox' = checkbox, cursor' = cursor, status' = status,
    parkedAt' = parkedAt, autonomous' = autonomous, approved' = approved,
    stanceRecorded' = stanceRecorded,
    lastDirective' = DNone,
    lastAction' = "promote_practices",
    synced' = synced,
    foreign' = foreign,
  }

  // TaskUpdate 同期 (v2.8): 本家 `aidlc-utility set-status --stage <slug>` の射影。
  // Running 中に任意の stage を現在位置にし、その checkbox だけを InProgress にする。旧 stage の
  // checkbox は戻さない (両方 `[-]` になり得る)。実効 SKIP の stage も受理し、その場合は
  // synced を立てて cursor_in_scope の適用を外す — `next` はその位置で DError (skip の不整合) を
  // 返し、report_skipped の回復で再び実効 EXECUTE へ進む。
  action actTaskSync = {
    nondet t = STAGES.oneOf()
    all {
      status == Running,
      t != 0,
      checkbox' = checkbox.set(t, InProgress),
      cursor' = t,
      synced' = not(inScope(t)),
      // カーソルは同期が置き直す — 跳躍由来の印はここで落ちる (Rust 側 mutate の
      // TaskSynchronized 腕と同じ規則)。
      foreign' = false,
      plan' = plan, foreignPlan' = foreignPlan, overlay' = overlay, conditional' = conditional,
      status' = status, parkedAt' = parkedAt, autonomous' = autonomous,
      approved' = approved,
      lastDirective' = DNone,
      stanceRecorded' = stanceRecorded,
      reviewed' = reviewed, advisory' = advisory,
      practicesStage' = practicesStage, affirmed' = affirmed,
      reqCount' = reqCount, pending' = pending, terminal' = terminal,
      lastAction' = "task_sync",
    }
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
    all { snapshot, actForeignJumpForward },
    all { snapshot, actJumpBackward },
    all { snapshot, actJumpRedo },
    all { snapshot, actPark },
    all { snapshot, actUnpark },
    all { snapshot, actRecompose },
    all { snapshot, actSetAutonomy },
    all { snapshot, actDoneStutter },
    all { snapshot, actRecordSkeletonStance },
    all { snapshot, actSingleRun },
    all { snapshot, actRequestReview },
    all { snapshot, actRetryReview },
    all { snapshot, actRecordVerdict },
    all { snapshot, actPromotePractices },
    all { snapshot, actTaskSync },
  }

  // === 不変条件 (DoD — ADR 0003 決定 7) ===

  // 有効プランが SKIP のステージに run-stage を放出しない (観測射影)
  val no_run_stage_for_skip = match lastDirective {
    | DRunStage(s) => effectivePlan(s) == Execute
    | _ => true
  }

  // 状態レベルの守り: Running 中、エンジン遷移が置いたカーソルは常に実効 EXECUTE 上にある
  // (単一ガード破壊を直接検出する — mutation テストで確認済み)。TaskUpdate 同期が実効 SKIP へ
  // 置いた位置 (synced) と、別 scope を名指した直接 execute が自実効計画の外へ置いた位置
  // (foreign — v2.9、裁定 jump-contract Q1 = A) は本家の観測どおり受理し、この守りの対象外と
  // する。
  val cursor_in_scope =
    (status == Running and not(synced) and not(foreign)) implies (effectivePlan(cursor) == Execute)

  // redo (別 scope を名指さない直接 execute) は、跳躍に使う計画 = 自実効計画で EXECUTE の
  // 位置にしか届かない — 集約の jump_execute_guard が jump_plan(None) = in_scope で判定し、
  // 外れた位置は InvalidTarget を返す (裁定 jump-redo-guard Q1 = A、モデルを実装へ揃える)。
  // TaskUpdate 同期 (synced) や別 scope の跳躍 (foreign) が計画外へ置いたカーソルも例外では
  // ない。
  val jump_redo_on_plan =
    (lastAction == "jump_redo") implies (effectivePlan(cursor) == Execute)

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

  // マーカーが立っているなら status は WorkflowParked である (ヘッダの簡約規約
  // 「3 値 status は『マーカー有 ∧ 位置一致』の合成状態のみを表す」をそのまま不変条件に
  // したもの)。parked_position / unpark_restores_position は status == WorkflowParked を
  // 前件に持つため、park が status を動かさない変異では空虚に成立してしまう。本条件は
  // マーカー側を前件に置くので、その穴を塞ぐ (b45 / #74 — mutation テストで確認済み)。
  val parked_marker_status = (parkedAt != -1) implies (status == WorkflowParked)

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

  // (v2.8) at_most_one_active は降格した — 本家 2.7.1 の実観測に反例がある (ヘッダ参照)。
  // 到達性 witness として残し、複数 active の経路が実在することを固定する。
  val w_two_active = STAGES.filter(s => or {
    checkbox.get(s) == InProgress,
    checkbox.get(s) == AwaitingApproval,
    checkbox.get(s) == Revising,
  }).size() >= 2
  val w_task_sync = lastAction == "task_sync"

  // 別 scope 由来の跳躍がカーソルを自実効計画の外へ置く経路が実在する (v2.9 —
  // cursor_in_scope の foreign 除外が空虚に成立していないことの証拠)
  val w_foreign_jump = foreign

  // redo 経路が実在する (v2.10 — actJumpRedo に足した inScope(cursor) ガードが
  // アクションを死なせていないことの証拠)
  val w_jump_redo = lastAction == "jump_redo"

  // 隔離実行 (--single) は本流の状態を 1 つも動かさない (フレーム条件 — 仕様 I10 の
  // 強制手段)。観測用の lastDirective / lastAction は更新される — 検査対象は本流の
  // 状態変数だけである。
  // 設計 §0 の裁定 B: 疑似ワークフロー ID 付きの対を IntentExecution のイベントとして
  // 記録するので、「本流を進めない」ことは適用のフレーム空性で保証するしかない。
  val single_run_frame = (lastAction == "single_run") implies and {
    cursor == prevCursor,
    status == prevStatus,
    checkbox == prevCheckbox,
    overlay == prevOverlay,
    parkedAt == prevParkedAt,
    autonomous == prevAutonomous,
    approved == prevApproved,
    stanceRecorded == prevStanceRecorded,
  }

  // stance の記録が動かすのは stanceRecorded だけである (フレーム条件)。
  val stance_frame = (lastAction == "record_skeleton_stance") implies and {
    cursor == prevCursor,
    status == prevStatus,
    checkbox == prevCheckbox,
    overlay == prevOverlay,
    parkedAt == prevParkedAt,
    autonomous == prevAutonomous,
    approved == prevApproved,
  }

  // === b48: レビュー受領証の不変条件 ===

  // 段 11 — レビュアーを宣言したゲート付きステージの承認は、その時点で終端受領証を
  // 持っていた (前状態スナップショットで見る — 承認そのものが次ステージのフロアを
  // 動かすため、承認後の値ではなく承認直前の値が正しい前提である)。
  val approve_requires_terminal_receipt =
    (lastAction == "report_forward" and gated(prevCursor) and reviewed.get(prevCursor))
      implies prevTerminal.get(prevCursor)

  // 試行のフロア — 差し戻しはカーソルを、ジャンプ 4 種は全ステージを、前進と読み飛ばしは
  // 動いた先の新カーソルを空へ戻す。
  val review_attempt_floor = and {
    (lastAction == "report_rejected") implies attemptEmpty(cursor),
    (lastAction == "jump_forward" or lastAction == "jump_backward" or lastAction == "jump_redo"
      or lastAction == "foreign_jump")
      implies STAGES.forall(s => attemptEmpty(s)),
    ((lastAction == "report_forward" or lastAction == "report_skipped") and cursor != prevCursor)
      implies attemptEmpty(cursor),
  }

  // 予算と通し番号の健全性 — 依頼数は予算を超えず、判定待ちの番号は [1, reqCount] に居る。
  val review_budget = STAGES.forall(s => and {
    reqCount.get(s) <= budget(s),
    pending.get(s).forall(i => i >= 1 and i <= reqCount.get(s)),
  })

  // 受領証の記録は本流の状態を動かさない (フレーム条件)。呼び直しは受領証の会計すら
  // 動かさない。
  val review_frame = and {
    (lastAction == "request_review" or lastAction == "record_verdict"
      or lastAction == "retry_review") implies and {
      cursor == prevCursor,
      status == prevStatus,
      checkbox == prevCheckbox,
      overlay == prevOverlay,
      parkedAt == prevParkedAt,
      autonomous == prevAutonomous,
      approved == prevApproved,
      stanceRecorded == prevStanceRecorded,
    },
    (lastAction == "retry_review") implies and {
      reqCount == prevReqCount,
      pending == prevPending,
      terminal == prevTerminal,
    },
  }

  // === b49: 昇格受領証の不変条件 ===

  // 段 12 — practices-discovery の承認は、その時点で昇格受領証を持っていた
  // (前状態スナップショットで見る — 承認そのものが次ステージのフロアを動かすため、
  // 承認後の値ではなく承認直前の値が正しい前提である)。
  val approve_requires_practices_receipt =
    (lastAction == "report_forward" and prevCursor == practicesStage) implies prevAffirmed

  // 試行のフロア — 差し戻しは practices ステージの受領証を、ジャンプ 4 種は無条件に、
  // 前進と読み飛ばしはカーソルが practices ステージへ動いたときに落とす。
  val practices_receipt_floor = and {
    (lastAction == "report_rejected" and cursor == practicesStage) implies not(affirmed),
    (lastAction == "jump_forward" or lastAction == "jump_backward" or lastAction == "jump_redo"
      or lastAction == "foreign_jump")
      implies not(affirmed),
    ((lastAction == "report_forward" or lastAction == "report_skipped")
      and cursor != prevCursor and cursor == practicesStage) implies not(affirmed),
  }

  // 昇格は本流の状態も stance もレビュー会計も動かさない (フレーム条件)。
  val promote_frame = (lastAction == "promote_practices") implies and {
    cursor == prevCursor,
    status == prevStatus,
    checkbox == prevCheckbox,
    overlay == prevOverlay,
    parkedAt == prevParkedAt,
    autonomous == prevAutonomous,
    approved == prevApproved,
    stanceRecorded == prevStanceRecorded,
    reqCount == prevReqCount,
    pending == prevPending,
    terminal == prevTerminal,
  }

  // === 到達性 witness (負形式で検査 — ADR 0003 決定 7) ===

  // 再スタンプ経路が実在する: park 済み (prevParkedAt != -1) の状態から park を打てる
  val w_repark = (lastAction == "park") and (prevParkedAt != -1)

  // 隔離実行の経路が実在する (フレーム空アクションが到達可能でなければ
  // single_run_frame は空虚に成立してしまう)
  val w_single_run = lastAction == "single_run"

  // stance 記録の経路が実在する (同上 — stance_frame の空虚成立を防ぐ)
  val w_stance_recorded = lastAction == "record_skeleton_stance"

  // 依頼の経路が実在する (review_budget / review_frame の空虚成立を防ぐ)
  val w_review_requested = lastAction == "request_review"

  // 終端受領証に到達する経路が実在する (承認ガードが通れる歴史が在る)
  val w_verdict_terminal =
    (lastAction == "record_verdict") and STAGES.exists(s => terminal.get(s))

  // **レビュアーを宣言したゲートを実際に承認する**経路が実在する —
  // approve_requires_terminal_receipt が空虚に成立していないことの証拠である。
  val w_approved_reviewed =
    (lastAction == "report_forward") and gated(prevCursor) and reviewed.get(prevCursor)

  // 呼び直しの経路が実在する (review_frame の後半の空虚成立を防ぐ)
  val w_retry_review = lastAction == "retry_review"

  // 昇格の経路が実在する (promote_frame の空虚成立を防ぐ)
  val w_practices_affirmed = lastAction == "promote_practices"

  // **practices-discovery のゲートを実際に承認する**経路が実在する —
  // approve_requires_practices_receipt が空虚に成立していないことの証拠である。
  val w_approved_practices =
    (lastAction == "report_forward") and prevCursor == practicesStage
}
