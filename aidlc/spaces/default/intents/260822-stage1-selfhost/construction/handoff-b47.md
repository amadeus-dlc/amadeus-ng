# ハンドオフ — b47: `--single`（synthetic-id pair）と `--skeleton-stance`（classify round-trip）を `IntentExecution` のイベントで（2026-09-04）

対象: GitHub #73 の残り（b46 が「not wired in this build」で止めていた 2 面）。設計書: [`b47-single-skeleton/design.md`](b47-single-skeleton/design.md)。
前段: [`handoff-b46.md`](handoff-b46.md)。オーナー裁定（着手前に質問）: 新集約は作らず `IntentExecution` のイベントにする（B）、
skeleton stance も `IntentExecution` のコマンドとイベント（A）。I10 の強制手段は E1 → E4 + 単体（フレーム条件）へ改訂。

## やったこと
- **ドメイン**: `IntentExecutionEvent` を 13 変種へ（`SingleStageRunCommitted { stage }` / `SkeletonStanceRecorded { stance }`）。
  `IntentExecution` に `skeleton_stance: Option<SkeletonStance>` を追加。コマンド `record_single_stage_run`（取り違え → initialization /
  範囲外 `InvalidTarget` だけ。**本流の状態に依らず受理** — Completed / park 中 / autonomous でも通る。**適用はフレーム空**で、
  通番以外が `==` のまま — I10 の実体）と `record_skeleton_stance`（現在地が**静的計画**の Construction 最初の EXECUTE ステージでなければ
  `InvalidTarget`。recompose overlay は見ない。再記録は上書き）。`next_decision(&self, intent, request)` は `RunStage.gate` を 3 値
  `GateDecision`（`Gated` / `Ungated` / `Unresolved`）で返す — `Unresolved` = ゲート付き ∧ skeleton-gate stage ∧ stance 未記録。
- **DTO**: 2 変種の永続化 DTO、`dto_vocabulary` に stance の綴り、スナップショット行に `skeleton_stance`（欄不在は `None`）。ワイヤ形式
  13 変種を両側のゴールデンコーパスで固定。
- **ユースケース**: `RecordSingleStageRunUseCase` / `RecordSkeletonStanceUseCase`（find → コマンド → store、`Conflict` 1 回再試行）と封筒 2 本。
- **RMU**: `SingleStageRunCommitted` → 監査 2 行だけ（`STAGE_STARTED` {Stage, Agent, Workflow: single-stage:<slug>} → `STAGE_COMPLETED`
  {Stage, Details: Single-stage run of <slug> completed, Workflow}。状態ファイルと `read_*` は不変）。`SkeletonStanceRecorded` →
  `## Runtime State` の `Skeleton Stance` 欄を setOrInsert（監査行なし、`Last Updated` 不変）と `read_execution.skeleton_stance`。
  `read_next_answer.gated INTEGER` → `gate TEXT`（3 綴り、正本は `GateDecision::spelling`）、`read_run_stage.in_scope`（`--single` の scope 外ガードの材料）。
- **クエリ側**: `GateField::parse`、`NextAnswerView::gate() -> Option<GateField>`、`RunStageView::in_scope()`。
- **app**: `next --single` を pinned `emitSingleRunStage` に揃えた（`--phase` 併用 / `--stage` 必須（逐語訂正）/ 未知 / initialization / scope 外の
  5 拒否、`single: true` / `gate: false` / `next_stage` 不在を `directive_drawing` の single 経路で強制、state は読まない）。
  `report --single` は result → FORWARD → `--stage`（trim なし）→ 未知 → initialization → 記録 → `catch_up` → `done`
  `Single-stage run of "<slug>" committed under synthetic workflow "single-stage:<slug>". The main workflow's Current Stage is untouched.`。
  `report --skeleton-stance` は値検証 → state 必須 → 記録 → `catch_up` → `print`
  `Recorded walking-skeleton stance "<v>" for "<slug>". Re-run \`next\` to continue — the gate is now determined.`、拒否
  `Current stage "<slug>" is not the skeleton-gate stage for scope "<scope>" — ...`、失敗 `Failed to record ... for "<slug>": <detail>` 2 形。
  b46 の「not wired」2 本と短い旧 `SINGLE_REQUIRES_STAGE` を撤去。
- **Quint v2.4**: `stanceRecorded` 変数、`actRecordSkeletonStance`（guard `cursor == skeletonGateStage` — 静的計画由来）と `actSingleRun`
  （全変数不変）、不変条件 `single_run_frame` / `stance_frame`（12 本へ）、witness `w_single_run` / `w_stance_recorded`。
  mutation 検査: `actSingleRun` に `cursor' = cursor + 1` → `single_run_frame` が検出、`actRecordSkeletonStance` に
  `checkbox' = checkbox.set(cursor, CompletedBox)` → `stance_frame`（と `no_gate_bypass`）が検出。ITF フィクスチャは状態変数が
  増えたため既存 9 本を同じ seed で再採取（採取条件は v2.2 のモデルで既存ファイルをバイト再現できるものを総当たりで確定）+ 新規
  `trace-0x404`（`not(w_single_run)`）/ `trace-0x505`（`not(w_stance_recorded)`）。準拠テストは `stanceRecorded` を `skeleton_stance().is_some()` と
  突き合わせ、網羅リストに `single_run` / `record_skeleton_stance` を追加、合成計画のフェーズ割当を「索引 1 以降 = Construction」へ。
- **テスト**: 新規 54 本（`#[test]` / `#[tokio::test]` の増分）。フレーム空の固定 `an_isolated_run_records_the_stage_without_moving_the_workflow`、
  監査 2 行の逐語 `an_isolated_run_appends_the_two_audit_rows_verbatim_and_touches_nothing_else`、round-trip
  `the_skeleton_gate_round_trip_turns_unresolved_into_a_determined_gate`、`next --single` の 5 拒否と成功、`report --single` / `--skeleton-stance` の逐語。

- **設計との差分 8 点**（`design.md` §9「設計との差分」）: 実装先行の層は mutation で red を代替、`next_decision(&Intent)` の署名、名指しステージに対するゲート判定、`Option<StageSlug>`、`read_run_stage.in_scope`、`report --single` の拒否順、`ungated` の未到達、網羅コメントの実測合わせ。

## 積み残し（記録のみ、起票しない）
- **b48（裁定済み）**: B10 のレシート鮮度（#51 = A）と段 12 の practices-discovery 受領証 — (A) 受領証は `IntentExecution` のイベント
  `ReviewReceiptRecorded`、(i) 鮮度は順序だけ。受領証を書く動詞（`aidlc-audit append` 相当）の配線を含む。#7 キュー 5 の残り。
- 段 11（completion-evidence）は slice 2、turn-shape marker は CP5（変わらず）。
- **記録の訂正 — `trace-0x202`**: commit `08917406` のメッセージは「0x202 は `report_skipped` / `report_revised` を踏む負形式採取」と書いているが、
  v2.2 の時点で素の採取（`--max-samples 1`、status `ok`、縮退誕生）に置き換わっていた。b47 の再採取はその実態（素の採取条件）を踏襲した。
  既存フィクスチャ 9 本の採取条件は記録が部分的にしか無く、v2.2 のモデル（`bf494c5e`）で候補コマンドを総当たりしてバイト再現できた
  ものを確定した — 一覧は `design.md` §9。今後は再採取のコマンドを設計記録に必ず残す。
- **設計の訂正**: skeleton-gate stage の Quint 抽象は `cursor == 1` ではなく静的計画由来の `skeletonGateStage`（`design.md` §3）。
- **`engine_loop_conformance.rs` 冒頭 doc の dangling 参照**（b44 で削除した `engine_loop_ladder_conformance.rs`）を本 Bolt で直した。

## 次
b48（受領証）→ #7 キュー 6 以降（#72 set-autonomy → #71 WorkspaceScanner → …）。
