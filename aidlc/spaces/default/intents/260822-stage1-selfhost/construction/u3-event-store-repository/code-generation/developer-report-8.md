# developer-report-8 — 委任 8: 内部可変性の除去（`&self` への偽装の是正）（U3 / Bolt B5）

**担当**: aidlc-developer-agent
**ブランチ**: `bolt/b5-u3-event-store-repository`（コミットはしていない — 変更は作業ツリーに残してある）
**正本**: `coding-rules/interior-mutability.md` / `coding-rules/command-query-separation.md`

## 1. 到達点（一言）

U3 のプロダクトコードから内部可変性（`RefCell` / `Rc<RefCell<_>>`）を**完全に除去**した。
可変操作はすべて `&mut self`、読取は `&self` になり、同じ可変状態を指す別ハンドルを配る
`Clone` も消えた。挙動（トランザクション手順・エラー写像・スキーマ刻印・`journal_mode`）は
変えていない。テストは 664 → 664 で全緑、検査 6 種すべて PASS。**設計質問なし・未了なし**。

`find_by_id` は `&self` のまま成立した（rusqlite の読取は `Connection::prepare` /
`query_row` がいずれも `&self` で足りる）。止まる必要はなかった。

## 2. TDD — Red の実測

シグニチャ変更なので、先にテスト側だけを新シグニチャへ書き換えてコンパイルエラーを実測した。

### Red A — ポート `WorkflowExecutionRepository`（`cargo test -p core-use-case`）

インラインテストの `FakeRepository::store` を `&mut self` にした時点（trait 本体は未変更）:

```
   Compiling core-use-case v0.1.0 (/Users/j5ik2o/orca/workspaces/amadeus-ng/docs/modules/core/use-case)
error[E0053]: method `store` has an incompatible type for trait
   --> modules/core/use-case/src/orchestration/workflow_execution_repository.rs:105:13
    |
105 |             &mut self,
    |             ^^^^^^^^^ types differ in mutability
    |
note: type in trait
   --> modules/core/use-case/src/orchestration/workflow_execution_repository.rs:48:9
    |
 48 |         &self,
    |         ^^^^^
    = note: expected signature `fn(&FakeRepository, &WorkflowExecutionEvent, &WorkflowExecution) -> _`
               found signature `fn(&mut FakeRepository, &WorkflowExecutionEvent, &WorkflowExecution) -> _`
help: change the self-receiver type to match the trait
    |
105 -             &mut self,
105 +             &self,
    |

For more information about this error, try `rustc --explain E0053`.
error: could not compile `core-use-case` (lib test) due to 1 previous error
```

### Red B — アダプタ（`cargo test -p core-interface-adapter`）

契約テスト装置と ITF 準拠テストを「ストアは Repository の単一所有」という前提へ書き換えた時点:

```
error[E0308]: mismatched types
   --> modules/core/interface-adapter/tests/journal_protocol_conformance.rs:327:9
    |
326 |     assert_projection(
    |     ----------------- arguments to this function are incorrect
327 |         repository.event_store(),
    |         ^^^^^^^^^^^^^^^^^^^^^^^^ expected `&InMemoryEventStore`, found `InMemoryEventStore`

error[E0599]: no method named `event_store_mut` found for struct `InMemoryWorkflowExecutionRepository` in the current scope
   --> modules/core/interface-adapter/tests/journal_protocol_conformance.rs:402:41
    |
402 |                 let reader = repository.event_store_mut();
    |                                         ^^^^^^^^^^^^^^^
    |
help: there is a method `event_store` with a similar name

error[E0308]: mismatched types
   --> modules/core/interface-adapter/tests/journal_protocol_conformance.rs:438:27
    |
438 |         assert_projection(repository.event_store(), &projection, &writers, m, &label).await;
    |         ----------------- ^^^^^^^^^^^^^^^^^^^^^^^^ expected `&InMemoryEventStore`, found `InMemoryEventStore`
    |         |
    |         arguments to this function are incorrect

error: could not compile `core-interface-adapter` (test "journal_protocol_conformance") due to 3 previous errors
```

このとき同時に出た `unused_mut` 警告群（`contract.rs:95,110,169,192` ほか計 9 件）も
証拠として意味がある — 「`&mut repository` にしたのに `mut` は要らない」と rustc が言うのは、
**ポートがまだ `&self` で嘘をついていた**からである。ポートを `&mut self` へ直した後、
これらの警告はすべて消えた（現在 `-D warnings` で緑）。

### Green

```
cargo test -p core-use-case          → 50 passed; 0 failed
cargo test -p core-interface-adapter → 110 + 1 + 5 + 34 + 9 + 6 + 1 + 30 + 24 + 12 passed; 0 failed
```

## 3. 是正した 4 箇所（before / after）

### (1) ポート `WorkflowExecutionRepository`（`core-use-case`）

```rust
// before
async fn find_by_id(&self, id: &IntentId) -> Result<WorkflowExecution, RepositoryError>;
async fn store(&self, event: &WorkflowExecutionEvent, aggregate: &WorkflowExecution)
    -> Result<(), RepositoryError>;

// after
async fn find_by_id(&self, id: &IntentId) -> Result<WorkflowExecution, RepositoryError>;  // Query — 据置
async fn store(&mut self, event: &WorkflowExecutionEvent, aggregate: &WorkflowExecution)
    -> Result<(), RepositoryError>;                                                        // Command
```

trait の doc から「書込のための内部可変性 (`RefCell`) は実装が持つ」という一文を削除し、
CQS と偽装禁止の 2 正本を参照する説明へ差し替えた。

### (2) `WorkflowExecutionRepositoryImpl<C>`

```rust
// before
#[derive(Debug)]
pub struct WorkflowExecutionRepositoryImpl<C> { store: RefCell<EventStoreImpl<C>> }
pub const fn new(store: EventStoreImpl<C>) -> Self { Self { store: RefCell::new(store) } }
pub fn event_store(&self) -> EventStoreImpl<C> { self.store.borrow().clone() }  // 別ハンドルを配る
async fn find_by_id(&self, ..) { let store = self.store.borrow().clone(); .. }
async fn store(&self, ..)      { let mut store = self.store.borrow().clone(); .. }

// after
#[derive(Debug)]
pub struct WorkflowExecutionRepositoryImpl<C> { store: EventStoreImpl<C> }
pub const fn new(store: EventStoreImpl<C>) -> Self { Self { store } }
pub const fn event_store(&self) -> &EventStoreImpl<C> { &self.store }              // Query
pub const fn event_store_mut(&mut self) -> &mut EventStoreImpl<C> { &mut self.store } // Command 側の口
async fn find_by_id(&self, ..)   { let store = &self.store; .. }
async fn store(&mut self, ..)    { self.store.persist_event_and_snapshot(..).await .. }
```

### (3) `EventStoreImpl<C>`

```rust
// before
pub struct EventStoreImpl<C> { path: StorePath, connection: Rc<RefCell<Connection>>, clock: Rc<C> }
impl<C> Clone for EventStoreImpl<C> { /* 同じ接続を指す別ハンドル */ }
// 書込: let mut connection = self.connection.borrow_mut(); connection.transaction_with_behavior(..)
// 読取: let connection = self.connection.borrow(); connection.query_row(..) / prepare(..)

// after
pub struct EventStoreImpl<C> { path: StorePath, connection: Connection, clock: C }
// Clone は削除（手書き実装ごと消した）
// 書込 (&mut self): self.connection.transaction_with_behavior(TransactionBehavior::Immediate)
// 読取 (&self):     self.connection.query_row(..) / self.connection.prepare(..)
```

レシーバの内訳（`EventStore` / `JournalReader` の trait 定義は変えていない — 元から
この形だった）:

| `&self`（Query） | `&mut self`（Command） |
| --- | --- |
| `get_latest_snapshot_by_id` / `get_events_by_id_since_seq_nr` / `events_after` / `checkpoint` / `journal_is_empty` / `path` / `now` | `persist_event` / `persist_event_and_snapshot` / `advance_checkpoint` / `within_write_transaction` |

`&self` の裏で `borrow_mut()` していた偽装は 0 件になった。書込メソッドは以前から
`&mut self` だったが、実体は `RefCell` 越しだったため排他性は嘘だった — いまは
`Connection` の直接所有により借用チェッカが本当に排他を保証する。

モジュール doc の「# なぜ共有ハンドルなのか」節は「# 接続は単一所有である」へ全面差し替え。

### (4) InMemory 側（SQLite 側と同型）

```rust
// before
#[derive(Debug, Clone, Default)] pub struct InMemoryEventStore { tables: Rc<RefCell<Tables>> }
#[derive(Debug, Default)] pub struct InMemoryWorkflowExecutionRepository { store: RefCell<InMemoryEventStore> }
pub fn event_store(&self) -> InMemoryEventStore { self.store.borrow().clone() }

// after
#[derive(Debug, Clone, Default)] pub struct InMemoryEventStore { tables: Tables }   // Tables に Clone を derive
#[derive(Debug, Default)] pub struct InMemoryWorkflowExecutionRepository { store: InMemoryEventStore }
pub const fn event_store(&self) -> &InMemoryEventStore { &self.store }
pub const fn event_store_mut(&mut self) -> &mut InMemoryEventStore { &mut self.store }
```

`InMemoryEventStore` の `Clone` は**残した**が意味が変わった: 以前は「同じ 3 表を指す別
ハンドル」、いまは「3 表を丸ごと写した独立した別のストア」である。禁止パターンは
「`Clone` が**同じ可変状態を指す別ハンドル**を配る型」であり、通常の値としての深い複製は
これに当たらない。契約テストの「別プロセスからの再オープン」を表現するのにこの複製を使う
（下記 §5）。この意味の変化は doc コメントに明記し、専用のテスト
`a_clone_carries_the_rows_but_not_the_mutable_state`（写した後の追記が写しに及ばないこと）で
固定した。

## 4. `event_store()` の形と `clock` の扱い（判断とその理由）

### `event_store()` — 参照返し + `_mut` を別に置いた

ブリーフの選択肢どおり **Query は `&self -> &EventStore`、Command 側の口は
`event_store_mut(&mut self) -> &mut EventStore` として分離**した。理由:

- 所有権や複製を返すと「同じストアを指す 2 つ目の口」ができ、`&mut self` の排他性が
  また嘘になる（interior-mutability.md の禁止パターン「`Clone` が同じ可変状態を指す別
  ハンドルを配る」と実質同じ穴）。参照返しならその口は借用チェッカの管理下に入る。
- `JournalReader::advance_checkpoint` は `&mut self` なので、読取専用の口だけでは
  ITF 準拠テストの catchup が書けない。CQS どおり口を 2 つに割った（`command-query-separation.md`
  の「分離できるなら分離する」）。
- 両方 `const fn`（`missing_const_for_fn` が deny なので必要。Rust 2024 / const_mut_refs で
  `&mut` を返す `const fn` は通る）。

副作用として、テストで「読取ハンドルを握ったまま Repository へ書く」という以前の書き方は
借用エラーになる。これは**規則が意図した検出**なので、テスト側を「書くたびに口を取り直す」
形へ直した（`the_repository_hands_out_a_reader_over_the_same_store` 両実装）。

### `clock` — `Rc<C>` をやめて `C` を直接所有

`Rc<C>` は `EventStoreImpl` の手書き `Clone` のためだけに存在していた（`C: Clone` を
要求せずにハンドルを複製するため）。`Clone` を削除した以上、共有する相手がいない。
interior-mutability.md の判定フロー「1. この型は共有される必要があるか？ → No → 直接所有」
に素直に従い `clock: C` にした。`Rc<C>` 自体は不変共有なので偽装ではないが、
**不要な間接参照を残す理由がない**ので消した。

## 5. 契約テスト装置（`StoreFixture`）の作り直し — 影響が最も大きかった点

これが本委任で唯一、単なる機械的置換で済まなかった箇所である。

**なぜ変更が要ったか**: 旧 `StoreFixture` は `fn open(&self) -> Repository` /
`fn reader(&self) -> Reader` で、in-memory 側は fixture が持つ `InMemoryEventStore` を
`clone()` して「同じ 3 表を指す別ハンドル」を配ることで「同じストアを開き直す」を表現していた。
共有ハンドルを廃した以上、この表現は成立しない（各 Repository が自分の写しへ書いて終わる）。

**どう直したか**: 開き直しと Reader を「**書き終えた Repository** から作る」形にした。

```rust
pub(crate) trait StoreFixture {
    type Repository: WorkflowExecutionRepository;
    type Reader: JournalReader;
    fn open(&self) -> Self::Repository;                                  // 空のストア
    fn reopen(&self, repository: &Self::Repository) -> Self::Repository; // 別インスタンスで開き直す
    fn reader(&self, repository: &Self::Repository) -> Self::Reader;     // 同じ内容を読む口
}
```

- **SQLite**: 引数を無視して同じファイルへ**新しい接続**を開く（従来と同一の実体。
  ファイルに残っている行が見える）。
- **in-memory**: `repository.event_store().clone()`（書き終えた 3 表を引き継いだ別インスタンス）。

どちらも「それまでに書き終えた行が見える別インスタンス」という同じ観測になる。
契約テスト 12 本は 2 実装 = 24 本とも意味を変えずに通っている（`round_trip` の
「16 属性一致・版 5」など期待値は 1 つも緩めていない）。

**忠実さの限界を明記しておく**（コンダクタ判断の材料）: in-memory の写しは、写した後に
元の Repository が書いた行を見ない。SQLite の 2 接続はファイル越しに見える。契約テストは
どれも「書き終えてから開き直して読む」順序なのでこの差は現れないが、将来「開き直した後も
元へ書き続けて双方から観測する」契約を足す場合は in-memory 側の表現を作り直す必要がある。
これは**共有ハンドルを廃した代償として構造的に生じるもの**であり、規則に従う限り避けられない。

同じ理由で ITF 準拠テスト（`journal_protocol_conformance.rs`）も、`store` / `reader` /
`repository` の 3 インスタンスを Repository 単一所有へ畳んだ:

- 射影の突合 → `assert_projection(repository.event_store(), ..)`
- catchup（`checkpoint` / `events_after` / `advance_checkpoint`） → `repository.event_store_mut()`
- crash（プロセス再起動） → `let carried = repository.event_store().clone();` で 3 表を
  引き継いだ別インスタンスへ差し替え

フィクスチャ 6 本・全ステップの射影突合はそのまま通っている（テスト 1 本・全緑）。

## 6. 呼び出し側で `&mut` が必要になった箇所（波及範囲）

プロダクトコードの波及は**ゼロ**である（U5 / U6 / U7 未着手のため、ポートの利用者はまだ
テストしか存在しない）。波及はすべてテストコード。

| ファイル | 内容 |
| --- | --- |
| `modules/core/use-case/src/orchestration/workflow_execution_repository.rs` | `FakeRepository.stored` を `RefCell<Option<_>>` → `Option<_>`、`store(&mut self)`、`let mut repository` ×3 |
| `.../tests/support/mod.rs` | `StoreFixture` に `reopen` 追加・`reader` に引数追加（§5） |
| `.../tests/support/contract.rs` | `seed(&mut R)`、契約関数 12 本の `let mut repository`、`reopen` / `reader(&repository)` への差し替え |
| `.../tests/workflow_execution_repository_contract.rs` | `InMemoryFixture` を unit struct 化、両 fixture に `reopen` / `reader` 実装 |
| `.../tests/in_memory_workflow_execution_repository_test.rs` | `let mut repository` 1 箇所、読取の口を書込のたびに取り直す形へ |
| `.../tests/workflow_execution_repository_impl_test.rs` | `seed(&mut …)`、`let mut repository` 11 箇所、`let mut other_repository` 3 箇所、`event_store()` 参照化 1 箇所 |
| `.../tests/crash_reconstruction_test.rs` | `write_five(&mut …)`、`let mut repository` 5 箇所 |
| `.../tests/journal_protocol_conformance.rs` | §5 のとおり Repository 単一所有へ |
| `event_store_impl.rs` インラインテスト | `connection.borrow()` 除去 3 箇所、`a_cloned_handle_points_at_the_same_connection` を `the_same_path_can_be_opened_again_without_recreating_the_schema` へ差し替え |
| `in_memory_event_store.rs` インラインテスト | `tables.borrow_mut()` 除去 4 箇所、`let mut store` 化、`a_cloned_handle_sees_the_same_store` を `a_clone_carries_the_rows_but_not_the_mutable_state` へ差し替え |

**テスト本数の増減**: 差し替え 2 件はいずれも 1:1（削除 2・追加 2）なので合計は不変。
`cargo test --workspace` は 664 → 664。

`event_store_impl_test.rs`（1,008 行）は**無変更で通った** — 元から `fixture.store()` が
`open()` で新しい接続を開き、書込側は `let mut store` で受けていたため。共有ハンドルに
依存していなかったことの裏付けでもある。

## 7. 記述が実態と食い違う箇所（直していない — コンダクタが同期する）

`docs/specs/**` と `docs/adr/**` には `RefCell` / 内部可変性 / 本ポートのレシーバに
言及する記述は**存在しない**（grep 0 件）ので、仕様正本の同期は不要である。
食い違うのは設計成果物側の以下:

| # | ファイル:行 | 現在の記述 | 実態 |
| --- | --- | --- | --- |
| 1 | `inception/contract-design/contract-summary.md:115` | `async fn store(&self, event: &WorkflowExecutionEvent, aggregate: &WorkflowExecution)` | `&mut self`。**共有契約 C3 の本文**であり、所有者は U5 / U6。U5 / U6 が着手前に読む唯一の正本なので、ここは優先度が高い（既存の `usize` → `u64` 未改訂と同じ性質の食い違いが 2 件目になる） |
| 2 | `inception/contract-design/contract-summary.md:313` | Finding #3「`&self` の中から `&mut self` を呼ぶには内部可変性（`Mutex`/`RefCell` 等）が要る … functional-design で保持方法を明記する」 | 前提ごと解消された（`&mut self` にしたので内部可変性は不要）。所見自体が失効 |
| 3 | `construction/.../functional-design/functional-spec.md:26` | `store(&self, …)` — C3 どおり | `&mut self` |
| 4 | `construction/.../functional-design/functional-spec.md:31-32` | `WorkflowExecutionRepositoryImpl { store: RefCell<SqliteEventStore> }` … `InMemoryWorkflowExecutionRepository { store: RefCell<InMemoryEventStore> }` も同形 | どちらも `RefCell` なしの直接所有。加えて型名は `EventStoreImpl`（旧名 `SqliteEventStore` の残存は既知 — `code-summary.md` Review 所見 3 / `pending-revision.md` #7） |
| 5 | `construction/.../functional-design/entities.md:23` | `store, type: "async fn(&self, …)"` | `&mut self` |
| 6 | `construction/.../functional-design/entities.md:174` | `store, type: "std::cell::RefCell<SqliteEventStore>"` + constraints に内部可変性の理由を長文で明記 | `EventStoreImpl<C>` を直接所有。constraints の説明ごと差し替えが要る |
| 7 | `construction/.../functional-design/entities.md:182` | `InMemoryWorkflowExecutionRepository`（`RefCell<InMemoryEventStore>` を内包 — Impl と同じ内部可変性） | 直接所有 |
| 8 | `construction/.../functional-design/entities.md:240` | Review Major 所見 3（内部可変性戦略の欠落） | #2 と同じく前提ごと失効 |
| 9 | `construction/.../functional-design/rules.md:16` | BR1.1 statement 末尾「Repository 実装は EventStore を `RefCell` で内包して `&self` → `&mut self` を橋渡しする（借用は await をまたがない）」 | 橋渡し自体が不要。`&mut self` で素直に書く |
| 10 | `construction/.../nfr-design/logical-components.md:16` | `find_by_id` / `store`（`RefCell<SqliteEventStore>`） | 直接所有 |
| 11 | `construction/.../nfr-design/logical-components.md:30` | 「Repository 実装は `RefCell` で内部可変性を閉じ、借用は各メソッド内で完結（await をまたがない）」 | 内部可変性なし。借用制約の記述ごと不要 |
| 12 | `construction/.../nfr-design/security-design.md:98` | Minor 所見 2 の本文中「`RefCell` 内部可変性戦略を明記済みだった（3 件とも解消を確認）」 | 監査証跡としての記述。`RefCell` 戦略はもう存在しない旨の追記が要る |
| 13 | `construction/.../nfr-requirements/tech-stack-decisions.md:12` | 「内部可変性は `RefCell`（FD BR1.1）」 | 内部可変性なし |
| 14 | `construction/.../code-generation/code-generation-plan.md:26` | `async fn store(&self, …)` | `&mut self` |
| 15 | `construction/.../code-generation/code-generation-plan.md:33,45,100,108` | `WorkflowExecutionRepositoryImpl { store: RefCell<SqliteEventStore> }` / 「Repository 実装（RefCell）」ほか | 直接所有 |
| 16 | `construction/.../code-generation/code-summary.md:263` | 検証表の行「`EventStoreImpl` の内部可変性（`Rc<RefCell<Connection>>` 共有ハンドル、手動 `Clone`）… `borrow().clone()` 後に await する形で借用が await をまたがない → code-summary 決定7 の記述どおり」 | `Connection` 直接所有・`Clone` 削除。**決定 7 そのものが撤回された**ので、決定表の項目 7 と本行の両方を書き直す必要がある |

`code-summary.md` の `## Review` 所見 2（`within_write_transaction` が `rusqlite::Transaction`
を公開 API に露出している層漏れ）は**本委任では触れていない**（別論点であり、ブリーフの
スコープ外・`pending-revision.md` #8 で U7 へ申し送り済み）。ただしレシーバは `&mut self`
のままで規則には適合している。

## 8. 検査結果（実測）

| 検査 | 結果 |
| --- | --- |
| `cargo fmt --all --check` | PASS（出力なし、exit 0） |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS（警告 0、exit 0） |
| `cargo lint` | PASS（出力なし、exit 0） |
| `PROPTEST_RNG_SEED=20260823 cargo test --workspace` | **664 passed / 0 failed**（ベースラインと同数） |
| `bash scripts/quint-gate.sh` | PASS（typecheck 3 / invariants 3 / witness 12 / `quint test r_.*` すべて緑）|
| `PROPTEST_RNG_SEED=20260823 bash scripts/coverage.sh --base origin/main` | PASS。head **98.394147…%** ≥ 絶対床 90.0%、base（origin/main）97.387970…%、相対ゲート PASS（tolerance 0.01）|

ベースラインは着手前に同じコマンドで実測した `664 passed; 0 failed`。**減っていない**。

### 内部可変性が残っていないことの証明

```
$ grep -rn "RefCell\|Cell<" modules/core/interface-adapter/src modules/core/use-case/src modules/core/domain/src
modules/core/interface-adapter/src/orchestration/workflow_execution_repository_impl.rs:29:/// 書込中の排他は借用チェッカが保証するので、`RefCell` の実行時借用検査も
modules/core/interface-adapter/src/orchestration/event_store_impl.rs:5://! 接続は本型が**直接所有**し、内部可変性 (`RefCell`) も共有ハンドル (`Rc`) も持たない。
modules/core/use-case/src/orchestration/workflow_execution_repository.rs:18:/// (`RefCell` 等) で隠すのは「`&self` への偽装」であり禁止されている
```

残る 3 件は**すべて doc コメント**であり、いずれも「`RefCell` を使っていない」ことを
説明する文である（コードとしての内部可変性は 0 件）。規則の根拠を型のそばに残す意図で
あえて置いた。削除が望ましければ落とせる。

追加で、共有可変ラッパーの不在も確認した:

```
$ grep -rn "Rc<\|Arc<\|Mutex<\|RwLock<" modules/core/interface-adapter/src modules/core/use-case/src modules/core/domain/src
(出力なし — exit 1)
```

テストコード（`#[cfg(test)]` と `tests/`）にも `RefCell` は 1 件も残っていない
（不要になったので全部外した）。

## 9. 設計質問

**なし。** `find_by_id` は `&self` のまま成立したので、止まる条件には当たらなかった。

判断が要った 2 点（`event_store()` の形・`clock` の `Rc` 撤去）はブリーフが明示的に
委ねた範囲なので自分で決め、理由を §4 に書いた。§5 の `StoreFixture` 作り直しだけは
ブリーフに書かれていない設計変更だが、これは「共有ハンドルを廃す」という指示から
機械的に導かれる帰結（旧 fixture は共有ハンドルの存在を前提にしていた）であり、
テストの意味・期待値は 1 つも変えていないため、止めずに進めた。忠実さの限界は
§5 末尾に明記してある。

## 10. 未了

**なし。** 所有ファイルの範囲で、ブリーフが挙げた 4 箇所すべてを是正し、検査 6 種を通した。

スコープ外として意図的に手を付けなかったもの:

- 仕様書・設計正本の同期（§7 の 16 件） — ブリーフどおりコンダクタへ委ねる。
  うち **#1（C3 本文の `store(&self)`）は U5 / U6 の着手前に必ず処理が要る**。
- `within_write_transaction` の `rusqlite::Transaction` 露出（`code-summary.md` Review 所見 2 /
  `pending-revision.md` #8） — 本委任の論点ではなく、U7 の設計で裁定される。
- `git add` / `commit` / `push` は行っていない。変更は作業ツリーに残してある。

---

## 11. 追記 — 正本更新（`*Shared` / SharedLock パターン）への適合確認

コンダクタからの追加指示を受け、更新後の `coding-rules/interior-mutability.md`
（§「`*Shared`（SharedLock）ラッパーパターン」および「本プロジェクトでの前提」）を
読み直した。**成果物の変更は不要**であり、以下を実測で再確認した。

正本自身が「現状の U3 は単一所有・単一接続であり、このパターンを要しない
（`&mut self` で足りる）」と明記しており、本委任の成果はその記述と一致している。

### 禁止パターン 7 項目との突合（すべて実測）

| 禁止パターン | 本委任の成果 | 実測 |
| --- | --- | --- |
| 可変操作を `&self` にし `RefCell` / `Cell` / ロックで隠す | 該当なし | `grep -rn "RefCell\|Cell<" modules/` → 3 件、**すべて doc コメント**（`//` / `///` / `//!` を除外すると 0 件） |
| 既存の `&mut self` メソッドを `&self` + 内部可変性へ変更 | **逆方向**（`&self` → `&mut self`）なので該当なし | §3 (1) |
| `Rc<RefCell<T>>` / `Arc<Mutex<T>>` / `Arc<RwLock<T>>` の手書きラッパー新規作成 | 該当なし（既存の `Rc<RefCell<Connection>>` / `Rc<RefCell<Tables>>` は**削除**した） | `grep -rn "Rc<\|Arc<\|Mutex<\|RwLock<" modules/` → **0 件**（exit 1、テストコード含む） |
| 共有が不要な型を `*Shared` でラップ | 該当なし | `grep -rn "SharedLock\|SharedRwLock\|Shared\b" modules/` → **0 件**（exit 1） |
| `*Shared` 適用時に元のロジック型を削除 | `*Shared` を作っていないので該当なし | 同上 |
| ガードやロックを外部へ返す | ロックが存在しないので該当なし。`event_store_mut(&mut self) -> &mut EventStoreImpl<C>` は**所有フィールドへの通常の可変アクセサ**であり、同期プリミティブのガードではない（借用チェッカが区間を管理する） | §4 |
| `Clone` が同じ可変状態を指す別ハンドルを配る | `EventStoreImpl` の手書き `Clone` は削除。`InMemoryEventStore` の `Clone` は独立した深い写し | `grep -rn "impl.*Clone for" modules/core/interface-adapter/src` → **0 件**（手書き `Clone` は 1 つも残っていない）。写しの独立性はテスト `a_clone_carries_the_rows_but_not_the_mutable_state` で固定 |

### 判定フローの適用

```
1. この型は共有される必要があるか？  → No
   （U3 は単一所有・単一接続。共有していたのは「Repository が &self で書けるように
     見せる」ためだけの偽装であり、実需ではなかった）
   → &mut self で設計（第1選択。ここで終わり）
```

第 2 選択（`*Shared` ラッパー）には**到達しない**。よって `SharedLock<T>` /
`SharedRwLock<T>` の新設は行っていない（オーナー裁定が要る別委任の案件であることを了解）。

### 逃げ道を残していないことの確認

「どうしても共有が必要で `&mut self` では表現できない」箇所は**発生しなかった**。
唯一、共有ハンドルの廃止で表現の作り直しが要ったのは契約テスト装置と ITF 準拠テスト
（§5）だが、これは**テストコードの表現の問題**であり、そこでも `RefCell` は残していない
（`grep` のとおりテストコードにも 0 件）。プロダクト・テストのどちらにも内部可変性の
逃げ道は残っていない。

再確認時の実測: `cargo test -p core-use-case -p core-interface-adapter` → **282 passed / 0 failed**。

---

## 12. 追記 2 — `SharedLock` 新設の許可（制限解除）への回答

コンダクタから「必要な実装は作ってよい」という訂正（オーナー裁定 2026-08-23）を受け、
更新後の正本 §「本プロジェクトでの前提」を読み直した。

**結論: U3 では `SharedLock<T>` / `SharedRwLock<T>` を作らない。成果物の変更は不要。**

正本自身が同じ結論を明記している —「投機的に作らない。共有の必要が実際に生じた時点で
作る」「現状の U3 は単一所有・単一接続であり、このパターンを要しない（`&mut self` で
足りる）」。判断順序の 1（まず `&mut self` を尽くす）で完結し、2（投機的に作らない）に
より新設は誤りになる。

### 例外 4 条件に照らした判定

正本「ロックを取り合うメソッド」の成立条件は次のすべてを満たすことだが、U3 は
**4 条件すべてを満たさない**。

| # | 条件 | U3 の実態 | 判定 |
| --- | --- | --- | --- |
| 1 | その型が**実際に共有される**（複数の所有者・複数の呼び手が同時に存在する） | 単一プロセスのワンショット CLI。`EventStoreImpl` の所有者は `WorkflowExecutionRepositoryImpl` 1 つだけで、同一プロセス内に 2 つ目の呼び手が同時に存在する経路は無い。旧コードの「共有」は Repository を `&self` に見せるためだけの偽装であり、実需ではなかった | ✗ |
| 2 | 同期プリミティブを持ち、競合を直列化することがその型の責務である | 直列化の責務は SQLite 側にある — ADR-007 でロック機構を退役させ、`BEGIN IMMEDIATE` + `busy_timeout`（既定 5000ms）が**唯一の直列化機構**である（BR2.4）。競合は別プロセス間で起き、Rust の型が直列化するのではない。楽観 version の競合検出も DB の `UNIQUE (aggregate_id, seq_nr)` と `WHERE version = expected` が担う | ✗ |
| 3 | ロック区間がクロージャ内に閉じ、ガードを外へ返さない | ロックが存在しないので不成立 | ✗ |
| 4 | なぜ `&mut self` では表現できないかが doc コメントに書かれている | `&mut self` で表現**できた**（本委任の成果そのもの）。書くべき理由が無い | ✗ |

### 唯一「共有があれば楽だった」箇所と、それでも作らない理由

§5 に記した契約テスト装置の忠実さの限界（in-memory の `reopen` は写しなので、写した後に
元へ書いた行を見ない。SQLite の 2 接続はファイル越しに見える）だけが、共有があれば
より忠実に書けた箇所である。それでも作らない理由:

1. **理由が弱い。** `memory/` は `src/` 配下の**プロダクトコード**である。そこへ内部可変性を
   持ち込む動機が「契約テスト装置の表現を揃えたいから」では、正本が名指しで却下している
   「便利だから」「借用が面倒だから」の類に当たる。立証責任は採る側にあり、これは果たせない。
2. **直したものを戻すことになる。** `InMemoryWorkflowExecutionRepository` が
   `InMemoryEventStoreShared` を持てば、`store(&mut self)` の書込は借用チェッカではなく
   `with_write` のロック区間を通ることになる。オーナーが今回是正させた当の構図
   （借用チェッカが効かないコード）へ半分戻る。
3. **実害が無い。** 契約テスト 12 本 × 2 実装 = 24 本は期待値を 1 つも緩めずに通っており、
   検出力は落ちていない。忠実さの差が現れるのは「開き直した後も元へ書き続けて双方から
   観測する」契約を将来足した場合だけで、**その契約はいま存在しない**。
4. **投機になる。** 3 のとおり必要が実際に生じていない。正本の「使われない型は死んだコードに
   なり、`dead_code` とカバレッジ 90% 床の両方に当たる」がそのまま当てはまる。

### 将来 `SharedLock` が要ることになる条件（コンダクタへの申し送り）

本委任で判断した限り、以下のいずれかが実際に起きた時点が新設のタイミングである。
いずれも**現時点では発生していない**ので、今は起こさない。

- 契約に「同じストアを指す 2 つのインスタンスが**同時に生存**し、片方の書込を他方が観測する」
  という約束を足す必要が出たとき（in-memory 側の表現がそこで初めて足りなくなる）。
- U4（投影）が Repository と**同時に**同じストアを保持する設計になったとき。現状の
  ITF 準拠テストは投影の catchup を `event_store_mut()` 経由の逐次アクセスで表現できており、
  同時保持は要らない。
- ユースケース層（U5 / U6）が 1 つの Repository を複数の所有者へ配る必要が出たとき。
  ただし静的束縛・`&mut` 引き回しで書けるなら、まずそちらを尽くすべきである。

作る場合の作法（`modules/shared/` に 1 度だけ・`SharedAccess` trait で `with_read` /
`with_write` のみ公開・ロック実装は内部詳細・型と同時に TDD でテスト・迷ったら
`SharedLock`）は正本に明記されているとおりで、了解している。

### 逃げ道を残していないことの再掲

`Rc<RefCell<T>>` / `Arc<Mutex<T>>` / `Arc<RwLock<T>>` の手書きは 1 件も無い
（`grep -rn "Rc<\|Arc<\|Mutex<\|RwLock<" modules/` → **0 件**、テストコード込み）。
「型が無いから手書きへ逃げた」箇所も、「型が無いから止まった」箇所も無い。
`&mut self` で全部書けたので、そもそも共有の必要が生じなかった。
