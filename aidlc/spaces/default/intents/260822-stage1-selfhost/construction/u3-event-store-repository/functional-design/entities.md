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
      - { name: store, type: "async fn(&mut self, &WorkflowExecutionEvent, &WorkflowExecution) -> Result<(), RepositoryError>", required: true }
    constraints:
      - "dyn 禁止（静的束縛、use-case-rules §2）。Send / Sync を要求しない（tokio current_thread、Q3 = A）"
      - "ユースケースは Tx を持たない（Tx 所有は実装 — C3 ②）"
  - name: EventStore
    kind: port-trait
    layer: use-case
    description: "event-store-adapter-rs 同形のローカル定義（ADR-006）。ジェネリック <AID, A, E>。Repository 実装が内部で使う下位ポート。数値パラメータは C3 の `usize` を実装済みドメイン型（`seq_nr` / `version` = u64 — B3 実装）に合わせて **u64** に具体化した（無言の変更にしない: C3 の改訂提案として contract-summary の所有者（U5 / U6）へ申し送り、ゲートで契約改訂 — レビュー所見 2）"
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
  - name: EventStoreImpl
    kind: gateway-impl
    layer: interface-adapter
    description: "EventStore<IntentId, WorkflowExecution, WorkflowExecutionEvent> と JournalReader の SQLite 実装（rusqlite bundled、同期 API を async fn 内で呼ぶ — Q3 = A）。C6 の 3 テーブルを所有"
    attributes:
      - { name: path, type: StorePath, required: true }
      - { name: connection, type: "rusqlite::Connection", required: true, constraints: "プロセス内 1 接続。open 時に user_version 検査 / 初期化、busy_timeout 5000ms。EventStore / JournalReader の書込メソッドは C3 どおり `&mut self`" }
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
      - { name: version, type: INTEGER, required: true, constraints: "楽観 version = 永続化済みイベント数 = 最後の seq_nr（store ごとに +1）。集約側の `version()` は遷移で変わらず Repository が `with_version` で載せる（B3 実装契約）" }
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
    description: "WorkflowExecutionRepository の実 Gateway（1 trait 1 Impl）。EventStoreImpl を内包し、store / find_by_id を実装"
    attributes:
      - { name: store, type: "EventStoreImpl<C>", required: true, constraints: "EventStoreImpl<C> を直接所有する。可変操作は `&mut self`（内部可変性は使わない — coding-rules/interior-mutability.md）。`event_store(&self) -> &EventStoreImpl<C>` / `event_store_mut(&mut self) -> &mut EventStoreImpl<C>` に分けて公開する（オーナー裁定 2026-08-23、委任 8 で是正）" }
  - name: InMemoryEventStore
    kind: test-double
    layer: interface-adapter (memory/)
    description: "EventStore + JournalReader の in-memory 実装（BTreeMap）。SQLite と同じ契約テストを通す（gateway-taxonomy §6、先に書く）"
  - name: InMemoryWorkflowExecutionRepository
    kind: test-double
    layer: interface-adapter (memory/)
    description: "WorkflowExecutionRepository の in-memory 実装（InMemoryEventStore を直接所有 — Impl と同じ形。内部可変性は使わない、coding-rules/interior-mutability.md）。ユースケース（U5 / U6）のテストはこれで組む（C3 ④）"

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
  - { from: WorkflowExecutionRepositoryImpl, to: EventStoreImpl, cardinality: "one-to-one", description: "内包（合成）。Tx は EventStoreImpl が所有" }
  - { from: EventStoreImpl, to: "JournalRow / SnapshotRow / CheckpointRow", cardinality: "one-to-many", description: "C6 の 3 テーブルを所有" }
  - { from: EventStoreImpl, to: "EventPayloadWire / StateWire", cardinality: "one-to-many", description: "payload 列の符号化・復号（canon-json）" }
  - { from: WorkflowExecutionRepositoryImpl, to: "WorkflowExecution (U2)", cardinality: "many-to-one", description: "from_state + apply_event による再構成、state() による写し" }
  - { from: "InMemoryWorkflowExecutionRepository", to: "InMemoryEventStore", cardinality: "one-to-one", description: "同じ契約テストを SQLite 実装と共有" }
  - { from: JournalProtocolModel, to: "InMemoryEventStore + fake projector", cardinality: "one-to-one", description: "ITF 準拠テストの再生先（adapter tests）" }
  - { from: "U4 ReadModelUpdater", to: JournalReader, cardinality: "many-to-one", description: "差分読取とチェックポイント（本 Unit は実装のみ、利用は U4）" }
  - { from: "U7 intent create", to: "EventStoreImpl.within_write_transaction", cardinality: "many-to-one", description: "intents.json の直列化（Q2 = A）。本 Unit は原語を提供" }
```

## 2. 要約

- **新設**: ポート 3 本 + エラー 2 型 + 値 2 型（use-case）、SQLite ストア + ワイヤ 2 型 + Repository 実装 + InMemory 2 本 + StorePath（adapter）、
  `IntentDirName`（domain workspace）、Quint モデル `journal_protocol.qnt`。
- **是正**: `IntentId` = UUIDv7、`WorkflowExecutionSnapshot` → `WorkflowExecutionState`（`state()` / `from_state()`、`StateError`、`…StateBuilder`）。
- **退役**: mkdir ロック系（use-case / adapter / domain / infra-io / formal / lint / md5）。
- 配置規約: ポートはユースケース層、実装は `XxxRepositoryImpl` + `InMemoryXxx`（gateway-taxonomy §5）、機構 `Clock` はクレート root の機構モジュール（§1）。

## Review

**Verdict:** NOT-READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-08-23T08:40:40Z
**Iteration:** 1（advisory, unit: u3-event-store-repository）

### Findings

| # | Severity | Location | Finding | Recommendation |
|---|---|---|---|---|
| 1 | Critical | `rules.md` BR1.2 / BR1.3 / BR2.3、`functional-spec.md` §3.1 手順1・§3.2 手順3 | 楽観 version の算出式が、既に実装済みの U2 `WorkflowExecution` API の実挙動と矛盾する。実コード（`modules/core/domain/src/orchestration/workflow_execution.rs`）を確認した: `version()` の doc コメント（287-290 行）は「楽観 version（集約の遷移では変わらない — Repository が `with_version` で載せる）」と明記し、`apply_event`（744-759 行）は `self.seq_nr` のみを更新して `self.version` には一切触れない（テスト `with_version_replaces_only_the_optimistic_version`、1889-1894 行がこの契約を固定）。genesis では `start_with_entries`（214 行）が `version: 0` を無条件にセットする一方、返す `WorkflowExecutionEvent` は `seq_nr: 1`（191 行）。BR1.3 は「期待 version = `aggregate.version() − 1`（= `event.seq_nr − 1`）」と両式を同値主張するが、genesis では `aggregate.version() − 1 = 0u64 − 1` で **u64 アンダーフロー**（デバッグビルドは panic、リリースビルドは `u64::MAX` へ wrap）となり、`event.seq_nr − 1 = 0` と一致しない。非 genesis でも、`find_by_id` がロードした `version`（Repository が `with_version` で載せた値 `V`）は `apply_event` を挟んでも不変のため、store 時点の `aggregate.version()` は常に「現在 DB に永続化済みの version」そのもの（`V`）であり、そこから `− 1` した `V−1` を `expected` として `UPDATE … WHERE version = :expected` に渡すと、実際の行は `version = V` なので必ず影響行 0 になり、実際には競合が無いにもかかわらず**毎回** `Conflict { expected: V-1, actual: V }` を返す。つまり本 Unit の中核機構（FR1.2 の主たる合格基準である store の楽観ロック）は、記述どおりに実装すると genesis で panic/暴走し、以降の通常書込みも恒常的に偽陽性の競合エラーになる。 | `expected` の算出を `event.seq_nr() − 1`（`aggregate.version()` を経由しない）に一本化するか、`aggregate.version()` を使うなら `− 1` を外して `expected = aggregate.version()` とし、書込み成功後に `with_version(expected + 1)`（または `event.seq_nr()`）で更新後の集約に載せ替える手順を明記する。あわせて BR1.2 / `functional-spec.md` §3.2 の「適用 1 件ごとに version + 1」の記述も、`apply_event` 自体は version を変更しないという実コード契約を踏まえ「replay ループ終了後に Repository が明示的に `with_version` を呼ぶ」と書き改める。 |
| 2 | Major | `entities.md`（EventStore の `persist_event` / `get_events_by_id_since_seq_nr`）と `contract-summary.md` C3 の対応 | 承認済み契約 C3（`inception/contract-design/contract-summary.md` 97-135 行）の `EventStore<AID, A, E>` trait は `persist_event(&mut self, event: &E, version: usize)` / `get_events_by_id_since_seq_nr(&self, aid: &AID, seq_nr: usize)` のように数値パラメータを **`usize`** で定義しているが、本設計の `entities.md`（32 行・35 行）は同じメソッドを **`u64`** で定義している。`rules.md` BR1.1 は「trait の形は C3 のコードを正とし、型名だけ本設計…に具体化」と明言しており、原子型そのものの変更は想定されていない。C3 の所有者は使う側のユースケース層（U5/U6、`contract-summary.md` §3）であり、U3 は「準拠」する側 — U3 が無断で型を変えると、U5/U6 が C3 どおり `usize` で実装した場合に trait 実装がコンパイルエラー（型不一致）になるリスクがある（実ドメイン型 `seq_nr`/`version` が `u64` である事実 — `workflow_execution.rs:84-85` — に照らせば `u64` への変更自体は妥当と思われるが、無言の変更である点が問題）。 | `entities.md` に「C3 の `usize` を実装済みドメイン型（`u64`）に合わせて具体化した」という一文を明記するか、C3 側を `u64` に合わせて改訂する（改訂は所有者 U5/U6 側で）。いずれかを選び、無言の型変更を残さない。 |
| 3 | Major | `entities.md`（`WorkflowExecutionRepositoryImpl.store: SqliteEventStore`）、`functional-spec.md` §2 | `WorkflowExecutionRepository::store` は C3 で `&self`（`entities.md` 23 行も同じ）だが、内部で使う `EventStore::persist_event_and_snapshot` は `&mut self`（`entities.md` 33 行、C3 も同じ）。`WorkflowExecutionRepositoryImpl` が保持する `store` フィールドの型は `entities.md` 174 行で単なる `SqliteEventStore`（値型・ラッパーなし）としか書かれておらず、`&self` メソッドの中から `&mut self` メソッドを呼ぶために必要な内部可変性の機構（`Mutex` / `RefCell` / `Cell` 等）がどこにも記載されていない（`entities.md` / `rules.md` / `functional-spec.md` 全文を検索しても `Mutex` / `RefCell` / `Cell` / interior は 0 件）。この点は上流の contract-design レビュー（`contract-summary.md` の `## Review` Finding #3、Minor）が「functional-design（U3）で `WorkflowExecutionRepositoryImpl` 内の `EventStore` 保持方法（`tokio::sync::Mutex` 等）を明記する」と名指しで本ステージへ申し送っていたが、本設計では未対応のまま残っている。`within_write_transaction`（`&mut self` — `functional-spec.md` §2）を含め、同種の問題が複数メソッドに連鎖する。 | `SqliteEventStore` の内部可変性戦略（例: 内部で `tokio::sync::Mutex<rusqlite::Connection>` を持たせて trait メソッド自体を `&self` で実装する、あるいは `WorkflowExecutionRepositoryImpl` が `store: tokio::sync::Mutex<SqliteEventStore>` を保持する）を `entities.md` に明記する。 |

> （2026-08-23 追記: 所見 3 は前提ごと失効した。`WorkflowExecutionRepository::store` を `&mut self` に是正したことで、`&self` から `&mut self` を呼ぶための内部可変性の機構そのものが不要になった（オーナー裁定 2026-08-23、正本 `coding-rules/interior-mutability.md` / `coding-rules/command-query-separation.md` を新設。委任 8 で実装是正・本文同期済み）。）

### Validation Tool Results

| Tool / Check | Result | Interpretation |
|---|---|---|
| `bun .claude/tools/aidlc-sensor-traceability.ts --stage functional-design --output-path .../traceability.json` | `"pass":false`、`gaps:[]`、`orphans:[]`、`missing_from_table:[]`、`invalid_entries:[]`、`invalid_targets:[]`、`missing_from_upstream_ids` に FR1/FR2/…/FR9 系 36 件 | ブリーフの想定どおり構造的なノイズ（U3 は FR1.2/FR1.3/NFR3 のみ担当、`upstream_ids` はその 3 件のみを列挙）。実害となる `gaps`/`orphans`/`invalid_targets`/`invalid_entries` はすべて空 — 実質合格 |
| `bun .claude/tools/aidlc-sensor-required-sections.ts --stage functional-design --output-path <各 md>`（entities/rules/functional-spec の 3 回） | 3 ファイルとも `"pass":true`（H2 見出し 2 / 2 / 8 本） | 合格 |
| `grep -rnE 'WorkspaceLock\|FsWorkspaceLock\|LockProtocol\|LockIdentity\|reap_eligible\|OwnerStamp\|AcquireBudget\|LockGuard\|process_alive\|ProcessProbe\|audit_lock\|reap-decision-locality' modules tools scripts formal .github Cargo.toml`（BR3.1 の合格条件そのものを現状コードに事前実行） | 255 件・16 ファイル、すべて `entities.md` `RetiredLockMachinery` の列挙（use-case workspace mod / adapter fs_workspace_lock・process_probe / domain lock_protocol・lock_identity / infra-io process_probe / formal audit_lock.qnt / lint check.rs・赤例 / quint-gate.sh / テスト）で説明が付く | 退役対象の網羅性を確認 — 抜け漏れなし |
| `WorkflowExecution::version()` / `apply_event` / `start_with_entries` の実コード確認（`modules/core/domain/src/orchestration/workflow_execution.rs`） | doc コメント・実装・専用テストが「version は集約の遷移では不変、Repository が `with_version` で載せる」契約を明示 | 所見1（Critical）の直接根拠 |
| C3（`contract-summary.md`）の `EventStore` trait シグネチャと `entities.md` の対比 | `usize`（C3）↔ `u64`（`entities.md`） | 所見2（Major）の直接根拠 |
| `entities.md`/`rules.md`/`functional-spec.md` 全文検索 `mutex`/`refcell`/`cell<`/`interior`（大小無視） | 0 件 | 所見3（Major）の直接根拠 |
| `WorkflowExecutionSnapshot` の実フィールド数（`workflow_execution_snapshot.rs`） | 16 属性、`entities.md` の列挙（intent_id〜version）と完全一致 | 合格（BR5.2 / U2 pending-revision 9 の反映を確認） |
| C6 楽観 version 制約の genesis 経路（`contract-summary.md` `## Review` Major 所見1 — UPDATE-only では初回 store が誤って Conflict）の反映確認 | `rules.md` BR1.3 / BR2.3 が `expected == 0` の場合を `INSERT` 経路に分岐しており、この所見自体は解消済み | 合格（ただし所見1 の `aggregate.version()` 由来の別経路のバグが新たに存在） |

### Summary

上流成果物（unit-of-work・requirements・contract-summary・decisions・U2 code-summary/pending-revision）の読み込みと突合は丁寧で、retirement 対象の網羅性（BR3.1 grep 255 件・16 ファイルすべて説明可能）、traceability/required-sections センサーの合格、C6 genesis 経路（contract-design レビュー Major 所見1）の解消、`WorkflowExecutionState` の 16 属性の実コード一致などは高品質である。しかし本 Unit の中核機構である store の楽観バージョン制御（FR1.2 の主たる合格基準）が、既に実装済みの U2 `WorkflowExecution::version()` の実挙動（集約の遷移では不変、Repository が明示的に `with_version` で載せ替える）と矛盾する式（`aggregate.version() − 1`）に基づいており、記述どおり実装すると genesis で u64 アンダーフロー（panic/wrap）、以降の通常書込みは恒常的に偽陽性 Conflict になる（Critical 所見1）。加えて、承認済み契約 C3 との無言の型変更（Major 所見2）と、`&self`/`&mut self` の食い違いに対する内部可変性戦略の欠落（Major 所見3 — 契約レビューで名指しで申し送られていたにもかかわらず未対応）がある。Critical 1 件があるため advisory の閾値（Critical 0）を満たさず、NOT-READY と判定する。承認ゲートでは、特に所見1（バージョン算出式）を実装着手前に必ず是正するよう優先度高く扱われたい — U2 の code-generation で先例（上流設計の欠陥を実装エージェントが独自に発見・自力修正した経緯、`code-summary.md` `## Review` 所見1 参照）があり、同じパターンの再発（設計ゲートの欠陥を実装側が肩代わりする）は避けるべきである。
