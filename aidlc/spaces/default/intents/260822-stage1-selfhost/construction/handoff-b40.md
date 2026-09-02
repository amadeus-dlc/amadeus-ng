# ハンドオフ — b40（#7 キュー 2c）: ドメインイベントの ID `XxxEventId` と `aggregate_id`（2026-09-03）

設計書: [`b40-domain-event-id/design.md`](b40-domain-event-id/design.md)。裁定: オーナー 2026-09-02（イベントはエンティティ / Q1 = A 採番は集約内 / Q2 = A 封筒は従来どおり）。

## やったこと
- 値オブジェクト 4 種（`IntentExecutionEventId` / `IntentEventId` / `WorkflowDefinitionEventId` / `CompiledDefinitionEventId`、`parse` + `generate`（UUIDv7）+ `as_str`）。domain の `uuid` に `v7` を明示。
- 全 19 変種が `{ id: XxxEventId, aggregate_id: XxxId, .. }`。`Unparked` は struct へ。genesis 変種の `id: XxxId` は `aggregate_id` に改名、`Redefined` 等に `aggregate_id` を追加。enum に `id()` / `aggregate_id()`。
- 採番は集約のコマンド内（`next_event_id()` ヘルパ）。`From<(Genesis, at)>` は `aggregate_id()` から集約 id。
- 両側 DTO・ゴールデン・app 横断適合を更新。書く側は `Created` のジャーナル面を `CreatedDto` として `IntentDto`（スナップショット面）から分離、`WorkflowDefinitionDto` は `flatten` をやめ平坦宣言（行のバイトは不変）。
- 復号境界（Repository 再生・RMU `decode_*`）で全変種の `aggregate_id` を行の `aid` と照合（不一致は `Corrupt`）。`Redefined` の「行の aid が id」フォールバックは照合に置換。intent 行も `StageEntry::check_plan` を復号境界で検査。

## 申し送り
- スナップショット復号は誕生記録（`From<(Created, at)>` 等）を種にするため、その場で `generate()` したイベント id を捨てる（集約は id を保持しないので観測不能。スナップショット面専用の完全コンストラクタを置くかは、必要が出た時点で判断）。
- ローカルの `.aidlc-store.sqlite` は旧形を復号できない（未配布・再鋳造）。

## 次
b41（Bolt 2 後半: `read_run_stage` / `read_scope_change` / `read_execution.scope` / `read_steering_plan` / `read_steering_part` と参照入力ダイジェスト。ドラフトは統合側の控え）→ 2b（#85 = A）→ Bolt 3。
