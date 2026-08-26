# developer-brief-2 — 委任 2: ポート / 値 / エラー（use-case）、InMemory、ワイヤ、契約テスト（U3 / Bolt B5）

Conversation language: 日本語（コメント・rustdoc・報告はすべて日本語。識別子・固定トークンは英語）。

## 役割と範囲

あなたは aidlc-developer-agent。Unit **u3-event-store-repository**（Bolt B5）の委任 2。リポジトリルート `/Users/j5ik2o/orca/workspaces/amadeus-ng/docs`、ブランチ
`bolt/b5-u3-event-store-repository`（委任 1 の退役 + U2 是正はコミット済み: `IntentId` = UUIDv7、`WorkflowExecutionState` / `state()` / `from_state()`）。
**コーディング規則の正本 `aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/`（README + 7 ルール）を最初に読む。**

所有ファイル: `modules/core/use-case/src/orchestration/**`（既存 `workflow_definition_repository.rs` は読取のみ）、`modules/core/use-case/src/lib.rs`、
`modules/core/use-case/Cargo.toml`（dev-dependency `tokio` のみ）、`modules/core/interface-adapter/src/orchestration/{memory/in_memory_event_store.rs,memory/workflow_execution_repository.rs,memory/mod.rs,wire/mod.rs,wire/event_wire.rs,wire/state_wire.rs}`、
`modules/core/interface-adapter/src/orchestration/mod.rs`（`mod wire; mod memory…` と `pub use memory::{InMemoryEventStore, InMemoryWorkflowExecutionRepository}` の追記のみ）、
`modules/core/interface-adapter/Cargo.toml`（dev-dependency `tokio` のみ — `rusqlite` は委任 3）、`modules/core/interface-adapter/tests/support/**`、
`modules/core/interface-adapter/tests/workflow_execution_repository_contract.rs`、`Cargo.toml`（workspace root: `[workspace.dependencies]` に `tokio = { version = "1", features = ["rt", "macros"] }` を追加するだけ）、
報告 `.../u3-event-store-repository/code-generation/developer-report-2.md`（新規）。

触らないもの: 計画・検査手順・質問票、`modules/core/domain/**`（読取のみ — U2 の公開 API を使う）、`sqlite_event_store.rs` 等（委任 3）、`docs/specs/**`、`formal/**`。
`git add` / `git commit` はしない。`.claude/` のツールは実行しない。

## 先に読むもの（順に）

1. `.../u3-event-store-repository/code-generation/code-generation-plan.md`（§1、§2 公開 API、§5.2 Step 3〜5、§7）
2. `.../u3-event-store-repository/functional-design/rules.md`（BR1.1〜BR1.5、BR2.5、BR2.7、BR2.8）、`entities.md`（use-case 層の型、EventPayloadWire / StateWire、InMemory）、
   `functional-spec.md`（§2 ポート、§3.1 / §3.2 の手順 — InMemory も同じ規則、§4 ワイヤ形式、§7 テスト）
3. `.../u3-event-store-repository/nfr-design/security-design.md`（§2 検査点 1・2、§3）、`.../nfr-requirements/security-requirements.md`（NFR2.2 / NFR3.x / NFR4.3 / NFR4.5）
4. `.../inception/contract-design/contract-summary.md` C3（trait の形 — 数値型は u64 に具体化）
5. 既存コード: `modules/core/domain/src/orchestration/{mod.rs,workflow_execution.rs,workflow_execution_event.rs,workflow_execution_state.rs,intent_id.rs,stage_entry.rs}`、
   `modules/core/domain/src/workflow_definition/{plan_action.rs,phase.rs,workflow_definition_id.rs,definition_revision.rs}`、`modules/core/domain/src/workspace/checkbox.rs`
   （`CheckboxState` の文字列形）、`modules/shared/canon-json/src/{lib.rs,value.rs,writer.rs}`（`to_value` / `serialize`）、`modules/core/use-case/src/orchestration/workflow_definition_repository.rs`
   （既存ポートの書き方・エラー様式）、`modules/core/interface-adapter/src/orchestration/memory/workflow_definition_repository.rs`（InMemory の書き方）、`modules/core/domain/tests/engine_loop_conformance.rs`
   （`start_with_entries` で集約を組む方法 — テスト用の集約生成に流用）。

## 作業（計画 Step 3〜5、TDD）

### Step 3 — Data model（use-case）
- Red: `GlobalSeqNr`（`ZERO`、`u64`、`From`/`value()`、順序）、`ProjectionName::parse`（kebab `^[a-z][a-z0-9-]*$`、1〜64 字 — 受理 / 拒否）、
  `RepositoryError` / `EventStoreError` / `CorruptCause` の `Display`（材料のみ）と `std::error::Error`、`From<EventStoreError> for RepositoryError`（`Conflict` / `Io` /
  `Corrupt` は同名、`Schema` → `Corrupt(SchemaVersion)`、`CheckpointRegression` は Repository 面に出ない — 変換は `Corrupt(InvariantViolation)` で材料に projection 名を
  残すか、`From` を使わず Repository 実装側で明示変換するかは実装判断で可。決めたら報告）。各 5〜8 本。
- Green / Refactor: `modules/core/use-case/src/orchestration/{event_store.rs,journal_reader.rs,workflow_execution_repository.rs,repository_error.rs,event_store_error.rs,global_seq_nr.rs,projection_name.rs}`、
  `mod.rs` の `pub use`（旧名なし）。trait は `async fn`（AFIT）、`dyn` なし、`Send`/`Sync` 境界なし。`EventStore<AID, A, E>` の 4 メソッド（`persist_event(&mut self, &E, version: u64)`、
  `persist_event_and_snapshot(&mut self, &E, &A)`、`get_latest_snapshot_by_id(&self, &AID) -> Option<A>`、`get_events_by_id_since_seq_nr(&self, &AID, u64) -> Vec<E>`）、
  `JournalReader`（`events_after(GlobalSeqNr)` / `checkpoint(&ProjectionName)` / `advance_checkpoint(&mut self, &ProjectionName, GlobalSeqNr)`）、
  `WorkflowExecutionRepository`（`find_by_id(&self, &IntentId)` / `store(&self, &WorkflowExecutionEvent, &WorkflowExecution)`）。`# Errors` 必須。

### Step 4 — Business logic（ワイヤ、adapter `wire/`）
- Red: PBT（`proptest`、`PROPTEST_RNG_SEED` 固定で決定的）— 任意の `WorkflowExecutionEventPayload`（12 変種）と `WorkflowExecutionState`（16 属性、`start_with_entries` +
  コマンド列で生成するか Builder で直接）について encode → decode が恒等、同一入力の encode がバイト同一（正準 JSON）。拒否テスト: 未知 `type` → `Corrupt(UnknownEventType)`、
  未知フィールド → `Corrupt(UndecodablePayload)`、`schema_version ≠ 1` → `Corrupt(SchemaVersion)`、型不一致 / 不正な IntentId / PlanAction 文字列 → `Corrupt(UndecodablePayload)`。
- Green: `wire/event_wire.rs`（`{"type": "<変種名>", …材料}` — functional-spec §4.1 の表）、`wire/state_wire.rs`（§4.2 の 16 属性）。serde 構造体は `pub(crate)`、
  固定トークンは upstream 綴り（CheckboxState のマーク、EXECUTE / SKIP、PhaseId の 5 語、autonomous / gated）、他は snake_case。正準 JSON は `canon_json::to_value` →
  `serialize(..., <既定プロファイル>)`（canon-json の公開 API に従う — 不明点は報告）。復号は parse-don't-validate（Domain Primitive の parse を通す）。
  公開面: `wire` は `pub(crate)`（`EventPayloadWire::encode(&payload) -> String` / `decode(type, json) -> Result<Payload, EventStoreError>`、`StateWire::encode` / `decode` 程度）。
- Refactor: 添字アクセスなし（`get()`）、`unwrap` なし、エラーは材料のみ。

### Step 5 — API（契約テスト + InMemory）
- Red: `tests/support/mod.rs` + `tests/support/contract.rs` にジェネリック契約テスト関数群 `async fn contract_<case><R: WorkflowExecutionRepository>(repo: &R, fixture: &TestFixture)`:
  ラウンドトリップ（`start` → `complete_stage` ×2 → `open_gate` … で 5 イベント以上 → 各イベントを `store` → 新しい Repository インスタンス（同じストア）で `find_by_id` →
  `state()` が `PartialEq` で一致、`version()` = 最後の seq_nr）、NotFound、Conflict（同じ集約を 2 回 `find_by_id` → 片方でコマンド + store 成功 → もう片方で
  コマンド + store → `Conflict { expected, actual }`）、genesis の store（expected 0）、`store` の前提検査（seq_nr 不一致 → `Corrupt(SequenceGap)`）、
  `JournalReader`: `events_after(ZERO)` が全イベントを global 昇順で返す / `events_after(n)` が差分のみ / `checkpoint` 未登録 = ZERO / `advance_checkpoint` 増加 OK・
  同値 no-op・後退 → `CheckpointRegression`。Corrupt（MissingSnapshot / UndecodablePayload / SchemaVersion）は、ストア実装がテスト支援として行を直接いじれるフック
  （InMemory: `pub fn corrupt_for_test(...)` 等は**作らない** — 代わりにテスト側で InMemory の内部 map を触れる `#[cfg(test)]` 専用 API か、契約テストからは除外して
  実装固有テストに置く。判断して報告）。
- Green: `memory/in_memory_event_store.rs`（`BTreeMap<(IntentId, u64), (GlobalSeqNr, WorkflowExecutionEvent)>` 相当の journal、snapshot map、checkpoint map、
  global counter。Conflict 規則は SQLite と同一: expected = aggregate.version() = event.seq_nr − 1、UNIQUE 違反 / version 不一致 → Conflict、genesis は INSERT）、
  `memory/workflow_execution_repository.rs`（`InMemoryWorkflowExecutionRepository { store: RefCell<InMemoryEventStore> }`、`find_by_id` = snapshot → `from_state` →
  `with_version(snapshot.version)` → replay（`apply_event`）→ `with_version(last seq_nr)`；`store` = 前提検査 → `persist_event_and_snapshot`）。
  テスト用に InMemory は `Clone` 可でもよい（同一ストアを 2 つの Repository で共有するには `Rc<RefCell<…>>` を Repository が持つ形にする — 設計は RefCell 内包
  なので、契約テストの「新インスタンスで find_by_id」は `InMemoryEventStore` を `Rc` 共有して組む。決めたら報告）。
- 契約テストを InMemory で全緑に。`cargo test -p core-interface-adapter --test workflow_execution_repository_contract`。

## 作法（厳守）

- TDD、`unwrap` / `expect` / `panic!` / 添字アクセス禁止（プロダクトコード）。テストは `clippy.toml` で unwrap 許容。フィールド private、mod private + `pub use`。
- `unused_async` が trait 実装で発火したら `#[allow]` せず報告の「設計質問」へ。
- 設計に無い判断は報告に書いて進める（保留は最小限）。

## 報告（`developer-report-2.md`）

「Red の失敗出力」「実装概要（ファイル・公開面）」「判断（From 変換 / 契約テストのフィクスチャ / Rc 共有 / Corrupt テストの置き場）」「検査結果（cargo test -p core-use-case、
cargo test -p core-interface-adapter、clippy、fmt）」「設計質問」「未了」。最終応答は要約（日本語、10 行以内）。
