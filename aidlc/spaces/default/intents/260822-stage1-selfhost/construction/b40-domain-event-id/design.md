# b40 設計 — ドメインイベントの ID（`XxxEventId`）と `aggregate_id`（#7 キュー 2c、オーナー裁定 2026-09-02）

**裁定**: 「ドメインイベントはエンティティの一種。イベントごとに id を持たせる。集約の ID をイベントの
ID にしているのはまずい。`XxxEvent { id: XxxEventId, aggregate_id: XxxId, .. }`」/ Q1 = A（採番は
集約のコマンド内）/ Q2 = A（`seq_nr` / `occurred_at` は封筒のまま）。正本: `aggregate-commands.md` /
`domain-object-kinds.md`（記録済み）。

## 1. 原則からの導出

- ドメインイベントは**エンティティ**なので自前の識別子を持つ。集約 ID は「どの集約の事実か」を示す
  別フィールド `aggregate_id`。b39 の `Started { id: IntentExecutionId }`（→ `aggregate_id` に改名済み）や
  `Created { id: IntentId }` / `Defined { id: WorkflowDefinitionId }` / `Compiled { id: CompiledDefinitionId }`
  は集約 ID の流用で誤り。
- 採番は集約のコマンド内 `XxxEventId::generate()`（UUIDv7）。本家サンプル `UserAccountEventId::new()` と
  同型。ID は識別だけで投影・ITF の答えに影響しないので、ドメインの純粋性の例外として認める
  （`occurred_at` は従来どおり呼出側が渡す）。
- 本家 v3 の `EventEnvelope` はイベント ID を持たない（`aggregate_id` / `seq_nr` / `occurred_at` /
  `manifest` / payload）。したがって ID は **payload（ドメインイベント）に閉じる**。`seq_nr` /
  `occurred_at` は封筒のまま（ADR-010 / B7 維持）。

## 2. ドメイン（`modules/core/command/domain`）

- 新設の値オブジェクト（1 ファイル 1 型、`parse` + `generate` + `as_str` + `Display` + `Eq/Hash`）:
  `IntentExecutionEventId`（orchestration）/ `IntentEventId`（orchestration）/
  `WorkflowDefinitionEventId`（workflow_definition）/ `CompiledDefinitionEventId`（workflow_definition）。
  `generate()` は `uuid::Uuid::now_v7()`（domain の `Cargo.toml` の `uuid` に `features = ["v7"]` を明示 —
  workspace 既定は `std` のみ）。`parse` は `IntentExecutionId::parse` と同じ厳格さ（UUIDv7・小文字正準形）。
- **全 19 変種**が `id: XxxEventId` と `aggregate_id: XxxId` を持つ:
  - `IntentExecutionEvent`（12）: `Started` / `StageCompleted` / `GateOpened` / `GateApproved` / `GateRejected` /
    `StageRevised` / `StageSkipped` / `Jumped` / `Parked` / **`Unparked`（unit 変種 → struct `Unparked { id, aggregate_id }`
    に昇格、ファイル `intent_execution_event/unparked.rs`）** / `Recomposed` / `AutonomyModeSet`
  - `IntentEvent`（1）: `Created`（`id: IntentId` → `aggregate_id: IntentId`、`id: IntentEventId` を追加）
  - `WorkflowDefinitionEvent`（2）: `Defined`（同様）/ `Redefined`（`aggregate_id` を**追加** — 現状は id を運ばない）
  - `CompiledDefinitionEvent`（4）: `Compiled`（同様）/ `Recompiled` / `ScopeRegistered` / `PluginSelectionApplied`
    （`aggregate_id` を追加）
- 各ペイロードの基本コンストラクタは `new(id, aggregate_id, ..材料)`。イベント enum に `id()` /
  `aggregate_id()` の match アクセサ。
- decide（コマンド）は `XxxEventId::generate()` で採番し `aggregate_id: self.id.clone()`。genesis
  （`start` / `create` / `define` / `compile`）も同じ。`From<(Genesis, at)>` は `aggregate_id()` から集約 id を取る。
- `apply_event` は `aggregate_id` を見ない（封筒の aid 照合はアダプタ / RMU の復号境界の仕事。集約は自分の
  ストリームだけを受け取る前提 — 既存どおり）。

## 3. 両側 DTO・ワイヤ・復号境界

- 各変種 DTO に `id` / `aggregate_id` キー（順序: `id` → `aggregate_id` → 内容）。`Unparked` は unit から
  struct DTO へ。両側（コマンド側 `dto/` と RMU `dto/`）のゴールデンを更新し、app の横断適合テストで
  バイト一致を固定。
- 復号境界の照合: コマンド側 Repository の `find_by_id` 再生と RMU の `decode_*` は、行の `aid` と payload の
  `aggregate_id` を**全変種**で照合（不一致は `Corrupt(InvariantViolation)`）。b39 で genesis だけに入れた
  照合を全変種へ広げ、`Redefined` の「行の aid が id」フォールバックは照合に変わる。
- `JournalEntry` / `DefinitionEntry`（RMU）はイベント id を**持たない**（`event().id()` で足りる。
  `read_*` にイベント id の列は不要 — 現時点で読取コマンドが要らない）。
- ローカルの `.aidlc-store.sqlite` は旧形を復号できなくなる（未配布・再鋳造。b39 と同じ扱い）。

## 4. b39 からの申し送りも同時に揃える

- intent 行（`Created`）の復号で計画不変条件（`StageEntry::check_plan`）を復号境界で検査し `Corrupt` に写す
  （`Started` と同じ）。既存の should_panic テスト `a_row_that_breaks_an_aggregate_invariant_crashes_reconstruction`
  は「復号境界で拒める破損は `Corrupt`、それを通り抜けた不変条件違反はクラッシュ」の 2 段に書き分ける
  （`replay` / `apply_event` のクラッシュ規律 2026-08-30 は変えない）。

## 5. テスト

- 各 `XxxEventId`: `parse` の厳格さ、`generate` が UUIDv7 形式で毎回異なる、`Display` / `as_str`。
- 各集約: コマンドが返すイベントの `aggregate_id == self.id()`、連続 2 コマンドの `id` が異なる。
- DTO 往復・ゴールデン（両側）、横断適合（app）。RMU / Repository の `aggregate_id` 不一致 → `Corrupt`。
- 既存 ITF（`engine_loop_conformance` 等）は不変（id は答えに影響しない）。golden パリティ（配布束）不変。

## 6. 正本の更新

- ADR-002 に「イベントはエンティティ — `id: XxxEventId` / `aggregate_id`、採番は集約内」を追記。ADR-010 に
  「イベント ID は payload、封筒は `seq_nr` / `occurred_at`」を追記。仕様 10 §2.1 のイベント表と仕様 12 の
  イベント記述に `id` / `aggregate_id` を明記。`aggregate-commands.md` の「genesis イベントは集約 id を運ぶ」
  を `aggregate_id` の語で読み替え（記録済みの追記を整える）。
