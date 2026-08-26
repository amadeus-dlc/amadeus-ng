# developer-report-2 — 委任 2: ストア差し替えと自前実装の削除（乗り換え本体）

Conversation language: 日本語。ブランチ `bolt/b6-esa-v2-conformist`。**未コミット**（`git add` /
`commit` / `push` は行っていない）。

## 1. 一文サマリ

永続化を本家 `EventStoreForSqlite` / `EventStoreForMemory` に切り替え、自前ストア一式
（`event_store_impl.rs` / `schema.rs` / ローカル `EventStore` trait / in-memory ストア /
ワイヤ形式 3 ファイル、実測 **5,480 行**）を削除。横断読取（本家 `journal` の rowid カーソル）+
自前 checkpoint 表 + スキーマガードを `JournalReaderImpl` として新規実装した。
**Quint モデル `journal_protocol.qnt` は 1 文字も変えていない**——同じ ITF トレースが本家の
ストア上でそのまま再生できることを実測した（§4）。検査 7 種すべて緑（テスト **593** 全緑、
カバレッジ絶対 98.43% / 相対ゲート PASS）。

## 2. オーナー裁定 2 件の実装

### 2.1 (A) serde 復号は memento 経由

`WorkflowExecution` に `#[serde(into = "WorkflowExecutionState", try_from = "WorkflowExecutionState")]`
を付け、`From<WorkflowExecution> for WorkflowExecutionState`（= `state()`）と
`TryFrom<WorkflowExecutionState> for WorkflowExecution`（= `from_state()`）を実装した。
`WorkflowExecutionState` に `Serialize` / `Deserialize` を derive。

したがって**直列化形式の正本は 17 属性の memento** になり、復号は必ず `from_state` の検査点
（`check_invariants`）を通る。security-design §2 の検査点 3（集約不変条件）が本家の
バックエンド越しでも生き残る。

TDD の Red を実測してから実装した（`cargo test -p core-domain --lib
a_tampered_serialised_aggregate_is_refused` → `不変条件を破る写しは復号できない: WorkflowExecution {...}`
で FAILED）。恒久化したテストは 2 本:

| 場所 | テスト | 固定している内容 |
| --- | --- | --- |
| `domain/.../workflow_execution.rs` | `a_tampered_serialised_aggregate_is_refused` | `"cursor":0` → `"cursor":99` に書き換えた JSON が `from_str` で **Err**（`invariant violation`） |
| `interface-adapter/tests/workflow_execution_repository_impl_test.rs` | `a_snapshot_that_breaks_an_aggregate_invariant_is_refused_by_the_decoder` | 同じ改竄を**本家の `snapshot` 行に生 SQL で**入れ、`find_by_id` が `Corrupt(UndecodablePayload)` を返す（通し確認） |

### 2.2 (B) version はストアが採番する不透明トークン

`version == seq_nr − 1` 系の前提検査を**全廃**した。除去した箇所は §5 に全列挙。

## 3. 差し替えの中身

### 3.1 Repository は 1 実装・ストアだけが型引数

```rust
pub struct WorkflowExecutionRepositoryImpl<S> { store: S, location: Option<StorePath> }

impl WorkflowExecutionRepositoryImpl<EventStoreForSqlite<..>> { pub fn open(path: &StorePath) -> .. }
impl WorkflowExecutionRepositoryImpl<EventStoreForMemory<..>> { pub fn in_memory() -> .. }
impl<S: Clone> WorkflowExecutionRepositoryImpl<S> { pub fn reopened(&self) -> Self }
impl<S: EventStore<AID=IntentId, AG=WorkflowExecution, EV=WorkflowExecutionEvent>>
    WorkflowExecutionRepository for WorkflowExecutionRepositoryImpl<S> { .. }
```

`InMemoryWorkflowExecutionRepository`（自前のテストダブル）は**削除**した。本家の memory
バックエンドを内包すれば実装コードが 1 行も分岐しないので、テストダブルを別に書く理由が
消えたためである。契約テストは同じ関数群を 2 バックエンドに流す形で維持した（BR2.7）。

### 3.2 利用作法は本家 example の実測に従った

`gh api` で `examples/user-account-sqlite/` と `lib/src/event_store_for_sqlite.rs` /
`event_store_for_memory.rs` / `generic_event_store.rs` を取得して実測した。従った点:

| 事項 | 本家の実測 | 我々の実装 |
| --- | --- | --- |
| シリアライザ | 既定（`JsonEventSerializer` / `JsonSnapshotSerializer` = `serde_json::to_vec`） | **既定のまま**。差し替えない（`with_*_serializer` は使わない） |
| genesis / update の呼び分け | `GenericEventStore` が `Event::is_created()` で分岐。`create` は CAS せず**渡された集約の `version()` をそのまま初期スロット値に記録**、`update` は `WHERE version = expected` の CAS を通して `expected + 1` を記録 | 同じ。呼ぶのは `persist_event_and_snapshot` 1 本 |
| 差分読取の境界 | `seq_nr >= n`（**その番号を含む**） | `find_by_id` は `aggregate.seq_nr() + 1` を渡す（example と同じ） |
| genesis の version | example の `UserAccount::new` は **`version: 1`** で作る | 集約は `version = 0`（= 未永続）のまま。**Gateway が genesis のときだけ**、ストアへ渡す写しに `FIRST_STORED_VERSION = 1` を載せる（§9 の設計質問 A で理由と代案を書いた） |

### 3.3 `JournalReaderImpl`（新規・301 行 + テスト 409 行）

- 同一 DB ファイルへ**自前の別接続**（`Connection::open`）。本家は接続を露出しないので別接続に
  するしかない（ADR-010 決定 4）。`busy_timeout` は既定 5000ms、試験用に
  `open_with_busy_timeout` を持つ。
- カーソルは本家 `journal` の **`rowid`**。`GlobalSeqNr(u64)` に包む。payload は本家と同じ
  形式（`serde_json::from_slice::<WorkflowExecutionEvent>`）で復号する。
- **checkpoint は自前表** `amadeus_projection_checkpoint(projection TEXT PRIMARY KEY,
  last_global_seq INTEGER NOT NULL)`。`CREATE TABLE IF NOT EXISTS` で冪等に作る。
  単調・後退拒否（`CheckpointRegression`）・同値 no-op を契約テストで固定。
- `open` は本家の `journal` 表が**まだ無ければ `Io { kind: NotFound }`** で止まる。本家が
  所有する表を我々が先に作らない（DDL の正本を 2 か所にしない）。

### 3.4 スキーマガード（中身）

`journal_reader_impl.rs` のインラインテスト 2 本。本家のストアを一時ファイルに開いてから
`sqlite_master` を読み、**ピン留めした期待値と逐語比較**する。

```
the_upstream_journal_schema_is_the_pinned_one
  SELECT sql FROM sqlite_master WHERE type='table' AND name='journal'
    == "CREATE TABLE journal (\n  pkey TEXT NOT NULL,\n  skey TEXT NOT NULL,\n  aid TEXT NOT NULL,\n
        seq_nr INTEGER NOT NULL,\n  payload BLOB NOT NULL,\n  occurred_at INTEGER NOT NULL,\n
        PRIMARY KEY (pkey, skey)\n)"
  SELECT sql FROM sqlite_master WHERE type='index' AND name='journal_aid_seq_nr_idx'
    == "CREATE UNIQUE INDEX journal_aid_seq_nr_idx ON journal (aid, seq_nr)"
  失敗時の文言: 「本家スキーマが変わった。event-store-adapter-rs の =2.0.0 固定を見直せ」

the_journal_table_keeps_a_rowid_so_the_cursor_is_well_defined
  SELECT count(*) FROM journal WHERE rowid >= 0   -- WITHOUT ROWID 表なら SQL 自体が落ちる
```

2 本目を分けたのは、カーソルの前提が「列構成」ではなく **rowid の存在**そのものだからである
（`WITHOUT ROWID` 化は列定義を変えずに前提を壊せる）。

### 3.5 エラー型

- ローカル `EventStore` ポートの削除に伴い `EventStoreError` を **`JournalReadError` へ改名**
  （`JournalReader` の口だけが返す）。変種は `Io` / `Corrupt` / `CheckpointRegression` の 3 種。
  `Conflict`（書込の関心）と `Schema`（`PRAGMA user_version` の関心）は削除した。
- `CorruptCause` は `JournalReadError` と `RepositoryError` の共有語彙なので独立ファイル
  `corrupt_cause.rs` へ。`UnknownEventType` / `SchemaVersion` は到達不能になったので削除
  （§8-3 / §8-4 に理由）。
- **本家エラーからの写像は Gateway に置いた**（`workflow_execution_repository_impl.rs`）。
  use-case 層に本家依存を入れないため、`RepositoryError::from_event_store` は削除した。

| 本家 | 我々 | 備考 |
| --- | --- | --- |
| `EventStoreWriteError::OptimisticLockError(String)` | `RepositoryError::Conflict { expected, actual }` | 文言は解析しない。`expected` は渡した集約の `version()`、`actual` は**ストアを読み直して**得る |
| `EventStoreWriteError::SerializationError` | `Corrupt(UndecodablePayload)` | |
| `EventStoreWriteError::IOError(Box<dyn Error>)` | `Io { kind, path }` | `Box` を `rusqlite::Error` へ downcast して `SQLITE_BUSY → WouldBlock` を保つ（NFR3.5） |
| `EventStoreWriteError::OtherError(String)` | `Io { kind: Other, path }` | |
| `EventStoreReadError::DeserializationError` | `Corrupt(UndecodablePayload)` | |
| `EventStoreReadError::IOError` / `OtherError` | `Io { .. }` | 同上 |

`std::io::ErrorKind` への分類は `store_failure.rs`（新規・42 行）に**1 か所へ集約**した
（Repository 面と `JournalReader` 面で綴りが割れないため）。

## 4. Quint モデルは無改変で通った（本委任のいちばん重い検収）

`formal/orchestration/journal_protocol.qnt` は**未変更**（`git status` に出ない）。ITF 準拠
テストを本家 SQLite ストア + `JournalReaderImpl` へ載せ替えて、コミット済み 6 フィクスチャ全数を
再生し全ステップで射影が一致した。

不変条件 `version_equals_journal`（`snapVersion == journalLen`）が成立するのは、
§3.2 の「genesis の初期スロット値を 1 にする」設計を採ったからである。集約の `version = 0`
のまま渡すと初期スロットが 0 になり、以後 `snapVersion = journalLen − 1` と 1 ずれてモデルと
食い違う。**この 1 点だけがモデル無改変の分かれ目**だったので、設計質問 A として §9 に挙げた。

射影の変更点は 2 つだけ:

- `snapVersion` / `snapSeq` を本家 `get_latest_snapshot_by_id` から読む（自前ストアの 3 表を
  直接見ていたのを置換）。
- `store_ok` の直後、writer は `find_by_id` で**握り直す**（新しい version を知るのはストア
  だけになったため。従来は `set_version(event.seq_nr())` で自前採番していた — 裁定 (B)）。

`loadedVersion` の射影規則（writer が握る集約の `version()`）と、`store_conflict` の材料
（`Conflict { expected: prev.loadedVersion, actual: prev.snapVersion }`）は**そのまま**通った。

## 5. version 結合を除去した全箇所

| # | 場所 | before | after |
| --- | --- | --- | --- |
| 1 | `workflow_execution_repository_impl.rs::check_preconditions` | `if aggregate.version() != event.seq_nr() - 1 { Err(SequenceGap) }` | **削除**。残すのは識別子一致・`event.seq_nr() == aggregate.seq_nr()`・`seq_nr >= 1`（ドメインの関心のみ） |
| 2 | `memory/workflow_execution_repository.rs::check_preconditions` | 同上 | ファイルごと削除 |
| 3 | `workflow_execution_repository_impl.rs::find_by_id` | replay 中に `version = event.seq_nr()` を追い、最後に `aggregate.set_version(version)` | **削除**。ストアが載せた値をそのまま保つ |
| 4 | `memory/workflow_execution_repository.rs::find_by_id` | 同上 | ファイルごと削除 |
| 5 | `event_store_impl.rs::persist_event_and_snapshot` | `new_version = event.seq_nr()` を列と payload の両方に書く | ファイルごと削除（本家が採番する） |
| 6 | `in_memory_event_store.rs::persist_event_and_snapshot` | 同上 | ファイルごと削除 |
| 7 | `use-case/.../workflow_execution_repository.rs` の doc | 「永続化済みの最後の `seq_nr` を楽観 version として載せる」「期待 version は `aggregate.version()`（= `event.seq_nr() - 1`）」 | 「ストアが載せた値をそのまま保つ」「不透明トークン」に改稿 |
| 8 | 同ファイルの `FakeRepository::store` | `stored.set_version(event.seq_nr())` | `stored.set_version(aggregate.version() + 1)`（ストア採番の模擬）。テスト名も `the_version_a_rehydration_carries_is_the_one_the_store_assigned` へ改名 |
| 9 | `tests/support/mod.rs::advanced(aggregate, event)` | `set_version(event.seq_nr())` | **削除**し、`store_and_reload(repo, event, agg)`（書いてから `find_by_id` で握り直す）に置換 |
| 10 | `tests/journal_protocol_conformance.rs::Writer::commit` | `aggregate.set_version(event.seq_nr())` | `commit(stored)`（`find_by_id` の結果を握り直す） |
| 11 | 契約テスト `genesis_expects_version_zero` | 「genesis の書込は版 0 を前提にする」 | `the_store_assigns_the_first_version_on_genesis` へ改名し、「呼出側の集約は動かない・採番したのはストア」を固定 |
| 12 | 契約テスト `sequence_gap_is_refused` | 版 0 のまま次を書くと `Corrupt(SequenceGap)`（= version 前提検査が捕まえていた） | `a_write_from_a_stale_version_conflicts` へ置換（**`Conflict` が正しい分類**）。`seq_nr` の前提検査は `a_sequence_that_disagrees_with_the_aggregate_is_refused` で別途固定 |
| 13 | `round_trip` の assert 文言 | 「版は永続化済みの最後の seq_nr」 | 「5 回の書込ぶんストアが採番した版」 |

数値としての `version` は単一集約なら書込回数と一致するため、既存の契約テストの**期待値
（1 / 5 / `Conflict{expected:5, actual:6}` など）はすべて無改訂で通った**。変えたのは「何を
根拠にその値になるか」の記述と、version を前提に使っていた検査だけである。

## 6. 削除の実測

### 6.1 削除したファイル（10 本 = 5,480 行。2026-08-27 訂正: 表題を「8 本」としていたが、下表の実測は 10 本で、直後の注記も当初から「ファイル 10 本」としていた — 表題側の誤記を実測に合わせて修正）

| ファイル | 行数 | 削除理由 |
| --- | --- | --- |
| `interface-adapter/src/orchestration/event_store_impl.rs` | 1,015 | 自前 SQLite ストア。本家 `EventStoreForSqlite` が置換 |
| `interface-adapter/src/orchestration/schema.rs` | 179 | C6 の 3 表 DDL と `PRAGMA user_version`。表は本家が作る |
| `interface-adapter/src/orchestration/memory/in_memory_event_store.rs` | 739 | 自前 in-memory ストア。本家 `EventStoreForMemory` が置換 |
| `interface-adapter/src/orchestration/memory/workflow_execution_repository.rs` | 141 | in-memory 専用 Repository。実装が 1 本になったので不要 |
| `interface-adapter/src/orchestration/wire/mod.rs` | 503 | 自前ワイヤ形式（封筒列 + 正準 JSON payload）。本家は payload 1 列に serde を書く |
| `interface-adapter/src/orchestration/wire/event_wire.rs` | 835 | 同上 |
| `interface-adapter/src/orchestration/wire/state_wire.rs` | 607 | 同上 |
| `use-case/src/orchestration/event_store.rs` | 231 | 本家と同形のローカル `EventStore` trait（ADR-006 の写し） |
| `interface-adapter/tests/event_store_impl_test.rs` | 1,026 | 上記実装の統合テスト |
| `interface-adapter/tests/in_memory_workflow_execution_repository_test.rs` | 204 | 同上（in-memory 側） |

（ファイル 10 本・合計 5,480 行。ADR-010 が見積もった「約 2,400 行」は
`event_store_impl` + `schema` + その統合テスト + ローカル trait の 4 本を数えたもので、
実際にはワイヤ形式 3 本と in-memory 2 本も一緒に消えた。）

### 6.2 差分の総計（`modules/` 配下）

```
28 files changed, 1153 insertions(+), 6617 deletions(-)   → 正味 −5,464 行
```

新規追加（4 ファイル・1,186 行、うちテスト 769 行）:
`journal_reader_impl.rs` 710（product 301 / test 409）、`store_failure.rs` 120（42 / 78）、
`tests/journal_reader_impl_test.rs` 295、`corrupt_cause.rs` 61。

## 7-1. テスト数の増減内訳（689 → 593）

`cargo test --workspace -- --list` のテスト名を HEAD（委任 1 のコミット）と突き合わせた実測。
**削除 176 / 追加 80 / 正味 −96**。

### 削除 176 本 — すべて削除・置換したモジュールのもの

| 出自 | 本数 | 削除理由 |
| --- | --- | --- |
| `wire/state_wire.rs` インライン | 20 | ワイヤ形式ごと削除（PBT 1 本を含む） |
| `wire/event_wire.rs` インライン | 16 | 同上（PBT 1 本を含む） |
| `wire/mod.rs` インライン | 8 | 同上 |
| `memory/in_memory_event_store.rs` インライン | 14 | 自前 in-memory ストアごと削除 |
| `event_store_impl.rs` インライン | 13 | 自前 SQLite ストアごと削除 |
| `schema.rs` インライン | 5 | 自前 DDL / `user_version` ごと削除 |
| use-case `event_store.rs` インライン | 6 | ローカル `EventStore` trait ごと削除 |
| use-case `event_store_error.rs` インライン | 8 | `JournalReadError` へ改名・変種削減（5 本を新ファイルへ、2 本を `corrupt_cause.rs` へ移設） |
| use-case `repository_error.rs` インライン | 5 | `from_event_store`（写像）を削除したためその 5 本が対象を失った |
| use-case `workflow_execution_repository.rs` インライン | 1 | 改名 1 本（§5-8） |
| `tests/event_store_impl_test.rs` | 33 | ファイルごと削除 |
| `tests/in_memory_workflow_execution_repository_test.rs` | 6 | ファイルごと削除。うち 5 本は SQLite 側と**同名**で、baseline に 2 本ずつあったものが 1 本ずつに減っている（`a_store_without_any_row_reports_not_found` / `a_journal_without_a_snapshot_is_corrupt_not_missing` / `a_gap_in_the_replayed_journal_is_corrupt` / `a_replayed_event_naming_a_stage_outside_the_plan_is_corrupt` / `a_journal_row_with_an_unknown_event_type_is_corrupt`。実測: base=2 → now=1） |
| `tests/workflow_execution_repository_impl_test.rs` | 7 | 改名 2・置換 2・削除 2・Reader ハンドル 1（下表） |
| 契約テストのマクロ生成（in-memory 15 + sqlite 15） | 30 | 契約関数を 10 本へ整理（reader 系 5 本は `journal_reader_impl_test.rs` へ移設） |
| 契約テストの実装固有 4 本 | 4 | 「開いた後の書込が見えるか」の 4 本。Reader が Repository から独立したので `the_reader_observes_writes_made_after_it_was_opened` 1 本へ集約 |

`workflow_execution_repository_impl_test.rs` の 11 本の内訳（2026-08-27 訂正: 表題を「9 本」としていたが、
下表を実測すると個々のテスト名は 11 本ある — 1 行に複数名を "/" で束ねた行（本表の 1 行）を展開して数えた。
この 11 は「この 1 ファイルで言及した旧テスト名の総数」であり、§7-1 冒頭の削除 176 本の内訳表で
このファイルの寄与分として使った「7」（改名 2・置換 2・削除 2・Reader ハンドル 1）とはスコープが異なる —
「7」は 176 本の総計に加算した**純粋な削除本数**、「11」は本節が言及する**残置（同名で残った）テスト名も
含めた全件**である。表題側の数字を実測に合わせて修正した）:

（テスト名は同名重複を含む多重集合として突き合わせているので、上の 2 行の内訳は
「どちらのファイルに属していたか」ではなく「消えた本数」で数えている。）

| 旧テスト | 処置 |
| --- | --- |
| `the_version_after_a_read_without_replay_is_the_last_persisted_sequence` | 改名 → `..._is_the_one_the_store_assigned`（裁定 B） |
| `the_version_after_a_replay_is_the_sequence_of_the_last_applied_event` | 改名 → `a_replay_does_not_move_the_version_the_store_assigned`（裁定 B） |
| `a_stale_aggregate_is_refused_before_the_transaction_opens` | 置換 → 契約テスト `a_write_from_a_stale_version_conflicts`（分類が `Conflict` に変わった） |
| `an_event_of_another_aggregate_is_refused` | 置換 → 契約テスト `mismatched_identity_is_refused` + インライン `an_event_of_another_aggregate_fails_the_precondition` |
| `a_snapshot_written_by_another_wire_version_is_corrupt` | **削除**（`schema_version` 列が本家スキーマに無い — §8-4） |
| `a_journal_row_with_an_unknown_event_type_is_corrupt` | 残置（同名。原因が `UnknownEventType` → `UndecodablePayload` に変わった — §8-3） |
| `a_journal_without_a_snapshot_is_corrupt_not_missing` / `a_gap_in_the_replayed_journal_is_corrupt` / `a_replayed_event_naming_a_stage_outside_the_plan_is_corrupt` / `a_store_without_any_row_reports_not_found` | SQLite 側は**残置**（同名）。消えたのは in-memory 側の写しだけ |
| `the_repository_hands_out_a_reader_over_the_same_store` | **削除**（Repository は Reader を配らなくなった — 別オブジェクト） |

### 追加 80 本

| 場所 | 本数 | 内容 |
| --- | --- | --- |
| `journal_reader_impl.rs` インライン | 16 | スキーマガード 2・rowid 前提 1・open 4・失敗経路 7・復号 2 |
| `workflow_execution_repository_impl.rs` インライン | 8 | 前提検査 3・エラー写像 2・競合材料 1・場所 1・`reopened` 1 |
| `journal_read_error.rs` インライン | 5 | 材料の描画（旧 `event_store_error` から変種削減して移設） |
| `store_failure.rs` インライン | 4 | `ErrorKind` 分類の全腕 + downcast |
| `corrupt_cause.rs` インライン | 2 | 原因分類の描画（旧 `event_store_error` から移設） |
| `repository_error.rs` インライン | 1 | 材料欠落（`-`）の描画 |
| use-case `workflow_execution_repository.rs` インライン | 1 | §5-8 の改名先 |
| domain `workflow_execution.rs` インライン | 1 | **改竄 JSON の拒否**（裁定 A） |
| domain `checkbox.rs` インライン | 4 | マーカー閉集合の往復・文法外の行の拒否ほか（ワイヤ PBT が副次的に踏んでいた経路を直接固定し直した） |
| `tests/journal_reader_impl_test.rs`（新規） | 10 | 横断読取 4・チェックポイント 5・改竄 1 |
| `tests/workflow_execution_repository_impl_test.rs` | 8 | 改名 2・memento 通し確認 1・場所 2・その他 3 |
| 契約テストのマクロ生成（memory 10 + sqlite 10） | 20 | 契約関数 10 本 × 2 バックエンド |

**削除されたテストはすべて「削除・改名・置換した対象のもの」であり、生き残ったコードの
テストを落とした箇所は無い。**

## 7-2. 検査結果（実測値）

| 検査 | 結果 |
| --- | --- |
| `cargo fmt --all --check` | **PASS**（無出力） |
| `cargo clippy --workspace --all-targets -- -D warnings` | **PASS**（警告 0、`allow` の追加なし） |
| `cargo lint` | **PASS**（exit 0） |
| `PROPTEST_RNG_SEED=20260823 cargo test --workspace` | **PASS — 593 passed / 0 failed** |
| `bash scripts/quint-gate.sh` | **PASS — all steps green**（typecheck 3 / invariants 3 / witness 9 / `quint test r_.*` 1）。**モデルは無改変** |
| `cargo audit` | **PASS**（exit 0、120 crate、脆弱性 0） |
| `cargo audit --file tools/lint/Cargo.lock` | **PASS**（exit 0、5 crate、脆弱性 0） |
| `PROPTEST_RNG_SEED=20260823 bash scripts/coverage.sh --base origin/main` | **PASS** — 絶対 `98.43104%` ≥ 90.0%、相対 `98.43104%` ≥ base `98.39615%` − 0.01 |

`Cargo.lock` の差分は 2 行（`core-interface-adapter` から `proptest` が外れ、
`event-store-adapter-rs` に `rusqlite` が入った）。**新規パッケージは 0**（本家の `sqlite`
feature が使う rusqlite 0.40.2 は我々のものと同一で統合される）。

### カバレッジについての申し送り

初回計測は **相対ゲート FAIL**（head 97.83% < base 98.40%）だった。原因を分けて実測した:

- **委任 1 の時点（HEAD）で既に 98.33%** — origin/main の 98.396% から 0.06pp 下がっていた。
  委任 1 は coverage を検査項目に含めていない（developer-report-1 §10 に無い）ため未検出。
- 本委任で新規に書いた `JournalReaderImpl` / Repository のエラー経路が未到達だったぶんが
  残り 0.5pp。

対処として、削除ではなく**テストを足して**回復させた（§7-1 の追加 80 本のうち 33 本が
この目的を兼ねる）。実装側の是正も 1 件行っている: `io_kind` の写像が Repository 面と
`JournalReader` 面に**二重実装**になっていたのを `store_failure.rs` へ 1 本化した。

## 8. 記述が実態と食い違う箇所（コンダクタへの申し送り）

**コードは直したが、設計文書は触っていない**（同期はコンダクタ担当）。委任 1 の
developer-report-1 §6 の 8 件は依然として有効で、以下はそれに**追加**される分である。

| # | 文書 | 現在の記述 | 実態 |
| --- | --- | --- | --- |
| 1 | C6（格納形式） | 3 表 `journal` / `snapshot` / `checkpoint`。`journal` は `global_seq_nr INTEGER PRIMARY KEY AUTOINCREMENT` / `aggregate_id` / `seq_nr` / `schema_version` / `event_type` / `payload(TEXT)` / `occurred_at(TEXT)`、`UNIQUE(aggregate_id, seq_nr)` | **本家のスキーマ**に置換。`journal(pkey, skey, aid, seq_nr, payload BLOB, occurred_at INTEGER)` + `UNIQUE INDEX (aid, seq_nr)`、`snapshot(pkey, skey, aid, seq_nr, version, payload BLOB, last_updated_at INTEGER)`。**我々の表は `amadeus_projection_checkpoint` の 1 つだけ** |
| 2 | C6 / BR2.1（版の検査） | `PRAGMA user_version` に版を刻み、知らない版は `Schema` で拒否 | **廃止**。本家は `user_version` を使わない。代替はバージョンの完全固定（`=2.0.0`）+ スキーマガードテスト（§3.4）。`EventStoreError::Schema` 変種も削除 |
| 3 | C5（イベント封筒のワイヤ） | 封筒は列（`event_type` / `schema_version` / `occurred_at`）、payload は正準 JSON。未知 `type` は `Corrupt(UnknownEventType)` | payload 1 列に**封筒ごと serde で書く**（本家の既定シリアライザ）。未知の変種は serde の復号失敗 = `Corrupt(UndecodablePayload)` に畳まれる。`CorruptCause::UnknownEventType` は削除 |
| 4 | C6（`schema_version` 列） | 行ごとに版を持ち、対応外は `Corrupt(SchemaVersion)` | **列が無い**。`CorruptCause::SchemaVersion` は削除。イベント型の `schema_version` フィールド自体は残っているが、復号時に値を検査する経路は無くなった |
| 5 | BR1.7 / ADR 0001 決定 5（契約 JSON） | 直列化は canon-json の 1 経路 | ストアの payload は**本家が `serde_json::to_vec` で書く**。我々のコードは呼んでいない（`clippy.toml` の `disallowed-methods` にも触れない）ので規則違反ではないが、「ストアの payload は契約 JSON ではない」という射程を正本に書いたほうがよい |
| 6 | C6（`StateWire` の値域検査） | スナップショット payload は JSON の正確整数域（2^53）を超える値を拒否 | **検査が無くなった**（ワイヤ形式ごと削除）。ストアファイルは upstream 非観測なので実害は無いと判断したが、記述は残っている |
| 7 | BR2.1（`busy_timeout`） | 同一ホストの並行 CLI は 5000ms の範囲で直列化される | **本家の接続に `busy_timeout` を設定できない**（接続を露出しない）。SQLite 既定の 0ms なので、並行書込は待たずに `SQLITE_BUSY` を返す。§9 の設計質問 C |
| 8 | BR2.4（`within_write_transaction`） | 登録簿の read-modify-write を同じ Tx で守る | **口ごと削除**（本家経由では実現できない — ADR-010 が調査済み）。U7 の裁定待ち |
| 9 | 10-orchestration / C3 | `InMemoryWorkflowExecutionRepository` がテストダブル | **削除**。`WorkflowExecutionRepositoryImpl::in_memory()` が本家 memory バックエンドを内包する（実装コードは SQLite と同一） |
| 10 | C3（ローカル `EventStore` ポート） | Repository 実装が内部で使う下位ポート | **削除**。Repository は本家の `EventStore` を型引数で受ける |
| 11 | entities.md（`EventStoreError`） | `Conflict` / `Io` / `Corrupt` / `Schema` / `CheckpointRegression` の 5 変種 | **`JournalReadError`** に改名し `Io` / `Corrupt` / `CheckpointRegression` の 3 変種。`CorruptCause` は 4 分類（`MissingSnapshot` / `UndecodablePayload` / `InvariantViolation` / `SequenceGap`） |
| 12 | C6（`checkpoint` 表） | `projection` / `last_global_seq` / `updated_at` | `amadeus_projection_checkpoint(projection, last_global_seq)`。**`updated_at` を落とした**（誰も読んでいない列で、押印には時計が要るため）。Repository も `Clock` を持たなくなった |
| 13 | NFR3.1 / C3（`Clock` の注入） | ストアが `Clock` から押印時刻を取る | ストアは時刻を `event.occurred_at()` から取る（本家の作法）。`Clock` は**ユースケース側の注入シーム**として残っているが、現時点で利用者はいない |

## 9. 設計質問（裁定が要るもの）

### A. genesis の初期 version を Gateway が載せている（**要確認・重要**）

本家の `create_event_and_snapshot` は**渡された集約の `version()` をそのまま初期スロット値に
記録する**（CAS はしない）。本家サンプルはこれに合わせて genesis 集約を `version = 1` で作る。

我々は「未永続 = 0」という集約側の表現を保ちたかったので、**Gateway が genesis のときだけ**
写しに `set_version(1)` を載せている（呼出側の集約は動かない）。代案と評価:

| 案 | 中身 | 評価 |
| --- | --- | --- |
| **(a) 現状** | Gateway が create 経路でのみ `1` を載せる | Quint モデル無改変で通る。`Conflict` の `expected` が呼出側の版（0）になり材料が正しい。ただし「+1 の魔法」が Gateway に 1 か所ある |
| (b) 本家サンプルどおり | `start_from_plan_unchecked` が `version = 1` で作る | 本家に最も忠実。だが ITF の `loadedVersion` 初期値（モデルは 0）と食い違い、**未書込 genesis writer の衝突材料が `expected == actual == 1` という無意味な値になる** |
| (c) 集約 0 のまま渡す | 初期スロット = 0 | `snapVersion = journalLen − 1` になり **`version_equals_journal` が破れる**（モデル改訂が必要） |

(a) を採ったのは、(b) が衝突材料を壊し、(c) がモデル改訂を要求するためである。
**オーナー/コンダクタの確認が要る**（裁定 (B)「version はストアが採番する」の解釈として
Gateway が初期値を決めるのは許容範囲か）。

### B. `Conflict` の `actual` をストアの読み直しで得ている

本家は競合を整形済み文字列 1 本（`optimistic lock failed, aid=..., expected_version=N[,
actual_version=M]`）で返す。文言を解析するのは脆いので、**競合が起きたときだけ
`get_latest_snapshot_by_id` を 1 回追加で読んで** `actual` を作った。

- 長所: 本家の公開 API しか使わない。スキーマにも文言にも結合しない。
- 短所: 競合パスで 1 回余分に読む。読み直しと競合のあいだに第三者が書けば `actual` が
  1 つ先の値になりうる（材料の精度の問題であり、判定の正しさには影響しない）。

代案は「本家に構造化エラー（`OptimisticLockError { expected, actual }`）を提案する」で、
上流貢献の候補として挙げておく。

### C. `busy_timeout` が設定できない（**BR2.1 の実質的な後退**）

本家 `EventStoreForSqlite` は接続を露出せず、`busy_timeout` も設定しない。したがって
**別プロセスの並行書込は待たずに `SQLITE_BUSY`**（我々の写像では `Io { kind: WouldBlock }`）
になる。従来は 5000ms 待っていた。

選択肢: (a) 受け入れてユースケース側で `WouldBlock` を再試行する / (b) 本家へ
`with_busy_timeout`（または `PRAGMA` を打つ口）を提案する / (c) 我々の別接続で
`PRAGMA busy_timeout` を打つ（**効かない** — 設定は接続ごとなので本家の接続には及ばない）。
(a) か (b)。単一プロセス前提なら実害は無いので、U7 の並行モデルと併せて裁定したい。

### D. `within_write_transaction` の消失と BR2.4

ADR-010 が調査済みのとおり本家経由では実現できないため、口ごと削除した。ADR-010 は
「(b) 登録簿を SQLite へ移す」が筋と書いており、U7 と併せた裁定待ち。**本委任では触っていない**。

### E. コーディング規則の正本への追記（委任 1 から持ち越し + 1 件）

- 委任 1 の §7 D（`IntentId::value()` と `as_str()` の並立）/ §7 E（`thiserror` が推移依存）は
  未処理のまま。
- 追加: **BR1.7 の射程**（「契約 JSON」はストアの payload を含まない）を正本に一行足したい。
  本家が `serde_json::to_vec` で書くので、我々の規則の射程を明示しないと読み手が混乱する。

**正本ファイルは一切触っていない。**

## 10. 未了 / 次へ

- **設計文書の同期**（委任 1 §6 の 8 件 + 本報告 §8 の 13 件）— コンダクタ担当。
- **設計質問 A〜E の裁定** — 特に A（genesis の初期 version）は実装の根拠なので早めに確認したい。
- **U7（BR2.4 / 登録簿）** — ADR-010 (b) の裁定待ち。本委任のスコープ外。
- `Clock` / `SystemClock` / `FakeClock` は残置したが**現在利用者がいない**（ストアが時刻を
  イベントから取るようになったため）。ユースケース着手時に使われる想定。
- `audit-events` / `directive-schema` が `core-interface-adapter` の未使用依存になっている
  （**委任 1 以前からの既存状態**で、本委任が作ったものではない）。掃除の要否は別途。
- `.github/workflows/ci.yml` / `scripts/**` / `docs/**` / `formal/**` / `.claude/**` /
  `.coderabbit.yaml` は**一切触っていない**。`git add` / `commit` / `push` も行っていない。
