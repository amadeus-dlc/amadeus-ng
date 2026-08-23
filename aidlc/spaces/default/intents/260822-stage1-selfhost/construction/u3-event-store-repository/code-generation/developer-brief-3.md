# developer-brief-3 — 委任 3: SQLite ストア + Repository 実装 + 依存追加（U3 / Bolt B5）

Conversation language: 日本語（コメント・rustdoc・報告はすべて日本語。識別子・固定トークンは英語）。

## 役割と範囲

あなたは aidlc-developer-agent。Unit **u3-event-store-repository**（Bolt B5）の委任 3。リポジトリルート `/Users/j5ik2o/orca/workspaces/amadeus-ng/docs`、ブランチ
`bolt/b5-u3-event-store-repository`（委任 1・2 はコミット済み: ポート / エラー / 値（use-case）、`wire/`、`memory/`、契約テスト `tests/workflow_execution_repository_contract.rs`）。
委任 4（formal / conformance）と委任 5（docs）が**並行**して走る — 所有外のファイルには触れない。**コーディング規則の正本 `aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/`
（README + 7 ルール）を最初に読む。**

所有ファイル: `modules/core/interface-adapter/src/orchestration/{sqlite_event_store.rs,schema.rs,store_path.rs,workflow_execution_repository_impl.rs}`（新規）、
`modules/core/interface-adapter/src/orchestration/mod.rs`（`mod` と `pub use SqliteEventStore / StorePath / WorkflowExecutionRepositoryImpl` の追記）、
`modules/core/interface-adapter/tests/{sqlite_event_store_test.rs,workflow_execution_repository_impl_test.rs,crash_reconstruction_test.rs}`（新規）、
`modules/core/interface-adapter/tests/workflow_execution_repository_contract.rs`（SQLite 実装の呼出を追記するだけ — 既存の InMemory 呼出は変えない）、
`Cargo.toml`（workspace root: `[workspace.dependencies]` に `rusqlite = { version = "<最新安定>", features = ["bundled"] }` を追加。既存行は変えない）、
`modules/core/interface-adapter/Cargo.toml`（`rusqlite = { workspace = true }` 追加）、`Cargo.lock`、報告 `developer-report-3.md`（新規）。

触らないもの: `modules/core/use-case/**`、`modules/core/domain/**`、`wire/` `memory/`（読取・利用のみ）、`formal/**`、`tests/conformance/**`、`scripts/**`、`docs/**`、
計画・検査手順・質問票。`git add` / `git commit` はしない。`.claude/` のツールは実行しない。

## 先に読むもの（順に）

1. `.../u3-event-store-repository/code-generation/code-generation-plan.md`（§1 数値型、§2、§5.3 Step 6〜8、§7）
2. `.../u3-event-store-repository/functional-design/rules.md`（BR1.2 / BR1.3 / BR1.4 / BR2.1〜BR2.4 / BR2.6 / BR2.8）、`functional-spec.md`（§3.1〜§3.5 の手順、§4 ワイヤ）、
   `entities.md`（SqliteEventStore / StorePath / JournalRow / SnapshotRow / CheckpointRow / WorkflowExecutionRepositoryImpl）
3. `.../u3-event-store-repository/nfr-design/security-design.md`（§2 検査点の前段・3 段、§3 原子性・競合・Busy、§5）、`.../nfr-requirements/security-requirements.md`
   （NFR2.2 / NFR3.1〜3.5 / NFR4.1〜4.6）
4. `.../inception/contract-design/contract-summary.md` C6（DDL を逐語で使う）
5. 既存コード: 委任 2 の成果（`modules/core/use-case/src/orchestration/*.rs`、adapter `orchestration/{wire,memory}/**`、`tests/support/**`、契約テスト）、
   `modules/core/interface-adapter/src/clock.rs`（`Clock` trait / `SystemClock` / `FakeClock` — `now_ms`）、`modules/core/domain/src/workspace/space_name.rs`、
   `modules/core/interface-adapter/src/orchestration/workflow_definition_repository_impl.rs`（既存 Impl の書き方・エラー写像の様式）。

## 作業（計画 Step 6〜8、TDD）

### Step 6 — Red
- 契約テスト群（`tests/support/contract.rs`）を SQLite 実装で実行する呼出を `tests/workflow_execution_repository_contract.rs` に追加（`tempfile::tempdir()` 配下に
  `spaces/default/intents/` を作って `StorePath::for_space`、`FakeClock`）。
- `tests/sqlite_event_store_test.rs`: open / 初期化（新規 DB → `user_version` 1 と 3 テーブル、既存 1 → OK、`user_version` 2 → `Schema { found: 2, supported: 1 }`、
  親 dir 欠落 → `Io { kind: NotFound }`）、`PRAGMA table_info` / `sqlite_master` で C6 の列・型・制約（`UNIQUE(aggregate_id, seq_nr)`、PRIMARY KEY、AUTOINCREMENT）を突合、
  `busy_timeout` が 5000 であること、`persist_event`（snapshot 不変）、`persist_event_and_snapshot` の genesis / 通常 / Conflict（UNIQUE 違反・version 不一致の両経路で
  rollback を確認 — journal に行が残らない）、`get_events_by_id_since_seq_nr` の順序、`events_after` の global 昇順、`checkpoint` / `advance_checkpoint`、
  `within_write_transaction`（f が Ok → COMMIT、Err → rollback、2 接続: 片方が Tx 中は他方が待ち、`busy_timeout` 内に直列化 / 短い timeout の別接続で
  `Io(WouldBlock)` を観測）、rusqlite `Error` → `EventStoreError` 写像（`Busy` → `Io(WouldBlock)`、`SqliteFailure(Constraint)` → `Conflict`、I/O → `Io`、復号失敗 → `Corrupt`）。
- `tests/workflow_execution_repository_impl_test.rs`: Impl 固有 — `Corrupt(MissingSnapshot)`（journal 行あり・snapshot 削除）、`Corrupt(UndecodablePayload)`（payload 改竄）、
  `Corrupt(SchemaVersion)`、`NotFound`、前提検査 `Corrupt(SequenceGap)`、`find_by_id` 後の `version()` = 最後の seq_nr（replay 0 件 / 1 件以上の両方 — 後者は snapshot を
  古いものに差し替えて再現）。
- `tests/crash_reconstruction_test.rs`: store × n → 接続 drop → 新接続で `find_by_id` が同一 state、`events_after` が全件。
- 失敗出力を報告に記録。

### Step 7 — Green
- `schema.rs`: C6 の DDL を定数で（逐語）、`user_version` の検査 / 初期化関数。
- `sqlite_event_store.rs`: `SqliteEventStore::open(path: StorePath, clock: C)`（`Connection::open` → `busy_timeout(5000ms)` → user_version）、`EventStore` / `JournalReader` の実装
  （Tx は `BEGIN IMMEDIATE`: `conn.transaction_with_behavior(TransactionBehavior::Immediate)`、成功経路だけ `commit()`、drop = rollback）、BR2.3 の手順（expected =
  aggregate.version()、new_version = event.seq_nr()、事前検査 → journal INSERT → snapshot INSERT / UPDATE … WHERE version = expected → 影響 0 行なら SELECT actual →
  Conflict）、`within_write_transaction<T>(&mut self, f: impl FnOnce(&rusqlite::Transaction) -> Result<T, EventStoreError>)`、`updated_at` は `Clock::now_ms` から
  ISO 8601 UTC（`YYYY-MM-DDTHH:MM:SSZ` — 既存コードに時刻整形があれば流用、無ければ小さな純関数を同ファイルに）。u64 ⇄ i64 は `try_from`（失敗は `Corrupt`）。
- `store_path.rs`: `StorePath::for_space(aidlc_root: &Path, space: &SpaceName) -> StorePath` = `<root>/spaces/<space>/intents/.aidlc-store.sqlite`、`as_path()`。
- `workflow_execution_repository_impl.rs`: `WorkflowExecutionRepositoryImpl { store: RefCell<SqliteEventStore> }`、`new(store)`、`find_by_id` / `store`（BR1.2 / BR1.3 の手順、
  `EventStoreError → RepositoryError` の写像は委任 2 の決めに従う）。
- 依存: workspace `rusqlite`（bundled、最新安定版を `cargo search rusqlite` / `cargo add --dry-run` 等で確認して固定）、adapter に `rusqlite = { workspace = true }`。

### Step 8 — Refactor
- エラー写像の一本化、`# Errors` rustdoc、添字アクセスなし、`unwrap` なし、`cargo clippy --workspace --all-targets -- -D warnings` / `cargo fmt --all --check` 緑、
  `cargo audit` 実行（結果を報告 — advisory）、`cargo test -p core-interface-adapter` 全緑。

## 作法（厳守）

- TDD。プロダクトコードに `unwrap` / `expect` / `panic!` / 添字アクセスを書かない（`indexing_slicing` / `panic` は本 Bolt で deny に昇格する — 最初から守る）。
- フィールド private + アクセサ、mod private + `pub use`、エラーは材料のみ（文言を運ばない）、Clock は注入（`SystemTime` を直接読まない）、`std::env` を読まない、
  ログ出力なし。
- 設計に無い判断は報告の「設計質問」に書いて進める。

## 報告（`developer-report-3.md`）

「Red の失敗出力」「実装概要（ファイル・公開面・Tx 手順）」「依存（版・cargo audit）」「判断」「検査結果（cargo test -p core-interface-adapter、clippy、fmt、audit）」
「設計質問」「未了」。最終応答は要約（日本語、10 行以内）。
