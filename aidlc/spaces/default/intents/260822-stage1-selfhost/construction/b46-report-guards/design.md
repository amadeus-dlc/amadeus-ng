# b46 設計 — report の 13 段ガード連鎖（本流）: `report_dispatch` を集約のクエリへ、逐語 3c3146cf 準拠、完了投影の穴を塞ぐ（2026-09-04）

対象: GitHub #73（#7 キュー 5）の**本流部分**と、b45 で判明した「完了がリード面へ投影されない」穴。
契約の正本: `docs/specs/research/orchestration-report-guards.md`（13 段の契約マップ）と、ピン留め `3c3146cf` の
`core/tools/aidlc-orchestrate.ts` `handleReport`（`:5464-5927`）/ `handleResumeReport`（`:5383-5457`）/
`aidlc-lib.ts classifyStateVersion` / `aidlc-state.ts complete-workflow`（`:2415-2560`）。逐語は**ピン**から取る
（`.claude/tools/` の配布物は 2.7.1 で、ピンより新しい分岐 — 提示選択肢の照合 `Accept as-is` 等 — を含むので使わない）。

## 0. 分割（オーナー裁定 2026-09-03 = B「キューのまま」の範囲で、Bolt 3 と同じく PR を分ける）

| Bolt | 中身 | 状態 |
| --- | --- | --- |
| **b46（本 PR）** | 段 1（state-version guard）・段 4（resume ルーティング）・段 5〜10・段 13・forward ディスパッチ・後処理の逐語、`skipped` の受理 5 条件、RMU の完了投影。段 2 / 段 3 は**構文検証と逐語だけ**（本体は b47） | 着手 |
| b47 | `--single`（`SingleStageRun` — synthetic-id pair、遷移ポート非注入 = I10）と `--skeleton-stance`（新コマンド・イベント・skeleton-gate アンカー） | 次 |
| b48 | **B10 レシート鮮度（#51 = A）と段 12（practices-discovery の受領証）** — オーナー裁定 2026-09-04（本 Bolt 中に質問）: **(A) 受領証は集約 `IntentExecution` のイベント**（`record_review_receipt` → `ReviewReceiptRecorded`、監査行は RMU が投影、鮮度の判断は集約のクエリ）、**(i) 鮮度は順序だけ**（直近の開始・差し戻しより後。成果物ハッシュの照合は凍結検査として後続 intent）。受領証を書く動詞（`aidlc-audit append` 相当）の配線を含む。段 11（completion-evidence）は slice 2 | 裁定済み・b47 の後 |
| CP5 へ繰延 | 段 1 の turn-shape marker（`markEngineTouch` — Stop フックの最適化用マーカー。消費者のフックが未実装で、マーカーの置き場と形式はフック Bolt で決める） | 記録のみ |

## 1. 原則からの導出

- **判断は集約**: 「どの遷移を打つか / 拒否するか」を (verdict, checkbox, gated, final, moved-on, explicit-stage) の
  5 引数で決めるのは仕様 §2.3 の `report_dispatch`。独立ドメインサービスは作らず（オーナー規律「集約は FSM、導出を
  独立サービスに置かない」）、**集約のクエリメソッド** `IntentExecution::report_dispatch(&self, intent, &ReportRequest) -> Result<ReportDecision, ReportRefusal>` にする。
  `next_decision` と同じ形（`&self`、失敗しない材料の読み、判断は集約に閉じる）。
- **ユースケースは進行管理**: `CommitVerdictUseCase` は find → `report_dispatch` → 決定どおりに集約コマンド → store。
  拒否・no-op は**材料付きの値**で呼出側へ返し、言い換えない。
- **文言は出す側**: 逐語は app の `wording.rs`。拒否の材料（slug・checkbox の綴り・execution kind・現在地 slug・scope）は
  `ReportRefusal` / `ReportDecision` が運ぶ。app は文言の組み立てのために**リードモデルを読まない**（材料は決定が運ぶ）。
- **構文的な段だけ app が持つ**（値の有無で決まる段）: `--single` の有無、`--skeleton-stance` の有無、`--result` の
  有無・既知値、`resume` 分岐、`--stage` の有無、`--reason` / `--user-input` の空判定、env `AIDLC_SKIP_HUMAN_PRESENCE_GUARD`。
  状態の値で決まる分岐は 1 つも持たない。
- **RMU は計算結果を行と状態ファイルへ**: ワークフロー完了はイベント適用後の集約 `status()` から導き、upstream
  `complete-workflow` と同じ欄と監査 3 行を描く。

## 2. app（合成ルート）— 段の順序と逐語

`report(layout, args)` の順序（ピン `handleReport` と同順）:

1. **段 1 state-version guard**: `<record>/aidlc-state.md` があれば（0 バイトも「ある」）`State Version` を分類し、`ok` 以外は
   `error` directive。読むのはクエリ側 `StateFileDao`（新設、record の状態ファイルの生テキストを返す DAO — 媒体はファイル。
   port は `StateFileDao { fn read(&self) -> Result<Option<String>, ReadModelReadError> }`、View は生テキスト）+
   `FindStateFileUseCase`。分類は domain の `StateVersionClassification::classify(&text)`（app は domain に依存してよい）。
   `version()` アクセサが無ければ足す。文言は `wording::incompatible_state_version(kind, version)` — ピン
   `aidlc-lib.ts classifyStateVersion` の 3 形（unparseable / future / past、`CURRENT_STATE_VERSION = "8"`）を逐語で。
   turn-shape marker は CP5 へ繰延（§0）。
2. **段 2 `--single`**: `--result` 無し → `report --single requires --result <outcome>. Accepted: approved, completed, complete, done (the verdict for the single stage just run).`、
   FORWARD 以外 → `Unknown --result "<v>". report commits forward outcomes only; accepted: approved, completed, complete, done.`、
   `--stage` 無し → `report --single must not advance the main workflow. Pass --stage <slug> to commit the single stage's synthetic-id pair; --single never writes the main workflow's Current Stage.`、
   それ以外 → **b47 まで** `error` directive `Cannot complete isolated stage "<slug>": single-stage reporting is not wired in this build.`（b29 の park と同じ層。本流は絶対に進めない = I10）。
3. **段 3 `--skeleton-stance`**: 値検証 `Unknown --skeleton-stance "<v>". Accepted: on, off, scope-dependent (the walking-skeleton stance classified from the team's ## Walking Skeleton prose).`、
   state 無し → `No active intent workflow state found (aidlc-state.md is absent) — nothing to record a skeleton stance for.`、
   それ以外 → **b47 まで** `error` `Cannot record skeleton stance "<stance>": skeleton-stance reporting is not wired in this build.`。
4. **段 4 resume**（`--result resume|resumed`）: `--stage` あり → `A resume-choice report is not a stage transition; omit --stage.`、
   `--user-input` 空 → `report --result resumed requires --user-input with the human's resume choice.`、state 無し →
   `No active intent workflow state found (aidlc-state.md is absent) - nothing to resume.`（ハイフンは ASCII `-`）、
   現在地は `FindExecutionUseCase` の `cursor_slug`（無ければ `State file has no Current Stage field - cannot resume from the last checkpoint.`）、
   `1`〜`4` の正規化 → `redo` / `jump` / `fresh|start over` / `resume|checkpoint|continue` を `print` で逐語（ピン `:5383-5457`）。
   redo の命令綴りは逸脱台帳 #1 の写像で `aidlc-jump execute --target <slug> --direction redo --scope <scope>`（`next/stage-jump-print` と同じ扱い — cli_golden_test の記載）。
   非該当 → `Unrecognized resume choice "<input>". Accepted choices: 1/resume from last checkpoint, 2/redo the current stage, 3/jump to a stage, or 4/start fresh.`
5. **段 5**: `--result` 無し → `report requires --result <outcome>. Accepted: approved, completed, complete, done, awaiting-approval, rejected, revised, resume, resumed, skipped (the verdict for the stage just acted on).`、
   未知 → `Unknown --result "<v>". accepted outcomes: approved, completed, complete, done, awaiting-approval, rejected, revised, resume, resumed, skipped.`（現行の `unknown_result` を逐語へ揃える）。
6. **段 6**: 実行カーソル不在（未鋳造）→ `No active intent workflow state found (aidlc-state.md is absent) — nothing to report a transition for.`（現行「No workflow execution to report against.」を置換）。壊れたカーソルは従来どおり `unreadable_execution_cursor`。
7. **段 7〜8** は集約側（§3）: `--stage` の値が slug 形でない → `Internal: reported stage "<v>" is not in the compiled graph — cannot commit its transition.`（現行「The --stage value is not a stage slug.」を置換）。計画に無い slug も同文。
8. **段 9 skipped の構文段**: `--stage` 無し → `report --result skipped requires an explicit nonblank --stage <slug>.`（集約より前）。`--reason` の空判定は**集約の順序**（execution 検査の後）で行うので、`ReportRequest` に `reason: Option<String>` のまま渡す。
9. **段 13 の env**: `human_presence_guard = env AIDLC_SKIP_HUMAN_PRESENCE_GUARD != "1"` を `ReportRequest` に載せる（判定自体は集約）。
10. ユースケース呼出 → 結果の描画（§4）。

## 3. 集約 — `IntentExecution::report_dispatch`（クエリ、`&self`）

入力 `ReportRequest`（値オブジェクト、use-case 層ではなく domain に置く — 集約のクエリの引数）:
`verdict: Verdict`（Resume は来ない）、`stage: Option<StageSlug>`（明示 `--stage`）、`user_input: Option<String>`、
`reason: Option<String>`、`human_presence_guard: bool`。

出力 `ReportDecision`（値）:
- `Commit { target: StageIndex, steps: Vec<TransitionStep> }` — `TransitionStep` は `GateStartRecovered | GateStart | Approve | Reject | Revise | Skip | Advance | CompleteWorkflow`。
  `subcommand()` が upstream の綴り（`gate-start` / `approve` / `reject` / `revise` / `skip` / `advance` / `complete-workflow`）を返し、`Committed <subs joined by " + ">` の材料になる。
- `NoOp(ReportNoOp)` — `AlreadyAwaiting { stage }`（print）/ `AlreadyCompletedMovedOn { stage, current }`（done）/ `WorkflowAlreadyCompleted { stage }`（done）。

拒否 `ReportRefusal`（材料つき、順序はピンの段どおり）:
`UnknownStage { named }`（段 8）、`SkipNotConditional { stage, execution }`、`SkipRequiresReason { stage }`、
`SkipMustNameCursor { named, current }`、`SkipPrecondition { stage, actual }`（段 9）、
`UngatedStage { stage, verdict }`、`GatePrecondition { stage, verdict, actual }`（段 10。awaiting/rejected/revised それぞれの前提）、
`RejectRequiresFeedback { stage }`、`HumanPresence { stage, verdict }`（段 13）、
`ForwardCommitsCompletionsOnly { stage, actual }`、`StillPending { stage }`、`InProgressRequiresExplicitStage { stage }`（forward 表）。

判定順（ピン `:5545-5860`）: 対象 = 明示 slug か cursor → 計画に無ければ `UnknownStage` → **skipped** の腕（execution が
CONDITIONAL でも実効 SKIP でもない → `SkipNotConditional`；reason 空 → `SkipRequiresReason`；named ≠ cursor →
`SkipMustNameCursor`；checkbox ∉ {in-progress, revising, skipped} → `SkipPrecondition`；OK → `Commit[Skip]`）→ gated =
`phase != initialization` → **gate 系**（!gated → `UngatedStage`；awaiting: 既に `[?]` → `NoOp::AlreadyAwaiting`、`[-]` 以外 →
`GatePrecondition`、OK → `Commit[GateStart]`；rejected: `[-]`/`[?]` 以外 → `GatePrecondition`、feedback = user_input ?? reason
が空 → `RejectRequiresFeedback`、OK → `Commit[Reject]`；revised: `[R]` 以外 → `GatePrecondition`、OK → `Commit[Revise]`）→
**段 13**: gated ∧ checkbox ≠ completed ∧ autonomy ≠ autonomous ∧ human_presence_guard ∧ user_input 空 → `HumanPresence` →
**forward 表**: `[S]`/`[R]` → `ForwardCommitsCompletionsOnly`；`[ ]` → `StillPending`；`[x]` ∧ final ∧ Completed →
`NoOp::WorkflowAlreadyCompleted`；`[x]` ∧ 非 final ∧ cursor が別 stage で pending でない → `NoOp::AlreadyCompletedMovedOn`
（我々の ES では「approve は済んだが advance 前」は原子的に存在しないので、`[x]` の再報告は常にこの no-op か
`WorkflowAlreadyCompleted`）；gated ∧ `[-]` ∧ 明示無し → `InProgressRequiresExplicitStage`；gated ∧ `[-]` ∧ 明示あり →
`Commit[GateStartRecovered, Approve]`；gated ∧ `[?]` → `Commit[Approve]`；非 gated は誕生 = 初期化完了済み（b34）以降
到達不能（`Advance` / `CompleteWorkflow` は表の完全性のために列挙するが、テストで到達不能を固定する）。

`ReportRefusal` は `CommandError` の変種ではなく**別の型**（`report_dispatch` の戻り値）。既存の `CommandError` は
集約コマンド自身のガード用に残す。集約コマンド側の変更は不要（`approve_gate` は BR1.3 で `[-]` からの承認を受理済み）。

## 4. ユースケース `CommitVerdictUseCase` の改訂

- `execute(execution_id, request: ReportRequest, occurred_at) -> Result<CommitOutcome, CommitError>`。
  `CommitOutcome::Committed { stage: StageSlug, scope: String, steps: Vec<TransitionStep> }` / `NoOp { stage, scope, no_op }`。
  scope は intent から（`Committed ... (scope: <scope>)` の材料）。
- `CommitError` に `Refused(ReportRefusal)` を追加。`UnknownStage` は `ReportRefusal` 側へ移る（既存変種は撤去 — 後方互換は残さない）。
- `steps` の実行: `GateStartRecovered` は独立イベントではない — `Approve` が `[-]` から承認する 1 イベント（BR1.3）で、監査の
  `STAGE_AWAITING_APPROVAL`（recovered）行は RMU が `GateApproved` の投影で `[-]` からの承認と判って描く（現状の投影を確認し、
  無ければ `Recovered: true` 相当の行を足す — ピン `gate-start --recovered` の監査行の形に合わせる）。`Skip` → `skip_stage`、
  `Reject` → `reject_gate`、`Revise` → `revise_stage`、`GateStart` → `open_gate`。
- stale re-report（`is_stale_re_report`）と `gate_is_already_open` のフロー制御は `report_dispatch` に吸収し、ユースケースから消す。
- `Conflict` の 1 回再試行は現状どおり。

## 5. app の描画（`wording.rs`）

- 成功: `Commit` → `done` `Committed <subs joined by " + "> for "<slug>" (scope: <scope>). State advanced; run next to continue.`
  ただし `Skip` は `Committed skip for "<slug>" (scope: <scope>). State routed forward; run next to continue.`；
  gate 系（GateStart / Reject / Revise）は `print` `Recorded <result> for "<slug>".`（result は報告語そのまま）。
- no-op: `AlreadyAwaiting` → `print` `Stage "<slug>" is already awaiting approval.`；`AlreadyCompletedMovedOn` → `done`
  `Stage "<slug>" is already completed and the workflow has moved on to "<current>" (scope: <scope>); idempotent re-report, no transition needed.`；
  `WorkflowAlreadyCompleted` → `done` `Workflow is already completed at "<slug>" (scope: <scope>); no transition was needed.` + `NEW_WORK_HINT`（既存定数）。
- 拒否の逐語（ピン）: `Stage "<slug>" is execution: <ALWAYS|CONDITIONAL>; only a CONDITIONAL stage can report skipped.` /
  `report --result skipped requires a nonblank --reason <text>.` / `Cannot skip stage "<slug>": Current Stage is "<current>". A skip report must name the active stage exactly.` /
  `Stage "<slug>" is <state>; only an active, revising, or interrupted skipped stage can be routed as skipped.` /
  `Stage "<slug>" is an ungated initialization stage; it cannot report <result>.` /
  `Stage "<slug>" is <state>; only an in-progress stage can open a gate.` / `Stage "<slug>" is <state>; only an active or awaiting-approval stage can be rejected.` /
  `report --result rejected for "<slug>" requires nonblank --user-input or --reason feedback.` / `Stage "<slug>" is <state>; only a revising stage can re-enter its gate.` /
  `report --result <result> for "<slug>" requires --user-input with the human's exact approval choice.` /
  `Stage "<slug>" is <state>; report commits forward completions only.` / `Stage "<slug>" is still pending. Run the stage before reporting it complete.` /
  `Stage "<slug>" is still in-progress. To approve a gated stage that has not entered awaiting-approval, report the acted directive explicitly with --stage "<slug>" so the engine cannot mistake a freshly advanced Current Stage for the completed one.`
  `<state>` の綴りは upstream の checkbox 名（`pending` / `in-progress` / `awaiting-approval` / `revising` / `completed` / `skipped`）— `CheckboxState` に綴りのアクセサが無ければ domain に足す（RMU の `spelling` と重複させず、domain の 1 箇所を正本にして RMU もそれを使う）。
- 集約コマンド・ポートの失敗（ガード通過後）: `Transition rejected by aidlc-state.ts <sub> for "<slug>": <detail>`（`<sub>` は
  失敗した step の `subcommand()`、detail は `chained(error)`）。現行 `transition_rejected` を置換。

## 6. RMU — 完了の投影（b45 の所見）

イベント適用後の集約が `Status::Completed` になった投影（`GateApproved` / `StageSkipped` が最後の in-scope ステージを畳んだとき）で、
ピン `aidlc-state.ts complete-workflow`（`:2415-2560`）と同じに描く:
- 状態ファイル: `Completed`（数え直し）、`Status: Completed`、`Last Updated`、`Last Completed Stage: <slug>`、`In Progress: none`、
  `Next Stage: none`、`Next Action: Workflow complete`、`Current Stage` は最終 slug のまま、`## Phase Progress` の当該フェーズを `Verified`。
- 監査: `STAGE_COMPLETED`（`Details: Final stage <Name> completed`）→ `PHASE_COMPLETED`（`From phase: <phase>` / `To phase: (end)` /
  `Stages completed: N`）→ `PHASE_VERIFIED`（`Phase boundary: <phase> → end`）→ `WORKFLOW_COMPLETED`（`Scope: <scope>` /
  `Details: Scope: <scope>, N stages completed`、`workflowRollupFields` は cost ledger 由来で我々には無いので鍵ごと省く）。
  現行 `leave_for(None)` の素の `WORKFLOW_COMPLETED` 1 行を置換。既存の投影テスト（`projection.rs:1772` / `:1901`）を改訂。
- `read_execution.status` は既に `completed` を綴る — 変更なし。

## 7. テスト

- domain: `report_dispatch` の表テスト（(verdict, checkbox, gated, final, moved-on, explicit) の全組合せで決定/拒否/no-op を固定。
  到達不能な `Advance` / `CompleteWorkflow` は「誕生後に到達する状態が作れない」ことを固定）。
- use-case: `CommitOutcome` の 3 形、`Refused` の伝播、`Conflict` 再試行の維持。
- app 結合（`intent_lifecycle.rs` ほか）: 段 1〜13 の各逐語（state-version の 3 形は状態ファイルを書き換えて固定）、resume 4 経路 +
  2 拒否、skipped 5 条件、gate 3 系 + 前提違反、human-presence（env 有無）、forward の no-op 2 形と拒否 3 形、成功の `Committed ... + ...`。
- CLI ゴールデン: `report/approved` / `awaiting-approval` / `awaiting-approval-repeat` / `rejected` / `revised` を `cli_golden_test.rs` に
  追加 — `kind` はバイト一致、`reason` / `message` は slug（`practices-discovery` ↔ 合成グラフの `domain-design`）を置換して一致。
  `completed-ungated`（初期化ステージの advance）は b34 以降到達不能（#85 = A）— 駆動不能として理由を記載。
- RMU: 完了投影の状態差分と監査 4 行。

## 8. 仕様・記録

- `docs/specs/10-orchestration.md` §2.3 の `report_dispatch` 行を「集約のクエリ」に訂正、§10 に「report の実装ノート（b46）」を追加
  （分割・繰延・到達不能の根拠）。`deviations.md` は追記しない（逸脱台帳 #1 の写像を resume の redo 命令に適用するだけ）。
- 裁定待ち（§0）はハンドオフと #7 本文に載せ、Issue は起こさない。

## 9. 検証記録（2026-09-04 実測、実装は Opus サブエージェント、統合レビューは Fable 5）

- **テスト**: 新規 77 本（`#[test]` / `#[tokio::test]` の増分）。domain の表テスト `the_dispatch_table_pins_every_verdict_against_every_checkbox`
  （6 checkbox × 4 verdict = 24 組合せ）+ 軸別（gated=false / 計画外の名指し / 明示 `--stage` / final ∧ Completed / moved-on の有無 /
  段 13 と抜け道 2 つ / skipped 5 条件 / Resume / 非ゲート 3 形 / 構成不能の記録）、use-case の `CommitOutcome` 3 形と `Refused` /
  `Transition` / `UnwiredTransition`、クエリ側 DAO 4 本、RMU の完了投影 2 経路、app 結合の段 1〜13 逐語（state-version 3 形 × 3 経路、
  `--single` 4 形、`--skeleton-stance`、resume 4 経路 + 拒否、skipped 5 条件、gate 3 系、human-presence、forward の no-op と拒否、
  `Committed ... + ...`）。各レイヤーで赤を先に確認（domain: `ReportRequest` 未存在でコンパイルエラー / use-case: 新 API で先に
  書き替え / RMU: `Field not found: "Status"` で赤 / app: 旧逐語の 8 本が赤）。
- `cargo fmt --all --check` / `cargo clippy --workspace --all-targets -- -D warnings` / `cargo lint` / `cargo test --workspace` — **全緑（49 スイート、1,805 本）**。`tools/lint` 自己テスト 69 本も緑
- `scripts/quint-gate.sh` — **全 PASS（35 ステップ、モデル無変更の回帰確認）**
- `scripts/coverage.sh --base origin/main` — 絶対 **99.12% ≥ 90.0%**、相対 **99.12% ≥ 99.11%**（base を上回る）**PASS**
- ゴールデン `cli/report/{approved,awaiting-approval,awaiting-approval-repeat,rejected,revised}` — `kind` バイト一致、`reason` / `message` は slug（`practices-discovery` ↔ 合成グラフの `domain-design`）を置換して一致。`completed-ungated`（初期化ステージの `advance`）は b34 以降到達不能（#85 = A）
- **到達不能の 2 段**: `advance` / `complete-workflow` に対応する集約コマンドは無い（b42、#85 = A）。初期化ステージだけが in-scope の
  縮退計画でのみ `report_dispatch` がこれらを名指しし、ユースケースは `CommitError::UnwiredTransition` で断る（逐語は
  `Transition rejected by aidlc-state.ts <sub> for "<slug>": ... not wired in this build`）。
- **`STAGE_COMPLETED` の `Final stage <Name> completed` 行**は report からは到達しない（承認経路は `approve` が既に書き、読み飛ばし
  経路は `STAGE_SKIPPED` のみ）。`complete-workflow` を直に叩く経路がこの build に無いため。
- **`.coderabbit.yaml` のパースエラー**（PR #101 で CodeRabbit が警告。b46 対象外、設定の整備は別途）。
