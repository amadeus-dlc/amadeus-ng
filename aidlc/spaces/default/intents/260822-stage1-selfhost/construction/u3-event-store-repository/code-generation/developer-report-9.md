# 委任 9 報告 — 契約テスト装置の意味論の是正 (U3 / Bolt B5)

**Unit**: `u3-event-store-repository`
**担当**: aidlc-developer-agent
**Testing Contract**: `sha256:303d9bb7b5d777d54a6761be9ed154d85d5bb3f2d6b9cce02f71f4ed1b3a4ff3`（TDD: red-green-refactor）
**所有ファイル**: `modules/core/interface-adapter/tests/support/mod.rs`、
`modules/core/interface-adapter/tests/support/contract.rs`、
`modules/core/interface-adapter/tests/workflow_execution_repository_contract.rs`、本報告

**プロダクトコード（`src/**`）は 1 行も変更していない**（`EventStoreImpl::path()` が
既に Query として存在したため、追加は不要だった）。

---

## 1. Red — 現状の分岐を検出する失敗テスト

まず「両実装が同じ観測になるか」を問う契約テストを 3 本書き、両実装に流し込んだ。
アーキテクチャレビュー所見 (a)(c) に加え、**同じ形の分岐が `reopen()` にもある**ことを
疑ったので、`reopen` 用の検出テストも同時に書いた。

- `open_twice_yields_independent_empty_stores` — (a) の検出
- `reader_ignores_writes_made_after_it_was_opened` — (c) の検出（暫定名）
- `reopened_ignores_writes_made_after_it_was_reopened` — (c) と同型の `reopen` 分岐の検出（暫定名）

実測（`cargo test -p core-interface-adapter --test workflow_execution_repository_contract`）:

```
test reader_ignores_writes_made_after_it_was_opened ... ok
test open_twice_yields_independent_empty_stores ... ok
test reopened_ignores_writes_made_after_it_was_reopened ... ok     (in-memory は 3 本とも緑)
test sqlite_open_twice_yields_independent_empty_stores ... FAILED
test sqlite_reopened_ignores_writes_made_after_it_was_reopened ... FAILED
test sqlite_reader_ignores_writes_made_after_it_was_opened ... FAILED

failures:

---- sqlite_open_twice_yields_independent_empty_stores stdout ----
thread 'sqlite_open_twice_yields_independent_empty_stores' panicked at
modules/core/interface-adapter/tests/support/contract.rs:72:10:
2 度目の open は空のストアを指す: WorkflowExecution { intent_id: IntentId("01a02785-1bd8-76eb-aeea-5aa303ebd5b6"),
... seq_nr: 1, version: 1 }

---- sqlite_reopened_ignores_writes_made_after_it_was_reopened stdout ----
thread 'sqlite_reopened_ignores_writes_made_after_it_was_reopened' panicked at
modules/core/interface-adapter/tests/support/contract.rs:114:5:
assertion `left == right` failed: 開き直した後の書込は見えない
  left: 2
 right: 1

---- sqlite_reader_ignores_writes_made_after_it_was_opened stdout ----
thread 'sqlite_reader_ignores_writes_made_after_it_was_opened' panicked at
modules/core/interface-adapter/tests/support/contract.rs:97:5:
assertion `left == right` failed: Reader を開いた後の書込は見えない
  left: 2
 right: 1

failures:
    sqlite_open_twice_yields_independent_empty_stores
    sqlite_reader_ignores_writes_made_after_it_was_opened
    sqlite_reopened_ignores_writes_made_after_it_was_reopened

test result: FAILED. 27 passed; 3 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
```

**この Red がレビュー所見を実測で裏づけた**: 12 本の既存契約テストはどれも落ちず、
新規 3 本だけが SQLite 側で落ちた = 分岐は既存テストの死角にあった。
所見にない `reopen` にも同じ分岐があることも、この時点で判明した。

## 2. Green — (a)(b)(c) の是正

### (a) `open()` を両実装で同じ意味にする

**方針**: trait doc の「空のストアを指す新しい Repository を開く」を真にする（第一候補どおり）。
SQLite 側が単一ファイルを抱え込んでいたのが原因なので、**`open()` のたびに別のファイル**を
使う形にした。

**採った設計（ブリーフの提案からの変更点と理由）**:
ブリーフは「一時ディレクトリ内で連番など」を提案していたが、連番には試験装置側の可変状態
（カウンタ）が要り、`open(&self)` のままだと `Cell` を持ち込むことになる。これは
`coding-rules/interior-mutability.md` の禁止事項そのものであり、`&mut self` へ変えると
`StoreFixture` と契約関数 15 本すべてのシグニチャに波及する。そこで**可変状態を持たずに
一意な場所を得る**形にした:

```rust
fn open_fresh(&self) -> EventStoreImpl<FakeClock> {
    let workspace = tempfile::Builder::new()
        .prefix("workspace-")
        .tempdir_in(self.root.path())   // 一意な場所は tempfile が採番する
        .expect("open ごとの一時ディレクトリ")
        .keep();                        // 自動削除を外し、後片付けは root の drop に一本化
    SqliteFixture::open_at(StorePath::for_space(
        &workspace.join("aidlc"), &SpaceName::default_space(),
    ))
}
```

`keep()` で自動削除を外すのは、Repository より先にディレクトリが消えると開き直しが
ファイルを見失うためで、後片付けは試験装置の `root: TempDir` の drop 一箇所に集約している。

**before / after**:

| | before | after |
|---|---|---|
| 保持する状態 | `_dir: TempDir` + `path: StorePath`（唯一のパス） | `root: TempDir` のみ |
| `open()` | 抱え込んだ `path` を毎回開く（2 度目は空でない） | `open_fresh()` で毎回**別の空ファイル** |
| `reopen()` | 引数を無視し、抱え込んだ `path` を開く | 引数の `repository.event_store().path()` を開く |
| `reader()` | 同上 | 同上 |
| ヘルパ | `store()`（単一パス前提） | `open_fresh()` / `open_at(path)` / `reopen_store(repository)` |

ブリーフの見立てどおり `EventStoreImpl::path()` が既に存在したため、**プロダクトコードの
追加は不要**だった。試験装置が「唯一のパス」を持たなくなったので、`path` フィールドと
`store()` ヘルパは削除した（後方互換の残置なし）。

### (b) `SqliteFixture` の doc を実態に合わせる

「in-memory 側の**ハンドル複製**に対応する」という説明はハンドル複製の全廃で嘘になっていた。
削除し、実態（呼ぶたびに別ファイル、開き直しは引数からパスを取る）と、**なぜそうするか**
（以前は 1 ファイルを抱え込んでいて in-memory と意味が分岐していた）を書いた。
`StoreFixture` の trait doc からも同じ記述を除去した。

### (c) `reader()` の生存性の裁定

**裁定: ブリーフの見立てを採る** — 両実装が共通して保証できるのは
**「そのインスタンスを得た時点までに書き終えた行が見えること」だけ**である。

**理由**: 内部可変性を禁じている（`coding-rules/interior-mutability.md`）以上、in-memory の
`reader()` は 3 表の写しを渡すしかない。「生きた reader」にするには Repository と Reader が
同じ可変状態を共有する必要があり、それは `Rc<RefCell<_>>` の復活そのもの。逆に SQLite 側を
写しに揃えるには DB のスナップショットを取る必要があり、**実装に破壊用フックを開けない**
(BR2.8) の趣旨にも反する。したがって「揃える」道は両方向とも塞がっている。

**この裁定は `reader()` だけでなく `reopen()` にも同じく適用した。** Red で実測したとおり
`reopen()` にも同型の分岐があり、`reader()` だけ直すと同じ所見が `reopen()` で再発する。

裁定に沿って 3 段構えにした（ブリーフの 1〜3 に対応）:

1. **共通の保証を契約テストで明示的に固定**（先に書く → 開く → 見える）
   - `reader_reflects_the_writes_completed_before_it_was_opened`
   - `reopen_reflects_the_writes_completed_before_it_was_reopened`

   どちらも**別々の 2 時点で開いて**確かめる（genesis を書いて開く → 2 件目を書いて開き直す）。
   1 度だけだと「たまたま全部見えた」と区別がつかず、保証が「開いた瞬間」に紐づくことを
   固定できないため。

2. **契約の外であることを `StoreFixture` の trait doc に逸脱として明記**し、BR2.7 の適用範囲を
   書き下した（`## この trait が課す約束` と `## 契約の外 — 開いた後の書込 (適用範囲の明示)`
   の 2 節）。「契約テストは必ず**書き終えてから開く**順序で書くこと。逆順を契約テストに
   書くと片方の実装だけ通るテストになる」という運用規則も併記した。

3. **各実装の実際の挙動を実装固有テストで固定**（契約の外でも挙動が変われば必ず落ちる）:

   | テスト | 実装 | 固定した挙動 |
   |---|---|---|
   | `in_memory_reader_ignores_writes_made_after_it_was_opened` | in-memory | 写し = **見えない** |
   | `in_memory_reopened_repository_ignores_writes_made_after_it_was_reopened` | in-memory | 写し = **見えない** |
   | `sqlite_reader_observes_writes_made_after_it_was_opened` | SQLite | 生きた接続 = **見える** |
   | `sqlite_reopened_repository_observes_writes_made_after_it_was_reopened` | SQLite | 生きた接続 = **見える** |

## 3. テスト本数の変化と、新しく固定した性質

| | before | after |
|---|---|---|
| 契約テスト（ジェネリック関数） | 12 | **15**（+3） |
| 契約テストの実行本数（2 実装ぶん） | 24 | **30** |
| 実装固有テスト（本ファイル） | 0 | **4** |
| `workflow_execution_repository_contract.rs` 合計 | 24 | **34**（+10） |
| workspace 全体 | 664 | **674**（+10） |

新しく固定した性質:

1. `open()` は毎回**空のストア**を指す（2 度目が 1 度目の書込を見ない）— 両実装。
2. `open()` で得た 2 つのストアは**互いに独立**（片方の書込がもう片方に漏れない）— 両実装。
3. `reader()` は**開いた時点まで**に書き終えた行を見せる（2 時点で確認）— 両実装。
4. `reopen()` は**開き直した時点まで**に書き終えた行を見せる（2 時点で確認）— 両実装。
5. 開いた**後**の書込の見え方（契約の外）— in-memory = 見えない / SQLite = 見える、を
   実装固有テスト 4 本で固定。

補助ヘルパ `store_genesis` / `store_stage_completed` を `support/mod.rs` に追加し、
`contract.rs` の `seed` からも使うようにした（重複の除去 = refactor 段）。

## 4. 検査結果（実測）

| コマンド | 結果 |
|---|---|
| `cargo fmt --all --check` | 緑（無出力、exit 0） |
| `cargo clippy --workspace --all-targets -- -D warnings` | 緑（`Finished dev profile`、警告 0） |
| `cargo lint` | 緑（exit 0。alias が `--quiet` なので成功時は無出力） |
| `PROPTEST_RNG_SEED=20260823 cargo test --workspace` | **674 passed; 0 failed**（`test result: FAILED` の行数 = 0） |
| 契約テスト単体 | `test result: ok. 34 passed; 0 failed` |

内部可変性を復活させていないことの証明:

```
$ grep -rnE "RefCell|Cell<|Rc<|Arc<|Mutex<|RwLock<" \
    modules/core/interface-adapter modules/core/use-case modules/core/domain
modules/core/interface-adapter/src/orchestration/workflow_execution_repository_impl.rs:29:/// 書込中の排他は借用チェッカが保証するので、`RefCell` の実行時借用検査も
modules/core/interface-adapter/src/orchestration/event_store_impl.rs:5://! 接続は本型が**直接所有**し、内部可変性 (`RefCell`) も共有ハンドル (`Rc`) も持たない。
modules/core/use-case/src/orchestration/workflow_execution_repository.rs:18:/// (`RefCell` 等) で隠すのは「`&self` への偽装」であり禁止されている
```

3 件はいずれも doc コメント（既存）。コード上の内部可変性は 0 件。

## 5. 設計質問

なし。`EventStoreImpl::path()` が既に Query として存在したため、プロダクトコードの追加は
不要で、止まる点はなかった。

**ただし 1 点、裁定の追認をお願いしたい**（止まってはいない）: (c) の裁定を `reader()` だけ
でなく **`reopen()` にも同じく適用**した。ブリーフは `reader()` のみを指名していたが、Red で
`reopen()` にも同型の分岐が実測されたため、片方だけ直すと同じ所見が再発すると判断した。
不要であれば `reopen` 側の契約テスト 1 本と実装固有テスト 2 本を落とせる。

## 6. 未了

- なし。所有ファイルの範囲で (a)(b)(c) をすべて是正し、検査 5 種すべて緑。
- `git add` / `commit` / `push` は行っていない（ブリーフの禁止事項どおり）。
