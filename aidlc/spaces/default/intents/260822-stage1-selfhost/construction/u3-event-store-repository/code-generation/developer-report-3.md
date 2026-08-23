# developer-report-3 — 委任 3: SQLite ストア + Repository 実装 + 依存追加（U3 / Bolt B5）

> Code Generation（Construction 3.5）委任 3 の報告。計画 §5.3 Step 6〜8（Red → Green → Refactor）。
> ブランチ `bolt/b5-u3-event-store-repository`。`git add` / `git commit` は行っていない。

## §A Red の失敗出力（Step 6）

契約テストへの SQLite fixture 追加と新規テスト 3 ファイルを先に書き、実装が無い状態で
`cargo test -p core-interface-adapter` を実行した。4 つのテストターゲットすべてが
コンパイルに失敗する（型が存在しない）。

```
error[E0432]: unresolved imports `core_interface_adapter::orchestration::SqliteEventStore`,
              `core_interface_adapter::orchestration::StorePath`,
              `core_interface_adapter::orchestration::WorkflowExecutionRepositoryImpl`
  --> modules/core/interface-adapter/tests/workflow_execution_repository_contract.rs:76:5
   |
76 |     SqliteEventStore, StorePath, WorkflowExecutionRepositoryImpl,
   |     ^^^^^^^^^^^^^^^^  ^^^^^^^^^  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ no `WorkflowExecutionRepositoryImpl` in `orchestration`
   |     |                 |
   |     |                 no `StorePath` in `orchestration`
   |     no `SqliteEventStore` in `orchestration`
   |
help: a similar name exists in the module
   |
76 -     SqliteEventStore, StorePath, WorkflowExecutionRepositoryImpl,
76 +     SqliteEventStore, StorePath, WorkflowDefinitionRepositoryImpl,

（同じ E0432 が sqlite_event_store_test.rs:21 / workflow_execution_repository_impl_test.rs:18 /
  crash_reconstruction_test.rs:17 でも発生）

error: could not compile `core-interface-adapter` (test "crash_reconstruction_test") due to 1 previous error
error: could not compile `core-interface-adapter` (test "workflow_execution_repository_impl_test") due to 1 previous error
error: could not compile `core-interface-adapter` (test "sqlite_event_store_test") due to 1 previous error
error: could not compile `core-interface-adapter` (test "workflow_execution_repository_contract") due to 1 previous error
```

Green の途中で **2 度の実挙動の赤**を踏んでいる（どちらもテストが先に検出した）。

1. `updated_at` の期待値に置いた epoch ms 定数が誤り（`1_787_788_800_000` は 2026-08-27）。
   ISO 8601 整形の自前実装は正しく、テストの定数側が誤っていた。全ファイルで
   `1_787_443_200_000`（= 2026-08-23T00:00:00Z）へ訂正。

   ```
   ---- orchestration::sqlite_event_store::tests::the_epoch_is_rendered_as_iso_8601_utc stdout ----
   assertion `left == right` failed
     left: "2026-08-27T00:00:00Z"
    right: "2026-08-23T00:00:00Z"
   ```

2. SQL 定数を Rust の `\`+改行（行継続）で折り返していたため、**次行の先頭空白まで
   食われて SQL が壊れていた**（`"UPDATE snapshot\` + 改行 + ` SET …` → `UPDATE snapshotSET …`）。
   genesis の INSERT だけは偶然妥当な SQL になるため、2 件目の書込で初めて露見した。

   ```
   ---- a_new_connection_after_a_crash_reconstructs_the_same_aggregate stdout ----
   2 件目: Io { kind: Other, path: Some(".../intents/.aidlc-store.sqlite") }
   ```

   行継続をやめ、実改行を含む素の複数行リテラルへ全 SQL 定数を書き換えて解消。

## §B 実装概要（Step 7）

### 新規ファイル（プロダクトコード）

| ファイル | 中身 |
|---|---|
| `modules/core/interface-adapter/src/orchestration/schema.rs` | C6 の DDL 定数（逐語）と `ensure_schema`（`PRAGMA user_version` の検査・初期化）。インラインテスト 5 本 |
| `modules/core/interface-adapter/src/orchestration/store_path.rs` | `StorePath::for_space` / `as_path`。インラインテスト 4 本 |
| `modules/core/interface-adapter/src/orchestration/sqlite_event_store.rs` | `SqliteEventStore<C>`（`EventStore` + `JournalReader` + `within_write_transaction`）、SQL 定数、エラー写像、ISO 8601 整形。インラインテスト 9 本 |
| `modules/core/interface-adapter/src/orchestration/workflow_execution_repository_impl.rs` | `WorkflowExecutionRepositoryImpl<C>`（`RefCell<SqliteEventStore<C>>`） |

### 公開面（`core_interface_adapter::orchestration` のファサードに追記）

```rust
pub use sqlite_event_store::SqliteEventStore;
pub use store_path::StorePath;
pub use workflow_execution_repository_impl::WorkflowExecutionRepositoryImpl;
```

- `StorePath::for_space(aidlc_root: &Path, space: &SpaceName) -> StorePath` / `as_path()`。
  導出先は `<root>/spaces/<space>/intents/.aidlc-store.sqlite`（BR2.1）。生の `PathBuf` を
  受け取る口は置いていない（場所は導出するものであって渡すものではない）。
- `SqliteEventStore::<C>::open(StorePath, C) -> Result<Self, EventStoreError>`、
  `open_with_busy_timeout(StorePath, C, Duration)`、`path() -> &StorePath`、
  `within_write_transaction<T, F>(&mut self, F) -> Result<T, EventStoreError>`
  （`F: FnOnce(&rusqlite::Transaction<'_>) -> Result<T, EventStoreError>`、同期）。
- `WorkflowExecutionRepositoryImpl::<C>::new(SqliteEventStore<C>)`、`event_store() -> SqliteEventStore<C>`。

### 内部可変性（追補の裁定どおり）

`SqliteEventStore<C>` は **共有ハンドル**である。

```rust
pub struct SqliteEventStore<C> {
    path: StorePath,
    connection: Rc<RefCell<Connection>>,
    clock: Rc<C>,
}
```

`Clone` は手実装で、同じ接続と同じ時計を指すハンドルを返す（`C: Clone` を要求しない —
`FakeClock` は `Clone` ではないため `Rc<C>` を採った）。`Debug` も手実装（場所だけを描く）。
`WorkflowExecutionRepositoryImpl` は `RefCell<SqliteEventStore<C>>` を持ち、
`let store = self.store.borrow().clone();` で借用を閉じてから `await` する
（`clippy::await_holding_refcell_ref` は発火しない）。`Rc` なので `Send` ではない（設計どおり）。

### Tx の手順（BR2.3 / functional-spec §3.1）

`persist_event_and_snapshot(event, aggregate)`:

1. `expected = aggregate.version()`、`new_version = event.seq_nr()`。ワイヤ符号化は Tx の外
   （符号化の失敗で書込ロックを取らない）。`updated_at` は `Clock::now_ms()` から ISO 8601 UTC。
2. `conn.transaction_with_behavior(TransactionBehavior::Immediate)` で `BEGIN IMMEDIATE`。
3. journal INSERT。`UNIQUE (aggregate_id, seq_nr)` 違反（`ErrorCode::ConstraintViolation`）なら
   `SELECT version` で `actual` を読み `Conflict { expected, actual }`。`Transaction` を drop
   （= rollback）して返す。
4. `expected == 0` → snapshot INSERT（主キー違反なら同じく `Conflict`）。
   それ以外 → `UPDATE … WHERE aggregate_id = ? AND version = expected`。影響 0 行なら
   `SELECT version` で `actual` を読んで `Conflict`。
5. ここまで通った経路だけが `commit()`。

`persist_event(event, version)` は「ジャーナル追記のみ + `version` を楽観前提として検査」
（委任 2 §C-5 の InMemory と同じ意味論。スナップショット行は一切書かない）。

`within_write_transaction` は同じ `BEGIN IMMEDIATE` で `f` を包み、`f` が `Err` なら
`commit()` せずに抜ける（drop で rollback）。

### スナップショット payload の version（追補の裁定）

`aggregate.clone().with_version(new_version).state()` を符号化するので、列と payload の
`version` が一致する。**InMemory（`memory/in_memory_event_store.rs`）も同じ規則に揃えた**
（追補で許可された変更）。インラインテスト 1 本を
`the_snapshot_row_carries_the_new_version_even_though_the_payload_predates_it` →
`the_snapshot_row_and_its_payload_both_carry_the_new_version` に改名し、payload 側の
`"version":1` も検査するよう拡張した。

### テスト（新規）

| ファイル | 本数 | 中身 |
|---|---|---|
| `tests/workflow_execution_repository_contract.rs`（追記） | +12 | 委任 2 の契約 12 関数を `SqliteFixture` で実行（`sqlite_` 接頭辞。既存の InMemory 12 本は無改変） |
| `tests/sqlite_event_store_test.rs` | 27 | open / 初期化 / `user_version` 2 → `Schema` / 親 dir 欠落 → `Io(NotFound)`、`PRAGMA table_info` による C6 の 3 表突合（列名・型・NOT NULL・PK）と `AUTOINCREMENT` / `UNIQUE (aggregate_id, seq_nr)`、書込 3 経路と rollback、差分読取・global 順序、チェックポイント、`within_write_transaction` の COMMIT / rollback / 他接続締め出し、Busy → `Io(WouldBlock)` と解放後の直列化、破損 3 種、`StorePath` |
| `tests/workflow_execution_repository_impl_test.rs` | 10 | `NotFound`、replay 0 件 / 1 件以上の `version()`、`Corrupt` の 4 原因（MissingSnapshot / UndecodablePayload / SchemaVersion / UnknownEventType）+ replay 中の `SequenceGap`、前提検査 2 本。破壊はすべて生の SQL |
| `tests/crash_reconstruction_test.rs` | 5 | store × 5 → 接続 drop → 新接続で同一 state / 全ジャーナル / COMMIT 前の Tx は残らない / 開き直しの冪等 / 続きの `seq_nr` から書ける |

インラインテスト（`#[cfg(test)]`）は `schema.rs` 5・`store_path.rs` 4・`sqlite_event_store.rs` 9。
`sqlite_event_store.rs` のインラインは白箱でしか観測できないもの（`busy_timeout` の実値 5000、
`journal_mode` が既定のまま、`Clone` が同じ接続を指すこと、ISO 8601 整形、rusqlite の
`ErrorCode` → `ErrorKind` 写像、桁溢れ）に限った。

## §C 依存（版・`cargo audit`）

- `Cargo.toml`（workspace）: `rusqlite = { version = "0.40.2", features = ["bundled"] }` を追加
  （`cargo search` 実測の最新安定版）。`modules/core/interface-adapter/Cargo.toml` に
  `rusqlite = { workspace = true }`。`tokio` は委任 2 が dev-dependency として追加済みなので触っていない。
- `Cargo.lock` 実効版: `rusqlite 0.40.2` / `libsqlite3-sys 0.38.2`（同梱ビルド）。
- **ホストターゲットで実際にコンパイルされる推移依存**（`cargo tree -e normal --target <host>` 実測）:
  `bitflags 2.13.1` / `fallible-iterator 0.3.0` / `fallible-streaming-iterator 0.1.9` /
  `hashlink 0.12.1` / `hashbrown 0.17.1`（既存）/ `libsqlite3-sys 0.38.2` / `smallvec 1.15.2`。
  ビルド依存として `cc` / `pkg-config` / `vcpkg` / `find-msvc-tools`。
  `Cargo.lock` に増える `thiserror` / `wasm-bindgen*` / `js-sys` / `sqlite-wasm-rs` / `rsqlite-vfs` は
  **wasm ターゲット専用**であり、本リポジトリのビルドには入らない（coding-rules の
  「thiserror 不使用」に抵触しない — 自分のエラー型は従来どおり手実装 enum のまま）。
- `cargo audit`: **exit 0**（advisory DB 1225 件、workspace 100 crates を走査、脆弱性 0）。
  `cargo audit --file tools/lint/Cargo.lock` も 5 crates で 0 件。

## §D 判断（設計に無い / 裁量のある選択）

1. **親ディレクトリの存在検査を `Connection::open` の前に明示する**。SQLite の `SQLITE_CANTOPEN`
   に頼ると原因が「親 dir 欠落」か「権限」かを区別できないため、`path.parent()` が
   ディレクトリでないときは接続を開かずに `Io { kind: NotFound }` を返す。ファイルも作らない
   （テストで確認）。相対パスで parent が空文字列になる場合は検査しない。
2. **`Io { path }` は常にストアファイルのパス**にした（親 dir 欠落時も欠落した親ではなく
   ストアパス）。`EventStoreError::Io.path` の rustdoc が「対象パス」と定義しているため。
3. **ISO 8601 整形は自前の純関数**（`format_iso8601_utc` + Hinnant の `civil_from_days`）。
   `chrono` / `time` を足さない（NFR4.1 依存最小化）。秒精度・UTC 固定・ミリ秒は切り捨て。
   1970-01-01 / 2026-08-23 / 閏日 2024-02-29 / 2024-12-31T23:59:59 を固定テスト。
4. **競合時の `actual` は rollback 前に Tx 内で読む**。設計 §3 は「rollback 後に `SELECT version`」
   と書くが、どちらの競合経路でも snapshot 表はまだ変更されていないので観測値は同じであり、
   Tx を保持したまま読むほうが接続の往復が 1 回減る。
5. **`UPDATE snapshot` の SET に `schema_version` を含めた**（BR2.3 の SET 一覧には無い）。
   現状 `StateWire::SCHEMA_VERSION` は 1 固定なので**観測上の差はゼロ**だが、将来版を上げたとき
   「payload は新版・列は旧版」という静かな破損経路ができるため塞いだ。→ §E-2 で確認を求める。
6. **`open_with_busy_timeout` を公開面に追加**（§E-1）。
7. **非集約行の `Corrupt` 材料は `"-"`（`NO_AGGREGATE`）**。チェックポイント行や global 通番の
   桁溢れには「行が名乗った集約識別子」が存在しない。投影名を `aggregate_id` 欄に入れると
   型の意味（`EventStoreError::Corrupt.aggregate_id` = 集約識別子の生文字列）が濁るので、
   `Display` が欠落材料に使う綴りと同じ `"-"` を置いた（投影名の材料は落ちる — 委任 2 §E-3 と同型）。
8. **`ErrorCode` → `ErrorKind` の写像**: `DatabaseBusy` / `DatabaseLocked` → `WouldBlock`、
   `CannotOpen` / `NotFound` → `NotFound`、`PermissionDenied` / `ReadOnly` /
   `AuthorizationForStatementDenied` → `PermissionDenied`、`DatabaseCorrupt` / `NotADatabase`
   → `InvalidData`、`OperationInterrupted` → `Interrupted`、その他 → `Other`
   （`DiskFull` に対応する安定 `ErrorKind` が無いため `Other`）。
9. **`ensure_schema` は DDL と `PRAGMA user_version` の刻印を 1 つの Tx に閉じる**。
   途中で落ちても「版 0・表なし」に戻り、次回 open が最初からやり直せる。
   知らない版（1 以外の非 0、負値を含む）のときは**表を一切作らずに** `Schema` を返す。
10. **`journal_mode` は既定（`delete`）のまま**。WAL は `-wal` / `-shm` を増やし、逸脱台帳 # 4 の
    パスが 1 本で済まなくなる（BR2.1 どおり）。インラインテストで固定した。
11. **`C` に依らないヘルパは自由関数に置いた**（`current_version` / `insert_journal_row` /
    `decode_event` / `decode_snapshot`）。ジェネリック impl に置くと呼出側が
    `SqliteEventStore::<C>::` の turbofish を書くことになり読みにくいため（Refactor で移動）。
    ジャーナル追記の結末は `Result<(), ()>` ではなく `enum JournalInsert { Appended, Conflicted }`。

## §E 設計質問

1. **`open_with_busy_timeout` を公開面に足した**。設計（entities.md / functional-spec §2）は
   `open(path, clock)` だけを挙げている。追加した理由は、`busy_timeout` 超過の
   `Io(WouldBlock)` を**現実的な時間で**観測するには、待たされる側の接続の timeout を
   短くするしかないためである（既定の 5000ms のままだと 1 本で 5 秒かかる）。
   `open` は `DEFAULT_BUSY_TIMEOUT = 5000ms` に委譲するので BR2.1 は満たす。
   公開 API を増やすのが不可なら、この口を `pub(crate)` に落として当該テストを
   インライン `#[cfg(test)]` へ移すか、5 秒待つテストに書き換える必要がある。裁定を請う。
2. **`UPDATE snapshot` の SET に `schema_version` を含めてよいか**（§D-5）。BR2.3 の逐語からは
   外れるが、含めないと将来のワイヤ版上げで静かな破損経路が残る。BR2.3 側を改訂するのが
   筋だと考えるが、契約の所有は設計側なので確認したい。
3. **`SqliteEventStore` という型名が coding-rules `gateway-taxonomy.md` §5 と緊張する**。
   §5 は「`Fs` / `Sys` / `Postgres` のような技術接頭辞は使わない — 格納形式は実装の内部詳細」
   と定める。本 Unit の設計（entities.md / functional-spec §1 / 計画 §2）はこの名前を明示して
   おり、その上位の `WorkflowExecutionRepositoryImpl` は技術接頭辞なしで正しく命名されている
   （= ポート面には格納形式が出ない）ため**設計どおりの名前を採用した**。ただし
   `SqliteEventStore` はクレートのファサードから `pub use` されるので、`Sqlite` が公開 API に
   出ている。coding-rules 側に「§5 は Repository 実装の規則であり、下位の `EventStore`
   実装には及ばない」旨の但し書きを足すか、型名を変えるか、どちらかで整合を取りたい。
4. **`within_write_transaction` が `rusqlite::Transaction` を公開面に露出させる**。設計どおりの
   署名だが、これにより `core-interface-adapter` の利用者（U7 の登録簿処理）は `rusqlite` を
   直接名指しすることになる。閉包の引数を自前の薄いラッパ型に包む選択肢もあるが、
   「登録簿の read-modify-write を同じ Tx で走らせる」という用途では素の `Transaction` が
   必要になるはずなので、設計どおりにしてある。U7 の設計時に再確認されたい。
5. **`persist_event` の `version` 検査**は委任 2 の裁量（§C-5）を踏襲した。BR2.3 の逐語
   （「(1) のみ」）とは差があるので、両実装が同じ意味論であることを記録に残しておく。

## §F 検査結果

| 検査 | 結果 |
|---|---|
| `cargo test -p core-interface-adapter` | **全緑 193 本**（lib 89 / contract 24（InMemory 12 + SQLite 12）/ sqlite_event_store 27 / repository_impl 10 / crash_reconstruction 5 / definition_impl 27 / golden_parity 9 / journal_protocol_conformance 1 / append_only_symlink 1）。3 回連続で同一結果（不安定なテストなし） |
| `cargo test --workspace` | **全緑 623 本 / 0 失敗** |
| `cargo clippy --workspace --all-targets -- -D warnings` | **緑**（警告 0） |
| `cargo fmt --all --check` | **緑** |
| `cargo lint`（`tools/lint` カスタム） | **緑**（exit 0、出力なし） |
| `cargo audit` | **緑**（脆弱性 0 / 100 crates）。`tools/lint/Cargo.lock` も 0 件 |
| `bash scripts/coverage.sh`（絶対ゲート） | **PASS** — head line coverage **96.78%** ≧ 床 90.0% |
| 委任 6 の先取り確認 `-W clippy::indexing_slicing -W clippy::panic` | 本委任の新規・変更ファイルでの検出 **0 件**（プロダクト・テストとも） |

プロダクトコードに `unwrap` / `expect` / `panic!` / 添字アクセスは書いていない。
フィールドはすべて private + アクセサ、mod は private + ファサードの `pub use`、
エラーは材料のみ（文言なし）、`Clock` は注入、`std::env` の読取なし、ログ出力なし。

## §G 変更したファイル

新規（プロダクト 4 / テスト 3）:

```
modules/core/interface-adapter/src/orchestration/schema.rs
modules/core/interface-adapter/src/orchestration/sqlite_event_store.rs
modules/core/interface-adapter/src/orchestration/store_path.rs
modules/core/interface-adapter/src/orchestration/workflow_execution_repository_impl.rs
modules/core/interface-adapter/tests/crash_reconstruction_test.rs
modules/core/interface-adapter/tests/sqlite_event_store_test.rs
modules/core/interface-adapter/tests/workflow_execution_repository_impl_test.rs
```

変更:

```
Cargo.toml                                                     # workspace deps に rusqlite
Cargo.lock                                                     # rusqlite / libsqlite3-sys ほか
modules/core/interface-adapter/Cargo.toml                      # rusqlite = { workspace = true }
modules/core/interface-adapter/src/orchestration/mod.rs        # mod と pub use の追記
modules/core/interface-adapter/src/orchestration/memory/in_memory_event_store.rs
                                                               # payload version の揃え（追補の許可）+ 該当インラインテストの改名・拡張
modules/core/interface-adapter/tests/workflow_execution_repository_contract.rs
                                                               # SQLite fixture と sqlite_* 12 本の追記（既存 InMemory 呼出は無改変）
```

所有外（`modules/core/use-case/**`、`modules/core/domain/**`、`wire/`、`formal/**`、
`tests/conformance/**`、`scripts/**`、`docs/**`、計画・検査手順・質問票）には触れていない。
`git add` / `git commit` は行っていない。`.claude/` のツールは実行していない。

## §H 未了（本委任の範囲外・後続へ）

- Quint `journal_protocol.qnt` と ITF 準拠（委任 4）。`tests/journal_protocol_conformance.rs` は
  既にコミット済みで、本委任の変更を入れた状態でも緑である。
- 仕様・正本の同期（委任 5）。特に functional-spec §4.1 の `phase_boundary` 行（`string | null`
  → 入れ子オブジェクト）と、本報告 §E-2 / §E-3 の 2 点。
- `indexing_slicing` / `panic` の lint 昇格と既存コードの是正（委任 6）。本委任の新規コードは
  昇格後も無警告であることを先取りで確認済み。
- C3 の `usize` → `u64` 改訂の申し送り（所有者 U5 / U6）は本委任でも触れていない。
- `scripts/coverage.sh` の**相対ゲート**（`--base`）は未実行（base ref の採取はコンダクタの
  Step 0 / Step 14）。絶対ゲートのみ確認した。
