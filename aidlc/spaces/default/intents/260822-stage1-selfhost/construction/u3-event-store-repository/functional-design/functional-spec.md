# functional-spec — U3 SQLite EventStore と WorkflowExecutionRepository（`u3-event-store-repository`）

> Functional Design（Construction 3.1）成果物（Unit: U3、kind: library、Bolt: B5）。出典: `entities.md` / `rules.md`（同ディレクトリ）、C3 / C6、ADR-001 / 003 /
> 006 / 007、Bolt B3 実装（`modules/core/domain/src/orchestration/`）、`functional-design-questions.md`（Q1〜Q4 = A）。コードは署名の要点だけ（設計ステージ）。

> ## ⚠ 部分失効（2026-08-27 / ADR-010・Bolt B6 — event-store-adapter-rs v2.0.0 へ乗り換え）
>
> 本書のうち **「自前の SQLite イベントストアを実装する」ことを前提にした記述は失効した**。
> 該当は §1 の `event_store_impl.rs` / `schema.rs` / `wire/` / `memory/in_memory_event_store.rs` 行、
> §2 のローカル `EventStore` / `EventStoreImpl` / `InMemoryWorkflowExecutionRepository` / `u64` 化、
> §3.1（store の SQL 手順）、§3.2 の 1〜3（find_by_id の SQL 手順と `with_version`）、
> §3.4（`within_write_transaction`）、§3.5（`PRAGMA user_version` によるスキーマ版検査）、
> §4（ワイヤ形式の全体）、§5 の ITF 再生先、§7 の一部。各所に個別の失効注記を入れてある。
>
> **失効していないもの**: ポート `WorkflowExecutionRepository` / `JournalReader` の面（§2 の該当行）、
> §3.3（投影の差分読取の手順）、§5 の Quint モデル `journal_protocol.qnt` そのもの（**1 文字も
> 変えずに通った** — 乗り換えの意味論的な検収）、§6 の退役チェックリスト（ADR-007 由来で B6 とは独立）。
>
> 現在の正は実装（`modules/core/use-case/src/orchestration/`・`modules/core/interface-adapter/src/orchestration/`）と
> [ADR-010](../../../inception/domain-design/decisions.md)、
> [C3 / C6](../../../inception/contract-design/contract-summary.md)、
> [developer-report-1 §6](../../esa-v2-migration/developer-report-1.md) /
> [developer-report-2 §8](../../esa-v2-migration/developer-report-2.md)。

## 1. 配置（クレート = 層）

| 層 / クレート | モジュール（private mod + ファサード `pub use`） | 内容 |
|---|---|---|
| `core-use-case::orchestration` | `workflow_execution_repository.rs` / ~~`event_store.rs`~~ / `journal_reader.rs` / `repository_error.rs` / ~~`event_store_error.rs`~~ → `journal_read_error.rs` / `corrupt_cause.rs` / `global_seq_nr.rs` / `projection_name.rs` | ~~ポート 3 本~~ → **ポート 2 本**（2026-08-27 / ADR-010: ローカル `EventStore` ポートは削除、正本は本家 crate）、エラー 2 型 + CorruptCause、値 2 型（BR1.x） |
| `core-use-case` | `workspace/`（mod ごと削除） | 退役（BR3.1） |
| `core-domain::orchestration` | `intent_id.rs`（UUIDv7）、`workflow_execution_state.rs`（旧 snapshot）、`state_error.rs`（旧 snapshot_error）、`workflow_execution.rs`（state / from_state） | BR4.1 / BR4.3 |
| `core-domain::workspace` | `intent_dir_name.rs`（新設）。`lock_protocol.rs` / `lock_identity.rs` 削除 | BR4.2 / BR3.1 |
| `core-interface-adapter::orchestration` | ~~`event_store_impl.rs`（+ `schema.rs` DDL 定数、`wire/event_wire.rs`、`wire/state_wire.rs`）~~ / ~~`memory/in_memory_event_store.rs`~~ / ~~`memory/workflow_execution_repository.rs`~~ → **すべて削除**（2026-08-27 / ADR-010・Bolt B6 — 自前ストア約 5,480 行の撤去）。現在は `store_path.rs`、`store_failure.rs`、`workflow_execution_repository_impl.rs`、`journal_reader_impl.rs` | BR2.x |
| `core-interface-adapter` | `clock.rs`（既存、Fake 付き）。`process_probe.rs` / `workspace/fs_workspace_lock.rs` 削除、`workspace/state_file_io.rs` は維持（U4） | BR2.6 / BR3.1 |
| `infra-io` | `process_probe.rs` 削除 | BR3.1 |
| `formal/orchestration` | `journal_protocol.qnt`（新）。`formal/workspace/audit_lock.qnt` 削除 | BR3.3 |
| `tests/conformance/fixtures` | `journal_protocol/*.itf.json`（新）。`audit_lock/` 削除 | BR3.5 |
| `tools/lint` | `reap-decision-locality` ルール削除（checkbox-vocabulary / no-public-fields は維持） | BR3.1 |
| 依存 | workspace: ~~`rusqlite = { version = "0.3x", features = ["bundled"] }`~~ → **`event-store-adapter-rs = "=2.0.0"`（`sqlite` feature）**（2026-08-27 / ADR-010。`rusqlite` は `JournalReaderImpl` の別接続用に adapter が直接持つ。バージョンは完全固定 — 本家スキーマに結合しているため）、`tokio = { version = "1", features = ["rt", "macros"] }`、`chrono`（本家 trait が `DateTime<Utc>` を要求 — NFR4.1 の再検討対象）、`serde`。adapter から `md5` 除去 | P6 |

ファサード（`pub use`）に旧名は残さない（module-visibility.md）。

## 2. ポートの形（C3 の具体化 — 差分のみ）

- `WorkflowExecutionRepository::{find_by_id(&self, &IntentId), store(&mut self, &WorkflowExecutionEvent, &WorkflowExecution)}`。`store` は `&mut self`（C3 も
  2026-08-24 のオーナー裁定で `&self` → `&mut self` へ**改訂済み** — `contract-summary.md` §C3、`pending-revision.md` #9。
  内部可変性の禁止に伴う変更で、正本は `coding-rules/interior-mutability.md` / `command-query-separation.md`）。
- ~~`EventStore<AID, A, E>` の 4 メソッド — C3 どおり（`version: u64`、`seq_nr: u64`）。Repository 実装は `persist_event_and_snapshot` / `get_latest_snapshot_by_id` /
  `get_events_by_id_since_seq_nr` を使う。~~
  → **失効（2026-08-27 / ADR-010）**: ローカル `EventStore` ポートは削除。正本は本家
  `event_store_adapter_rs::types::EventStore`（関連型 `AID` / `AG` / `EV`、数値は `usize`、エラーは
  `EventStoreWriteError` / `EventStoreReadError`）。Repository 実装が使うメソッドは同じ 3 本である。
- `JournalReader::{events_after(GlobalSeqNr), checkpoint(&ProjectionName), advance_checkpoint(&ProjectionName, GlobalSeqNr)}`。
  （2026-08-27 改訂: エラー型は `JournalReadError`。`GlobalSeqNr` の実体は**本家 `journal` 表の `rowid`** である）
- ~~`EventStoreImpl::open(path: StorePath, clock: C) -> Result<Self, EventStoreError>` / `within_write_transaction<T>(&mut self, f) -> Result<T, EventStoreError>`。~~
  → **失効（2026-08-27 / ADR-010）**: `EventStoreImpl` ごと削除。`within_write_transaction` は本家が接続も
  トランザクションも露出しないため実現できず、口ごと消えた（**登録簿の扱いは U7 で裁定**）。
- ~~`WorkflowExecutionRepositoryImpl { store: EventStoreImpl<C> }` — `EventStoreImpl<C>` を直接所有する（内部可変性は使わない、coding-rules/interior-mutability.md）。
  可変操作は `&mut self`。`event_store(&self) -> &EventStoreImpl<C>` / `event_store_mut(&mut self) -> &mut EventStoreImpl<C>` に分けて公開する。
  `InMemoryWorkflowExecutionRepository { store: InMemoryEventStore }` も同形。~~
  → **改訂（2026-08-27 / ADR-010）**: `WorkflowExecutionRepositoryImpl<S>`（`S` は本家 `EventStore` を満たす
  バックエンド）が**ストアを単一所有**する。`open()` が SQLite、`in_memory()` が本家 memory を選び、
  **実装コードは同一**。内部可変性を使わない方針（Query は `&self` / Command は `&mut self`）は不変。
  テストダブル型 `InMemoryWorkflowExecutionRepository` は存在しない。ストアを外へ貸す
  `event_store()` / `event_store_mut()` も無い（本家の口を素通しする必要が無くなったため）。
- ~~数値パラメータは u64（C3 の usize を実ドメイン型に合わせて具体化 — C3 の改訂提案を所有者 U5 / U6 へ申し送り）。~~
  → **失効・撤回（2026-08-27 / ADR-010）**: 本家の `usize` に戻した。借り物の契約を我々のドメイン型に
  合わせて書き換えていたこと自体が `coding-rules/upstream-contracts.md` 違反だった。
- `StorePath::for_space(aidlc_root: &Path, space: &SpaceName) -> StorePath` / `as_path()`。
- **（2026-08-27 追加）楽観 version はストアが採番する不透明トークン**（BR5.3 / ADR-010 追記 (1)）。
  `find_by_id` はストアが載せた値をそのまま保ち、`store` の期待値は `aggregate.version()` である。
  genesis（`Event::is_created()` が真）だけは Gateway が**ストアへ渡す写しにのみ**初期値 1 を載せる
  （呼出側の集約は動かない）。`Conflict` の `actual` は競合時に `get_latest_snapshot_by_id` を
  1 回読み直して得る（本家は整形済み文字列しか返さないため。文言解析はしない）。

## 3. フロー

### 3.1 store（BR1.3 / BR2.3）

> **失効（2026-08-27 / ADR-010・Bolt B6）** — 下記 1〜5 は**自前ストアの SQL 手順**であり、本家へ
> 乗り換えたことで我々の手順ではなくなった。現在の store は
> ①前提検査（`event.id().intent_id() == aggregate.id()`、`event.seq_nr() == aggregate.seq_nr()`、
> `event.seq_nr() >= 1`。**`aggregate.version() == event.seq_nr() - 1` の検査は削除** — version を
> `seq_nr` から導く前提そのものが BR5.3 で否定された）→ ②genesis なら写しに初期 version 1 を載せる
> → ③本家 `persist_event_and_snapshot(event, aggregate)` を 1 回呼ぶ、の 3 手である。
> イベント追記・スナップショット更新・楽観 version の CAS は**本家が同一 Tx で**行い、我々は接続も
> Tx も持たない。競合は本家が `OptimisticLockError` で返し、我々が `Conflict { expected, actual }` へ
> 写す（`actual` は競合時のみ `get_latest_snapshot_by_id` の読み直しで得る）。

**（以下 1〜5 は失効した自前 SQL 手順の履歴記録 — 上記バナー参照。現行手順はバナー内の①②③）**

1. ~~前提検査: `event.intent_id() == aggregate.intent_id()`、`event.seq_nr() == aggregate.seq_nr()`、`event.seq_nr() >= 1`、`aggregate.version() == event.seq_nr() - 1`
   （違えば `Corrupt(SequenceGap)` — 呼出側のバグ）。`expected = aggregate.version()`（find_by_id が `with_version` で載せた「永続化済みの最後の seq_nr」。
   `apply_event` は version を変えない — B3 実装契約）、`new_version = event.seq_nr()`。genesis は expected 0 / new_version 1。~~
2. ~~`BEGIN IMMEDIATE`。~~
3. ~~`INSERT INTO journal(aggregate_id, seq_nr, schema_version, event_type, payload, occurred_at)`。UNIQUE 違反 → rollback、`Conflict { expected, actual: 現在 version }`。~~
4. ~~`expected == 0` → `INSERT INTO snapshot(aggregate_id, version = new_version, seq_nr = new_version, schema_version, payload, updated_at)`（既存行があれば rollback + Conflict）。
   それ以外 → `UPDATE snapshot SET version = new_version, seq_nr = new_version, payload = ?, updated_at = ? WHERE aggregate_id = ? AND version = expected`。影響 0 行 →
   `SELECT version` で actual を読み rollback + `Conflict { expected, actual }`。~~
5. ~~`COMMIT`。Io 失敗は `Io { kind, path }`。~~

### 3.2 find_by_id（BR1.2）

> **部分失効（2026-08-27 / ADR-010・Bolt B6）** — 下記 1〜3 の **SQL 手順と `with_version` の記述は失効**。
> 現在は ①本家 `get_latest_snapshot_by_id(aid)` → `None` なら `NotFound`（journal を数えて `MissingSnapshot`
> を分ける判定は本家の口では書けないので、`Corrupt(MissingSnapshot)` の経路は縮んだ）→ ②本家
> `get_events_by_id_since_seq_nr(aid, snapshot.seq_nr)`（**その番号を含む**。自前 trait の doc は
> 「より後」と書いていたが本家は境界が 1 つ違う）→ ③自分自身の `seq_nr` 以下のイベントを読み飛ばして
> 順に `apply_event`、である。
> **`with_version` は削除された** — 復号はストアが載せた version をそのまま保つのであって、
> Repository が `seq_nr` から載せ直すことはしない（BR5.3）。
> スナップショット payload の復号も **serde がメメント（`WorkflowExecutionState`）を経由する**ので、
> `from_state()` の不変条件検査を必ず通る（オーナー裁定 2026-08-27）。ステップ 4（集約を返す）は不変。

**（以下 1〜3 は失効した自前 SQL 手順・`with_version` の履歴記録 — 上記バナー参照。ステップ 4 のみ現行）**

1. ~~`SELECT version, seq_nr, schema_version, payload FROM snapshot WHERE aggregate_id = ?`。無ければ journal を数え、0 なら `NotFound { intent_id }`、1 以上なら
   `Corrupt(MissingSnapshot)`。~~
2. ~~StateWire を復号（schema_version 検査）→ `WorkflowExecution::from_state(state)`（Err → `Corrupt(InvariantViolation)`）→ `with_version(snapshot.version)`。~~
3. ~~`SELECT … FROM journal WHERE aggregate_id = ? AND seq_nr > snapshot.seq_nr ORDER BY seq_nr` を復号して順に `apply_event`（Err → `Corrupt(SequenceGap | InvariantViolation)`）。
   replay ループ終了後、Repository が明示的に `with_version(最後に適用した seq_nr)` を載せる（`apply_event` は version を変えない）。通常運転では 0 件
   （スナップショットは毎 store 更新）。~~
4. 集約を返す。

### 3.3 投影の差分読取（BR1.4、利用は U4）

`checkpoint(name)` → `events_after(cp)` → 投影を描く → `advance_checkpoint(name, last_global)`。advance は単調。再生成時は行削除（別 API `reset_checkpoint` は本 Unit では作らない — U4 の設計）。

### 3.4 登録簿の直列化（BR2.4、利用は U7）

> **全面失効（2026-08-27 / ADR-010・Bolt B6）— 代替は未定、U7 で裁定する。**
> 本家 `EventStoreForSqlite` は `Connection` を内部保持し `from_connection` は private、`transaction()` も
> `persist_*` の内部でしか使われない（調査済み）。したがって**本家経由では BR2.4 を実現できない**ため、
> `within_write_transaction` は口ごと削除した。ADR-010 は「登録簿 `intents.json` をやめてジャーナルと同じ
> DB のテーブルにし、RMU の投影対象にする」を筋と書いている（リードモデルをコマンド側が Tx で守る構造
> 自体が CQRS の境界に反する — `coding-rules/cqrs-boundaries.md`）が、U7 の設計に踏み込むため
> **本 Bolt では裁定していない**。「解決済み」ではなく**未決**である。

~~`store.within_write_transaction(|tx| { read intents.json; mutate; atomic write; Ok(()) })` — Tx は `BEGIN IMMEDIATE` で開くため、同じ DB を開く別プロセスの store /
登録簿変更は busy_timeout 内で直列化される。`f` が Err なら rollback（ファイル書込は tmp+rename で原子的、DB 側の変更は無い）。~~

### 3.5 open / 初期化（BR2.1 / BR2.2）

> **失効（2026-08-27 / ADR-010・Bolt B6）** — 本家は `PRAGMA user_version` を使わないため、版の検査ごと
> 廃止した（`EventStoreError::Schema` 変種も削除）。現在の `open` は
> `EventStoreForSqlite::new(path)` を呼ぶだけで、表と索引は**本家が冪等に作る**（親ディレクトリは
> upstream の既存 `intents/` なので我々は作らない — 無ければ `Io { kind: NotFound }`）。
> 版の固定は `event-store-adapter-rs = "=2.0.0"` の完全固定 ＋ スキーマガードテストが担う。
>
> **`busy_timeout` は未決（U7 で裁定）**: 本家の接続には設定できない（接続を露出しないため）。
> SQLite 既定の 0ms なので、別プロセスの並行書込は待たずに `SQLITE_BUSY`（我々の写像では
> `Io { kind: WouldBlock }`）になる。従来は 5000ms 待っていたので、BR2.1 の実質的な後退である。
> 我々が開く `JournalReaderImpl` 側の接続には 5000ms を設定済み。単一プロセス前提の現状は受容し、
> **U7 の並行モデルと併せて再裁定**する。

~~`Connection::open(path)` → `PRAGMA busy_timeout = 5000` → `PRAGMA user_version` → 0: DDL（C6）を実行し `user_version = 1`；1: 何もしない；他: `Schema { found, supported: 1 }`。~~

## 4. ワイヤ形式（BR2.5、正準 JSON）

> **全面失効（2026-08-27 / ADR-010・Bolt B6）** — 自前のワイヤ構造体（`wire/event_wire.rs` /
> `wire/state_wire.rs`）はファイルごと削除した。ストアの payload は**本家が `serde_json::to_vec` で
> 書く**（我々のコードは呼んでいない）ので、下記の表は「そういう形で書く」という規範ではなくなった。
> 材料そのもの（各イベント型が何を載せるか）はドメイン型として残っており、下表は**参考の記録**として
> 残す。差分として明示すべき点は次の 4 つ:
>
> - **ストアの payload は契約 JSON（BR1.7 / canon-json）ではない**。契約 JSON の射程は upstream 観測面
>   （監査行・状態ファイル・directive）に限られる。この射程は coding-rules 正本への追記候補。
> - **封筒は列に出ない** — `payload` 1 列に封筒ごと serde で書かれる。復号時に列から
>   `WorkflowExecutionEvent::new` を組み立てる経路は無い。
> - **未知の変種・対応外の版の判別が消えた** — どちらも serde の復号失敗に畳まれ
>   `Corrupt(UndecodablePayload)` になる（`CorruptCause::UnknownEventType` / `SchemaVersion` は削除）。
>   `schema_version` フィールド自体はイベント型に残るが、復号時に値を検査する経路は無い。
> - **`StateWire` の値域検査（JSON の正確整数域 2^53 超の拒否）が無くなった**。ワイヤ構造体ごと
>   削除したためで、ストアファイルは upstream 非観測なので互換上の実害は無いと判断した。

### 4.1 イベント（journal.payload、`type` タグ）

| type | 材料（フィールド名: 型） |
|---|---|
| `Started` | `definition_id: string`, `definition_revision: string`, `scope: string`, `request: string`, `depth: string \| null`, `test_strategy: string \| null`, `stages: [{slug, phase, plan_action, conditional}]` |
| `StageCompleted` | `stage: string`, `next_stage: string \| null` |
| `GateOpened` | `stage: string`, `artifacts: [string]` |
| `GateApproved` | `stage: string`, `user_input: string \| null`, `next_stage: string \| null`, `phase_boundary: {from_phase: string, to_phase: string} \| null`（2026-08-27 訂正: pending-revision.md 項目 3 の裁定どおりの入れ子形に統一。実装 `PhaseBoundary { from_phase, to_phase }` の既定 serde 表現とも一致 — 新しい設計判断ではなく既存裁定への追従） |
| `GateRejected` | `stage: string`, `feedback: string \| null`, `revision_count: u32` |
| `StageRevised` / `StageSkipped` / `Parked` | `stage: string`（StageSkipped は `reason: string`, `next_stage: string \| null` も） |
| `Jumped` | `direction: string`, `source: string`, `target: string`, `stages_reset: [string]`, `stages_skipped: [string]` |
| `Unparked` | （材料なし `{}`） |
| `Recomposed` | `skipped: [string]`, `added: [string]`, `stages_in_scope: [string]` |
| `AutonomyModeSet` | `mode: string`（autonomous / gated） |

~~封筒の `intent_id` / `seq_nr` / `schema_version` / `occurred_at` は列に出す（payload には含めない）。復号時に列の値から `WorkflowExecutionEvent::new` を組み立てる。~~ → **失効**（2026-08-27 / ADR-010。封筒は payload に serde で同梱され、`intent_id` + `seq_nr` は `WorkflowExecutionEventId` にまとまった）

### 4.2 状態（snapshot.payload、~~16 属性~~ → **17 属性**。2026-08-27 / ADR-010 で `last_updated_at` を追加。数値は `u64` ではなく `usize`）

`intent_id`, `definition_id`, `definition_revision`, `stages: [{slug, phase, plan_action, conditional}]`, `plan: [string]`, `overlay: [string]`, `conditional: [bool]`,
`checkbox: [string]`（6 マーク）, `cursor: u64`, `status: string`（running / completed）, `parked_at: u64 \| null`, `autonomy: string`, `approved: [bool]`,
`revision_count: [u32]`, `seq_nr: usize`, `version: usize`, `last_updated_at`（2026-08-27 追加）。復号後は `from_state` の不変条件検査が最終防衛線であり、これは **serde 経路でも変わらない** — 集約の `Deserialize` は `#[serde(try_from = "WorkflowExecutionState")]` でメメントを経由するので `from_state()` の検査点を必ず通る（オーナー裁定 2026-08-27）。

## 5. 検証モデル `journal_protocol.qnt`（BR3.3 / BR3.4 / BR3.5）

- 定数: `WRITERS = 2`。状態: §rules BR3.3。`init`: journalLen = 0, snapVersion = 0, snapSeq = 0, checkpoint = 0, readModelSeq = 0, loadedVersion = 全 writer 0。
- `store_ok(w)` は genesis（snapVersion == 0 かつ loadedVersion[w] == 0）も同じ規則で扱う（expected 0）。
- 不変条件は状態遷移レベル（prev → current）で書く（`snapshot` アクションで prev を取る — audit_lock v2 と同じ型）。
- mutation（code-summary に記録）: 各 invariant につき 1 変異 — 例: store_conflict が journalLen を増やす変異 → conflict_rejected 違反、store_ok のガード除去 →
  no_lost_update 違反、catchup が checkpoint を減らす変異 → checkpoint_monotone 違反、catchup が readModelSeq を journalLen+1 にする変異 → truth_is_journal 違反 …。
- ITF: `quint run … --out-itf` で 6 シード以上採取、`#meta` 正規化済みでコミット。再生先は ~~InMemoryEventStore~~ → **`WorkflowExecutionRepositoryImpl` + `JournalReaderImpl`**（2026-08-27 改訂 / ADR-010）+ フェイク投影（adapter tests）。**モデルは 1 文字も変えずに通った**（乗り換えの意味論的な検収）。

## 6. 退役チェックリスト（BR3.1 / BR3.2）

use-case `workspace/` mod、adapter `workspace/fs_workspace_lock.rs` / `process_probe.rs`、domain `workspace/{lock_protocol,lock_identity}.rs` と `pub use`、
infra-io `process_probe.rs`、tests `fs_workspace_lock_test.rs` / `audit_lock_conformance.rs`、`formal/workspace/audit_lock.qnt`、`tests/conformance/fixtures/audit_lock/`、
`scripts/quint-gate.sh` の audit_lock ステップ（→ journal_protocol）、`tools/lint` の `reap-decision-locality`（ルール本体・HELP・赤例テスト・README の記述）、
adapter `Cargo.toml` の `md5`。grep（BR3.1）で 0 件を確認。

## 7. テスト設計（TDD、層ごと）

| 層 | Red（先に書く） | 内容 |
|---|---|---|
| Data model（use-case / domain） | 値型・エラー型 | GlobalSeqNr / ProjectionName / IntentId(UUIDv7) / IntentDirName の parse 受理・拒否（各 5〜8 本）、エラー Display の材料、`WorkflowExecutionState` 改名後の既存テスト緑 |
| Repository（adapter） | 契約テスト（ジェネリック） | ラウンドトリップ（start → 数コマンド → store × n → 新インスタンスで find_by_id → state が等しい）、NotFound、Conflict（2 再水和の競合）、Corrupt（MissingSnapshot / UndecodablePayload / ~~SchemaVersion~~ → **失効**、2026-08-27 / ADR-010 で変種ごと削除）、events_after の順序、checkpoint 単調性・未登録 = ZERO、~~within_write_transaction の直列化（同一 DB 2 接続、busy_timeout 内）~~ → **失効**（口ごと削除。U7 で裁定）。**追加（2026-08-27）**: 同じ契約テストを SQLite と本家 memory の**両バックエンド**に課す（実装が同一なので同じ約束が課せる — BR2.7）、および**スキーマガード**（本家 `journal` の DDL・一意索引・rowid 前提の実測突合） |
| Business logic（adapter） | ワイヤ | ~~PBT: 任意イベント / 状態の encode→decode 恒等、未知フィールド・未知 type の拒否、正準 JSON のバイト決定性~~ → **失効**（2026-08-27 / ADR-010: ワイヤ構造体ごと削除。直列化は本家の serde であり我々の検証対象ではない）。残るのはドメイン型の serde 往復と、**改竄した JSON が `from_state` の不変条件検査で弾かれること**の確認 |
| API（adapter / formal） | ITF + クラッシュ再構成 | journal_protocol fixtures の再生（全アクション網羅）、クラッシュ再構成（store 後に接続を捨て、新接続で find_by_id → 同一 state）、~~SQLite スキーマ突合（PRAGMA table_info = C6）~~ → **本家 DDL のスキーマガード突合**（2026-08-27 改訂 / ADR-010） |

既存スイート（engine_loop ITF、ゴールデン、WorkflowDefinitionRepository）は IntentId のリテラル置換と State 改名の追随のみ。

## 8. 未決・申し送り

- U4: `reset_checkpoint`（再生成）と投影の描画。U5: Conflict の 1 回再試行。
  U7: ~~`within_write_transaction` での birth / archive~~ → **登録簿（`intents.json`）の直列化機構そのものを
  U7 で裁定する**（2026-08-27 / ADR-010 — 口が消えたため代替が未定。ADR-010 は「登録簿を SQLite の
  テーブルへ移し RMU の投影対象にする」を筋と書いている）、`IntentDirName` の予約ラベル拒否。
- **（2026-08-27 追加）U7: `busy_timeout` の再裁定** — 本家の接続には設定できず、並行書込は待たずに
  `SQLITE_BUSY` になる。単一プロセス前提なら実害は無いので、U7 の並行モデルと併せて判断する。
- **（2026-08-27 追加）`Clock` / `SystemClock` / `FakeClock` は残置したが現在利用者がいない** —
  ストアが押印時刻をイベントの `occurred_at` から取るようになったため（本家の作法）。
  ユースケース着手時に注入シームとして使われる想定。
- 複数クローン間のジャーナル交換は後続 intent（P7）。
