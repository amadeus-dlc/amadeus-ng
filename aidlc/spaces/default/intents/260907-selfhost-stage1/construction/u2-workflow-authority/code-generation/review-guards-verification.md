# レビュー保護フック（review-freeze / reviewer-scope）の実装記録

担当 `u2_review_guards`。対象は [remaining-step7-inventory.md](../remaining-step7-inventory.md) の
「review-freeze」「reviewer-scope」2 行である。本家 2.7.1（固定コミット
`a277af218f0df7f325d3b8be7b6d90fce2c5bd40`、実体は `vendor/aidlc-workflows/dist/claude/.claude/`）を
比較基準にした。TDD（失敗するテストを先に走らせてから最小実装する進め方）で
red → green → refactor の順に進め、各段の生の実行出力を本書へ残す。

用語: 「凍結（freeze）」は終端のレビュー受領証（それ以上レビューを回さない判定の記録）が
立っている間、そのステージの宣言成果物への書込みを拒否することを指す。
「fail-open」は判断に必要な材料が読めないとき、人間の作業を止めずに通す設計をいう。

## 実装した変更

### ドメイン（`core-command-domain`）

| ファイル | 要点 |
| --- | --- |
| `modules/core/command/domain/src/orchestration/write_target.rs` | 書込み先 1 件の値オブジェクト。構築時に `\` を `/` へ畳む（本家は照合直前に畳むが、畳み忘れで保護が外れる経路を作らないため構築時に寄せた）。 |
| `.../orchestration/write_target_error.rs` | 空・空白のみの綴りを拒否する理由。 |
| `.../orchestration/write_targets.rs` | 書込み先の一級コレクション。`first_declared_by` が「そのステージが宣言成果物として名乗る最初の書込み」を拒否材料ごと返す。 |
| `.../orchestration/unit_name.rs` / `unit_name_error.rs` | `construction/<unit>/` の 1 階層名。区切りを含む綴りは Unit ではない（本家 `unit.length > 0 && !unit.includes("/")`）。 |
| `.../orchestration/artifact_target.rs` | 本家 `producesArtifactUnit` の 3 値（`undefined` / `null` / `string`）を `Foreign` / `Stage` / `Unit` の閉集合で持つ。 |
| `.../orchestration/review_freeze_block.rs` | 拒否 1 件の材料（対象・ステージ・Unit）と、その監査項目 `audit_fields`。項目の組み立てを値自身が持つのは、外へ取り出して呼出側が組むと別の書込みの Unit を載せた行が書けるためである。 |
| `.../orchestration/review_freeze_verdict.rs` | 許可／拒否の閉集合。許可に材料は無い。 |
| `.../orchestration/intent_execution.rs` | 集約に `judge_review_freeze(intent, definition, targets)` を追加。 |
| `.../workflow_definition/stage_node.rs` | `is_per_unit` / `produced_artifact_files` / `reviewed_artifact_files` / `reviewed_artifact_target` を追加し、`PER_UNIT_FOR_EACH` を `for_each` の所有者側へ移した。 |
| `.../workflow_definition/workflow_definition.rs` | 上記の定数移動に追従。 |
| `.../workspace/session_audit_record.rs` | フック観測の監査語彙へ `REVIEW_FREEZE_BLOCKED` を追加（必須 `Tool` / `Target` / `Stage`、任意 `Unit`）。 |

`judge_review_freeze` が拒否するのは本家と同じ 3 条件がそろった書込みだけである。

1. レビュアーを宣言したステージの `produces` ∪ `optional_produces` を名指す。
2. そのステージがまだ完了・読み飛ばしでない（`CheckboxState::is_finished()` が偽）。
3. 現在の試行に終端の受領証がある（`ReviewAttempt::has_terminal(policy)`）。

材料は集約の状態と引数で渡す定義だけであり、監査台帳を読み返さない。差し戻し・ジャンプ・
開始が試行を空へ戻す既存の「フロア」の実装をそのまま使うので、本家が
`GATE_REJECTED` / `STAGE_JUMPED` / `WORKFLOW_STARTED` で凍結を解く挙動が同じ経路で成立する。

### 更新ユースケース（`core-command-use-case`）

| ファイル | 要点 |
| --- | --- |
| `modules/core/command/use-case/src/orchestration/guard_review_freeze_use_case.rs` | 実行・計画・定義を再構成して判定し、拒否のときだけ `REVIEW_FREEZE_BLOCKED` を 1 件保存する。 |
| `.../orchestration/review_freeze_error.rs` | 材料が読めない／記録に失敗したときの原因。判定の結末（許可・拒否）は誤りではないのでここに無い。 |

判定結果を戻り値にしたのは、ハーネスの PreToolUse 契約が「拒否なら exit 2 と理由」であり、
呼出側が結末を知らなければ保護が成立しないためである。返すのは値オブジェクトであって
表示材料ではなく、文言は出力側が組む。許可のときは何も書かない（本家も許可の監査行を持たない）。

### ハーネス側（`harness-claude`）

| ファイル | 要点 |
| --- | --- |
| `modules/harness/claude/src/write_tool_envelope.rs` | PreToolUse の封筒から工具名と書込み先を読む。本家 `WRITE_TOOLS` の 4 工具と `file_path` / `notebook_path` / `path` / `paths[]` を掲載順に集める。 |
| `modules/harness/claude/src/reviewer_scope_envelope.rs` | 本家 `candidateStrings` の工具別の鍵、レビュー専用エージェント 2 種、`construction/` 到達の判定。 |
| `modules/harness/claude/Cargo.toml` | 値オブジェクトを組むため `core-command-domain` を依存に追加。 |

### 実行入口（`aidlc`）

| ファイル | 要点 |
| --- | --- |
| `modules/app/aidlc/src/runtime.rs` | `run_hook` のフック名一覧へ `review-freeze` と `reviewer-scope` の 2 行を追加し、分岐を接続した。他担当の行は触っていない。 |
| `modules/app/aidlc/src/runtime/review_guards.rs` | 2 フックの本体。停止スイッチ → 稼働記録 → 封筒 → 台帳の有無 → 判定 → 拒否の保存と投影 → exit 2 の順。 |
| `modules/app/aidlc/tests/review_guards_contract.rs` | 実行バイナリを子プロセスで呼ぶ 12 件の契約テスト。 |
| `modules/app/aidlc/src/wording.rs` | 拒否理由 `review_freeze_blocked` と復帰案内 `review_freeze_recovery` を本家の逐語で追加。 |

## Red の再現

各項目は、実装を最小の無処理へ戻した状態でテストを実行し、意図した振る舞いの不一致で
失敗することを確かめてから実装した。5・7・8 は実装を書いた直後に同じ手順で戻して red を
取り直したものであり、テストを先に書いてから実装した 1〜4・6 とは順序が異なる。ここを
曖昧にしないため明記する。


### 1. 集約の凍結判定（`judge_review_freeze` を常に許可のまま置いた状態）

```
cargo test -p core-command-domain --lib orchestration::intent_execution::tests::
```

```
---- orchestration::intent_execution::tests::a_declared_artifact_write_is_frozen_while_the_terminal_receipt_stands stdout ----
thread '...' panicked at modules/core/command/domain/src/orchestration/intent_execution.rs:8613:37:
凍結する

---- orchestration::intent_execution::tests::a_completed_stage_no_longer_freezes_its_artifact stdout ----
assertion failed: run.execution.judge_review_freeze(&run.intent, &definition,
        &targets(&[REQUIREMENTS])).is_blocked()

---- orchestration::intent_execution::tests::an_effective_none_class_never_freezes stdout ----
adversarial の READY は終端である

test result: FAILED. 164 passed; 5 failed; 0 ignored; 0 measured; 580 filtered out; finished in 0.64s
```

### 2. 監査語彙（`REVIEW_FREEZE_BLOCKED` を追加する前）

```
cargo test -p core-command-domain --test session_audit_contract
```

```
---- a_review_freeze_block_carries_its_tool_target_stage_and_optional_unit stdout ----
必須 3 項目だけで構築できる: InvalidRecord

---- a_review_freeze_block_is_recorded_whatever_the_workflow_status_says stdout ----
called `Result::unwrap()` on an `Err` value: InvalidRecord

test result: FAILED. 7 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

### 3. 書込み封筒（`parse` が書込み先を返さない状態）

```
cargo test -p harness-claude --lib write_tool_envelope
```

```
---- write_tool_envelope::tests::a_write_names_its_file_path stdout ----
assertion `left == right` failed
  left: []
 right: ["/w/x/inception/requirements-analysis/requirements.md"]

test result: FAILED. 3 passed; 3 failed; 0 ignored; 0 measured; 14 filtered out; finished in 0.00s
```

### 4. reviewer-scope の封筒（`parse` が空の封筒を返す状態）

```
cargo test -p harness-claude --lib reviewer_scope_envelope
```

```
---- reviewer_scope_envelope::tests::a_reviewer_reading_a_construction_path_is_both_inspected_and_flagged stdout ----
assertion failed: envelope.inspected()

test result: FAILED. 2 passed; 4 failed; 0 ignored; 0 measured; 20 filtered out; finished in 0.00s
```

### 5. 逐語文言（2 関数を空文字返しに置いた状態）

```
cargo test -p aidlc --lib wording::tests::the_freeze
```

```
---- wording::tests::the_freeze_reason_names_the_scope_and_both_sanctioned_routes stdout ----
assertion `left == right` failed
  left: ""
 right: "review-freeze: \"a/requirements.md\" is this stage's output document for stage ..."

test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 296 filtered out; finished in 0.00s
```

### 6. フックの入口（2 フックを名前だけ通し、本体を無処理にした状態）

```
cargo test -p aidlc --test review_guards_contract
```

```
---- a_declared_artifact_write_is_refused_and_recorded_while_the_receipt_stands stdout ----
assertion `left == right` failed: Output { status: ExitStatus(unix_wait_status(0)), stdout: "", stderr: "" }
  left: Some(0)
 right: Some(2)

---- the_freeze_hook_records_its_heartbeat stdout ----
稼働記録が残る: ".../260909-guards/.aidlc-hooks-health/review-freeze.last"

---- the_reviewer_scope_hook_records_its_heartbeat_and_advises_once_on_a_missing_record stdout ----
assertion failed: health.join("reviewer-scope.last").is_file()

test result: FAILED. 6 passed; 3 failed; 0 ignored; 0 measured; 0 filtered out; finished in 13.80s
```

### 7. 差し向け記録が在るときの助言の絞り込み（絞り込む前の状態）

```
cargo test -p aidlc --test review_guards_contract a_present_dispatch_record
```

```
---- a_present_dispatch_record_is_reported_as_unported_enforcement_for_the_reviewer_only stdout ----
thread '...' panicked at modules/app/aidlc/tests/review_guards_contract.rs:425:5:
強制の対象でない呼び手に助言は出さない

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 10 filtered out; finished in 1.35s
```

### 8. シェル書込みが未検査であることの記録（記録する前の状態）

```
cargo test -p aidlc --test review_guards_contract a_shell_write
```

```
---- a_shell_write_is_allowed_but_recorded_as_uninspected stdout ----
thread '...' panicked at modules/app/aidlc/tests/review_guards_contract.rs:558:5:
未検査であることを目印に残す

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 11 filtered out; finished in 2.61s
```

いずれも「テストが見つからない」「依存不足」「構文不正」ではなく、意図した振る舞いの不一致で
失敗している。

## Green の一覧

### 追加・変更した境界

| 検査 | コマンド | 結果 |
| --- | --- | --- |
| 集約・値オブジェクト | `cargo test -p core-command-domain --lib orchestration::` | 442 passed / 0 failed |
| ドメイン全体 | `cargo test -p core-command-domain` | 754 passed（lib）ほか全 target 0 failed |
| 監査語彙 | `cargo test -p core-command-domain --test session_audit_contract` | 9 passed / 0 failed |
| 更新ユースケース | `cargo test -p core-command-use-case` | 0 failed |
| ハーネス封筒 | `cargo test -p harness-claude --lib` | 26 passed / 0 failed |
| 逐語文言 | `cargo test -p aidlc --lib wording::tests` | 39 passed / 0 failed |
| フックの入口 | `cargo test -p aidlc --test review_guards_contract` | 12 passed / 0 failed |

### 変更した境界の既存回帰

| コマンド | 結果 |
| --- | --- |
| `cargo test -p aidlc --lib` | 298 passed / 0 failed |
| `cargo test -p core-command-interface-adapter --test intent_execution_repository_contract` | 24 passed / 0 failed |
| `cargo test -p core-command-interface-adapter --test commit_verdict_use_case_wiring_test` | 1 passed / 0 failed |
| `cargo test -p core-command-interface-adapter --test golden_parity_test` | 11 passed / 0 failed |
| `cargo test -p core-read-model-updater --test projection_golden_test` | 20 passed / 0 failed |
| `cargo test -p core-read-model-updater --test audit_block_golden_test` | 1 passed / 0 failed |
| `cargo test -p core-read-model-updater --test publication_recovery_contract` | 33 passed / 0 failed |
| `cargo test -p core-query-interface-adapter --test read_model_dao_contract` | 34 passed / 0 failed |
| `cargo test -p aidlc --test intent_lifecycle` | 123 passed / 0 failed |
| `cargo test -p aidlc --test next_branches` | 44 passed / 0 failed |
| `cargo test -p aidlc --test session_hooks_contract` | 16 passed / 0 failed |
| `cargo test -p aidlc --test claude_hook_contract` | 10 passed / 0 failed |
| `cargo test -p aidlc --test cli_golden_test` | 10 passed / 0 failed |
| `cargo test -p aidlc --test steering_across_processes` | 6 passed / 0 failed |
| `cargo test -p aidlc --test journal_protocol_conformance` | 5 passed / 0 failed |
| `cargo test -p aidlc --test review_receipt_contract` | 7 passed / 0 failed |
| `cargo test -p aidlc --test diagnostic_record_contract` | 7 passed / 0 failed |
| `cargo test -p core-infrastructure --test golden_hash_canonical` | 7 passed / 0 failed |
| `cargo test -p core-infrastructure --test golden_corpus_read` | 14 passed / 0 failed |

### 静的検査

| 検査 | コマンド | 結果 |
| --- | --- | --- |
| 整形 | `rustfmt --edition 2024 --config-path rustfmt.toml --check <変更ファイル>` | 差分なし |
| 静的検査 | `cargo clippy -p core-command-domain -p core-command-use-case --all-targets -- -D warnings` | 所見なし |
| 静的検査（app） | `cargo clippy -p aidlc --all-targets -- -D warnings -A clippy::indexing_slicing -A clippy::string_slice` | 所見なし |
| 独自検査 | `cargo lint` | 0 件（1058 ファイル走査） |

`cargo lint` は途中 2 回所見を出し、いずれも是正した。

- `checkbox-vocabulary`（集約側）: `matches!(CheckboxState::Completed | Skipped)` を
  `CheckboxState::is_finished()` へ置き換えた。
- `use-case-domain-getter`（ユースケース側）: 監査項目の組み立てを
  `ReviewFreezeBlock::audit_fields` へ移し、ユースケースからドメインの getter を呼ばないようにした。
- `wording.rs` の復帰案内は本家の逐語 5 文への全域写像であり、既存 3 述語では
  Completed / Skipped / Revising を撃ち分けられない。分類をドメインへ足すと文言選択が
  ドメインへ漏れるため、理由を明示した許可注釈を置いた。
- `serde_json::json!` はテストで使えない（`clippy.toml` が契約 JSON の直列化を canon-json に
  固定している）。フック入力は逐語の文字列として組み直した。

### 実行していない検査

- **`cargo test --workspace` は完走していない。** 3 度開始し、いずれも他担当の同時編集で
  再コンパイルが挟まって無効化した。最後に到達した地点で 10 target が
  `test result: ok`（0 failed）だった。承認済み[テスト手順](unit-test-instructions.md)は
  workspace 全体・カバレッジ・Quint/ITF・CI・依存監査を「B1 統合前の共通検査」と位置づけ、
  本スライスの単位限定コマンドと区別している。ここでは上表の単位限定・回帰コマンドまでを実測した。
- **`cargo clippy --workspace` は完走していない。** 他担当が編集中の
  `modules/harness/claude/src/runtime_compile_envelope.rs` が `indexing_slicing` /
  `string_slice` で 6 件の所見を出し、依存クレートの段階で止まる。本担当のファイルは
  この 2 つを除外した実行で所見なしを確認した。**この 6 件は本担当の変更ではない。**
- カバレッジ（行カバレッジ床 90.0%・相対ゲート）と Quint / ITF は再実行していない。

## 本家との突き合わせ

### 確認した観測

| 観測 | 本家の出典 | 本 build |
| --- | --- | --- |
| 停止スイッチ `AIDLC_DISABLE_REVIEW_FREEZE_HOOK=1` / `AIDLC_DISABLE_REVIEWER_SCOPE_HOOK=1` で完全に無効化 | `aidlc-review-freeze.ts:257`、`aidlc-reviewer-scope.ts:758` | 同じ。`=1` の一致のみ。 |
| 稼働記録は判断より先で、失敗しても判断を変えない | 同 `:260-268` / `:761-769` | `observe_hook_health` の戻り値を無視する。 |
| 読めない標準入力・工具外の呼出しは通す | 同 `:270-282` | 封筒が書込み先 0 件を返し、そのまま通す。 |
| 監査台帳が無ければ状態・グラフを読まずに通す | 同 `:284-291` | 監査ディレクトリに `.md` が 1 枚も無ければ通す。 |
| 拒否は `Tool` / `Target` / `Stage`（per-unit のみ `Unit`）の `REVIEW_FREEZE_BLOCKED` を残す | 同 `:344-356` | 同じ項目・同じ任意性。 |
| 拒否は標準エラーへ理由を書いて exit 2、標準出力は汚さない | 同 `:371-372` | 同じ。 |
| 拒否理由の逐語と、ゲートでの引用／差し戻しという 2 経路の提示 | 同 `blockReason`（`:240-252`） | 同じ文面。 |
| 復帰案内の 5 分岐（SKIP・pending/skipped・in-progress/awaiting-approval・revising・completed） | `aidlc-lib.ts:16737-16790` | 同じ文面。状態ファイルの checkbox 行から読む。 |
| 完了・読み飛ばしのステージは凍結しない | `aidlc-review-freeze.ts:19-24,297-303` | `CheckboxState::is_finished()` で除外。 |
| 上限未満の NOT-READY は終端でないので凍結しない | 同 `:32-34` | `ReviewPolicy::is_terminal` の既存実装で成立。 |
| 差し戻しがフロアを戻して凍結を解く | 同 `:28-31` | 集約の試行リセットで成立（テストで確認）。 |
| 宣言成果物の照合は接尾辞 `/<slug>/<filename>` 一致 | `aidlc-lib.ts:7927-7956` | 同じ。`traceability` → `traceability.json` 等の例外表も同じ。 |
| per-unit の Unit 抽出は `/construction/` の最後の出現から 1 階層 | 同 `:7986-7998` | 同じ。区切りを含む綴りは Unit にしない。 |
| reviewer-scope の記録不在の助言は 10 分に 1 度 | `aidlc-reviewer-scope.ts:845-856` | 同じ窓。目印は `.aidlc-hooks-health/reviewer-scope.missing-record.last`。 |
| 助言の対象はレビュー専用 2 エージェントが `construction/` へ触れたときだけ | 同 `:748`、`:838-844` | 同じ 2 名・同じ文字列一致（`construction/` は区切り込み）。 |
| ゼロ Unit の実行で requirements-analysis と code-generation の両方に効く | 依頼の必須観測 | 実行バイナリで確認（`the_freeze_applies_to_code_generation_in_a_zero_unit_run`）。 |
| `Bash` も書込みとして検査する | `aidlc-review-freeze.ts:44-49`、`review-freeze-command.ts:1014-1022` | **未移植**。通すが drop へ残す。 |
| Unit 配下の書込みは拒否文言と監査に Unit を載せる | `aidlc-review-freeze.ts:244`、`:351` | 同じ（`unit "u1-x"` を確認）。 |

### 確認できなかった範囲・写していない分岐

- **本家フックの実走行との突き合わせは行っていない。** `tests/golden/selfhost-stage1/` に
  review-freeze / reviewer-scope の採取が無く、本工程で新規採取もしていない。突き合わせは
  固定コミットのソース読解と本 build のプロセス実測に限る。
- **`Bash` の書込み先解析を移していない。** 本家は `review-freeze-command.ts` の
  `shellWriteTargets`（リダイレクト先と変更系コマンドの操作対象の抽出、約 400 行）を通すが、
  本 build は `Bash` を書込み先 0 件として通す。**シェル経由の書込みは現状凍結されない。**
  黙って通してはおらず、`Bash` の呼出しは 10 分に 1 度
  `.aidlc-hooks-health/review-freeze.drops` へ「書込み先を検査せずに通した」と残す
  （`a_shell_write_is_allowed_but_recorded_as_uninspected` が実測する）。
- **per-unit の受領証は本 build に無い。** `ReviewPolicy::per_unit` は「未配線」と既存 doc が
  明記しており、`ReviewAttempt` はステージ 1 つに 1 つである。したがって per-unit ステージでも
  同じステージの試行を見る。本家は Unit ごとの受領証地図を持ち、ステージ水準の書込みでは
  Unit 側だけを見るので、**ゼロ Unit の per-unit ステージについて本家は凍結せず本 build は
  凍結する**という差が残る。今回の依頼は「ゼロ Unit でも requirements-analysis /
  code-generation に適用」であり、本 build の受領証の持ち方に沿って実装した。
  兄弟 Unit の分岐は依頼どおり対象外。
- **本家の `stagePending` / `unitPending` / `sourceStale`（成果物 fingerprint による復旧・
  中断の分岐）は写していない。** 既存 `ReviewClosures` の doc が「本 build は fingerprint を
  繰延している」と明記しており、非適用である。
- **reviewer-scope の越境拒否は移していない。** 差し向け記録
  （`.aidlc-reviewer-dispatch.json`）が在る呼出しは通すが、レビュー専用エージェントの
  呼出しに限り 10 分に 1 度 drop へ「強制が未配線である」と残す。黙って通してはいない。
  記録の TTL 掃除、`scoped_registration` による識別、team unit ownership の
  claimed-checkout 分岐も未移植。
- **`REVIEWER_SCOPE_BLOCKED` の保存経路は作っていない。** 上の越境拒否と対で、次のスライスの対象。
- `reverse-engineering` の codekb 分岐（`producesArtifactFile` の前半）は写していない。
  現行のコンパイル済みグラフでレビュアーを宣言するステージに codekb 出力は無く、
  凍結の判断に到達しない。

## 残課題

1. `Bash` の書込み先解析（`shellWriteTargets`）の移植。既存の
   `harness-infrastructure` のシェル解析部品を土台にできる。
2. reviewer-scope の越境拒否と `REVIEWER_SCOPE_BLOCKED` の保存、差し向け記録の TTL 掃除。
3. 本家フックの実走行採取と、状態・監査の逐語比較。
4. per-unit 受領証を配線するかどうかの裁定。配線しない限り、上記のゼロ Unit の差は残る。
5. `SessionAuditRecord` は「セッション」を名乗りながらフック観測一般の監査語彙になっている。
   改名は今回の範囲外だが、語彙の正本としては呼び名の裁定が要る。
6. `cargo test --workspace` と `cargo clippy --workspace` の完走。前者は同時編集の落ち着いた
   時点で、後者は `modules/harness/claude/src/runtime_compile_envelope.rs` の 6 件が
   是正された時点で行う。
7. フックの登録（`.claude/settings.json` を TS から `aidlc hook <name>` へ向ける）は
   セルフホスト切替の作業であり、本スライスでは触っていない。

## Sources

- 本家 `vendor/aidlc-workflows/dist/claude/.claude/hooks/aidlc-review-freeze.ts:1-46,250-375`、
  `hooks/aidlc-reviewer-scope.ts:756-860,863-932`、
  `hooks/review-freeze-command.ts:775-1034`、
  `tools/aidlc-lib.ts:7927-7998,16737-16790`、
  `tools/aidlc-artifact-vocabulary.ts:1-19`、
  `aidlc-common/protocols/stage-protocol-reviewer.md:96-98,155`。
- 現コード `modules/core/command/domain/src/orchestration/review_closures.rs`、
  `review_attempt.rs`、`intent_execution.rs`、
  `modules/core/command/domain/src/workspace/{audit_events.rs,session_audit_record.rs,checkboxes.rs}`、
  `modules/app/aidlc/src/runtime.rs`、`modules/app/aidlc/tests/review_receipt_contract.rs`。
- 承認済み [実装計画](code-generation-plan.md)、[テスト手順](unit-test-instructions.md)、
  [残作業棚卸し](../remaining-step7-inventory.md)。

## Assumptions & Open Questions

- 上の「確認できなかった範囲」に挙げた 3 点（シェル書込みの未検査、per-unit 受領証の差、
  本家実走行の未採取）は、裁定または後続スライスが要る。読み替えて閉じていない。
