# learnings の surface / persist 実装記録（Step 7 スライス）

2026-09-09〜10。担当 `u2_learnings`。固定本家 2.7.1 `a277af218f0df7f325d3b8be7b6d90fce2c5bd40` の
`tools/aidlc-learnings.ts`（学びの儀式のツール）を Rust の公開入口へ接続した。
[残作業の棚卸し](../remaining-step7-inventory.md) の「learnings surface」「learnings persist」
2 行が対象である。

読んだのは `git show a277af21:dist/claude/.claude/tools/aidlc-learnings.ts` の実バイト
（1,144 行）、同 `aidlc-common/protocols/stage-protocol.md` の §13（1,055–1,110 行相当）、
同 `tools/aidlc-lib.ts` の `parseMemoryEntries` / `appendUnderHeading` /
`redactProjectDirPrefix`、同 `tools/aidlc-audit.ts` の `renderAuditBlock` である。

**逐語の正本は既存ゴールデン `tests/golden/upstream-a277af21/learnings/cases.json`** である。
これは本家配布を一時ワークスペースへ展開して実行した 8 観測（`setup-intent` / `setup-runtime` /
`surface` / `persist-empty` / `persist-one` / `persist-repeat` / `wrong-stage` /
`malformed-selection`）で、stdout・stderr・終了コードに加えて**変更されたファイルの全文**
（監査シャードと `project.md`）を持つ。本スライスの期待値はこの実測から取っており、推測で
綴りを作っていない。Rust 側からこのゴールデンを読む検査は本スライスで初めて追加した。

## 実装した変更

### ドメイン（`modules/core/command/domain/`）

| ファイル | 要点 |
| --- | --- |
| `src/orchestration/memory_entry_heading.rs`（新規） | 日誌の 4 見出し。`## Interpretations` 等の錨から読み、surface の `source_heading` の逐語（`Open questions` を含む）を持つ。`is_parked` が「候補へ昇格しない」の判定材料。 |
| `src/orchestration/memory_entry.rs`（新規） | 数えられた 1 行を見出し・時刻・要約・文脈へ割る値オブジェクト。正典の綴り `- <ISO> — <要約>; <文脈>` から外れた行は**前の行へ併合せず**退化した 1 件になる。 |
| `src/orchestration/memory_entries.rs`（新規） | 記録の一級コレクション。読み飛ばし規則（空行・引用のみ・HTML コメントのみ・フェンス内・錨でない `## ` での打ち切り・BOM/CRLF 正規化）を**唯一所有**する。 |
| `src/orchestration/memory_journal.rs` | `parse` を `MemoryEntries::parse(raw).journal()` へ移譲。件数の不変条件（`entries.len() == journal.total()`）が構造から成り立つ。`counting` を追加。 |
| `src/orchestration/learning_scope.rs`（新規） | `project` / `team` と、その正本ファイル名。org 層への昇格路は無い。 |
| `src/orchestration/learning_source.rs`（新規） | `orchestrator` / `user_addition`（監査行 `Source` の逐語）。 |
| `src/orchestration/practice_heading.rs`（新規） | orchestrator が選んだ見出しの正規化。素の `Corrections` も `## Corrections` も同じ。空は既定へ倒す。 |
| `src/orchestration/learning_content_hash.rs`（新規） | 本文の SHA-256（小文字 16 進 64 桁、**切り詰めない**）。候補番号ではなくこれが同一性である。 |
| `src/orchestration/learning_candidate_id.rs`（新規） | 候補番号。監査行の 1 ラベル 1 行を壊す綴り（空・改行・制御文字・前後空白）を拒否する。 |
| `src/orchestration/learning_provenance.rs`（新規） | **surface の時点で固定した** space と記録。`destination` が監査行の `**Destination**:` の綴りを作る。 |
| `src/orchestration/learning.rs`（新規） | 確定した学び 1 件。同一性は本文から導く。実践行 `- <本文> (learned YYYY-MM-DD) <!-- cid:… -->` と印の綴りを所有する。 |
| `src/orchestration/learning_disposition.rs`（新規） | `Fresh` / `PracticeLineOnly` / `AuditRowOnly` の 3 値。両側在るものは事実にならない。 |
| `src/orchestration/learning_observation.rs`（新規） | 学び 1 件と、書込み直前に実測した両側の在否。 |
| `src/orchestration/learning_observations.rs`（新規） | 選択の一級コレクション。`captured()` が同一本文の重複を落とし（最初が勝つ）、両側在るものを外し、内訳を決める。 |
| `src/orchestration/captured_learning.rs` / `captured_learnings.rs`（新規） | 「この回に書く」と決まった学びと内訳の対と、その一級コレクション。`audit_rows()` が `rule_learned` の件数。 |
| `src/orchestration/intent_execution_event/learnings_captured.rs`（新規） | ドメインイベント。stage・固定素性・書く学びの列を運ぶ。 |
| `src/orchestration/intent_execution_event.rs` | 変種を 28 → 29 へ。`id` / `aggregate_id` / `affects_progress` / 網羅 match を追加。 |
| `src/orchestration/intent_execution.rs` | コマンド `capture_learnings`（1 コマンド 1 イベント）と適用を追加。適用は**何もしない** — 重複抑止の正本は監査シャードとメモリ層（人も編集する面）なので、集約に控えを持つと片側が消えたときに復旧できない。 |
| `src/workspace/markdown_sections.rs` | `ensure_heading` を追加（見出しが無ければ本文末に足す。在れば 1 バイトも変えない）。本家 `ensureHeading` の写し。 |

集約が持つ判断は**計画がその位置を知っているか**だけである（知らなければ `UnknownStage`)。
どの学びをどちらの側へ書くかは `LearningObservations::captured()` が決める — この判断は集約の
状態に依存せず、材料はディスクの実測だからである。

### 更新ユースケース・永続化

| ファイル | 要点 |
| --- | --- |
| `command/use-case/src/orchestration/capture_learnings_use_case.rs`（新規） | 学びの保存だけを行う。判定はしない。成功は `Result<(), E>`。 |
| `command/use-case/src/orchestration/learning_capture_error.rs`（新規） | Repository / Command の失敗。 |
| `command/interface-adapter/src/orchestration/dto/learnings_captured_dto.rs`（新規） | 保存側 DTO。同一性 `content_hash` は**列に持たない** — 本文から導けるので、保存した綴りと導出が食い違う余地を構造から無くす。 |
| `read-model-updater/src/orchestration/dto/learnings_captured_dto.rs`（新規） | 読取側 DTO（側ごとに別所有）。 |

### 投影（`modules/core/read-model-updater/`）

| ファイル | 要点 |
| --- | --- |
| `src/workspace/projection.rs` | `LearningsCaptured` の腕を追加。実践行を書く学びが在るときだけメモリ層を要求し、`ensure_heading` → `append_under_heading` で追記する。監査行 `RULE_LEARNED` は 6 欄（Stage / Candidate-ID / Content-Hash / Destination / Heading / Source）を本家の順で描く。**状態ファイルは動かさない**。 |

### 合成ルートと入口

| ファイル | 要点 |
| --- | --- |
| `app/aidlc/src/cli/face.rs` | 起動名 `aidlc-learnings` を面として追加。 |
| `app/aidlc/src/cli/learnings_args.rs`（新規） | `--slug` / `--selections-json`。値を伴わない末尾のフラグは落とす（本家 `parseFlags` と同じ拾い方）。 |
| `app/aidlc/src/cli/request.rs` | `surface` / `persist` / `--help` / 未知動詞へ振り分け。 |
| `app/aidlc/src/runtime/learnings.rs`（新規） | 2 つの動詞の本体。 |
| `app/aidlc/src/wording.rs` | 逐語文言（本家の出典を行番号で付記）。 |

`persist` の順序は「選択ファイルの検査 → 固定素性で配置を組み直す → 存在の確認 → 排他ロック →
投影の追いつき → 両側の実測 → コマンド → イベント → SQLite → RMU → 出力」である。ロックは
`aidlc/.aidlc-learnings.lock` で、本家の `withAuditLock` に対応する区間を直列化する。

### 検査側で触れた既存ファイル

| ファイル | 要点 |
| --- | --- |
| `app/aidlc/tests/learnings_contract.rs`（新規） | surface / persist の 23 件。試験装置は `intent-create --scope bugfix` で記録を作り、合成の `project.md` / `team.md` を置き、`next bugfix` と `rebuild-stage-graph` フックで `runtime-graph.json` を作る。ゴールデン `tests/golden/upstream-a277af21/learnings/cases.json` を逐語比較に使う。 |
| `app/aidlc/tests/journal_protocol_conformance.rs` | `LearningsCaptured` の変種を足し、件数の固定を 29 へ。 |
| `command/interface-adapter/src/orchestration/dto/tests.rs` | `every_variant()` に逐語行を足す。あわせて、既存の不変条件テストの**差し替え綴り一覧**に `requirements-analysis` を足した（一覧に無い綴りは差し替えが起きず、拒否の検査が素通りしていた。§7 参照）。 |

## Red の再現

### 1. 日誌の 1 行を割る（3 件）

```
cargo test -p core-command-domain --lib -- orchestration::memory_entry
```

```
---- orchestration::memory_entry::tests::the_canonical_bullet_splits_into_timestamp_summary_and_context stdout ----
thread '...' panicked at modules/core/command/domain/src/orchestration/memory_entry.rs:94:9:
assertion `left == right` failed
  left: ""
 right: "2026-05-20T10:14:32Z"

test result: FAILED. 4 passed; 3 failed; 0 ignored; 0 measured; 759 filtered out
```

全文は [red-01-memory-entry.log](learnings-logs/red-01-memory-entry.log)。

### 2. 記録の列と件数の不変条件（2 件）

```
cargo test -p core-command-domain --lib -- orchestration::memory_entries
```

```
---- orchestration::memory_entries::tests::the_entry_count_equals_the_journal_total stdout ----
assertion `left == right` failed: 不変条件が崩れた: "# Stage Memory\n\n## Interpretations\n\n- 2026-05-20T10:14:32Z — 解釈; 文脈\n…"
  left: 0
 right: 4

test result: FAILED. 1 passed; 2 failed; 0 ignored; 0 measured; 766 filtered out
```

全文は [red-02-memory-entries.log](learnings-logs/red-02-memory-entries.log)。

### 3. 学びの同一性と重複抑止の判断（8 件）

```
cargo test -p core-command-domain --lib -- orchestration::learning orchestration::captured \
  orchestration::practice_heading orchestration::intent_execution_event
```

```
---- orchestration::learning_content_hash::tests::the_hash_is_the_full_sha256_of_the_utf8_text ----
  left: "0000000000000000000000000000000000000000000000000000000000000000"
 right: "f543ed24a72a9b57b8fac723a95fa0a2c04240a25c8a6a67229322acb3de9bd3"

---- orchestration::learning_observations::tests::the_same_text_twice_in_one_batch_is_captured_once ----
  left: 0
 right: 2

test result: FAILED. 38 passed; 8 failed; 0 ignored; 0 measured; 755 filtered out
```

全文は [red-03-learning-values.log](learnings-logs/red-03-learning-values.log)。右辺の
`f543ed…` はゴールデン `learnings/persist-one` の監査行 `**Content-Hash**:` の実測である。

### 4. 投影（5 件）

```
cargo test -p core-read-model-updater --lib -- workspace::projection::tests
```

```
---- workspace::projection::tests::a_fresh_learning_writes_the_practice_line_and_the_audit_row ----
assertion failed: memory.is_dirty()

---- workspace::projection::tests::a_practice_line_without_the_memory_face_is_refused ----
  left: Ok(())
 right: Err(MemoryFilesMissing)

test result: FAILED. 42 passed; 5 failed; 0 ignored; 0 measured; 275 filtered out
```

全文は [red-04-projection.log](learnings-logs/red-04-projection.log)。

### 5. CLI 契約（1 件）

```
cargo test -p aidlc --test learnings_contract
```

```
---- a_team_scoped_learning_lands_in_the_team_method_file stdout ----
thread '...' panicked at modules/app/aidlc/tests/learnings_contract.rs:536:5:
# Team-Level Rules

## Way of Working

- ALWAYS squash-merge (learned 2026-09-09) <!-- cid:260909-learnings:reverse-engineering:2ee050b2… -->
## Corrections

test result: FAILED. 22 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 180.11s
```

全文は [red-05-cli-contract.log](learnings-logs/red-05-cli-contract.log)。

この 1 件は**テストの期待が誤っていた** red である。本家 `appendUnderHeading` は
**次の `## ` 見出しの直前**へ差し込むので、節末尾の空行はそのまま残り、追記行は見出しの
直後の行にならない。実装側は変えず、期待を「節の中に入る」へ直した（既存の
`practices_affirmed` の投影テストが同じ性質を逐語で固定している）。

### 6. 承認ゲートを跨ぐ窓（1 件、2 段で原因まで降りた）

```
cargo test -p aidlc --test learnings_contract -- a_persisted_learning
```

まず症状が出た（[red-06a-gate-not-opened.log](learnings-logs/red-06a-gate-not-opened.log)）。

```
---- a_persisted_learning_drives_the_runtime_graph_counter stdout ----
assertion `left == right` failed: {"stage_slug":"reverse-engineering",…,"outcome":"pending","learnings_captured":null}
  left: String("pending")
 right: "approved"
```

ビジネス拒否は **exit 0 の `error` directive** として出るため、終了コードだけを見ていると
「承認された」と読めてしまう。`report` の stdout に `"kind":"error"` が無いことを確かめる検査を
足したところ、原因が出た（[red-06-gate-window.log](learnings-logs/red-06-gate-window.log)）。

```
ゲートが開かなかった: {"kind":"error","message":"Cannot present \"reverse-engineering\" for approval
because these pipeline handoffs have not been recorded for the current run: aidlc-developer-agent,
aidlc-architect-agent. …"}
```

`mode: pipeline` の reverse-engineering はゲートを開く前に lead/support の受領証を要る
（本家 `aidlc-orchestrate.ts:7198-7227`）。テスト側に既存 `upstream_271_contract` と同じ
受領証の記録を足した。**実装側は変えていない。**

### 7. DTO のワイヤ形（1 件、逐語文字列の綴りで 2 段）

```
cargo test -p core-command-interface-adapter
```

`every_variant()` に足した `LearningsCaptured` の逐語行は、`aidlc` クレートの検査では
コンパイルされない（`#[cfg(test)]` の位置がこのクレートの lib テストのため）。クレート単体で
回して初めて 2 段の失敗が出た（[red-07-dto-wire.log](learnings-logs/red-07-dto-wire.log)）。

1 段目は**コンパイルできない**失敗である。逐語行に `"heading":"## Corrections"` が入るため、
生文字列の終端 `"#` が本文の中で先に現れる。`r##"` にしても本文の `"##` に当たるため、
`r###"` まで上げて初めて閉じる。

```
error: prefix `Corrections` is unknown
error: reserved multi-hash token is forbidden
error: could not compile `core-command-interface-adapter` (lib test) due to 25 previous errors
```

2 段目が本来の red である。既存の不変条件テスト
`a_malformed_stage_reference_in_any_variant_is_refused` は、**既知のステージ綴りの一覧**を
`"Not A Slug"` へ差し替えて DTO が拒むことを確かめる。`LearningsCaptured` の標本が使う
`requirements-analysis` はその一覧に無かったので、差し替えが起きず素通りしていた。

```
---- orchestration::dto::tests::a_malformed_stage_reference_in_any_variant_is_refused stdout ----
拒むべき行: {"LearningsCaptured":{…,"stage":"requirements-analysis",…}}
test result: FAILED. 108 passed; 1 failed; …
```

差し替え一覧に `requirements-analysis` を足して green（109 passed）。**実装側は変えていない。**
DTO は元から `StageSlug` を通して拒んでいたが、この変種だけ検査が届いていなかった。

## Green の一覧

最終の実測（`cargo fmt --all` を当てたあとに全て回し直した値）。

### この差分のために書いた検査

| 検査 | コマンド | 結果 |
| --- | --- | --- |
| CLI 契約（surface / persist の 23 件） | `cargo test -p aidlc --test learnings_contract` | 23 passed / 0 failed（52.42 秒） |
| 日誌の見出し・1 行・列 | `cargo test -p core-command-domain --lib -- orchestration::memory` | 20 passed / 0 failed |
| 学びの値オブジェクトと判断 | `cargo test -p core-command-domain --lib -- orchestration::learning orchestration::captured orchestration::practice_heading orchestration::intent_execution_event` | 46 passed / 0 failed |
| 集約のコマンド | `cargo test -p core-command-domain --lib -- intent_execution::tests::a_capture intent_execution::tests::nothing_selected intent_execution::tests::a_stage_the_plan intent_execution::tests::capturing_a_learning` | 4 passed / 0 failed |
| `ensure_heading` | `cargo test -p core-command-domain --lib -- workspace::markdown_sections::ensure_heading_tests` | 3 passed / 0 failed |
| 投影 | `cargo test -p core-read-model-updater --lib -- workspace::projection::tests` | 47 passed / 0 failed |

### クレート単位（回帰の確認）

| クレート | コマンド | 結果 |
| --- | --- | --- |
| `core-command-domain` | `cargo test -p core-command-domain` | 876 passed / 0 failed |
| `core-command-use-case` | `cargo test -p core-command-use-case` | 151 passed / 0 failed |
| `core-command-interface-adapter` | `cargo test -p core-command-interface-adapter` | 299 passed / 0 failed |
| `core-read-model-updater` | `cargo test -p core-read-model-updater` | 523 passed / 0 failed |
| `core-query-use-case` | `cargo test -p core-query-use-case` | 72 passed / 0 failed |
| `core-query-interface-adapter` | `cargo test -p core-query-interface-adapter` | 57 passed / 0 failed |
| `core-infrastructure` | `cargo test -p core-infrastructure` | 157 passed / 0 failed |
| `harness-claude` | `cargo test -p harness-claude` | 33 passed / 0 failed |
| `harness-infrastructure` | `cargo test -p harness-infrastructure` | 55 passed / 0 failed |
| `aidlc`（`learnings_contract` と `journal_protocol_conformance` を含む全ターゲット） | `cargo test -p aidlc` | 698 passed / 0 failed |

### 整形・静的検査

| 検査 | コマンド | 結果 |
| --- | --- | --- |
| 整形 | `cargo fmt --all --check` | 差分なし（終了 0）。当初 6 ファイルに差分が出たので `cargo fmt --all` を当て、対象 4 クレートを回し直した |
| clippy | `cargo clippy --workspace --all-targets -- -D warnings` | 終了 0、警告 0 |
| 設計ルール | `cargo lint` | 終了 0、出力なし |

`cargo test --workspace` は回していない。他のエージェントが同じ `target/` を使っており、
ビルドロックの待ちで測れる形にならなかったため、クレート単位に割って全て回した。
上の 10 クレートで、この差分が触れた全てのクレートを覆っている。

### 測定した環境

同じ `target/` を 3〜4 本の `cargo` が同時に使う状態で測っており（負荷平均 4〜6）、
`core-command-domain` の 1 回目はビルドロック待ちで 20 分以上かかった。テストの所要時間は
この待ちを含む値であり、単独実行の値ではない。契約テストの試験装置は 37MB のバイナリを
`fs::hard_link` で配る（複製への切替は失敗時のみ）。この負荷の下で macOS の動的リンカが
`_dyld_start` で停まる事象を観測したが、`sample` で確認したとおりコードの欠陥ではない。


## 本家との突き合わせ

### 確認した観測

| 観測 | 本家の出典 | 本実装 |
| --- | --- | --- |
| surface の封筒（`schema_version` / `stage_slug` / `phase` / `space` / `intent` / `memory_entries_total` / `candidates` / `parked_open_questions` の順） | `aidlc-learnings.ts:205-215,363-372`、ゴールデン `learnings/surface` の stdout | 契約テストで 1 行 JSON を逐語比較 |
| 候補の欄（`id` / `source_heading` / `ts` / `summary` / `context` / `default_scope`）と `c<n>` の採番 | 同 `:195-203,347-356` | 契約テストでキーの並びと値を確認 |
| `Open questions` は候補へ昇格せず別配列に残る | 同 `:341-345`、`stage-protocol.md` §13 の 2 | 契約テスト 1 件＋ドメイン単体 |
| 綴りから外れた日誌の行は前の行へ併合せず 1 件に数える（件数の不変条件） | `aidlc-lib.ts:21481-21491` の逐語注記 | `MemoryEntries` の不変条件テスト |
| 日誌が無い位置は 0 件で出す（ゲートを落とさない） | `aidlc-learnings.ts:325-327` | 契約テスト 1 件 |
| `phase` は `memory_path` の後ろから 3 番目のセグメント | 同 `:329-333` | 単体テスト＋契約テスト |
| 現在位置でない `--slug` を拒否する（逐語） | 同 `:283-293` | 契約テスト 1 件（逐語） |
| runtime-graph が無い／行が無い／`memory_path` が無いを別々に拒否する | 同 `:250-280,317-320` | 契約テスト 2 件 |
| persist の出力（`stage_slug` / `rule_learned` / `sensor_proposed` / `notes`） | 同 `:958-965`、ゴールデン `persist-empty` / `persist-one` / `persist-repeat` | 契約テストで 1 行 JSON を逐語比較 |
| `--slug` と選択ファイルの食い違いを拒否する（逐語） | 同 `:696-701`、ゴールデン `wrong-stage` の stderr | 契約テストでゴールデンと直接照合 |
| `space` の欄が無い選択ファイルを拒否する（逐語） | 同 `:484-486`、ゴールデン `malformed-selection` の stderr | 契約テストでゴールデンと直接照合 |
| 実践行 `- <本文> (learned YYYY-MM-DD) <!-- cid:<intent>:<stage>:<hash> -->` | 同 `:839`、ゴールデン `persist-one` の `project.md` 全文 | ドメイン単体＋契約テストで逐語 |
| 同一性は本文の SHA-256 全桁（切り詰めない） | 同 `:645-654` | ゴールデンの `f543ed…` と一致 |
| 監査行 `## Rule Learned` の 6 欄と並び | 同 `:849-862`、ゴールデン `persist-one` の監査全文 | 投影の単体テストで逐語、契約テストで各欄 |
| `**Destination**:` は `<project-dir>` へ伏せた正本のパス | `aidlc-audit.ts:527`（全欄に `redactProjectDirPrefix` を掛ける）＋ゴールデンの実バイト | `LearningProvenance::destination` が同じ綴りを組む |
| 同じ選択をもう一度流しても増えない | `aidlc-learnings.ts:829-832`、ゴールデン `persist-repeat` の `rule_learned: 0` | 契約テスト 1 件 |
| 同じ本文が 1 回の選択に 2 度あっても 1 本だけ書く | 同 `:790`（`batchRuleHashes`） | ドメイン単体＋契約テスト |
| 実践行だけが消えた状態からは監査行を増やさずに行を戻す | 同 `:834-842`（`!hasLine` で行だけ書く） | 契約テスト 1 件 |
| 監査行だけが消えた状態からは実践行を増やさずに監査行を戻す | 同 `:848-864`（`!hasRow` で監査だけ立てる） | 契約テスト 1 件 |
| 素性は surface の時点で固定し、実行時のカーソルを読み直さない | 同 `:703-711`（LOCAL FIX #2） | 契約テスト 1 件（別 intent を作ってカーソルを移してから persist） |
| 固定した space / 記録が無ければ拒否する（逐語） | 同 `:721-752` | 契約テスト 1 件 |
| 選ばれた見出しが正本に無ければ作ってから足す | 同 `:620-630,836-841` | 投影の単体＋契約テスト |
| 追記は次の `## ` 見出しの直前（節末尾の空行は残る） | `aidlc-lib.ts:22767-22790` | 契約テスト（上記 red 5 で検出力を確認） |
| 選択ファイルの各拒否（JSON 不正・骨格・space 文法・intent 型・intent 形・要素非オブジェクト・candidate_id 不在・heading/text 不在） | `aidlc-learnings.ts:424-521` | 契約テスト 1 件（10 形） |
| 何も選ばれなかった回は 1 バイトも書かない | 同 `:768-772`（空の `learnings`）、ゴールデン `persist-empty` の `changed_files` が空 | 契約テスト 1 件 |

### 確認できなかった範囲

- **センサーの選択**（本家 `:874-928` の manifest 生成と stage frontmatter への束ね、
  `SENSOR_PROPOSED`）は**実装していない**。親の指示で今回の範囲外である。黙って飛ばさず
  自己防衛拒否する（契約テスト 1 件）。`sensor_proposed` は常に 0 を出す。
- **`intent: null`（平置きレイアウト）** は実装していない。本 build の `Layout` は常に
  記録ディレクトリで解決し、状態ファイルも runtime-graph も記録の下にしか無いので、
  surface からこの値は出ない。persist は自己防衛拒否する。本家もこの経路は
  `readStateFile` の裸パスへ落ちるだけで、固定コミットの配布レイアウトには存在しない。
- **旧い印の互換**（本家 `:656-682` の pre-#735 / candidate-id 型 / 8 桁切り詰め）は
  写していない。本 build はこれらの印を一度も書いていないので、照合しても当たる行が無い
  （`coding-rules/no-backward-compatibility.md`）。
- **メモリ層 2 本が揃っていない場合の自動生成**（本家 `practiceFileTemplate`）は
  していない。本 build の投影はメモリ層を**2 本揃って初めて**載せる面として扱い、
  片方でも欠ければ `MemoryFilesMissing` で止まる設計が既にある（`PracticesAffirmed` と共通）。
  persist はその前に自己防衛拒否する。固定本家の配布は
  `aidlc/spaces/default/memory/{org,project,team}.md` を最初から持つので、実環境では
  この分岐に入らない。**フィクスチャ限定の差**である。
- **監査欄の一般的な `<project-dir>` 伏せ字**は本 build に無い。`Destination` は
  伏せ字済みの綴りを組んで書いているので観測は一致するが、他の欄へ絶対パスが載る経路が
  将来できたときは伏せられない。
- **`surface` が投影を追いつかせること**は本家に無い動作である。本家の `surface` は
  状態ファイルをそのまま読むが、本 build の状態ファイルは投影の産物なので、読む前に
  追いつかせないと直前の書込みが見えない。既存の `practices-promote` / `next` と同じ規律に
  合わせた。**読取専用という性質は変わる**ので、ここに記録する。
- **`--project-dir` の扱い**は既存の共通フラグ処理（`Invocation::strip_global_flags`）に
  乗せている。本家は `stripProjectDir` で同じ位置に置くが、綴り単位の比較はしていない。
- **`bun .claude/tools/aidlc-learnings.ts` からの呼出し**（配布フックや SKILL からの実起動）は
  U4 の配布接続の範囲であり、本スライスでは踏んでいない。

### 上流と現行コードの不一致（裁定待ちではない、記録のみ）

1. `IntentDirName::parse` は本 build の記録名の文法（`YYMMDD-<slug>`）を要求する。本家の
   `intent` 検査は「空でない／`.` でない／`..` を含まない／パス区切りを含まない」だけで、
   任意のディレクトリ名を通す。本 build は**より狭い**ので、本家が通す綴りを拒否しうる。
   拒否の文言は本家の逐語（`intent must be a non-empty record-directory name without path
   separators or ".."`）をそのまま使っている。
2. 本家の persist は `withAuditLock` の**中で**監査を読み直して判定し、同じ区間で書く。本
   実装は排他ロックの中で「投影の追いつき → 実測 → コマンド保存 → 投影」を行うが、監査の
   追記そのものは既存の投影経路が持つ排他に従っており、`compile` と同様に**1 つのロック区間
   ではない**。learnings のためだけに別のロックを導入していない。

## 実装上の判断（記録）

- **`rule_learned` の件数は更新ユースケースの戻り値にしていない。** CQS の「Command は
  戻り値なし」を守り、表示材料は同じ純関数（`LearningObservations::captured().audit_rows()`）
  を合成ルートで評価して得ている。報告結果のように読取モデルを新設していないのは、
  出す値が整数 1 つで、集約の状態に依存しない決定的な導出だからである。
- **集約は学びの控えを持たない。** `LearningsCaptured` の適用は空である。控えを持つと
  「監査行だけ／実践行だけが消えた」状態からの復旧ができなくなる（集約が「もう記録した」と
  答えてしまう）。同じ理由で `affects_progress` は偽である。
- **保存 DTO は `content_hash` を持たない。** 本文から導けるので、保存した綴りと導出が
  食い違う余地を構造から無くした。

## 残課題

1. センサーの選択（manifest 生成 + stage frontmatter への束ね + `SENSOR_PROPOSED`）。
   今回の範囲外として自己防衛拒否している。
2. メモリ層 2 本が揃っていないワークスペースでの自動生成。実環境では踏まないが、
   本家との差として残る。
3. 監査欄の一般的な `<project-dir>` 伏せ字。
4. 旧い印の互換（本 build では未使用のため実装していない）。
5. `surface` を配布の SKILL / フックから実起動する経路（U4）。
6. `--project-dir` を含む起動綴りの本家との逐語比較。
