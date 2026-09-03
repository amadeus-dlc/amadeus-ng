# ハンドオフ — b42（#7 キュー 2b）: 非ゲート完了パイプラインの撤去（2026-09-03）

設計書: [`b42-remove-ungated-completion/design.md`](b42-remove-ungated-completion/design.md)。裁定: #85 = A（オーナー 2026-09-02、ADR-002 の改訂に記録）。

## やったこと
- 削除: `IntentExecution::complete_stage`、`StageCompleted`（ペイロード・両側 DTO・enum の腕・`apply_event` の腕）、RMU `projection.rs` の非ゲート完了ライタ（文言 `"Stage {title} completed"`）、commit_verdict の非ゲート腕、追随テスト（proptest の `Cmd::Complete`、ITF の `report_forward` 非ゲート腕 等）。イベントは 11 変種。
- `require_gated(stage, gated: bool)` → `require_gated(stage)`（`false` 呼出が消えたため）。
- 残置: `EventType::StageCompleted`（監査行の型 — genesis / GateApproved が書く）、RMU のローカルヘルパ `complete_stage(read_model, stage)` と `initialization_completion_details`、ゲート経由文言、golden フィクスチャ `report/completed-ungated`（upstream 実バイトの証拠。参照テストは無い旨をコメント）。
- Quint モデル・フィクスチャは無変更（非 gated 腕はモデル自身が到達不能と宣言済み。全フィクスチャで `report_forward` の直前カーソル 0 は無し）。監査文言・Markdown 面・配布束の golden はすべて不変。

## 次
Bolt 3（b43 → b44）。設計ドラフトは統合側の控え（`bolt3-design-draft.md`）— b43 の設計記録に移す。
