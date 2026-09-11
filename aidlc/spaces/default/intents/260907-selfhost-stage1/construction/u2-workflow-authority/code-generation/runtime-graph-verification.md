# runtime-graph 再構築の実装記録（Step 7 スライス）

2026-09-09〜10。担当 `u2_runtime_graph`。固定本家 2.7.1 `a277af218f0df7f325d3b8be7b6d90fce2c5bd40` の
`hooks/aidlc-rebuild-stage-graph.ts`（runtime-graph の再構築）を Rust の公開入口へ接続した。
[残作業の棚卸し](../remaining-step7-inventory.md) の「runtime-graph再構築」行が対象である。

vendor submodule の HEAD は `801c570062f67dc8f4952ee5fc601381d09db7ec` だが、
`dist/claude/.claude/hooks/aidlc-rebuild-stage-graph.ts` と
`dist/claude/.claude/tools/aidlc-runtime.ts` は固定コミットとバイト同一である
（`git diff --stat a277af21 HEAD -- <2ファイル>` が空）。読んだのは `git show a277af21:<path>` の実バイトである。

## 実装した変更

### ドメイン（`modules/core/command/domain/`）

| ファイル | 要点 |
| --- | --- |
| `src/orchestration/memory_journal.rs`（新規） | ステージ日誌 `memory.md` の 4 見出しの件数を持つ値オブジェクト。`is_empty` が `MEMORY_EMPTY` の判定材料。数え方（`parse`）もこの型が所有する。 |
| `src/orchestration/stage_memory_journal.rs`（新規） | 位置と件数の対。日誌が**在った**位置だけがこの値になる。 |
| `src/orchestration/memory_journal_survey.rs`（新規） | 観測順の一級コレクション（`at` / `fold_left` / `filter` / `find`）。 |
| `src/orchestration/empty_memory_stages.rs`（新規） | 集約が「記録する」と決めた位置の一級コレクション。 |
| `src/orchestration/intent_execution_event/memory_journals_observed.rs`（新規） | 観測（survey）と判定（empty_stages）を運ぶドメインイベント。 |
| `src/orchestration/intent_execution_event.rs` | 変種を 27 → 28 へ。`id` / `aggregate_id` / `affects_progress` / 網羅 match を追加。 |
| `src/orchestration/intent_execution.rs` | コマンド `observe_memory_journals`（1 コマンド 1 イベント）と適用を追加。 |
| `src/orchestration/stage_slot.rs` | 8 番目の記録 `memory_empty_reported` を追加。**承認へ倒れたとき**だけ落ちる。 |
| `src/orchestration/stage_slots.rs` | `empty_memory_stages`（判定）と `apply_memory_empty`（印し）を追加。 |

判定は集約が持つ。「承認済みか」「この承認について記録済みか」は集約の状態でしか決まらないためである。
本家は `(slug, completed_at)` を鍵に 1 件だけ記録し、再跳躍して承認し直した位置には改めて記録する
（`aidlc-runtime.ts:786-804`）。本集約は承認時刻を持たないので、**承認へ倒れたこと**を
「別の承認になった」印として扱い、同じ観測可能な振る舞いを得ている。承認以外の進捗（TaskUpdate に
よる同期など）では印を落とさない — 落とすと、本家では抑止されたままの位置で再記録が起きる。

### 更新ユースケース・永続化

| ファイル | 要点 |
| --- | --- |
| `command/use-case/src/orchestration/observe_memory_journals_use_case.rs`（新規） | 観測の保存だけを行う。判定はしない。成功は `Result<(), E>`。 |
| `command/use-case/src/orchestration/memory_journal_error.rs`（新規） | 記録操作の失敗（Repository / Command）。 |
| `command/interface-adapter/src/orchestration/dto/memory_journals_observed_dto.rs`（新規） | 保存側の DTO。 |
| `command/interface-adapter/src/orchestration/dto/intent_execution_dto.rs` | スナップショットへ `memory_empty_reported` 列を追加（`#[serde(default)]` で欄不在は全 false）。 |
| `read-model-updater/src/orchestration/dto/memory_journals_observed_dto.rs`（新規） | 読取側の DTO（側ごとに別所有）。 |

### 投影（`modules/core/read-model-updater/`）

| ファイル | 要点 |
| --- | --- |
| `src/workspace/projection.rs` | `MemoryJournalsObserved` の腕を追加。判定された位置ぶんの `MEMORY_EMPTY` 監査行を描く。状態ファイルは動かさない。 |
| `src/orchestration/runtime_graph_targets.rs`（新規） | 記録ディレクトリ 1 本から監査置き場・状態ファイル・`runtime-graph.json` を導く。 |
| `src/orchestration/runtime_graph_read_model_updater.rs`（新規） | ジャーナル（計画・観測）と監査台帳（対）から runtime-graph を描いて原子的に書く。 |

`runtime-graph.json` は再生成可能な派生物なので、状態ファイル・監査シャードの通常取得ループとは
別の投影にした。日誌 `memory.md` はジャーナルの外で書き換わるため、compile は差分の有無に依らず
毎回描き直す必要がある。直列化は `canon_json::serialize(ContractPretty)`（2 スペース・宣言順・末尾改行）で、
本家の `JSON.stringify(graph, null, 2)` + 改行と同じ体裁である。

### ハーネスと入口

| ファイル | 要点 |
| --- | --- |
| `harness/claude/src/runtime_compile_envelope.rs`（新規） | PostToolUse (Bash) 封筒の解釈と、compile を発火するかの語彙的判定。 |
| `app/aidlc/src/runtime/runtime_graph.rs`（新規） | フック本体。封筒 → コマンド判定 → 監査末尾の遷移判定 → 心拍 → 観測 → 更新 → 投影 → graph。日誌の読取り（ファイル）だけを持ち、数え方は `MemoryJournal::parse` へ委ねる。 |
| `app/aidlc/src/runtime.rs` | `aidlc hook rebuild-stage-graph` を配線。 |

出力先は `<record>/runtime-graph.json` であり、配布の `.gitignore` の
`aidlc/spaces/*/intents/*/runtime-graph.json` に一致する。

## Red の再現

### 1. フック入口と compile（8 件）

```
cargo test -p aidlc --test runtime_graph_contract
```

```
---- a_transition_reporting_command_compiles_the_runtime_graph stdout ----
thread 'a_transition_reporting_command_compiles_the_runtime_graph' panicked at
modules/app/aidlc/tests/runtime_graph_contract.rs:170:5:
Output { status: ExitStatus(unix_wait_status(256)), stdout: "", stderr: "Unknown hook: rebuild-stage-graph\n" }

failures:
    a_filled_journal_and_a_running_stage_are_not_recorded
    a_fired_compile_writes_the_heartbeat
    a_malformed_envelope_is_ignored
    a_transition_reporting_command_compiles_the_runtime_graph
    an_audit_tail_without_a_transition_never_compiles
    an_empty_journal_of_an_approved_stage_is_recorded_once
    an_unrelated_command_never_compiles
    the_runtime_tool_itself_never_retriggers_the_compile

test result: FAILED. 0 passed; 8 failed; 0 ignored; 0 measured; 0 filtered out; finished in 9.64s
```

全文は [red-01-hook-and-compile.log](runtime-graph-logs/red-01-hook-and-compile.log)。

### 2. 集約の重複抑止（3 件のうち 2 件）

```
cargo test -p core-command-domain --lib -- intent_execution::tests::an_empty_journal \
  intent_execution::tests::a_journal_with_entries intent_execution::tests::a_stage_that_leaves
```

```
---- a_journal_with_entries_and_an_unapproved_stage_are_never_named stdout ----
panicked at modules/core/command/domain/src/orchestration/intent_execution.rs:8902:9: 進行中の位置

---- a_stage_that_leaves_and_re_enters_approval_is_named_again stdout ----
panicked at modules/core/command/domain/src/orchestration/intent_execution.rs:8917:44:
called `Result::unwrap()` on an `Err` value: InvalidTarget(StageIndex(0))

test result: FAILED. 1 passed; 2 failed; 0 ignored; 0 measured; 749 filtered out
```

この 2 件は**テストの前提が誤っていた**ことを検出した red である。合成計画 `all_exec(n)` の位置 0 は
initialization で誕生時から `[x]` であり、位置 0 は跳躍先にならない。位置を 1 つずらして green にした。
実装側は変えていない。

### 3. 対の順序（1 件、赤例で検出力を確認）

同一秒に「完了 → 再開始」が並ぶ台帳を単体テストで固定した。実装のタイブレークを
バッファ順へ戻すと、この 1 件だけが落ちる。

```
---- at_the_same_second_the_start_is_processed_before_the_completion stdout ----
assertion `left == right` failed: 同じ秒の完了は再開始の対を埋める
  left: None
 right: Some("2026-09-09T00:00:09Z")
test result: FAILED. 5 passed; 1 failed
```

本家は 2 つの列を 1 本へ混ぜるとき、完了側のタイブレーク添字に 100000 を足して必ず後ろへ送る
（`aidlc-runtime.ts:200-209`）。素のバッファ順だと再開始が対を持てない。

### 4. 封筒の語彙判定（1 件）

`git commit -m "aidlc state approve"` を「発火しない」と書いた最初の期待は red になった。
本家は発火側だけを素の正規表現で見る（`aidlc-lib.ts:1499-1522` の注記「Transition matching stays
intentionally lexical」）ので、**本家に合わせて**期待を「発火する」へ直した。再帰ガードだけが
区間解析を使う。

```
---- runtime_compile_envelope::tests::the_new_grammar_fires_only_on_the_public_transition_surface ----
panicked at modules/harness/claude/src/runtime_compile_envelope.rs:313:13:
git commit -m "aidlc state approve"
test result: FAILED. 7 passed; 1 failed
```

### 5. Green 化の途中で出た実装の red

初回実装では `events_through(GlobalSeqNr::new(u64::MAX))` を使ったため、フックが落ちて graph が
書かれなかった。落ちた理由は心拍の drops に残っていた。

```
.aidlc-hooks-health/rebuild-stage-graph.drops:
2026-09-09T19:12:26Z	runtime graph: read: corrupt: aggregate -, seq_nr -, cause invariant violation
```

`u64::MAX` は SQLite の `INTEGER`（i64）に収まらず `to_i64` が破損として止める。`events_after(ZERO)`
（全履歴読取）へ直した。

## Green の一覧

| 検査 | コマンド | 結果 |
| --- | --- | --- |
| フック入口と compile | `cargo test -p aidlc --test runtime_graph_contract` | 8 passed / 0 failed（`runtime-graph.json` と各行のキーの並びも逐語で固定） |
| 封筒の語彙判定 | `cargo test -p harness-claude --lib runtime_compile_envelope` | 9 passed / 0 failed |
| 台帳の対・区間 | `cargo test -p core-read-model-updater --lib -- runtime_graph_read_model_updater` | 6 passed / 0 failed |
| 日誌の値オブジェクトと数え方・集約の重複抑止 | `cargo test -p core-command-domain --lib -- memory_journal empty_memory_stages intent_execution::tests::{an_empty_journal,a_journal_with_entries,a_stage_that_leaves,a_task_update_sync}` | 19 passed / 0 failed |
| 保存側 DTO・Repository | `cargo test -p core-command-interface-adapter` | 全 target 0 failed（lib 109 passed） |
| 変更したコマンド側 4 クレート（最終形） | `cargo test -p core-command-domain -p core-command-use-case -p core-command-interface-adapter -p harness-claude` | 25 target ok、計 1,073 passed / 0 failed |
| 投影・読取・クエリ側 | `cargo test -p core-read-model-updater -p core-command-domain -p core-query-interface-adapter` | 33 target すべて ok、計 1,388 passed / 0 failed |
| 投影の回帰（個別） | `cargo test -p core-read-model-updater --test publication_recovery_contract --test projection_golden_test --test audit_block_golden_test --test read_model_updater_test` | 1 / 20 / 33 / 31 passed、0 failed |
| 合成ルート・基盤・ユースケース・ハーネス | `cargo test -p aidlc -p core-infrastructure -p core-command-use-case -p core-query-use-case -p harness-claude -p harness-infrastructure` | 44 target すべて ok、計 1,076 passed / 0 failed（終了コード 0） |
| 整形 | `cargo fmt --all --check` | 差分なし（最終実行時点） |
| 静的検査 | `cargo clippy --workspace --all-targets -- -D warnings` | 所見なし（最終実行時点） |
| 独自 lint | `cargo lint` | 所見 0 件（最終実行時点） |

途中の実行では他担当が同時編集中のファイル
（`modules/app/aidlc/src/runtime/review_guards.rs`、`modules/app/aidlc/src/wording.rs`、
`modules/core/command/use-case/src/orchestration/guard_review_freeze_use_case.rs`）に
所見・コンパイル失敗が出ていた。いずれも本担当は触っておらず、最終実行の時点では解消している。

`cargo test --workspace` の 1 本での通し実行は、他担当の `cargo test` と cargo のビルドロックを
取り合って完了しなかった。上の行はクレート単位の実行で、workspace の 10 メンバーをすべて覆っている。
最後の変更（承認への限定・baseline の絞り込み・日誌の数え方の移設）の後に、影響のあるクレートは
再実行して green を確認した。

## 本家との突き合わせ

### 確認した観測

| 観測 | 本家の出典 | 本実装 |
| --- | --- | --- |
| 遷移を書く公開面（`aidlc-state/jump/bolt/unit/utility.ts`、`aidlc-orchestrate.ts report`、新文法の `aidlc state\|jump\|bolt\|unit`・`status\|doctor\|version\|help`・`scope change`・`config set`・`report`・`orchestrate report`・`next … report`）で発火 | `aidlc-lib.ts:1505-1537` | `RuntimeCompileEnvelope::compiles`、契約テスト 9 件 |
| `workspace` / `gen` / `sensor` / `intent` / `space` の名詞と `aidlc next` 単体では発火しない | 同上（D2 の意図的な非対称） | 同上 |
| `aidlc-runtime.ts` / `aidlc runtime` は先に落として再帰させない。合成コマンド `A && B` でも落とす | `aidlc-lib.ts:1518-1522` | `invokes_runtime`、契約テスト 4 形 |
| 遷移の照合は語彙的（引用の内側でも当たる）。再帰ガードだけが実行区間を見る | `aidlc-lib.ts:1499-1501` の注記 | 専用テスト 2 件 |
| `bun` と対象スクリプトは同じ行にある必要がある（本家の `.` は改行に当たらない） | JS 正規表現の既定 | 専用テスト 1 件 |
| 監査末尾 3 ブロックに `GATE_APPROVED` / `STAGE_STARTED` / `STAGE_AWAITING_APPROVAL` / `AUDIT_MERGED` / `UNIT_MERGED` / `WORKFLOW_COMPLETED` があるときだけ compile | `aidlc-rebuild-stage-graph.ts:191-203` | `tail_carries_transition`、単体 3 件＋契約 1 件 |
| `MEMORY_EMPTY` は遷移クラスに入れない（compile 自身の監査で再入しない） | 同上の再帰ガード注記 | 単体テスト 1 件 |
| 心拍 `<record>/.aidlc-hooks-health/rebuild-stage-graph.last` を残す | `aidlc-rebuild-stage-graph.ts:175-181` | 契約テスト 1 件 |
| 監査が空なら何もしない | `aidlc-rebuild-stage-graph.ts:167-171` | フック本体（契約テストでは踏んでいない） |
| 承認済みかつ日誌が空の位置へ `MEMORY_EMPTY`（`**Stage**: <slug>` 1 項目）を記録 | `aidlc-runtime.ts:786-806`、`:391-402` | 契約テスト 1 件、監査ブロックの逐語を確認 |
| 同じ承認について 2 度目は記録しない | `aidlc-runtime.ts:788-800` | 契約テスト（2 回発火で 1 件）＋集約テスト |
| 再跳躍して承認し直した位置には改めて記録する | `aidlc-runtime.ts:794-800` の注記 | 集約テスト 1 件 |
| TaskUpdate による同期は抑止を解かない（監査の対を動かさないため） | `aidlc-runtime.ts:376-401`（対だけを見る） | 集約テスト 1 件 |
| 日誌に記録があれば対象外、進行中の位置も対象外 | `aidlc-runtime.ts:400` | 契約テスト 1 件＋集約テスト 1 件 |
| 日誌が**無い**位置は `memory_entries: null` で `MEMORY_EMPTY` の対象外（件数 0 と区別する） | `aidlc-runtime.ts:271-274` の `readMemory` | 契約テストで `null` を確認 |
| 4 見出しの数え方（空行・引用のみ・HTML コメントのみ・フェンス内・見出し行を数えない。錨でない `## ` で打ち切り。BOM と CRLF を正規化。錨の欠落は 0 件で読む） | `aidlc-lib.ts:21414-21477` | `MemoryJournal::parse`、単体 4 件 |
| `workflow_id` / `started_at` は最新の `WORKFLOW_STARTED` の時刻、`scope` は状態ファイル優先 | `aidlc-runtime.ts:242-256` | 契約テストで `workflow_id == started_at`・`scope == "bugfix"` を確認 |
| 位置の対は `STAGE_STARTED` / `STAGE_COMPLETED` から。`agent` は行の値、空なら計画の担当 | `aidlc-runtime.ts:175-235`、`:379` | 契約テストで `state-init` の開始・完了・`orchestrator` を確認 |
| 承認済みは `outcome: "approved"` と `learnings_captured`、未承認は `"pending"` と `null` | `aidlc-runtime.ts:376-388` | 契約テスト 1 件 |
| 出力先は `<record>/runtime-graph.json` | `aidlc-lib.ts:14430-14432` | 契約テスト（`.gitignore` の規約と一致） |
| 隔離実行（`--single`）の合成行は対から除く | `aidlc-runtime.ts:171-188` | `single_stage_row`、投影側の単体テスト 2 件 |
| 同一秒では開始行を完了行より先に処理する | `aidlc-runtime.ts:200-209` | 単体テスト 1 件（赤例で検出力を確認） |
| 孤児の打ち切り基準は stage 開始行とセンサー行だけの最大時刻 | `aidlc-runtime.ts:623-633` | `build_graph` の `baseline`（センサー行が無いので実データ未駆動） |

### 確認できなかった範囲

- **Kiro の `ide-audit-sync` 分岐**（コマンドフィルタの読み飛ばしと mtime による冪等ガード、
  `aidlc-rebuild-stage-graph.ts:210-236`）は写していない。他ハーネスは今回のスコープ外である。
- **`bolt_dag` ノード**（`unit-of-work-dependency.md` の辺ブロックから作る）は出していない。
  本 slice は DAG 表示全般を範囲外としており、誤った形を書くよりノードを出さない方を選んだ。
  本家は同ファイルが不在なら同じくノードを省くが、**在る場合の差は未検証**である。
- **`instances[]`（並行 Bolt）**は出していない。現コードに `STATE_FORKED` / `STATE_MERGED` を
  生む経路が無く、対を作れないためである。単一 instance の行だけを出している。
- **`sensor_firings`** は監査の `SENSOR_FIRED` と終端行（`SENSOR_PASSED` / `SENSOR_FAILED` /
  `SENSOR_BUDGET_OVERRIDE`）を `Fire id` で対にする処理を書いたが、現ビルドにセンサー発火の
  経路が無いため**実データで駆動していない**。契約テストで観測しているのは空配列だけである。
  worktree 配下の出力による instance 単位の絞り込み（`resolveAuditWorktreePath`）は写していない。
- **`learnings_captured`** も `RULE_LEARNED` / `SENSOR_PROPOSED` 行を窓で数える処理を書いたが、
  learnings persist が未接続なので実データで駆動していない。観測できたのは `{0, 0}` だけである。
- **`runtime-graph.json` のバイト単位のゴールデン比較**は行っていない。本家の実出力を固定元から
  採っていないため、キーの並びと体裁は本家のコードから読み取った規則に合わせただけである
  （封筒と行のキーの並びは契約テストで逐語に固定した）。
- **TaskUpdate 同期後の窓**：本家は同期で監査の対が動かないので `outcome` も抑止も変わらない。
  本実装も `outcome` は対から描くので同じだが、`MEMORY_EMPTY` の判定は集約の `[x]` を見るため、
  「承認済みだが日誌が空のまま一度も記録していない位置」を同期した直後だけ、本家が記録する場面で
  本実装は記録しない。抑止済みの位置では差が出ない（集約テストで固定した）。
- 計画に無い slug の行、`--init --force` による複数 `WORKFLOW_STARTED`、複数シャードの連結順、
  隔離実行の合成行の除外は実装しているが、契約テストで駆動していない
  （対の除外だけは投影側の単体テストで駆動した）。
- **`WORKFLOW_STARTED` が 1 行も無いときの空グラフ**は書いていない。本家 `compile` は
  `writeEmptyGraph` で空の `stages` を持つグラフを書くが、本実装は何も書かない。フックの
  発火条件（監査末尾の遷移）を満たす時点で `WORKFLOW_STARTED` は必ず先にあるため、
  この入口からは到達しない分岐である。compile を別の入口へ公開するときは要検討である。

### 上流と現行コードの不一致（裁定待ちではない、記録のみ）

`aidlc-runtime.ts` の compile は監査ロックの中で `MEMORY_EMPTY` を追記してから成果物を書く。
本実装は「更新ユースケース → 通常 RMU の投影（監査追記）→ runtime-graph の書込み」の順で、
同じ順序を保っているが、**1 つのロック区間ではない**。本リポジトリの監査追記は既存の
投影経路が持つ排他に従っており、compile のためだけに別のロックを導入していない。

## 実装上の重複（記録）

監査シャードの連結はフック側（`runtime_graph.rs::read_audit`、末尾 3 ブロックの遷移判定に要る）と
投影側（`runtime_graph_read_model_updater.rs::read_audit`、対を作るのに要る）の 2 か所にある。
本家も同じ形（フックが台帳を読んで発火を決め、compile の子プロセスがもう一度読む）なので、
片方へ寄せていない。

## 残課題

1. `bolt_dag` / `instances[]` / `sensor_firings` / `learnings_captured` を実データで駆動する。
   それぞれ units-generation 成果物、Bolt の fork/merge、センサー、learnings persist の
   各スライスに依存する。
2. `runtime-graph.json` のゴールデンを固定元から採り、バイト一致で比較する。
3. Kiro の `ide-audit-sync` 分岐（他ハーネス）。
4. 本家 rebuild hook が compile より前に行う **intent-create の会話帰属**（`:56-105,133-136`）の
   PostToolUse 封筒経路。直接 CLI 成功経路は
   [作成帰属の証跡](session-create-attribution-verification.md) で解消済みだが、封筒による補完は残る。
5. compile が learnings surface の前提になる点（`aidlc-learnings.ts:322-325` が runtime-graph の
   stage 行を読む）は、learnings スライスで受ける。
