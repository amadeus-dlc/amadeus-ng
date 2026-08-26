# developer-brief-8 — 委任 8: 内部可変性の除去（`&self` への偽装の是正）（U3 / Bolt B5）

Conversation language: 日本語（コメント・報告はすべて日本語）。

## 背景（オーナー裁定 2026-08-23）

オーナーから新しい設計規則が示され、正本に登録済みである。**先に必ず読むこと**:

- `aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/interior-mutability.md`
- `aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/command-query-separation.md`

要点:

- **シグニチャから内部の振る舞いが予想できるのが良い設計**であり、特別な理由なく隠蔽してはならない。
- 内部可変性は**既定で禁止**。可変操作はまず `&mut self` で設計する。
- `&self` メソッドの裏に `RefCell` / `Cell` / `Rc<RefCell<_>>` / ロックを置いて可変性を隠すのは
  **「`&self` への偽装」** であり禁止。`&self` + 内部可変性を採るには**強い理由**が要る（立証責任は採る側）。
- `Clone` が「同じ可変状態を指す別ハンドル」を配る型は禁止（`&mut self` の排他性が嘘になる）。
- CQS: Query = `&self` + 戻り値、Command = `&mut self` + 戻り値なし or `Result<(), E>`。

委任 2 でコンダクタが下した「`Rc<RefCell<Connection>>` の共有ハンドル」という裁定は**誤りだった**。
理由は「await をまたぐ借用の回避」だったが、`&mut self` なら最初からその問題は起きない。撤回する。

## 役割と範囲

あなたは aidlc-developer-agent。Unit **u3-event-store-repository**（Bolt B5）の委任 8。
リポジトリルート `/Users/j5ik2o/orca/workspaces/amadeus-ng/docs`、ブランチ `bolt/b5-u3-event-store-repository`
（委任 1〜7 はコミット済み。今はあなたしか走っていません）。

### 所有ファイル（書いてよいもの）

- `modules/core/use-case/src/orchestration/workflow_execution_repository.rs`（ポート）
- `modules/core/interface-adapter/src/orchestration/event_store_impl.rs`
- `modules/core/interface-adapter/src/orchestration/workflow_execution_repository_impl.rs`
- `modules/core/interface-adapter/src/orchestration/memory/in_memory_event_store.rs`
- `modules/core/interface-adapter/src/orchestration/memory/workflow_execution_repository.rs`
- 上記の変更に追随するために必要な範囲で、`modules/core/interface-adapter/tests/**` と
  `modules/core/use-case/src/orchestration/**` の既存インラインテスト、および呼び出し側
  （`modules/**` の他ファイルでコンパイルが通らなくなった箇所）
- 報告 `aidlc/spaces/.../u3-event-store-repository/code-generation/developer-report-8.md`（新規）

### 触ってはいけないもの

`docs/**`、`formal/**`、`scripts/**`、`.github/**`、`Cargo.toml`、`tools/lint`、
`aidlc/spaces/**`（上記の報告ファイルを除く）、`.coderabbit.yaml`。
**`git add` / `git commit` / `git push` はしない。`.claude/` のツールは実行しない。**

仕様書（`docs/specs/**`）と設計正本（`../functional-design/**`）の同期はコンダクタが行う。
あなたは「記述が実態と食い違う箇所」を報告に列挙するだけでよい。

## 是正する 4 箇所（実測済み）

### 1. ポート `WorkflowExecutionRepository::store`

```rust
// 現状 (workflow_execution_repository.rs:47) — Command なのに &self（偽装）
async fn store(&self, event: &WorkflowExecutionEvent, aggregate: &WorkflowExecution)
    -> Result<(), RepositoryError>;
```
→ `&mut self` にする。`find_by_id(&self)` は Query なのでそのまま。

### 2. `WorkflowExecutionRepositoryImpl<C>`

```rust
// 現状 — RefCell で &self から可変性を取り出している
pub struct WorkflowExecutionRepositoryImpl<C> { store: RefCell<EventStoreImpl<C>> }
async fn store(&self, ...) { let mut store = self.store.borrow().clone(); ... }
pub fn event_store(&self) -> EventStoreImpl<C>   // 同じ可変状態への別ハンドルを配っている
```
→ `store: EventStoreImpl<C>` を直接所有。`store` は `&mut self`。
`event_store` は所有権を配らず**参照を返す** Query にする（`&self -> &EventStoreImpl<C>`）。
書き込み用の口が要るなら `event_store_mut(&mut self) -> &mut EventStoreImpl<C>` を別に置く
（CQS: Query と Command を分ける）。

### 3. `EventStoreImpl<C>`

```rust
// 現状 (event_store_impl.rs:204-219)
pub struct EventStoreImpl<C> { path: StorePath, connection: Rc<RefCell<Connection>>, clock: Rc<C> }
impl<C> Clone for EventStoreImpl<C> { /* 同じ接続を指す別ハンドル */ }
```
→ `connection: Connection` を直接所有し、**手書き `Clone` を削除**する。
rusqlite は `Connection::prepare` が `&self`、`Connection::transaction` が `&mut self` なので、
読み取り（`get_latest_snapshot_by_id` / `get_events_by_id_since_seq_nr` / `checkpoint` /
`events_after`）は `&self`、書き込み（`persist_event` / `persist_event_and_snapshot` /
`advance_checkpoint` / `within_write_transaction`）は `&mut self` で素直に書けるはずである。
`clock` の `Rc<C>` は**共有ではなく単なる所有**にできるなら `C` を直接持つ（判断はあなたに委ねる。
`Rc` のままにする場合は理由を報告に書く — `Rc<C>` 自体は不変共有なので偽装には当たらない）。

### 4. InMemory 側（同型）

```rust
pub struct InMemoryEventStore { tables: Rc<RefCell<Tables>> }
pub struct InMemoryWorkflowExecutionRepository { store: RefCell<InMemoryEventStore> }
pub fn event_store(&self) -> InMemoryEventStore
```
→ `tables: Tables` / `store: InMemoryEventStore` を直接所有。`Clone` が同じ可変状態を配る形をやめる。
SQLite 側と**同じ形**に揃える（契約テストが両実装に同一に走るため）。

## 作業の進め方

1. 上記の正本 2 ファイルと、`code-generation-plan.md` §2（公開 API）、`unit-test-instructions.md` を読む。
2. **TDD**: まず既存テスト（契約テスト 12 本・各実装テスト・クラッシュ再構成・ITF 準拠）を新しい
   シグニチャへ書き換えて **Red を実測**（`cargo test -p core-interface-adapter` のコンパイルエラー出力を
   報告に貼る）。次に実装を是正して Green にする。
3. `#[cfg(test)]` の中で `RefCell` を使うのは可（テストの都合であり本ルールの対象は
   プロダクトコードの公開設計）。ただし不要なら外すこと。
4. **挙動は変えない**。トランザクション手順（BEGIN IMMEDIATE / 楽観 version / rollback 前の
   `actual` 読み）、エラー写像、スキーマ刻印、`journal_mode` 既定は現状のまま維持する。
5. `find_by_id` が `&self` のまま成立することを確認する（rusqlite の読み取りは `&self` で足りる）。
   もし構造上どうしても `&mut self` が要るなら、**勝手に決めずに報告の「設計質問」に書いて止める**。

## 検査（全部通すこと）

```
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo lint
PROPTEST_RNG_SEED=20260823 cargo test --workspace        # 現状 664 全緑。減らさない
bash scripts/quint-gate.sh
PROPTEST_RNG_SEED=20260823 bash scripts/coverage.sh --base origin/main   # 絶対 90% 床 + 相対ゲート、約5分
grep -rn "RefCell\|Cell<" modules/core/interface-adapter/src modules/core/use-case/src modules/core/domain/src
```
最後の grep は、プロダクトコード（`#[cfg(test)]` の外）に内部可変性が残っていないことの証明に使う。
残る場合は 1 件ずつ理由を報告に書くこと。

## 報告 `developer-report-8.md` に必ず含めるもの

- Red の実測出力（シグニチャ変更で落ちたコンパイルエラー）と Green の結果
- 4 箇所それぞれの before / after のシグニチャと構造体定義
- `event_store()` をどういう形にしたか（参照返しか、`_mut` を分けたか）とその理由
- `clock` を `Rc<C>` のままにしたか `C` 直接所有にしたか、その理由
- 呼び出し側で `&mut` が必要になった箇所の一覧（波及範囲）
- **記述が実態と食い違う箇所の一覧**（`docs/specs/**`、`../functional-design/**`、
  `code-generation-plan.md` の該当行）— コンダクタが同期するので直さずに列挙する
- 検査結果（上記コマンドの実測、grep の結果を含む）
- 設計質問（止まった点があれば）
- 未了

最終応答は日本語の要約 10 行以内。
