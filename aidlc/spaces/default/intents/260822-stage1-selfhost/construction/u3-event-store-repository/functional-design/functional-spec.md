# functional-spec — U3 SQLite EventStore と WorkflowExecutionRepository（`u3-event-store-repository`）

> Functional Design（Construction 3.1）成果物（Unit: U3、kind: library、Bolt: B5）。出典: `entities.md` / `rules.md`（同ディレクトリ）、C3 / C6、ADR-001 / 003 /
> 006 / 007、Bolt B3 実装（`modules/core/domain/src/orchestration/`）、`functional-design-questions.md`（Q1〜Q4 = A）。コードは署名の要点だけ（設計ステージ）。

## 1. 配置（クレート = 層）

| 層 / クレート | モジュール（private mod + ファサード `pub use`） | 内容 |
|---|---|---|
| `core-use-case::orchestration` | `workflow_execution_repository.rs` / `event_store.rs` / `journal_reader.rs` / `repository_error.rs` / `event_store_error.rs` / `global_seq_nr.rs` / `projection_name.rs` | ポート 3 本、エラー 2 型 + CorruptCause、値 2 型（BR1.x） |
| `core-use-case` | `workspace/`（mod ごと削除） | 退役（BR3.1） |
| `core-domain::orchestration` | `intent_id.rs`（UUIDv7）、`workflow_execution_state.rs`（旧 snapshot）、`state_error.rs`（旧 snapshot_error）、`workflow_execution.rs`（state / from_state） | BR4.1 / BR4.3 |
| `core-domain::workspace` | `intent_dir_name.rs`（新設）。`lock_protocol.rs` / `lock_identity.rs` 削除 | BR4.2 / BR3.1 |
| `core-interface-adapter::orchestration` | `event_store_impl.rs`（+ `schema.rs` DDL 定数、`wire/event_wire.rs`、`wire/state_wire.rs`）、`store_path.rs`、`workflow_execution_repository_impl.rs`、`memory/in_memory_event_store.rs`、`memory/workflow_execution_repository.rs` | BR2.x |
| `core-interface-adapter` | `clock.rs`（既存、Fake 付き）。`process_probe.rs` / `workspace/fs_workspace_lock.rs` 削除、`workspace/state_file_io.rs` は維持（U4） | BR2.6 / BR3.1 |
| `infra-io` | `process_probe.rs` 削除 | BR3.1 |
| `formal/orchestration` | `journal_protocol.qnt`（新）。`formal/workspace/audit_lock.qnt` 削除 | BR3.3 |
| `tests/conformance/fixtures` | `journal_protocol/*.itf.json`（新）。`audit_lock/` 削除 | BR3.5 |
| `tools/lint` | `reap-decision-locality` ルール削除（checkbox-vocabulary / no-public-fields は維持） | BR3.1 |
| 依存 | workspace: `rusqlite = { version = "0.3x", features = ["bundled"] }`、`tokio = { version = "1", features = ["rt", "macros"] }`。adapter から `md5` 除去 | P6 |

ファサード（`pub use`）に旧名は残さない（module-visibility.md）。

## 2. ポートの形（C3 の具体化 — 差分のみ）

- `WorkflowExecutionRepository::{find_by_id(&self, &IntentId), store(&mut self, &WorkflowExecutionEvent, &WorkflowExecution)}`。`store` は `&mut self`（C3 も
  2026-08-24 のオーナー裁定で `&self` → `&mut self` へ**改訂済み** — `contract-summary.md` §C3、`pending-revision.md` #9。
  内部可変性の禁止に伴う変更で、正本は `coding-rules/interior-mutability.md` / `command-query-separation.md`）。
- `EventStore<AID, A, E>` の 4 メソッド — C3 どおり（`version: u64`、`seq_nr: u64`）。Repository 実装は `persist_event_and_snapshot` / `get_latest_snapshot_by_id` /
  `get_events_by_id_since_seq_nr` を使う。
- `JournalReader::{events_after(GlobalSeqNr), checkpoint(&ProjectionName), advance_checkpoint(&ProjectionName, GlobalSeqNr)}`。
- `EventStoreImpl::open(path: StorePath, clock: C) -> Result<Self, EventStoreError>` / `within_write_transaction<T>(&mut self, f) -> Result<T, EventStoreError>`。
- `WorkflowExecutionRepositoryImpl { store: EventStoreImpl<C> }` — `EventStoreImpl<C>` を直接所有する（内部可変性は使わない、coding-rules/interior-mutability.md）。
  可変操作は `&mut self`。`event_store(&self) -> &EventStoreImpl<C>` / `event_store_mut(&mut self) -> &mut EventStoreImpl<C>` に分けて公開する。
  `InMemoryWorkflowExecutionRepository { store: InMemoryEventStore }` も同形。
- 数値パラメータは u64（C3 の usize を実ドメイン型に合わせて具体化 — C3 の改訂提案を所有者 U5 / U6 へ申し送り）。
- `StorePath::of(aidlc_root: &Path, space: &SpaceName) -> StorePath` / `as_path()`。

## 3. フロー

### 3.1 store（BR1.3 / BR2.3）

1. 前提検査: `event.intent_id() == aggregate.intent_id()`、`event.seq_nr() == aggregate.seq_nr()`、`event.seq_nr() >= 1`、`aggregate.version() == event.seq_nr() - 1`
   （違えば `Corrupt(SequenceGap)` — 呼出側のバグ）。`expected = aggregate.version()`（find_by_id が `with_version` で載せた「永続化済みの最後の seq_nr」。
   `apply_event` は version を変えない — B3 実装契約）、`new_version = event.seq_nr()`。genesis は expected 0 / new_version 1。
2. `BEGIN IMMEDIATE`。
3. `INSERT INTO journal(aggregate_id, seq_nr, schema_version, event_type, payload, occurred_at)`。UNIQUE 違反 → rollback、`Conflict { expected, actual: 現在 version }`。
4. `expected == 0` → `INSERT INTO snapshot(aggregate_id, version = new_version, seq_nr = new_version, schema_version, payload, updated_at)`（既存行があれば rollback + Conflict）。
   それ以外 → `UPDATE snapshot SET version = new_version, seq_nr = new_version, payload = ?, updated_at = ? WHERE aggregate_id = ? AND version = expected`。影響 0 行 →
   `SELECT version` で actual を読み rollback + `Conflict { expected, actual }`。
5. `COMMIT`。Io 失敗は `Io { kind, path }`。

### 3.2 find_by_id（BR1.2）

1. `SELECT version, seq_nr, schema_version, payload FROM snapshot WHERE aggregate_id = ?`。無ければ journal を数え、0 なら `NotFound { intent_id }`、1 以上なら
   `Corrupt(MissingSnapshot)`。
2. StateWire を復号（schema_version 検査）→ `WorkflowExecution::from_state(state)`（Err → `Corrupt(InvariantViolation)`）→ `with_version(snapshot.version)`。
3. `SELECT … FROM journal WHERE aggregate_id = ? AND seq_nr > snapshot.seq_nr ORDER BY seq_nr` を復号して順に `apply_event`（Err → `Corrupt(SequenceGap | InvariantViolation)`）。
   replay ループ終了後、Repository が明示的に `with_version(最後に適用した seq_nr)` を載せる（`apply_event` は version を変えない）。通常運転では 0 件
   （スナップショットは毎 store 更新）。
4. 集約を返す。

### 3.3 投影の差分読取（BR1.4、利用は U4）

`checkpoint(name)` → `events_after(cp)` → 投影を描く → `advance_checkpoint(name, last_global)`。advance は単調。再生成時は行削除（別 API `reset_checkpoint` は本 Unit では作らない — U4 の設計）。

### 3.4 登録簿の直列化（BR2.4、利用は U7）

`store.within_write_transaction(|tx| { read intents.json; mutate; atomic write; Ok(()) })` — Tx は `BEGIN IMMEDIATE` で開くため、同じ DB を開く別プロセスの store /
登録簿変更は busy_timeout 内で直列化される。`f` が Err なら rollback（ファイル書込は tmp+rename で原子的、DB 側の変更は無い）。

### 3.5 open / 初期化（BR2.1 / BR2.2）

`Connection::open(path)` → `PRAGMA busy_timeout = 5000` → `PRAGMA user_version` → 0: DDL（C6）を実行し `user_version = 1`；1: 何もしない；他: `Schema { found, supported: 1 }`。

## 4. ワイヤ形式（BR2.5、正準 JSON）

### 4.1 イベント（journal.payload、`type` タグ）

| type | 材料（フィールド名: 型） |
|---|---|
| `Started` | `definition_id: string`, `definition_revision: string`, `scope: string`, `request: string`, `depth: string \| null`, `test_strategy: string \| null`, `stages: [{slug, phase, plan_action, conditional}]` |
| `StageCompleted` | `stage: string`, `next_stage: string \| null` |
| `GateOpened` | `stage: string`, `artifacts: [string]` |
| `GateApproved` | `stage: string`, `user_input: string \| null`, `next_stage: string \| null`, `phase_boundary: string \| null` |
| `GateRejected` | `stage: string`, `feedback: string \| null`, `revision_count: u32` |
| `StageRevised` / `StageSkipped` / `Parked` | `stage: string`（StageSkipped は `reason: string`, `next_stage: string \| null` も） |
| `Jumped` | `direction: string`, `source: string`, `target: string`, `stages_reset: [string]`, `stages_skipped: [string]` |
| `Unparked` | （材料なし `{}`） |
| `Recomposed` | `skipped: [string]`, `added: [string]`, `stages_in_scope: [string]` |
| `AutonomyModeSet` | `mode: string`（autonomous / gated） |

封筒の `intent_id` / `seq_nr` / `schema_version` / `occurred_at` は列に出す（payload には含めない）。復号時に列の値から `WorkflowExecutionEvent::new` を組み立てる。

### 4.2 状態（snapshot.payload、16 属性）

`intent_id`, `definition_id`, `definition_revision`, `stages: [{slug, phase, plan_action, conditional}]`, `plan: [string]`, `overlay: [string]`, `conditional: [bool]`,
`checkbox: [string]`（6 マーク）, `cursor: u64`, `status: string`（running / completed）, `parked_at: u64 \| null`, `autonomy: string`, `approved: [bool]`,
`revision_count: [u32]`, `seq_nr: u64`, `version: u64`。復号後は `from_state` の不変条件検査が最終防衛線。

## 5. 検証モデル `journal_protocol.qnt`（BR3.3 / BR3.4 / BR3.5）

- 定数: `WRITERS = 2`。状態: §rules BR3.3。`init`: journalLen = 0, snapVersion = 0, snapSeq = 0, checkpoint = 0, readModelSeq = 0, loadedVersion = 全 writer 0。
- `store_ok(w)` は genesis（snapVersion == 0 かつ loadedVersion[w] == 0）も同じ規則で扱う（expected 0）。
- 不変条件は状態遷移レベル（prev → current）で書く（`snapshot` アクションで prev を取る — audit_lock v2 と同じ型）。
- mutation（code-summary に記録）: 各 invariant につき 1 変異 — 例: store_conflict が journalLen を増やす変異 → conflict_rejected 違反、store_ok のガード除去 →
  no_lost_update 違反、catchup が checkpoint を減らす変異 → checkpoint_monotone 違反、catchup が readModelSeq を journalLen+1 にする変異 → truth_is_journal 違反 …。
- ITF: `quint run … --out-itf` で 6 シード以上採取、`#meta` 正規化済みでコミット。再生先は InMemoryEventStore + フェイク投影（adapter tests）。

## 6. 退役チェックリスト（BR3.1 / BR3.2）

use-case `workspace/` mod、adapter `workspace/fs_workspace_lock.rs` / `process_probe.rs`、domain `workspace/{lock_protocol,lock_identity}.rs` と `pub use`、
infra-io `process_probe.rs`、tests `fs_workspace_lock_test.rs` / `audit_lock_conformance.rs`、`formal/workspace/audit_lock.qnt`、`tests/conformance/fixtures/audit_lock/`、
`scripts/quint-gate.sh` の audit_lock ステップ（→ journal_protocol）、`tools/lint` の `reap-decision-locality`（ルール本体・HELP・赤例テスト・README の記述）、
adapter `Cargo.toml` の `md5`。grep（BR3.1）で 0 件を確認。

## 7. テスト設計（TDD、層ごと）

| 層 | Red（先に書く） | 内容 |
|---|---|---|
| Data model（use-case / domain） | 値型・エラー型 | GlobalSeqNr / ProjectionName / IntentId(UUIDv7) / IntentDirName の parse 受理・拒否（各 5〜8 本）、エラー Display の材料、`WorkflowExecutionState` 改名後の既存テスト緑 |
| Repository（adapter） | 契約テスト（ジェネリック） | ラウンドトリップ（start → 数コマンド → store × n → 新インスタンスで find_by_id → state が等しい）、NotFound、Conflict（2 再水和の競合）、Corrupt（MissingSnapshot / UndecodablePayload / SchemaVersion）、events_after の順序、checkpoint 単調性・未登録 = ZERO、within_write_transaction の直列化（同一 DB 2 接続、busy_timeout 内） |
| Business logic（adapter） | ワイヤ | PBT: 任意イベント / 状態の encode→decode 恒等、未知フィールド・未知 type の拒否、正準 JSON のバイト決定性 |
| API（adapter / formal） | ITF + クラッシュ再構成 | journal_protocol fixtures の再生（全アクション網羅）、クラッシュ再構成（store 後に接続を捨て、新接続で find_by_id → 同一 state）、SQLite スキーマ突合（PRAGMA table_info = C6） |

既存スイート（engine_loop ITF、ゴールデン、WorkflowDefinitionRepository）は IntentId のリテラル置換と State 改名の追随のみ。

## 8. 未決・申し送り

- U4: `reset_checkpoint`（再生成）と投影の描画。U5: Conflict の 1 回再試行。U7: `within_write_transaction` での birth / archive、`IntentDirName` の予約ラベル拒否。
- 複数クローン間のジャーナル交換は後続 intent（P7）。
