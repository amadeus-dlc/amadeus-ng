# ファクトリ命名の監査 — 名前と振る舞いの乖離

**監査者**: aidlc-architecture-reviewer-agent
**日付**: 2026-08-23T23:19:55Z（UTC、`date -u` 実測）
**判定**: NOT-READY — 該当 **10 件**（High 2 / Medium 5 / Low 3）
**基準**: `aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/factory-naming.md`（裁定日 2026-08-24 の更新版。「本表は『他に言うことが無いとき』の既定」「正確なドメイン語が勝つ」「やってはいけない機械化」節を反映済み）
**変更**: 本ファイル以外は一切変更していない（コード・設計文書とも未変更）。

**挙げてよい違反の定義**（コンダクタ指示）:

- **(A)** 現在の名前が**振る舞いを誤って伝えている**（`read_state` が I/O に見えたのが実例）
- **(B)** 現在の名前が**何も語っていない**（`make` / `construct` / `build_new` のように、表の動詞に置き換えても情報が減らない）

「表の動詞（`new` / `of` / `from_*` / `parse` / `create` / `generate` / `open`）になっていない」だけでは違反として挙げていない。正確なドメイン語を表の動詞へ矯正する提案は改悪なので出していない。

---

## 1. 走査した範囲

### 1.1 対象クレート

`modules/` 配下のプロダクトコードのみ。`#[cfg(test)]` 内と `tests/` ディレクトリは除外。指示どおり `.claude/` と `tools/lint` は不参照。

| クレート | パス | 状態 |
| --- | --- | --- |
| core-domain | `modules/core/domain/src/` | 全 42 ファイル走査 |
| core-use-case | `modules/core/use-case/src/` | 全 9 ファイル走査（ポート trait 定義 4 本を含む） |
| core-interface-adapter | `modules/core/interface-adapter/src/` | 全 14 ファイル走査 |
| infra-io | `modules/infra-io/src/` | 全 4 ファイル走査 |
| shared/audit-events | `modules/shared/audit-events/src/` | 走査 |
| shared/canon-json | `modules/shared/canon-json/src/` | 全 7 ファイル走査 |
| shared/directive-schema | `modules/shared/directive-schema/src/` | 走査 |
| shared/message-catalog | `modules/shared/message-catalog/src/` | 走査 |
| harness/claude | `modules/harness/claude/src/lib.rs` | 走査 — **関数定義ゼロ**（マニフェストデータのみ） |
| app/aidlc | `modules/app/aidlc/src/main.rs` | 走査 — **`const fn main()` のスタブのみ**、フェーズ A 未着手 |

### 1.2 抽出方法

全 `.rs` を `awk` で走査し、`#[cfg(test)]` の出現行以降を捨てたうえで次の 2 パターンを別々に抽出した。

1. `pub` / `pub(crate)` / `pub(super)` + `fn`
2. 同 + `const` / `async` / `unsafe` / `extern` + `fn`

**(2) を最初に取り漏らしていたため再走査している**（`pub const fn` が最初の正規表現から漏れていた。気づいて追加走査済み）。加えて、trait 定義内の `pub` を持たないメソッド（use-case 層の 4 ポート）を手動で確認した。

### 1.3 目視した件数

- 公開関数・メソッド宣言: **489 件**（パターン(1) 323 + パターン(2) 166）＋ use-case 層の trait メソッド **10 件** = **計 499 件**
- うち**ファクトリと分類したもの: 約 150 件**。抽出結果からの実測内訳は次のとおり。
  - `new`: 54
  - `parse` 系: 37（型の関連関数 27 + `wire` モジュールの `parse_*` 自由関数 10）
  - `from_*`: 8
  - `of`: 3
  - `open`: 4
  - ドメイン語・自由関数ファクトリ: 約 40
  - ビルダー終端 `build`: 2
- ファクトリ判定から外したもの: `&self` を取るアクセサ・述語・変換メソッド（`as_str` / `is_*` / `to_*` メソッド）、`&mut self` のコマンド、`with_*` の消費型ビルダー約 12 本（正本の「対象外」節に従う）

### 1.4 実装本体を読んで確認したファイル（名前だけで判断していない箇所）

`state_writers.rs` / `checkbox.rs` / `bolt_refs.rs` / `state_field_value.rs` / `state_version.rs` / `jump_direction.rs` / `plan_action.rs` / `stage_number.rs` / `stage_node.rs` / `scope_grid.rs` / `scope_metadata.rs` / `stage_graph.rs` / `workflow_execution.rs` / `autonomy_mode.rs` / `clock.rs` / `event_store_impl.rs` / `in_memory_event_store.rs` / `store_path.rs` / `schema.rs` / `wire/mod.rs` / `wire/event_wire.rs` / `wire/state_wire.rs` / `workflow_definition_repository_impl.rs` / `workflow_execution_repository_impl.rs` / `state_file_io.rs` / `append_only.rs` / `atomic.rs` / `fs_meta.rs` / `digest.rs` / `value.rs` / `parse.rs` / `writer.rs` / `profile.rs` / `message-catalog/lib.rs` / `directive-schema/lib.rs` / `audit-events/lib.rs`、および use-case の 4 ポート trait。

### 1.5 横断確認の結果（正本の「機械化してよい 3 つ」に対する実測）

| 検査 | `modules/` 内の該当 |
| --- | --- |
| inherent な `fn from(` | **0 件** |
| `get_` 接頭辞のファクトリ | **1 件**（下表 F4） |
| 同じ型に `new` と `try_new` が共存 | **0 件**（`try_new` 自体が 0 件） |

---

## 2. 違反（(A) 名前が振る舞いを誤って伝える / (B) 名前が何も語っていない）

**該当 10 件。0 件ではない。**

| # | 深刻度 | 場所（ファイル:行） | 現在の名前 | 実際の振る舞い | 類型 | なぜ誤解を招くか | 提案する名前 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| F1 | **High** | `modules/core/domain/src/workspace/state_writers.rs:20` | `set_field(content, field, value) -> String` | 該当行があれば置換した**新しい String** を構築して返す。**該当行が無ければ入力をそのまま複製して返す**（無言 no-op） | (A) | 「フィールドを設定した」と伝えるが、設定できなかった場合も成功時とまったく同じ形の `String` が返り、呼び手には区別する手段が無い。モジュール冒頭の doc 自身が「無言 no-op は検出不能なドリフト」と書いており、名前がその危険を隠している | `with_field_if_present()` — 「写しを作る」ことと「無ければ何もしない」ことの両方を名前に出す |
| F2 | **High** | `modules/core/interface-adapter/src/orchestration/wire/mod.rs:348` | `StageEntryWire::to_entry(value: &JsonValue) -> Result<StageEntry, CorruptCause>` | `JsonValue` を解析して `StageEntry` を作る**関連関数**。`StageEntryWire` を構築も参照もしない | (A) | `to_*` は C-CONV の「このレシーバを変換する」という約束。self が無く、実際の源は `JsonValue`。同じ impl の `from_entry`（:338、`&StageEntry -> StageEntryWire`）と対の**逆変換に見えるが逆変換ではない**ため、呼び手はまず `StageEntryWire` を作ろうとして詰まる。同ファイルには `parse_slug` / `parse_phase` / `parse_plan` という正しい綴りの兄弟が既に 10 本ある。※可視性は `pub(super)` なので影響範囲はモジュール内 | `parse_entry(value)`（同ファイルの `parse_*` 群に揃える） |
| F3 | Medium | `state_writers.rs:51` / `:79` / `:121`、`modules/core/domain/src/workspace/checkbox.rs:165` | `set_field_strict` / `set_or_insert_field` / `remove_field` / `set_checkbox` | いずれも純粋な `&str -> String`。引数を書き換えず、**新しい文字列を構築して返す** | (A) | 命令形の動詞は「呼べば対象が変わる」と伝えるが、可変対象が存在しない。`set_checkbox` はエラー型名 `CheckboxWriteError` が「書込」を重ねて主張しており誤解を強める | `with_field()` / `with_field_or_insert()` / `without_field()` / `with_checkbox_marker()`、エラー型は `CheckboxUpdateError` |
| F4 | Medium | `state_writers.rs:10` | `get_field(content, field) -> Option<String>` | 全行を走査し、最初の一致行の値を trim して**新しい String** を構築 | (A) | `get_` は C-GETTER 違反。正本の「機械化してよい 3 つ」の第 2 項が **「`get_` 接頭辞のファクトリ — 正当な例外は無い」**と名指ししており、`modules/` 内でここが唯一の該当箇所（他の `get_*` は `EventStore` trait のメソッドでファクトリではない）。加えて self のアクセサではなく全行スキャンで、`get` の O(1) 参照取得という含意とも合わない | `find_field()` |
| F5 | Medium | `modules/core/interface-adapter/src/orchestration/workflow_definition_repository_impl.rs:94` | `read_error_message(error: &GraphReadError) -> String` | `GraphReadError` の変種で分岐し、逐語文言 `String` を組み立てる純関数（I/O なし） | (A) | 同一ファイルの private が `load_harness_identity`(:412) / `load_graph`(:429) / `load_grid`(:457) / `load_scopes`(:484)、隣モジュールが `read_user_version` という I/O 群なので、先頭の `read_` が動詞として読める（「error_message を読む」）。さらに同ファイルの兄弟 2 本は `stage_graph_not_readable_message`(:71) / `stage_graph_invalid_json_message`(:88) と「主語＋条件＋`_message`」で揃っており、**この 1 本だけ主語が無く I/O 動詞で始まる** | `graph_read_error_message()`（兄弟 2 本の規約に揃える） |
| F6 | Medium | `wire/mod.rs:40` | `corrupt(aggregate_id, seq_nr, cause) -> EventStoreError` | `EventStoreError::Corrupt` を組み立てて返す | (B) | 裸の形容詞（あるいは他動詞）で、**何を作るのかを何も語っていない**。`corrupt(...)` は「壊す」とも「壊れているか」とも読める。同じ綴りの private 版が `memory/in_memory_event_store.rs:76` にもあり、モジュールをまたいで同名が並ぶ | `corrupt_error()` — 作るものを名前に出す |
| F7 | Medium | `modules/core/domain/src/orchestration/workflow_execution.rs:156` | `WorkflowExecution::start_with_entries(...)` | `start`(:103) の委譲先。**スコープ名の妥当性検査を行わない**（定義を持たないので `UnknownScope` を返せない） | (A) | 集約 genesis の公開入口が 2 つあり、正本の禁止パターン「同じ用途に複数の入口を残す」に該当。名前が語るのは「何を渡すか（entries）」だけで「何を失うか（`UnknownScope` 検査）」を語らないため、entries を持っている呼び手は自然にこちらを選び、検査を静かに落とす | `start_from_plan_unchecked()` — Rust 慣用の `_unchecked` で「検査を落とした入口」だと名前に出す |
| F8 | Low | `modules/core/interface-adapter/src/orchestration/store_path.rs:31` | `StorePath::of(aidlc_root, space)` | `spaces/<space>/intents/<固定ファイル名>` を 5 段 `join` で**導出** | (B)-lite | `of` は「与えた値を集約して包む」に読めるが、実体は固定レイアウトの導出。加えて**正本 `factory-naming.md` の適用例行が `StorePath::for_space` と記録しているのにコードは `of`** で、正本とコードが食い違っている（どちらかを直す必要がある） | `StorePath::for_space()`（正本の記述に合わせる） |
| F9 | Low | `modules/infra-io/src/atomic.rs:68` | `open_exclusive_new(path) -> io::Result<File>` | `OpenOptions::new().write(true).create_new(true).open(path)` | (A)-lite | Rust では `open` は既存資源を開く含意が強い（`File::open`）。この関数は逆に**既存なら `AlreadyExists` で拒否する**。std がこの動作に与えている名前は `create_new` | `create_new_file()` |
| F10 | Low | `modules/shared/message-catalog/src/lib.rs:35,47,80,97,130` | `field_not_found` / `file_not_found` / `acquire_failed_for_key` / `merge_acquire_failed` / `invalid_mode` | いずれも逐語文言の `String` を組み立てる | (B)-lite | 条件を述べる名詞句なので述語に読める（`if field_not_found(f)` と書けそうな形）。同一リポジトリの `stage_graph_not_readable_message` は `_message` を付けており、規約が 2 つに割れている。ただし `message_catalog::state::` というモジュールパスが意味の大半を運んでいるため実害は薄い。加えて `field_not_found` は `state_writers.rs:26` のエラー型 `FieldNotFound` と綴りが衝突し、`FieldNotFound::new(msg::field_not_found(field))` という読みにくい呼出になっている | `*_message` を付けて `stage_graph_*_message` 側に揃える |

### 2.1 スコープ判断の明示（採否の判断材料）

F1・F3・F4 の 5 本は「新しい `String` を構築して返す自由関数」であり、コンダクタの定義「値を新規に構築して返す自由関数」に照らしてファクトリとして扱った。**文字列を「インスタンス」に数えない立場を採るなら、この 5 本は丸ごとスコープ外になる。** 判断はコンダクタに委ねる。

---

## 3. 正当な例外として据え置くべきファクトリ

表の動詞（`new` / `of` / `from_*` / `parse` / `create` / `generate` / `open`）になっていない、あるいは表の動詞を表どおりでない用途に使っているが、**現在の名前のほうが良い**と判断したもの。矯正案は出さない。

### 3.1 正本が明示的に良い例として挙げているもの（実装を読んで再確認済み）

| # | 場所 | 名前 | なぜ現在の名前のほうが良いか |
| --- | --- | --- | --- |
| K1 | `canon-json/src/digest.rs:65` / `:74` | `hash_canonical` / `hash_compact` | 「どのプロファイルで sha256 するか」を語っている。`Digest::generate` にすると 2 本の区別が消える |
| K2 | `canon-json/src/writer.rs:19` | `serialize(value, profile) -> String` | 直列化という操作そのものが名前。`of` は「何かを作る」としか言わない |
| K3 | `canon-json/src/value.rs:116` | `to_value<T: Serialize>(..)` | serde 生態系の確立語で、`JsonValue` へ写すことを正確に言っている |
| K4 | `domain/orchestration/workflow_execution.rs:103` | `WorkflowExecution::start` | 集約 genesis のドメイン語。`create` では「ワークフローが始まる」という出来事が消える |
| K5 | `interface-adapter/orchestration/event_store_impl.rs:233` | `EventStoreImpl::open` | 外部資源のハンドル取得で `File::open` と同型。表にも `open` として載っている |

### 3.2 表に無いが「何をするか」を正確に述べている（表の動詞へ矯正すると情報が減る）

| # | 場所 | 名前 | なぜ現在の名前のほうが良いか |
| --- | --- | --- | --- |
| K6 | `domain/workspace/state_version.rs:41` | `classify_state_version(content) -> StateVersionClassification` | 「分類する」という操作が名前になっており、doc の「唯一の分類器 (runtime / doctor の双方がこれを呼ぶ)」がそのまま読める。`StateVersionClassification::of` は何を基準に分類したかを消す |
| K7 | `interface-adapter/orchestration/event_store_impl.rs:97` | `map_sqlite_error(error, path) -> EventStoreError` | 「どの外界のエラーを写すか」を名前が特定している。`map_` は写像であって復元でないことまで言えており、`from_rusqlite` よりむしろ勝る |
| K8 | `wire/event_wire.rs:160,179`、`wire/state_wire.rs:68,83` | `encode` / `decode` | ワイヤ境界の確立語で、方向が名前だけで決まる。`to_*` / `from_*` へ矯正すると 2 型 4 本の対称性が崩れる |
| K9 | `wire/mod.rs:252,272,289,306` | `checkbox_token` / `autonomy_token` / `status_token` / `direction_token` | 「ワイヤ上のトークンを作る」ことを語っている。`of` にすると 4 本が区別できない |
| K10 | `wire/mod.rs:222-247`（10 本） | `parse_slug` / `parse_intent_id` / `parse_definition_id` / `parse_definition_revision` / `parse_phase` / `parse_plan` / `parse_checkbox` / `parse_autonomy` / `parse_status` / `parse_direction` | 自由関数だが `parse_<作る型>` で用途と生成物の両方を言えている。F2 が本来揃うべき規約でもある |
| K11 | `wire/mod.rs:56` / `:65` | `to_canonical_json` / `parse_json` | 「どの正準形へ／から」を名前が特定している |
| K12 | `infra-io/src/append_only.rs:21` | `open_append_only(path) -> io::Result<File>` | 「追記専用で開く」という、返り値の型（`File`）だけでは絶対に分からない契約を名前が運んでいる。`open` だけでは足りない |
| K13 | `infra-io/src/fs_meta.rs:29` | `dev_ino(file) -> io::Result<(u64,u64)>` | 作る値そのもの（`(dev, ino)` の組）が名前。表の動詞では何の組か言えない |
| K14 | `interface-adapter/workspace/state_file_io.rs:66` | `read(path) -> Result<String, StateFileReadError>` | `state_file_io::read(..)` として読み、モジュール名が主語を運んでいる。実際に I/O をするので `read` が正しい |
| K15 | `workflow_definition_repository_impl.rs:71` / `:88` | `stage_graph_not_readable_message` / `stage_graph_invalid_json_message` | 「主語＋条件＋作るもの」が全部入っている。F5 が揃うべき手本 |
| K16 | `domain/workspace/checkbox.rs:126` | `parse_checkboxes(content) -> Vec<CheckboxEntry>` | 複数を作ることまで複数形で言えている |

### 3.3 表の動詞を「表どおりでない用途」に使っているが、読みが自然で誤解が無い

| # | 場所 | 名前 | なぜ現在の名前のほうが良いか |
| --- | --- | --- | --- |
| K17 | `domain/orchestration/jump_direction.rs:20` | `JumpDirection::of(cursor, target)` | 表の `of` は「集約して生成」だが、ここは比較からの**導出**。それでも「この 2 つに対する方向」と自然に読め、誤解が生じない |
| K18 | `domain/workflow_definition/scope_grid.rs:39` | `ScopeGrid::from_graph(graph)` | 単純変換ではなく転置導出（initialization 特例つき）だが、`from_<源>` は源を正確に言っており嘘が無い。`transposed_from_graph` のほうが情報は多いが、現在の名前も誤解を招かない |
| K19 | `domain/orchestration/autonomy_mode.rs:27` | `AutonomyMode::from_state_field(Option<&str>)` | **今回の発端（旧 `read_state`）の是正結果を再確認。** `from_<源>` で源が state ファイルのフィールド値だと言えており、doc も「I/O はしない（読み終わった値を写像するだけ）」を明記。適合と判断 |
| K20 | `interface-adapter/orchestration/event_store_impl.rs:245` | `open_with_busy_timeout(path, clock, busy_timeout)` | 「同じ用途に複数の入口」に見えるが、`open` がこれに既定値を渡して委譲する `X` / `X_with_<差分>` の標準形（`Connection::open` / `open_with_flags`）。名前が差分そのものを言っているので取り違えようがない |
| K21 | `canon-json/src/parse.rs:55` / `:70` | `parse` / `parse_bytes` | 同上。入力型の差（`&str` / `&[u8]`）を名前が言っており、`str`/`bytes` 二口は Rust の標準的な分け方 |
| K22 | `domain/workflow_definition/plan_action.rs:30`、`shared/audit-events/src/lib.rs:44`、`shared/directive-schema/src/lib.rs:74` | `parse(s) -> Option<T>` | 表の綴りは `Result<Self, E>` だが、いずれも**閉じた 2〜12 語の集合で、失敗理由が「集合の外」以外に存在しない**。エラー型に載せる情報が無いため `Option` が正しく、doc も「既定値へフォールスルーさせない」と意図を書いている |
| K23 | `domain/workflow_definition/scope_metadata.rs:145` ほか計 54 箇所 | `new(..) -> Self` / `new(..) -> Result<Self, E>` | `ScopeMetadata::new` は検証つきだが `try_new` を並立させず `new -> Result` に一本化しており、正本どおり。`modules/` 内に `try_new` は 1 件も存在しない（実測） |
| K24 | `domain/orchestration/workflow_execution_state.rs:277`、`workflow_definition/stage_node.rs:733` | `build(self) -> T` | 正本が「対象外（ビルダーパターンの語）」と明記 |
| K25 | `use-case/orchestration/repository_error.rs:62`、`wire/mod.rs:338`、`canon-json/src/parse.rs:120` | `from_event_store` / `from_entry` / `from_serde` | いずれも inherent な `from_<源の名前>` で、正本の指定どおりの綴り。素の `fn from(` は `modules/` 内に 1 件も無い（実測） |

**据え置き件数: 25 件。** 特に K6 / K7 / K12 / K9 の 4 系統は、表の動詞へ矯正すると情報が確実に減るため、将来 lint ルールを書く際の**反例カタログ**としてそのまま使える。

---

## 4. 未確認・判断に迷った点

### 4.1 未確認（読んでいない・推測で埋めていない）

- `modules/shared/canon-json/src/canonical.rs` — `member_order`（`pub(crate)`、`Vec<usize>` を返す）だけを抽出結果で見て、ファクトリではないと判断した。**本体は読んでいない。**
- `modules/core/domain/src/orchestration/` のエラー型 4 本（`apply_error.rs` / `command_error.rs` / `start_error.rs` / `state_error.rs`）— 公開関数の抽出結果がゼロ（`Display` 実装のみ）だったため**本体を読んでいない**。ファクトリは無いはずだが目視確認はしていない。
- `modules/shared/audit-events/src/lib.rs` — `parse` / `as_str` / `category` / `is_mandatory` / `is_cli_protected` の 5 本のみ確認し、**定数テーブル部分は読んでいない**。

### 4.2 判断に迷った点

1. **F1 / F3 / F4 をファクトリに数えるか**（§2.1 に再掲）。`String` を新規構築する自由関数をファクトリと見るかで、5 本が丸ごと入るか外れるかが決まる。コンダクタの定義に素直に従って「入る」としたが、これは判断であって自明ではない。
2. **F2 の深刻度**。「名前を信じて呼ぶと間違った使い方をする」という High の定義には文字どおり当てはまる（`StageEntryWire` を作ろうとして詰まる）が、可視性が `pub(super)` で影響範囲がモジュール内に閉じている。High と Medium の境界にあり、High に倒した。
3. **F7 を (A) と呼べるか**。`start_with_entries` は「entries を渡して開始する」という説明としては嘘をついていない。誤っているのではなく**言い足りていない**（検査を落とすことを語らない）。(A) と (B) のどちらにも完全には収まらず、正本の禁止パターン「同じ用途に複数の入口を残す」に該当する点を根拠に (A) として挙げた。
4. **K17 `JumpDirection::of` の引数**。命名としては据え置きでよいと判断したが、**2 引数とも裸の `usize` なので取り違えると Forward/Backward が静かに反転する**（`StageIndex` 型が既に存在するのに使っていない）。これは命名ではなく型の問題であり、本監査のスコープ外として据え置き側に置いた。別途の検討に値する。
5. **K18 `ScopeGrid::from_graph`**。`transposed_from_graph` のほうが情報量は多い。現在の名前が「誤解を招く」とまでは言えないので据え置きにしたが、改善余地があること自体は事実。
6. **`EventStore::get_latest_snapshot_by_id` / `get_events_by_id_since_seq_nr`**（`use-case/orchestration/event_store.rs:55,62`）。`get_` 接頭辞（C-GETTER 違反）だが、**trait のメソッドでありファクトリではない**ため §2 には挙げていない。event-store-adapter-rs の語彙を写した意図的選択の可能性があるが、**その免除の記録がどの正本にも見当たらなかった**。ファクトリ以外も対象にするなら再検討の対象。

---

## 5. 総括

ファクトリに絞ると該当 **10 件**（High 2 / Medium 5 / Low 3）。

**根は 1 つ** — upstream（TypeScript）の可変オブジェクト前提の関数名が、Rust の「新しい値を作って返す」自由関数に貼られたまま残っていること。F1・F3・F4 の 5 本がこれで、`set_*` / `get_*` という命令・取得の動詞が、実際には `String` を新規構築するファクトリに付いている。F1 だけが High なのは、名前が誤解を招くうえに**失敗しても成功と同じ形の値を返す**から。

F2 は別系統で、`to_*` という C-CONV の綴りをレシーバ無しの関連関数に使ったために、対になっている `from_entry` との関係が読めなくなっている。同じファイルに `parse_slug` / `parse_phase` という正しい兄弟が 10 本もあるので、揃えるだけで済む。

機械強制については、正本の「やってはいけない機械化」節に全面的に同意する。今回の 10 件のうち **`cargo lint` に落とせるのは F4（`get_` 接頭辞のファクトリ）1 本だけ**で、残り 9 件はレビューでしか捕まらない。逆に言えば F4 は、正本が「正当な例外は無い」と断じた 3 検査のひとつに実コードで該当した唯一の実例なので、赤例テスト付きルールの最初の 1 本として素性が良い。他の 2 検査は `modules/` 内に該当ゼロを実測済み（§1.5）。

---

## 付録 A — 対象外の参考所見（ファクトリ以外。採否は別途判断）

初回依頼の 8 類型のうち、②可変/問い合わせの取り違え・④コストを偽る名前・⑤失敗を偽る名前・⑥ドメイン語との乖離・⑦同義語の並立・⑧否定の二重化で挙げていたもの。ファクトリでないため本監査のスコープ外だが、作業済みのため捨てずに残す。

| 場所 | 名前 | 所見 |
| --- | --- | --- |
| `domain/workflow_definition/stage_node.rs:464` | `enabled(&self) -> Option<bool>` | **`None` が「有効」を意味する。** `enabled().unwrap_or(false)` が既定ケースで答えを反転させる。正しい述語は隣の `is_enabled()`(:470)。前回 High 判定 |
| `interface-adapter/orchestration/event_store_impl.rs:312` / `memory/in_memory_event_store.rs:129` | `journal_is_empty` | 同名で片方は `SELECT COUNT(*)`（`Result<bool>`）、片方は純粋（`bool`）。`is_empty` の O(1) 全域述語という含意に反する |
| `domain/orchestration/workflow_execution.rs:931` | `state(&self) -> WorkflowExecutionState` | Vec 6 本の `clone()` + 2 本の走査生成。周囲は全部 `const fn` の Copy ゲッタ。C-CONV なら `to_state()` |
| `domain/workspace/bolt_refs.rs:54` | `emit(&self) -> String` | 副作用なし。`parse` の対なら `Display` / `to_*` |
| `use-case/orchestration/event_store.rs:55,62` | `get_latest_snapshot_by_id` / `get_events_by_id_since_seq_nr` | `get_` 接頭辞（C-GETTER）で非同期 I/O。ES 拡張語彙として正本が明示許可しているのは `store` / `find_by_id` だけ（ADR-006）。免除の記録が無い（§4.2-6） |
| `domain/workflow_definition/stage_node.rs:371` | `workspace_requires() -> bool` | 主語と目的語が逆。兄弟の `StageMode::requires_support_agents()`(:86) が正しい語順 |
| `interface-adapter/clock.rs:56,62` | `FakeClock::set(&self)` / `advance(&self)` | `AtomicU64` を `&self` 越しに書き換え。`interior-mutability.md` の「`&self` への偽装」禁止に該当し、doc に例外の理由が無い |
| `wire/mod.rs:98` | `WireObject::only(allowed) -> Result<(), _>` | フィルタに読めるが実体は表明（未知キー拒否） |
| `wire/mod.rs:73` | `exact_integer(u64) -> Result<u64, _>` | 変換器に読めるが実体はガード（入力をそのまま返すのでファクトリではない） |
| `domain/workspace/state_field_value.rs:31` | `unsafe_line_char(s) -> Option<char>` | 名詞句だが実体は先頭からの探索。`unsafe` は `forbid(unsafe_code)` 環境で紛らわしい語 |
| `domain/workflow_definition/stage_number.rs:86` vs `:121` | `numeric_cmp` と `impl Ord::cmp` | 順序の入口が 2 つあり、`sort()` は静かに「文書順が残らない」方を選ぶ。doc が警告で打ち消している |
| `scope_grid.rs:62,69` vs `workflow_definition.rs:158,164` | `scope_names` / `contains_scope` と `valid_scopes` / `is_valid_scope` | 名前が近すぎ、両方の doc が「**〜ではない**」と打ち消している |
| `stage_graph.rs:97,107` ほか計 8 本 | `numeric_order` / `declared_scopes` ほか | 無印名詞のアクセサ名で毎回 Vec 確保＋ソート／dedup |
| `workflow_definition_repository_impl.rs:387,395,404` | `stage_graph_path` ほか | フィールド取得に読めるが override 解決＋確保 |
| `workflow_execution.rs:319` / `:341`(private) / `stage_entry.rs:63` | `gated` / `is_gated` / `is_gated` | 同じ問いに 3 入口。公開版だけ `is_` が無い |
| `workflow_execution.rs:253` | `stage_index(usize) -> Option<StageIndex>` | アクセサ名だが実体は範囲検査つき生成（doc 曰く「作る唯一の公開経路」）。ファクトリ寄りだが `&self` メソッドなので本監査では除外 |
| `workflow_execution.rs:1088,1117` | `jump_resolve` / `stale_report` | 一方は動詞後置、他方は名詞句。どちらも `&self` の判定クエリ |
| `canon-json/src/digest.rs:53` | `rendered() -> String` | 隣の `hex()` が `&str` なのに毎回確保 |
| `infra-io/src/atomic.rs:18` vs `state_file_io.rs:80` | `write_file_atomic` / `write_atomic` | 語順違いのほぼ同名が 2 層に並ぶ |
| `directive-schema/src/lib.rs:81` | `is_placeholder()` | 真の意味は「エンジンが構築してはいけない」。真偽の向きを取り違えやすい |
| `workflow_execution_state.rs:193-256`、`stage_node.rs:576-726` | ビルダーのセッタ群 | 同一モジュールで同じ識別子が「読む」と「書く」の両方を指す（Rust ビルダーとしては慣用寄り） |
