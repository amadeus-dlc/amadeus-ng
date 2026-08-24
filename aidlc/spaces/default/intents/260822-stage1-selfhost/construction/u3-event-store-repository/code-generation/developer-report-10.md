# 委任 10 報告 — 命名監査 10 件の是正

**担当**: aidlc-developer-agent
**Unit**: `u3-event-store-repository`（Bolt B5）
**ブランチ**: `bolt/b5-u3-event-store-repository`
**日付**: 2026-08-24
**判定**: 10 件すべて是正済み。検査 4 種すべて緑（`cargo test --workspace` 674 passed / 0 failed — 委任前と同数）。
**コミットしていない**（指示どおり `git add` / `commit` / `push` は実行していない）。

**挙動は変えていない。** 変更はすべて識別子の改名と doc コメントの追随であり、関数本体のロジックには
一切触れていない。唯一の例外は F7 の doc 追記（コメントのみ）と、F2 の関連関数→自由関数への移設
（本体は逐語で同一、`impl` ブロックの外へ移しただけ）である。

---

## 1. 是正 10 件 — before / after と波及

「外部波及」は**定義ファイル以外**での出現行数（`pub use` 再エクスポート・呼出・doc 参照を含む）。

| # | before | after | 定義ファイル | 外部波及 | 波及ファイル数 |
| --- | --- | --- | --- | --- | --- |
| F1 | `state_writers::set_field` | `with_field_if_present` | `modules/core/domain/src/workspace/state_writers.rs:20` | 1 | 2 |
| F2 | `wire::StageEntryWire::to_entry` | `wire::parse_entry`（**自由関数化**） | `modules/core/interface-adapter/src/orchestration/wire/mod.rs:348` | 4 | 3 |
| F3a | `state_writers::set_field_strict` | `with_field` | 同 `state_writers.rs:51` | 1 | 2 |
| F3b | `state_writers::set_or_insert_field` | `with_field_or_insert` | 同 `state_writers.rs:79` | 1 | 2 |
| F3c | `state_writers::remove_field` | `without_field` | 同 `state_writers.rs:121` | 1 | 2 |
| F3d | `checkbox::set_checkbox` / `CheckboxWriteError` | `with_checkbox_marker` / `CheckboxUpdateError` | `modules/core/domain/src/workspace/checkbox.rs:165` / `:156` | 1 / 1 | 2 |
| F4 | `state_writers::get_field` | `find_field` | 同 `state_writers.rs:10` | 1 | 2 |
| F5 | `workflow_definition_repository_impl::read_error_message` | `graph_read_error_message` | `modules/core/interface-adapter/src/orchestration/workflow_definition_repository_impl.rs:94` | 1 | 2 |
| F6 | `corrupt`（**3 箇所**、下記 §2 参照） | `corrupt_error` | `wire/mod.rs:40` / `memory/in_memory_event_store.rs:76` / `event_store_impl.rs:137` | 計 14 出現 | 5 |
| F7 | `WorkflowExecution::start_with_entries` | `start_from_plan_unchecked` | `modules/core/domain/src/orchestration/workflow_execution.rs:161` | 7 | 7 |
| F8 | `StorePath::of` | `StorePath::for_space`（**元に戻した**） | `modules/core/interface-adapter/src/orchestration/store_path.rs:31` | 9 | 6 |
| F9 | `infra_io::atomic::open_exclusive_new` | `create_new_file` | `modules/infra-io/src/atomic.rs:68` | 0 | 1 |
| F10 | `message_catalog` の文言関数 **6 本**（下記 §2） | 末尾に `_message` | `modules/shared/message-catalog/src/lib.rs` | 4 | 4 |

### F1 — 危険な側に長い名前

`set_field` → `with_field_if_present`。
「該当行が無ければ入力をそのまま複製して返す（無言 no-op）」という危険な性質が名前に出た。
安全な側（失敗を `Err` にする F3a）が短い `with_field` になり、深刻度と名前の長さが逆転していた
状態が解消されている。

### F2 — 自由関数にした（判断とその理由）

**自由関数 `pub(super) fn parse_entry(value: &JsonValue) -> Result<StageEntry, CorruptCause>` にした。**
`StageEntryWire` の関連関数のままにはしていない。理由は次の 3 点。

1. **作るものが `StageEntry`（ドメイン値）であって `StageEntryWire` ではない。**
   関連関数として `StageEntryWire::parse_entry` と綴ると、名前を `parse_*` に直しても
   「`StageEntryWire` を作る口」という誤読は残る。監査 F2 が指摘した誤読の根が消えない。
2. **同じ mod の兄弟 10 本（`parse_slug` / `parse_phase` / `parse_plan` ほか）がすべて自由関数**で、
   いずれも「ワイヤ上の表現 → ドメイン型」という同一の形をしている。`parse_entry` は
   入力が `&str` から `&JsonValue` に変わるだけで、役割はまったく同じ。
3. **呼出側の書き味が変わらない。** 既存の 2 箇所は `.map(StageEntryWire::to_entry)` という
   関数値渡しで、これが `.map(parse_entry)` になるだけ。引数を明示する必要は生じていない。

`from_entry`（`&StageEntry -> StageEntryWire`）は `StageEntryWire` を実際に構築するので、
`impl StageEntryWire` に残した。結果として impl ブロックには `from_entry` 1 本だけが残り、
「対の逆変換に見えるのに逆変換ではない」という監査の指摘そのものが構造から消えている。
`STAGE_ENTRY_KEYS` は元から mod レベルの `const` だったため、移設で可視性の調整は不要だった。

移設後の doc に理由を 1 行残してある（`factory-naming.md` の「表のどれにも当てはまらないときは
なぜ表に載せなかったかを doc に書く」に準じた記録）。

### F7 — 検査を落とす入口であることを名前と doc の両方に出した

`start_with_entries` → `start_from_plan_unchecked`。doc に次を追記した（本体は未変更）。

- `start` と違い `StartError::UnknownScope` を**返せない**こと
- 返せない理由（照合すべき `WorkflowDefinition` を受け取らないので検査の材料が無い）
- `_unchecked` がこの検査の欠落を指すこと
- 定義を持っている呼出側は `WorkflowExecution::start` を使うこと

`# Errors` 節にも「スコープ名の妥当性（`UnknownScope`）は検査しない」を明記した。

### F8 — `StorePath::for_space` へ戻した

正本 `factory-naming.md` の適用例行および `code-generation-plan.md:33` は既に `for_space` と
書いてあり、コード側だけが `of` になっていた。**この改名によりコードと正本が一致した**
（§3 のドリフト一覧から 2 件が解消されている）。

### F10 — `_message` サフィックス（**6 本**。ブリーフの 5 本 + 1 本）

| before | after |
| --- | --- |
| `state::field_not_found` | `state::field_not_found_message` |
| `state::file_not_found` | `state::file_not_found_message` |
| `lock::acquire_failed` | `lock::acquire_failed_message` ← **ブリーフ外**（§2 参照） |
| `lock::acquire_failed_for_key` | `lock::acquire_failed_for_key_message` |
| `lock::merge_acquire_failed` | `lock::merge_acquire_failed_message` |
| `bolt::invalid_mode` | `bolt::invalid_mode_message` |

エラー型 `FieldNotFound`（`state_writers.rs:26`）は**変更していない**（指示どおり）。呼出は
`FieldNotFound::new(msg::field_not_found_message(field))` になり、型名と関数名の綴り衝突が解消した。

`GoldenStatus` 定数（`FIELD_NOT_FOUND_STATUS` 等 5 本）は関数ではないため対象外とし、据え置いた。

---

## 2. ブリーフの表を超えて触った 2 件（**要確認**）

いずれも「同じ欠陥・同じ直し方・挙動変更なし」だが、ブリーフの 10 件表には無い。差し戻すべきなら
戻せる（どちらも private ないしライブラリ内の関数で、呼出側は全数直してある）。

### (a) `event_store_impl.rs:137` の 3 つ目の `corrupt`

ブリーフ F6 は「`wire::corrupt` および `memory/in_memory_event_store::corrupt` の**2 箇所とも**」と
書いているが、実測では**同名の private 関数が 3 つ**あった。

| 場所 | 可視性 | 引数 |
| --- | --- | --- |
| `wire/mod.rs:40` | `pub(super)` | `&str` |
| `memory/in_memory_event_store.rs:76` | private | `&IntentId` |
| `event_store_impl.rs:137` | private | `&str` |

3 つ目を `corrupt` のまま残すと、F6 が挙げた理由（「モジュールをまたいで同名が並ぶ」）が
そのまま残り、しかも `orchestration` 配下に `corrupt` と `corrupt_error` が並立してかえって悪化する。
監査 §3（据え置き 25 件）にもこの関数は挙がっていない＝意図的な据え置きではなく**監査の取り漏らし**
と判断し、3 つとも `corrupt_error` にした。

### (b) `message_catalog::lock::acquire_failed`

ブリーフ F10 は 5 本を列挙しているが、`lock` モジュールには**もう 1 本**同型の関数
（`pub const fn acquire_failed() -> &'static str`、`lib.rs:66`）がある。監査表の行番号
（35/47/80/97/130）から漏れており、§3 の据え置きにも無い。

同じ `lock` モジュール内で `acquire_failed_for_key_message` / `merge_acquire_failed_message` だけが
`_message` を持ち `acquire_failed` だけ持たない状態は、F10 の指摘そのもの（「規約が 2 つに割れている」）
を新たに作り出す。よって `acquire_failed_message` に揃えた。

---

## 3. 触っていない — 判断つき

### (a) `WorkflowExecution::set_checkbox`（`workflow_execution.rs:380`、private）

```rust
fn set_checkbox(&mut self, stage: StageIndex, value: CheckboxState) { ... }
```

**改名していない。** これは `&mut self` を取って自身を書き換える真正のコマンドであり、
`command-query-separation.md`（Command は `&mut self`）に適合している。隣に同型の
`set_approved(&mut self, ...)` があり、`set_` はここでは正しい動詞である。監査 F3d が対象にしたのは
純関数 `checkbox::set_checkbox`（新 `with_checkbox_marker`）だけで、この集約メソッドは §2 の表にも
§3 の据え置きにも挙がっていない。名前が実態と一致しているので直す理由が無い。

これがブリーフ指定の grep に 11 行残る唯一の実体である（§5 参照）。

### (b) `state_writers` というモジュール名

関数がすべて `with_*` / `find_field` になった結果、モジュール名 `state_writers`（「書き手」）と
中身（純粋な `&str -> String`）のずれが目立つようになった。**ブリーフにも監査にも無いので改名して
いない。** 直すなら `state_fields` などが候補だが、`docs/specs/11-workspace.md:64` が
`state_writers` を表の行名として使っているため、仕様同期とセットで行うべき作業である。

### (c) `docs/specs/**` と設計文書

指示どおり**一切変更していない**。列挙のみ（§4）。

---

## 4. 記述が実態と食い違う箇所（直さずに列挙）

### 4.1 `docs/specs/` — 2 ファイル 3 箇所

| 場所 | 現在の記述 | 実態 |
| --- | --- | --- |
| `docs/specs/11-workspace.md:64` | `` `set_field`（無言 no-op）/ `set_field_strict`（不在で throw）/ `set_or_insert_field` / `remove_field` の 4 種 `` | `with_field_if_present` / `with_field` / `with_field_or_insert` / `without_field`。なお `get_field` → `find_field` は同行に無い |
| `docs/specs/11-workspace.md:132` | W14 の実装欄が `` `set_field_strict` `` | `with_field` |
| `docs/specs/11-workspace.md:169` | 「`set_or_insert_field` 化」 | `with_field_or_insert` 化 |

`docs/specs/12-workflow-definition.md:242` は `state::field_not_found` / `state::file_not_found` /
`lock::acquire_failed` / `bolt::invalid_mode` を**文言カタログのエントリ識別子**として列挙している。
関数名が `_message` 付きになったので、ここも `_message` を付けるか、あるいは「エントリ名であって
関数名ではない」と読むかの裁定が要る（後者なら変更不要）。

`docs/specs/research/golden-3c3146cf-lib.md:1009-1012,1016` の 5 行は upstream TypeScript 側の綴り
（`setField` / `getField` 等）を採取した記録なので、**変更してはいけない**。ドリフトではない。

### 4.2 `aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/` — 1 箇所

| 場所 | 現在の記述 | 実態 |
| --- | --- | --- |
| `gateway-taxonomy.md:37` | 「外科的ライタ（`set_field` 等）は `XxxRepositoryImpl` の内部詳細に限り」 | `with_field_if_present` 等。ただしこれは**一般則の例示**なので、具体名を消して「外科的ライタ」だけにする選択肢もある |

`factory-naming.md:43` の `` `StorePath::of(root, space)` は一度採用したが `for_space` へ戻した ``
という記述は、**F8 でコードが `for_space` に戻ったので実態と一致した**（ドリフト解消）。

### 4.3 `aidlc/spaces/default/codekb/docs/` — 2 箇所（自動生成物の可能性あり）

| 場所 | 現在の記述 |
| --- | --- |
| `architecture.md:136` | 「純関数群 `get_field`/`set_field` 等」 |
| `api-documentation.md:27` | 「状態ファイル純関数サービス 10 本: `get_field` / `set_field` / `set_field_strict` / `set_or_insert_field` / `remove_field` / `parse_checkboxes` / `set_checkbox` / `count_completed` / `classify_state_version` / `reap_eligible`」 |

CodeKB は再スキャンで再生成される性質のものと思われるので、手で直すか再生成に任せるかは
コンダクタの判断。

### 4.4 本 Unit（U3）の設計文書 — 6 箇所

| 場所 | 現在の記述 | 状態 |
| --- | --- | --- |
| `nfr-design/logical-components.md:15` | `` `StorePath::of` `` | **要修正** → `StorePath::for_space` |
| `nfr-design/logical-components.md:56` | 「composition root（U7）が `StorePath::of` と Clock を配線する」 | **要修正** |
| `functional-design/functional-spec.md:37` | `` `StorePath::of(aidlc_root: &Path, space: &SpaceName)` `` | **要修正** |
| `functional-design/rules.md:58` | BR の statement 内に `StorePath::of(aidlc_root, &SpaceName)` | **要修正**（BR 文面なので慎重に） |
| `functional-design/pending-revision.md:25` | 「`StorePath::for_space` → `StorePath::of`（複数の値を集約 = `of`）」 | **この改訂項目自体が F8 で撤回された。** 項目を取り消し扱いにする必要がある |
| `functional-design/entities.md:251` | `` `WorkflowExecution::version()` / `apply_event` / `start_with_entries` の実コード確認 `` | **要修正** → `start_from_plan_unchecked` |

`code-generation-plan.md:33` は既に `StorePath::for_space` と書いてあり、F8 で**コードが計画に
追いついた**（修正不要）。

### 4.5 過去の委任記録（凍結記録。修正不要と判断）

`u2-domain-es-core` / `u3-event-store-repository` の `developer-brief-N.md` /
`developer-report-N.md` / `code-summary.md` / `unit-test-instructions.md` /
`code-generation-questions.md` に旧名の出現がある（`start_with_entries` が中心、
`developer-report-7.md:53` には `every_closed_set_field_...` テスト名）。これらは**その時点の
作業記録**なので、後から書き換えると記録としての価値が損なわれる。修正しない前提で列挙のみ。

---

## 5. 検査結果（実測）

すべてリポジトリルート `/Users/j5ik2o/orca/workspaces/amadeus-ng/docs` で実行。

| 検査 | 結果 |
| --- | --- |
| `cargo fmt --all --check` | **緑**（差分 0。改名で 3 ファイルに整形差分が出たので `cargo fmt --all` を適用済み） |
| `cargo clippy --workspace --all-targets -- -D warnings` | **緑**（warning 0、`Finished dev profile`） |
| `cargo lint` | **緑**（exit 0、出力なし） |
| `PROPTEST_RNG_SEED=20260823 cargo test --workspace` | **674 passed / 0 failed**（委任前と同数。減っていない） |

### 5.1 旧名 grep

ブリーフ指定の grep を**そのまま**実行すると 38 行が残る。内訳は次の 3 種で、
**旧名の残骸は 0 件**である。

```
grep -rnE "set_field|get_field|remove_field|set_checkbox|to_entry|read_error_message|corrupt\(|start_with_entries|StorePath::of|open_exclusive_new|CheckboxWriteError" --include='*.rs' modules/
→ 38 行
```

| 内訳 | 行数 | 説明 |
| --- | --- | --- |
| **`graph_read_error_message`** | 10 | パターン `read_error_message` が **F5 の新名そのものに部分一致**する。パターン側の自己一致であり残骸ではない |
| **末尾が `_corrupt()` のテスト関数名** | 17 | パターン `corrupt\(` が `a_gap_in_the_replayed_journal_is_corrupt(` 16 本と `..._folds_into_corrupt(` 1 本に部分一致する。ドメイン語の述語として正しいテスト名であり、改名対象ではない |
| **`WorkflowExecution::set_checkbox`** | 11 | §3(a) の `&mut self` コマンド。改名しないと判断した実体 |

内訳の合計は 10 + 17 + 11 = 38 で、全 38 行が説明できている。
偽陽性 2 種を除いた厳密版の grep（`_` 直後の一致を除外し、F5 の新名を除く）:

```
grep -rnE "(^|[^_[:alnum:]])(set_field|get_field|remove_field|set_checkbox|to_entry|read_error_message|corrupt|start_with_entries|open_exclusive_new|CheckboxWriteError)\(|StorePath::of\b" --include='*.rs' modules/ \
 | grep -v "graph_read_error_message" | grep -v "_is_corrupt("
→ 11 行（すべて WorkflowExecution::set_checkbox）
```

`set_checkbox` を除外すれば **0 件**になる。

### 5.2 付随して直したテスト関数名（挙動不変）

grep を通すため、および旧名が読み手を誤導しないために、次のテスト関数名を追随させた。

- `state_writers.rs`: `get_field_returns_...` → `find_field_returns_...` ほか計 5 本
- `checkbox.rs`: `set_checkbox_edits_only_the_marker_...` → `with_checkbox_marker_edits_only_the_marker_...` ほか 2 本
- `workflow_definition_repository_impl.rs`: `read_error_message_renders_every_variant` → `graph_read_error_message_renders_every_variant`
- `atomic.rs`: `open_exclusive_new_refuses_an_existing_path` → `create_new_file_refuses_an_existing_path`
- `message-catalog/lib.rs`: `*_is_verbatim` 6 本を新名に追随
- **`workflow_definition_repository_impl_test.rs:949`**:
  `every_closed_set_field_is_reported_as_malformed_with_the_key_that_caused_it`
  → `every_enum_valued_field_is_reported_as_malformed_with_the_key_that_caused_it`
  これは**偶然の部分一致**である（「closed set」＝閉じた列挙 ＋「field」であって `set_field` とは無関係）。
  grep を 0 に近づけるために言い換えたもので、テストの意味は変えていない（「閉じた集合の値を取る
  フィールド」＝「列挙値のフィールド」）。**元に戻したければ戻せる。**

---

## 6. 設計質問 / 未了

1. **§2 の 2 件（ブリーフ外の改名）の可否。** `event_store_impl.rs` の 3 つ目の `corrupt` と
   `message_catalog::lock::acquire_failed`。いずれも「同じ欠陥を残すと F6 / F10 の理由が成立しなく
   なる」という判断で直した。差し戻す場合は該当の 2 コミット相当を戻すだけでよい（呼出側は
   それぞれ 5 箇所 / 4 箇所）。
2. **`docs/specs/12-workflow-definition.md:242` の扱い。** ここに並ぶ
   `state::field_not_found` 等は「文言カタログのエントリ名」なのか「Rust の関数名」なのか。
   前者なら変更不要、後者なら `_message` を付ける必要がある。仕様の意図が読み取れなかった。
3. **`functional-design/pending-revision.md:25` の項目 10 の取り消し。** この項目は
   「`for_space` → `of` へ改名する」という改訂指示であり、F8 でその改訂自体が撤回された。
   単に文言を直すのではなく、**項目を取り消し扱いにする**必要がある。
4. **`state_writers` モジュール名（§3(b)）。** 中身が `with_*` になった今、名前が実態から離れた。
   仕様（`11-workspace.md:64` の表の行名）と連動するので、別の委任で扱うべきか判断を仰ぐ。
5. **監査 §4.2-6 の `EventStore::get_latest_snapshot_by_id` / `get_events_by_id_since_seq_nr`。**
   `get_` 接頭辞（C-GETTER 違反）だが、監査自身が「trait のメソッドでありファクトリではない」として
   §2 から外している。ブリーフの 10 件にも無いので**触っていない**。event-store-adapter-rs の語彙を
   写した意図的選択なら、その免除をどこかの正本に記録しておくべき（監査も「免除の記録がどの正本にも
   見当たらなかった」と書いている）。

**未了なし。** ブリーフの 10 件は全件是正し、検査 4 種すべて緑である。
