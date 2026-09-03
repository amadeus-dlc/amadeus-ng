# b42 設計 — #85 = A: 非ゲート完了パイプラインの撤去（2026-09-03）

**裁定**: オーナー 2026-09-02（#85 = A、記録: decisions.md ADR-002 の 2026-09-02 改訂）。**調査**: 2026-09-03 の全数洗い出し（下記）。

## 原則
b34（誕生 = 初期化完了済み）以降、カーソルは常にゲート付きステージに立つ。非ゲート完了の経路は実行時到達不能。
no-backward-compatibility の精神で消す。観測互換への影響は無い（実行時に出ない文言）。Quint モデルは変更不要
（`actReportForward` の非 gated 腕はモデル自身が到達不能と宣言済み、フィクスチャに `prevCursor == 0` の report_forward は無い）。

## 削除
- domain: `IntentExecution::complete_stage`（intent_execution.rs:625-645）、`StageCompleted` ペイロード（intent_execution_event/stage_completed.rs）、
  enum 変種 + `id()`/`aggregate_id()` の腕、`apply_event` の腕（:975-979）、mod.rs の再輸出。`require_gated(stage, gated: bool)` は `false` の呼出が消えるので
  `require_gated(stage)` に単純化可（任意）。`EventType::StageCompleted`（監査行の型）は**残す**（genesis / GateApproved が書く）。
- domain テスト: `Run::complete_stage` ラッパ、`at_initialization_cursor` ヘルパ、`complete_stage_is_refused_on_a_gated_stage`、
  `a_command_equals_the_old_state_plus_its_event` の b34 存置分岐、`every_initialization_stage_is_non_gated_and_the_rest_are_gated` の踏破部分、
  proptest の `Cmd::Complete`。他の 8 テストは別変種 / 別コマンドへ差し替え。`intent_execution_event.rs` の「12 変種」テスト群 → 11。
- ITF `tests/engine_loop_conformance.rs` の `report_forward` 腕 → `approve_gate` 単独に縮退（ヘッダ説明も更新）。
- use-case: `commit_verdict_use_case.rs:336-345` の非ゲート腕 → `approve_gate` 単独。テスト `a_forward_report_on_an_ungated_stage_completes_the_stage` 削除。
  `ReportedTransition` は無変更。これが `IntentExecution::gated` の唯一のプロダクション呼出 → `gated(&intent, ..)` は RMU の行組立でだけ使われる。
- adapter: `dto/stage_completed_dto.rs` 削除、`IntentExecutionEventDto` の変種と `of`/`to_domain` の腕、ゴールデン
  `{"StageCompleted":{"id":..,"aggregate_id":..,"stage":"state-init"}}` の組、`intent_execution_repository_impl.rs:504,641` と
  `tests/intent_execution_repository_impl_test.rs:19,325,370` は他変種へ。
- RMU: `dto/stage_completed_dto.rs` 削除、DTO enum の腕、`workspace/projection.rs` の `project_one` の腕（:325-327）と `fn stage_completed`（:697-731、
  文言 `format!("Stage {title} completed")`）削除。**残す**: ローカルヘルパ `complete_stage(read_model, stage)`（:679）、`initialization_completion_details`（genesis）、
  ゲート経由の `"Stage {title} approved by gate"`。単体テスト `completing_a_non_gated_stage_uses_the_completed_wording` 削除、
  `an_unknown_plan_suffix_token_falls_back_to_the_static_plan` は他変種へ。ゴールデンテスト `completing_an_ungated_stage_writes_its_own_details_wording`
  （fixture `report/completed-ungated`）削除 → `tests/golden/upstream-3c3146cf/cli/report/completed-ungated/` は upstream 実バイトの証拠なので**残す**（孤児化の注記）。
- app: `tests/journal_protocol_conformance.rs` の `next_command()` 非ゲート腕（:296-317）、「12 変種」表記、`every_execution_variant`。`crash_reconstruction_test.rs` はコメントのみ。
- docs: 仕様 10 :50-51（コマンド 12 → 11、イベント 12 → 11）、domain `orchestration/mod.rs` :15,36,132。「12 変種」表記 13 件。

## 網羅 match（腕の削除が要る 9 箇所）
domain `intent_execution_event.rs` の `id()`/`aggregate_id()`/テスト `name()`、`intent_execution.rs` の `apply_event`、両側 DTO の `of`/`to_domain`、RMU `projection.rs` の `project_one`。

## 残り 11 変種
Started / GateOpened / GateApproved / GateRejected / StageRevised / StageSkipped / Jumped / Parked / Unparked / Recomposed / AutonomyModeSet

## 進め方
1. 削除は一斉に（互換口・別名・`#[deprecated]` を残さない — no-backward-compatibility）。網羅 match 9 箇所はビルドが教える。
2. テストは「削除」と「別変種 / 別コマンドへの差し替え」を区別し、検出力を落とさない（差し替え先の変種で同じ性質を固定する）。
3. golden フィクスチャ `tests/golden/upstream-3c3146cf/cli/report/completed-ungated/` は upstream 実バイトの証拠として残し、参照テストが無い旨を
   フィクスチャ隣の README か `projection_golden_test.rs` のコメントに 1 行書く。
4. Quint モデルは無変更（注記 :41-44 を「撤去済み」に更新するのは任意 — 本 Bolt では行わない）。
5. 正本: 仕様 10 :50-51（コマンド 11 / イベント 11）、domain `orchestration/mod.rs` の表と doc、「12 変種」表記 13 件。
