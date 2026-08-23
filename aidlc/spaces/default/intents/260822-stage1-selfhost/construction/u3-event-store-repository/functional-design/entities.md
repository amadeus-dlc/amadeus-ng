# entities — U3 SQLite EventStore と WorkflowExecutionRepository（`u3-event-store-repository`）

> Functional Design（Construction 3.1）成果物（Unit: U3、kind: library、Bolt: B5、規模 L）。出典: `../../../inception/units-generation/unit-of-work.md`（U3）、
> `../../../inception/requirements-analysis/requirements.md`（FR1.2 / FR1.3 / NFR1 / NFR3）、`../../../inception/contract-design/contract-summary.md`（C3 / C6）、
> `../../../inception/domain-design/decisions.md`（ADR-001 / 003 / 004 / 006 / 007 / 008）、`../../../inception/domain-design/components.md`（PersistenceGateways）、
> Bolt B3 実装（`modules/core/domain/src/orchestration/`）、U2 機能設計 pending-revision 項目 8 / 9、確認質問 `functional-design-questions.md`（Q1〜Q4 = A、
> P1〜P7、Looks correct）、Bolt B4 で改訂した仕様（10 号 §2.1 / §3、11 号 §2.1 / §2.2 / §10、`deviations.md` # 4）。
>
> 下の fenced `yaml` が正本。ドメイン層の集約・イベントは U2 のもの（`WorkflowExecution` / `WorkflowExecutionEvent`）を使い、本 Unit が新設するのは
> **ポート（ユースケース層）・ストアとワイヤ（アダプタ層）・値オブジェクト 2 型の是正（ドメイン層）・検証モデル**である。

## 1. エンティティ（正本）

```yaml
entities:
  # --- ユースケース層（core-use-case::orchestration）— ポートとエラー ---
  - name: WorkflowExecutionRepository
    kind: port-trait
    layer: use-case
    description: "集約 WorkflowExecution の ES 形 Repository（C3）。store = 1 イベント + 適用後集約を同一 Tx で永続化、find_by_id = 最新スナップショット + 以降の replay で完全再構成。save は持たない（ES 拡張語彙 store — ADR-006）"
    attributes:
      - { name: find_by_id, type: "async fn(&self, &IntentId) -> Result<WorkflowExecution, RepositoryError>", required: true }
      - { name: store, type: "async fn(&self, &WorkflowExecutionEvent, &WorkflowExecution) -> Result<(), RepositoryError>", required: true }
    constraints:
      - "dyn 禁止（静的束縛、use-case-rules §2）。Send / Sync を要求しない（tokio current_thread、Q3 = A）"
      - "ユースケースは Tx を持たない（Tx 所有は実装 — C3 ②）"
  - name: EventStore
    kind: port-trait
    layer: use-case
    description: "event-store-adapter-rs 同形のローカル定義（ADR-006）。ジェネリック <AID, A, E>。Repository 実装が内部で使う下位ポート"
    attributes:
      - { name: persist_event, type: "async fn(&mut self, &E, version: u64) -> Result<(), EventStoreError>", required: true, constraints: "スナップショットを更新しない追記。本 Unit では store が persist_event_and_snapshot のみを使う（ADR-001: 毎 store でスナップショット更新）" }
      - { name: persist_event_and_snapshot, type: "async fn(&mut self, &E, &A) -> Result<(), EventStoreError>", required: true, constraints: "journal INSERT + snapshot 条件付き UPSERT を同一 Tx" }
      - { name: get_latest_snapshot_by_id, type: "async fn(&self, &AID) -> Result<Option<A>, EventStoreError>", required: true }
      - { name: get_events_by_id_since_seq_nr, type: "async fn(&self, &AID, seq_nr: u64) -> Result<Vec<E>, EventStoreError>", required: true, constraints: "seq_nr より大きいイベントを seq_nr 昇順で" }
  - name: JournalReader
    kind: port-trait
    layer: use-case
    description: "投影（U4）が使う差分読取とチェックポイント（C3）。ストア実装が同時に実装する"
    attributes:
      - { name: events_after, type: "async fn(&self, GlobalSeqNr) -> Result<Vec<(GlobalSeqNr, WorkflowExecutionEvent)>, EventStoreError>", required: true, constraints: "global_seq_nr 昇順、全集約横断" }
      - { name: checkpoint, type: "async fn(&self, &ProjectionName) -> Result<GlobalSeqNr, EventStoreError>", required: true, constraints: "未登録の投影は GlobalSeqNr::ZERO" }
      - { name: advance_checkpoint, type: "async fn(&mut self, &ProjectionName, GlobalSeqNr) -> Result<(), EventStoreError>", required: true, constraints: "単調: 現在値未満なら CheckpointRegression" }
  - name: RepositoryError
    kind: error-enum
    layer: use-case
    description: "Repository ポートの失敗（材料のみ、手実装 Display / Error — coding-rules/error-handling.md）"
    attributes:
      - { name: NotFound, type: "{ intent_id: IntentId }", required: true }
      - { name: Conflict, type: "{ expected: u64, actual: u64 }", required: true, constraints: "楽観 version 不一致。ユースケースが再水和して 1 回だけ再試行（C3）" }
      - { name: Io, type: "{ kind: std::io::ErrorKind, path: Option<PathBuf> }", required: true }
      - { name: Corrupt, type: "{ aggregate_id: IntentId, seq_nr: Option<u64>, cause: CorruptCause }", required: true, constraints: "復号不能・スナップショット欠落・不変条件違反（from_state の Err）" }
  - name: EventStoreError
    kind: error-enum
    layer: use-case
    description: "EventStore / JournalReader の失敗（材料のみ）。Repository 実装が RepositoryError へ写す"
    attributes:
      - { name: Conflict, type: "{ expected: u64, actual: u64 }", required: true }
      - { name: Io, type: "{ kind: std::io::ErrorKind, path: Option<PathBuf> }", required: true }
      - { name: Corrupt, type: "{ aggregate_id: String, seq_nr: Option<u64>, cause: CorruptCause }", required: true }
      - { name: Schema, type: "{ found: u32, supported: u32 }", required: true, constraints: "PRAGMA user_version が対応範囲外" }
      - { name: CheckpointRegression, type: "{ projection: ProjectionName, current: GlobalSeqNr, requested: GlobalSeqNr }", required: true }
  - name: CorruptCause
    kind: value-enum
    layer: use-case
    description: "Corrupt の原因分類（材料）"
    attributes:
      - { name: variants, type: enum, allowed_values: [MissingSnapshot, UndecodablePayload, UnknownEventType, SchemaVersion, InvariantViolation, SequenceGap], required: true }
  - name: GlobalSeqNr
    kind: value-object
    layer: use-case
    description: "全集約横断のジャーナル通番（C6 journal.global_seq_nr）。投影チェックポイントの単位"
    attributes:
      - { name: value, type: u64, required: true, constraints: "0 = 『まだ何も読んでいない』（ZERO 定数）。ジャーナル行は 1 以上" }
  - name: ProjectionName
    kind: value-object
    layer: use-case
    description: "投影の名前（C6 checkpoint.projection）"
    attributes:
      - { name: value, type: string, required: true, constraints: "kebab `^[a-z][a-z0-9-]*$`、1〜64 字。例: state-file / audit-shard" }

  # --- ドメイン層（core-domain）— 是正 2 型 + 改名 ---
  - name: IntentId
    kind: value-object
    layer: domain (orchestration)
    description: "集約 WorkflowExecution の identity = intents.json の uuid。**UUIDv7**（01 号 §3.3、Q2 = A の裁定を B5 で実装）。現行の kebab 受理は廃止"
    attributes:
      - { name: value, type: string, required: true, constraints: "小文字 36 字 `^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$`（version nibble 7、variant 10xx）。大文字・短縮形・他 version は拒否" }
    constraints:
      - "IntentIdError の変種: Empty / Length { actual } / Format { position } / Version { found } / Variant { found }（材料のみ）"
      - "文字列ソートがミリ秒粒度の作成順になる性質（48-bit Unix-ms プレフィクス）は upstream 同等 — 検証はしない（形式のみ）"
  - name: IntentDirName
    kind: value-object
    layer: domain (workspace)
    description: "記録ディレクトリ名（`intents.json` の dirName、`<record>` のパスセグメント）。IntentId とは別の値で、投影先のパス解決に使う（11 号 §2.2、オーナー裁定 2026-08-23）"
    attributes:
      - { name: value, type: string, required: true, constraints: "`^[0-9]{6}-[a-z0-9](?:[a-z0-9-]*[a-z0-9])?$`、全体 64 字以下（`<YYMMDD>-<slug>` の kebab。衝突サフィックス `-2`… を含む）。予約ラベル拒否は birth（U7）の責務" }
  - name: WorkflowExecutionState
    kind: value-object (memento)
    layer: domain (orchestration)
    description: "集約の全状態の写し（旧 WorkflowExecutionSnapshot — B5 で改名、U2 pending-revision 項目 9）。`WorkflowExecution::state()` が作り、`from_state()` が不変条件つきで復元する唯一の経路。serde を知らない"
    attributes:
      - { name: fields, type: "16 属性", required: true, constraints: "intent_id / definition_id / definition_revision / stages: Vec<StageEntry> / plan / overlay / conditional / checkbox / cursor / status / parked_at / autonomy / approved / revision_count / seq_nr / version（B3 実装と同一）" }
    constraints:
      - "Builder は `WorkflowExecutionStateBuilder`、エラーは `StateError`（旧 SnapshotError）。旧名の再エクスポート・型エイリアスは残さない"

  # --- アダプタ層（core-interface-adapter::orchestration）— ストア・ワイヤ・実装 ---
  - name: SqliteEventStore
    kind: gateway-impl
    layer: interface-adapter
    description: "EventStore<IntentId, WorkflowExecution, WorkflowExecutionEvent> と JournalReader の SQLite 実装（rusqlite bundled、同期 API を async fn 内で呼ぶ — Q3 = A）。C6 の 3 テーブルを所有"
    attributes:
      - { name: path, type: StorePath, required: true }
      - { name: connection, type: "rusqlite::Connection", required: true, constraints: "プロセス内 1 接続。open 時に user_version 検査 / 初期化、busy_timeout 5000ms" }
      - { name: clock, type: "Clock（機構）", required: true, constraints: "updated_at の供給元。Gateway には数えない" }
    constraints:
      - "書込 Tx は常に BEGIN IMMEDIATE（書込ロック先取り）。同一 Tx で journal INSERT + snapshot 条件付き更新（楽観 version）"
      - "within_write_transaction(f) を公開し、intents.json の read-modify-write（U7）を同じ Tx で直列化する（Q2 = A）"
  - name: StorePath
    kind: value-object
    layer: interface-adapter
    description: "ストアファイルの場所（Q1 = A）"
    attributes:
      - { name: value, type: PathBuf, required: true, constraints: "`<aidlc root>/spaces/<SpaceName>/intents/.aidlc-store.sqlite`（`for_space(aidlc_root, &SpaceName)` で導出。既存 .gitignore `aidlc/spaces/*/intents/.aidlc-*` で git 管理外）" }
  - name: JournalRow
    kind: table-row
    layer: interface-adapter
    description: "C6 journal の 1 行"
    attributes:
      - { name: global_seq_nr, type: "INTEGER PRIMARY KEY AUTOINCREMENT", required: true }
      - { name: aggregate_id, type: TEXT, required: true, constraints: "IntentId（UUIDv7 文字列）" }
      - { name: seq_nr, type: INTEGER, required: true, constraints: "集約内 +1、UNIQUE(aggregate_id, seq_nr)" }
      - { name: schema_version, type: INTEGER, required: true, constraints: "イベントワイヤの版 = 1" }
      - { name: event_type, type: TEXT, required: true, constraints: "WorkflowExecutionEventPayload の変種名（Started / StageCompleted / … / AutonomyModeSet の 12 語）" }
      - { name: payload, type: TEXT, required: true, constraints: "EventPayloadWire の正準 JSON（canon-json）" }
      - { name: occurred_at, type: TEXT, required: true, constraints: "呼出側が渡した ISO 8601 UTC 文字列を素通し" }
  - name: SnapshotRow
    kind: table-row
    layer: interface-adapter
    description: "C6 snapshot の 1 行（集約 1 行）"
    attributes:
      - { name: aggregate_id, type: "TEXT PRIMARY KEY", required: true }
      - { name: version, type: INTEGER, required: true, constraints: "楽観 version = 適用済みイベント数（store ごとに +1）" }
      - { name: seq_nr, type: INTEGER, required: true, constraints: "このスナップショットが含む最後の seq_nr（= version）" }
      - { name: schema_version, type: INTEGER, required: true, constraints: "状態ワイヤの版 = 1" }
      - { name: payload, type: TEXT, required: true, constraints: "StateWire（16 属性）の正準 JSON。revision_count はここに含む（列追加なし — P4）" }
      - { name: updated_at, type: TEXT, required: true, constraints: "Clock から" }
  - name: CheckpointRow
    kind: table-row
    layer: interface-adapter
    description: "C6 checkpoint の 1 行（投影 1 行）"
    attributes:
      - { name: projection, type: "TEXT PRIMARY KEY", required: true }
      - { name: last_global_seq, type: INTEGER, required: true, constraints: "単調増加（巻き戻しは行削除のみ — 再生成時）" }
      - { name: updated_at, type: TEXT, required: true }
  - name: EventPayloadWire
    kind: wire-struct (serde)
    layer: interface-adapter
    description: "WorkflowExecutionEventPayload のワイヤ表現（adapter に閉じる。ドメイン型へは parse-don't-validate）。JSON は `{\"type\": \"<変種名>\", ...材料}`"
    attributes:
      - { name: type, type: string, required: true, constraints: "12 語の閉集合。未知は Corrupt(UnknownEventType)" }
      - { name: fields, type: object, required: true, constraints: "変種ごとの材料（functional-spec §4 の表）。固定トークンは upstream 綴り（CheckboxState のマーク、PlanAction EXECUTE / SKIP、PhaseId）、列挙は snake_case 文字列。未知フィールドは拒否（Corrupt）" }
  - name: StateWire
    kind: wire-struct (serde)
    layer: interface-adapter
    description: "WorkflowExecutionState のワイヤ表現（16 属性、正準 JSON）"
    attributes:
      - { name: fields, type: object, required: true, constraints: "functional-spec §4 の表。復号後 `WorkflowExecution::from_state` の不変条件検査を通す" }
  - name: WorkflowExecutionRepositoryImpl
    kind: gateway-impl
    layer: interface-adapter
    description: "WorkflowExecutionRepository の実 Gateway（1 trait 1 Impl）。SqliteEventStore を内包し、store / find_by_id を C3 の約束どおりに実装"
    attributes:
      - { name: store, type: "SqliteEventStore", required: true }
  - name: InMemoryEventStore
    kind: test-double
    layer: interface-adapter (memory/)
    description: "EventStore + JournalReader の in-memory 実装（BTreeMap）。SQLite と同じ契約テストを通す（gateway-taxonomy §6、先に書く）"
  - name: InMemoryWorkflowExecutionRepository
    kind: test-double
    layer: interface-adapter (memory/)
    description: "WorkflowExecutionRepository の in-memory 実装。ユースケース（U5 / U6）のテストはこれで組む（C3 ④）"

  # --- 検証モデル（formal/orchestration/journal_protocol.qnt）---
  - name: JournalProtocolModel
    kind: quint-model
    description: "集約の永続化協定（Q4 = A）。1 集約・2 writer・1 投影の抽象。真実源はジャーナル、スナップショットは毎 store 更新、投影はチェックポイントから冪等キャッチアップ"
    attributes:
      - { name: vars, type: list, required: true, constraints: "journalLen / snapVersion / snapSeq / checkpoint / readModelSeq / loadedVersion: int -> int（writer ごと）/ lastAction / lastActor + prev* スナップショット" }
      - { name: actions, type: list, required: true, constraints: "load(w) / store_ok(w) / store_conflict(w) / catchup / crash / idle" }
      - { name: invariants, type: list, required: true, constraints: "conflict_rejected / snapshot_tracks_journal / version_equals_journal / checkpoint_monotone / checkpoint_bounded / projection_idempotent / truth_is_journal / no_lost_update" }
      - { name: witnesses, type: list, required: true, constraints: "w_conflict / w_crash_then_catchup / w_interleaved_writers / w_idempotent_catchup" }
    constraints:
      - "DoD（ADR 0003）: named invariant ごとの mutation 検出 + 状態遷移レベル不変条件の併置 + in-module witness（負形式 run）"

  # --- 退役（本 Unit で削除）---
  - name: RetiredLockMachinery
    kind: retirement-list
    description: "ADR-007 の退役対象（コード・テスト・モデル・lint・依存）"
    attributes:
      - { name: use_case, type: list, constraints: "`core_use_case::workspace::{WorkspaceLock, AcquireBudget, LockGuard, AcquireError}`（workspace mod ごと）" }
      - { name: adapter, type: list, constraints: "`core_interface_adapter::workspace::fs_workspace_lock`、`core_interface_adapter::process_probe`、テスト `fs_workspace_lock_test.rs`、依存 `md5`" }
      - { name: domain, type: list, constraints: "`core_domain::workspace::{LockProtocol, LockIdentity, reap_eligible, LockError}`（mod lock_protocol / lock_identity）" }
      - { name: infra_io, type: list, constraints: "`infra_io::process_alive`（process_probe.rs）" }
      - { name: formal, type: list, constraints: "`formal/workspace/audit_lock.qnt`、`tests/conformance/fixtures/audit_lock/`、`modules/core/domain/tests/audit_lock_conformance.rs`、`scripts/quint-gate.sh` の audit_lock ステップ" }
      - { name: lint, type: list, constraints: "`tools/lint` の `reap-decision-locality` ルールとその赤例テスト（`reap_eligible` が消えるため対象を失う）" }

relationships:
  - { from: WorkflowExecutionRepositoryImpl, to: SqliteEventStore, cardinality: "one-to-one", description: "内包（合成）。Tx は SqliteEventStore が所有" }
  - { from: SqliteEventStore, to: "JournalRow / SnapshotRow / CheckpointRow", cardinality: "one-to-many", description: "C6 の 3 テーブルを所有" }
  - { from: SqliteEventStore, to: "EventPayloadWire / StateWire", cardinality: "one-to-many", description: "payload 列の符号化・復号（canon-json）" }
  - { from: WorkflowExecutionRepositoryImpl, to: "WorkflowExecution (U2)", cardinality: "many-to-one", description: "from_state + apply_event による再構成、state() による写し" }
  - { from: "InMemoryWorkflowExecutionRepository", to: "InMemoryEventStore", cardinality: "one-to-one", description: "同じ契約テストを SQLite 実装と共有" }
  - { from: JournalProtocolModel, to: "InMemoryEventStore + fake projector", cardinality: "one-to-one", description: "ITF 準拠テストの再生先（adapter tests）" }
  - { from: "U4 ReadModelUpdater", to: JournalReader, cardinality: "many-to-one", description: "差分読取とチェックポイント（本 Unit は実装のみ、利用は U4）" }
  - { from: "U7 intent create", to: "SqliteEventStore.within_write_transaction", cardinality: "many-to-one", description: "intents.json の直列化（Q2 = A）。本 Unit は原語を提供" }
```

## 2. 要約

- **新設**: ポート 3 本 + エラー 2 型 + 値 2 型（use-case）、SQLite ストア + ワイヤ 2 型 + Repository 実装 + InMemory 2 本 + StorePath（adapter）、
  `IntentDirName`（domain workspace）、Quint モデル `journal_protocol.qnt`。
- **是正**: `IntentId` = UUIDv7、`WorkflowExecutionSnapshot` → `WorkflowExecutionState`（`state()` / `from_state()`、`StateError`、`…StateBuilder`）。
- **退役**: mkdir ロック系（use-case / adapter / domain / infra-io / formal / lint / md5）。
- 配置規約: ポートはユースケース層、実装は `XxxRepositoryImpl` + `InMemoryXxx`（gateway-taxonomy §5）、機構 `Clock` はクレート root の機構モジュール（§1）。
