# u2_cov_app 検証報告 — `aidlc` crate（`modules/app/aidlc/`）のカバレッジ向上

担当: `u2_cov_app`。所有 crate は `aidlc` のみ。プロダクトコードは変更していない（テスト追加のみ）。

## 1. 計測条件

- コマンド: `cargo llvm-cov --no-report -p aidlc --tests --no-fail-fast` → `cargo llvm-cov report --json|--text --ignore-filename-regex '(^|/)modules/app/aidlc/src/main\.rs$'`、`PROPTEST_RNG_SEED=20260823`。
- **`CARGO_TARGET_DIR=target/u2_cov_app`（担当専用）**で計測した。共有の `target/llvm-cov-target` は並行する他担当の `cargo llvm-cov clean --workspace` に消され、1 回目の着手前計測がテストバイナリ消失で失敗したため（[01-before-measure.log](u2_cov_app/01-before-measure.log) の冒頭に残した）。閾値・除外・seed は `scripts/coverage.sh` と同じで、変更していない。
- 数値は llvm-cov の JSON summary（`files[].summary.lines`）で、`scripts/coverage.sh` の判定と同じ土俵である。crate 内 `#[cfg(test)]` の行も分母に入る（追加したテストコードで着地後の総行数が 16,061 → 16,867 に増えている）。

## 2. 着手前後の行カバレッジ

| 対象 | 着手前 | 着地後 |
| --- | --- | --- |
| `aidlc` crate 全体（`main.rs` 除外） | 14,849 / 16,061 = **92.45 %**（未カバー 1,212） | 16,212 / 16,867 = **96.12 %**（未カバー 655） |
| 対象 38 ファイル（[targets-app.md](targets-app.md)）の未カバー合計 | **1,195** | **638**（−557、**46.6 % 減**） |
| 同・`count=0` の行（`report --text` で実行回数 0 の行）合計 | 約 790（crate 全体 805） | **276**（crate 全体 291） |

**完了条件 1（85 % 減）には届いていない。** 減らせたのは 46.6 % である。理由は §5 に書く。テストは crate 全体で 889 件成功・0 失敗（[02-after-measure.log](u2_cov_app/02-after-measure.log)、llvm-cov 計測下の `--no-fail-fast` 実行）。`cargo fmt -p aidlc -- --check`・`cargo clippy -p aidlc --all-targets -- -D warnings`・`cargo lint` はいずれも成功（[04](u2_cov_app/04-fmt.log) / [05](u2_cov_app/05-clippy.log) / [03](u2_cov_app/03-lint.log)）。

### ファイル別（対象 38 ファイル）

| ファイル | 着手前 未カバー | 着地後 未カバー | 着地後 count=0 行 | 残りの count=0 行 |
| --- | ---: | ---: | ---: | --- |
| `runtime.rs` | 234 | 164 | 79 | 262 316 395 514 549 571-572 574 590 890 898-899 905 923 935 940 942 955 959-961 964 1065 1115 1123 1238-1240 1251-1258 1310 1330 1343 1345 1347 1353 1356 1359 1433 1442-1443 1446 1449-1450 1522 1525 1531-1532 1541-1543 1570 1664 1670 1902 2038 2146-2147 2254 2291 2375 2531 2545-2547 2549 2675-2677 2679 3718 4206 4232 |
| `runtime/session_start.rs` | 109 | 5 | 1 | 68 |
| `runtime/plan_approval.rs` | 84 | 68 | 12 | 52 111 191 211 244 342 348 499 633 641-642 645 |
| `turn.rs` | 80 | 61 | 25 | 334-338 497 713 725 890-893 900-903 921-924 965-968 1006 |
| `runtime/continuation.rs` | 75 | 60 | 21 | 114 135 142 147 150 206 232 249 253 258-260 264-266 297 310 329 361 469 480 |
| `runtime/learnings.rs` | 57 | 25 | 13 | 93 97-98 151 179 268 276 290 299 372 375 423-424 |
| `runtime/jump.rs` | 52 | 19 | 0 |  |
| `runtime/testing_posture.rs` | 50 | 23 | 2 | 166 169 |
| `source_baseline.rs` | 49 | 22 | 10 | 101 200 205 226 277 281 312 316 346 581 |
| `session_navigation.rs` | 48 | 3 | 2 | 196 214 |
| `runtime/pipeline_link.rs` | 42 | 33 | 14 | 47 91 103 119 133 174-177 186-189 225 |
| `wording.rs` | 33 | 0 | 0 |  |
| `runtime/review_documents.rs` | 25 | 20 | 7 | 53 93 110 118 121 150 157 |
| `runtime/review_guards.rs` | 24 | 19 | 19 | 52 63 73 98 102 107-110 113 148 167 183 188 215-217 221 315 |
| `runtime/session_hooks.rs` | 23 | 14 | 8 | 39 145 165 206 219 224 242 248 |
| `session_processes.rs` | 21 | 20 | 16 | 23 27-32 61 69-71 85 120 127 194 203 |
| `source_fingerprint.rs` | 16 | 8 | 6 | 42 47 58-59 83 93 |
| `summary_questions_input.rs` | 16 | 6 | 1 | 35 |
| `runtime/task_sync.rs` | 16 | 4 | 2 | 16 19 |
| `layout.rs` | 14 | 0 | 0 |  |
| `runtime/runtime_graph.rs` | 14 | 9 | 3 | 55 70 80 |
| `validation_basis.rs` | 13 | 6 | 2 | 324 421 |
| `cli/request.rs` | 10 | 6 | 5 | 334 341 348 618 672 |
| `usage_ledger/stage_bucket.rs` | 10 | 0 | 0 |  |
| `usage_ledger/model_rates.rs` | 9 | 2 | 1 | 115 |
| `runtime/dispatch_rules.rs` | 8 | 7 | 5 | 265-268 271 |
| `runtime/continuation_cursor.rs` | 8 | 6 | 1 | 188 |
| `intent_location.rs` | 8 | 0 | 0 |  |
| `directive_drawing.rs` | 6 | 6 | 2 | 151 156 |
| `usage_ledger.rs` | 6 | 6 | 6 | 98-99 113 121 173 253 |
| `runtime/fold_usage.rs` | 6 | 1 | 0 |  |
| `workspace_scanner.rs` | 6 | 1 | 1 | 85 |
| `usage_ledger/ledger_cursor.rs` | 5 | 0 | 0 |  |
| `usage_ledger/ledger_lock.rs` | 4 | 4 | 4 | 42-45 |
| `runtime/log_failure.rs` | 4 | 2 | 1 | 21 |
| `lexical_path.rs` | 4 | 4 | 4 | 7-10 |
| `presenter.rs` | 3 | 3 | 2 | 254 262 |
| `cli/interaction_args.rs` | 3 | 1 | 1 | 27 |

「着地後 未カバー」（summary）と「count=0 行」の差は、**1 度も呼ばれなかったクロージャ**（`.map_err(|error| error.to_string())` のような失敗経路専用の 1 行クロージャ）の行である。llvm-cov の summary は関数（クロージャを含む）ごとに行を数えるので、その行を含む文が実行済みでもクロージャ自身が未実行なら未カバーに数える。`report --text` の行表示では実行回数が付くため見えない。crate 全体では summary 655 行のうち 364 行がこの種で、対象ファイルの残り 638 行のうち 362 行を占める。

## 3. 追加したテスト（合計 90 件）

### 3.1 契約テスト `modules/app/aidlc/tests/refusal_paths_contract.rs`（新規、51 件）

配布定義（`tests/golden/upstream-a277af21/data`）を持つ一時ワークスペース上で、`aidlc::runtime::run` を同一プロセスで起動名ごとに呼び、stdin を要するフックだけ子プロセス（`coverage_profile_env()` を渡す）で打つ。検証した契約は次のとおり。

| 領域 | 件数 | 検証した契約 |
| --- | ---: | --- |
| `aidlc-learnings` | 8 | `--help`/`-h` の本文、未知動詞の逐語、記録なし／曖昧カーソル（`learnings_ambiguous_intent`）の区別、`persist` の欠落 space・欠落 intent・method file 欠落・`candidate_id` 複数行の拒否、`Current Stage` の無い状態・slug 不一致の拒否、手書き `runtime-graph.json`（壊れた JSON／`memory_path` 無し／stage 無し）の拒否と正常な surface、実行カーソル喪失／破損の拒否 |
| `aidlc-log answer/decision` | 4 | フラグ値欠落、`--details` 欠落、未知 `--checkpoint`、summary choice 不一致の逐語、`--unit`/`--single` 未配線の拒否、実行なしの拒否（ストアを作らない）、plan-approval の対象指定（`--unit` xor `--stage-level`）と選択肢（`Approve Plan`/`Request Changes`）の拒否、questions-file の記録外／不在／不正構造／`[Answer]:` 不一致 |
| `aidlc-log review` | 2 | 保留中の反駁を残したままの次回依頼（`PendingIterations`、合成 adversarial 定義）、依頼前から在る `## Review` 節は鮮度の無い証拠（`StaleAppendix`） |
| `aidlc-log link` | 1 | `./`・`..` を含む綴りの正規化、FIFO の handoff を読まずに断る、`--repo` の未登録拒否、カーソル喪失／破損の拒否 |
| `aidlc-jump` | 3 | 未知動詞・不正方向・`resolve` の使い方・未知 phase・到達不能 phase の逐語、`codekb/` 配下のリポジトリ列挙、拡張子付き成果物名の観測 |
| `aidlc-testing-posture` | 2 | 未知動詞／未配線 `verify`、対象指定の 3 種の拒否、記録なしの拒否、`resolve` がゴールデン `empty-bugfix-brownfield` の契約 JSON を出す、memory 層の矛盾（`contradicting-project-method`）を `resolve`/`render` が逐語で断る |
| 壊れた保存物 | 3 | 読めない実行カーソル・開けないストア・文法外 active-space を、各動詞（answer/decision/review/link/set-autonomy/jump/report/testing-posture）が「未鋳造」と混ぜず原因の文言で断る |
| `next` | 4 | `plugin`/`knowledge` 名詞トークンの終端案内（ワークフローを読まない）、brownfield のコスト節が greenfield より 1 段多い、リード帽と支援エージェントの語り（支援 0/1/2 名）、`--single`/継続の連鎖 |
| hooks: `session-start` | 6 | 別 intent に印された `resume` の再選択案内と署名の抑制、`rebind_check` 照会、別 space への案内、`Active Unit` 行、未コンパイル stage の列挙、状態ファイル無し／読めない状態／保存失敗の drop／固定と食い違う印の消去 |
| hooks: `continue-workflow` | 4 | 規則配送前の block 文言（`rules_content` を含む）、会話の切り出し（人間の印がエンジンより新しい）、64 KiB 超の指示は待ちにしない、壊れた block-count は履歴なしとして扱う |
| hooks: その他 | 12 | 未知フック名、`write-audit-log` の空パス／記録外／台帳自身／相対パス／codekb 文脈／状態ファイル無しの記録、`sync-workflow-state` の沈黙とストア失敗、`validate-state` の印の段階、`rebuild-stage-graph` の発火しない条件と文法外 stage の走査、`log-subagent` の非文字列 message 拒否と保存失敗の drop、`session-end` の drop と版違い印、`review-freeze`/`reviewer-scope` の開いて通す条件、`deliver-stage-rules` のグラフ不読、共有承認ストアの初期化印（版違い／壊れ／ディレクトリ／印なし／ストア喪失）、runtime ストアがディレクトリ、出所ストアを失った保留発行の回復拒否 |

### 3.2 `src` 内の単体テスト（39 件）

| ファイル | 件数 | 検証した契約 |
| --- | ---: | --- |
| `source_baseline.rs` | 5 | worktree 根の拒否と `read` の `None`/空、壊れた登録簿の拒否 3 種、FIFO／リンク先種別の一覧化、ビルド定義ファイル名の source 判定、リンクの輪で止まる・先の無いリンクは `missing` |
| `source_fingerprint.rs` | 2 | `.git` 不算入と FIFO の `Unsupported`、ハーネスディレクトリ判定の 6 条件 |
| `session_navigation.rs` | 4 | 印・案内・引継ぎの書込／空値／冪等消去、ディレクトリで塞がれた印の失敗、transcript の 2 か所への複写と失敗の黙殺、`bootstrap` の規則スタブ書換 |
| `intent_location.rs` | 2 | 文法外 space・壊れた registry・空 uuid・状態ファイル無しの飛ばし、`current` の registry 引き |
| `layout.rs` | 3 | 切替印の新鮮さ／印一致／registry 一致、期限外の清掃と読めない印の温存、`boundAt` 空の固定無効と space だけの固定 |
| `validation_basis.rs` | 3 | 重複成果物の一度採取、per-unit 段の scope ゲート、ファイルでない／読めない成果物の要約、`read` の既定と未知 stage の警告 |
| `workspace_scanner.rs` | 2 | 言語の数え上げ（1/5 閾値・リンク不追跡）、Greenfield／Cargo の Brownfield |
| `session_processes.rs` | 2 | pid 記録の読取条件、掃除（死んだ pid・読めない記録）と祖先からの `resolve` |
| `wording.rs` | 3 | learnings 面の逐語 8 種、`resume_menu`、heartbeat EISDIR 描画の限定 |
| `runtime/continuation_cursor.rs` | 3 | 非数値の版・非文字列欄、上限超 marker、記録なしの `Current`、開けないロックの `Busy` |
| `runtime/dispatch_rules.rs` | 1 | 明示 stage パスの境界（区切り・slug 先頭・`.md\b`） |
| `runtime/fold_usage.rs` | 2 | session 鍵の優先順、workflow 鍵の後退 |
| `runtime/review_documents.rs` | 2 | 通常成果物の束縛と任意成果物の欠落、ディレクトリ／ハードリンク／シンボリックリンク経路の不安定判定 |
| `usage_ledger/{stage_bucket,ledger_cursor,model_rates}.rs` | 4 | 旧形の包み直し・壊れた内訳、`byteOffset` の型規則、壊れた単価ファイルの無視と provider 接頭辞 |
| `turn.rs` | 1 | `read_steering_plan` を引けない run-stage の読取失敗 |

## 4. dead code 候補（プロダクトは変更していない。根拠付き）

| 箇所 | 根拠 |
| --- | --- |
| `turn.rs:334-338`（分岐 6 `resume-menu` の Ask）と `wording::resume_menu`（`wording.rs:423-427`） | `grep -rn "NextDecision::ResumeMenu" modules/core modules/app` の構築箇所は 0 件（`engine_signal.rs`/`next_answer_row.rs`/`spelling.rs` の match 腕とテストのみ）。`core/read-model-updater/tests/read_tables_test.rs:916` は `!kinds.contains("resume-menu")`「集約がもう答えない」を固定している。文言関数自体は単体テストで固定した。 |
| `runtime.rs:1250-1259`（`layout_for_artifact_only` の `record_dir().is_some()` 枝） | `Layout::state_file()` は `record_dir.as_ref().map(...)`（`layout.rs:254-258`）なので `record_dir` が `Some` なら直前の `state_file().is_some()` で必ず返る。到達不能。 |
| `runtime.rs:898-899, 905, 923, 935, 940, 942`（`committed_directive` の `invalid()` 腕） | 投影された `report_result` の `steps`/`no_op_reason` が閉集合外のときの防御。RMU は自分の綴りしか書かないので、読み面を手で壊さない限り到達しない。 |
| `runtime/pipeline_link.rs:47`、`runtime/review_guards.rs:52` | `state_file()` が `Some` なら `record_dir()` も `Some`、`audit_dir()` が `Some` なら `record_dir()` も `Some`（同じ `map`）。到達不能。 |
| `runtime/pipeline_link.rs:133` | `relative` は `record.join(...)` を `project_dir` から剥いだ綴りなので `Normal` 以外の成分を持たない。 |
| `runtime/session_start.rs:68`、`runtime/session_hooks.rs:39,145,219` | `AuditFieldKey::parse` に渡すのは固定リテラル（`"Source"`/`"Reason"`/`"Current Stage"`/`"Agent Type"` など）で、失敗しない。 |
| `turn.rs:713, 725` | 同じ `read_definition_scope` 表を直前の `FindScopeUseCase::execute`（`turn.rs:695`）が引いて成功した後にしか呼ばれないので、ここだけ失敗する経路が無い。 |
| `cli/request.rs:334,341,348,618,672`、`runtime.rs:3718,4206,4232`、`presenter.rs:254,262` | テストヘルパの `panic!` 腕（`#[cfg(test)]` 内）。 |

## 5. 残った未カバー行と理由

対象 38 ファイルの残り 638 行（summary）の内訳。

1. **未実行の失敗経路クロージャ（約 362 行）** — `map_err(|e| e.to_string())`・`ok_or_else(|| ...)` の 1 行クロージャで、ストア I/O・投影の欠落・ロック取得失敗などを注入しないと呼ばれない。塞いだストア／ディレクトリ化／文法外 space／カーソル破損で踏めるものは本報告のテストで踏んだ（`runtime.rs` の `report`/`log_*`、`pipeline_link`、`review_guards`、`session_hooks`）。残りは `IntentExecutionRepositoryImpl::open` 成功後に個別のユースケースだけを失敗させる必要があり、テスト容易化の公開 API 追加が禁止のため踏んでいない。多いのは `runtime.rs`（85）、`plan_approval.rs`（56）、`continuation.rs`（39）、`turn.rs`（36）、`testing_posture.rs`（21）、`jump.rs`（19）。
2. **共有承認ストアの回復経路** `plan_approval.rs:52,111,191,211,244,342,348,499` — 保留中の `answer`/`response` 操作を積むには先行する発行（`publication`）と質問文書の往復が要る（`upstream_271_contract.rs` の plan 系フィクスチャ相当）。`publication` の出所喪失（:387）だけは踏んだ。
3. **Stop フックの construction/unit 観測** `continuation.rs:232-266` — 単位（unit）付きの run-stage を出す construction 段まで進めた記録が要る。`:147`（`ask` で沈黙）は §4 の `resume-menu` 廃止で `next` が `ask` を返す状態が無い。`:469` は投影の途中状態。
4. **環境変数で決まる予算** `source_baseline.rs:200,205,226,346`、`source_fingerprint.rs:42,47,83` — `AIDLC_TEST_SOURCE_MAX_*` を読むが、`std::env::set_var` はテスト内で他テストと競合するため使わず、踏んでいない。
5. **プラットフォーム条件** `session_processes.rs:23,27-33,85` — `/proc` を読む Linux 枝と `win32`。macOS で計測したため未実行（Linux CI では踏まれる）。
6. **競合・レース** `pipeline_link.rs:174-177,186-189`、`review_documents.rs:93,118,121`、`source_fingerprint.rs:93` — 読取中のファイル差替えの検知。決定的に再現できない。
7. §4 の dead code 候補（約 30 行）。

## 6. 所見（プロダクト側。裁定を要するものは実装を変えていない）

- **配布 scope ファイルの `keywords:` はブロック列（`- fix`）だが、frontmatter の最小 YAML 読取（`compiled_definition_repository_impl.rs:1150-1215`）はフロー列 `[a, b]` しか受理しない。** そのため配布ファイルのままではキーワード推論が一度も効かない（`next fix the crash` は compose 提案へ落ちる）。テストではフロー列に書き換えて推論経路を踏んだ（`a_brownfield_workspace_prices_the_inferred_scope_with_reverse_engineering`）。上流仕様との不一致として人間の裁定を求める。
- `.aidlc-execution` を一度消して戻すと、以後の `catch_up_before_reading` が `projection: legacy shared projection requires migration: orchestration` で止まる（`learnings_persist_refuses_a_record_whose_cursor_is_lost_or_broken` で観測。テストは復元後の成功を主張しない）。
- `continue-workflow` フックは子プロセスで `next` を起動するため、並行負荷下では 1 回 30〜40 秒かかる（既存 `upstream_271_contract.rs` の stop 系も同じ）。新規テストのうち stop 系 4 件がこの遅さを引き継ぐ。
- 途中で一度だけ `cargo fmt --all` を実行した。以後は `cargo fmt -p aidlc` に限定した。他 crate の 3 ファイル（`core/command/use-case/src/orchestration/command_error_material_tests.rs`、`core/command/domain/tests/{plan_authority_contract,value_object_rejection_contract}.rs`）の更新時刻が近接していたが、整形以外の変更はしていない。

## 7. ログ

`coverage-logs/u2_cov_app/`: [01-before-measure.log](u2_cov_app/01-before-measure.log)（着手前の crate 計測）、[02-after-measure.log](u2_cov_app/02-after-measure.log)（着地後の crate 計測 = crate 全テスト 889 件成功）、[03-lint.log](u2_cov_app/03-lint.log)、[04-fmt.log](u2_cov_app/04-fmt.log)、[05-clippy.log](u2_cov_app/05-clippy.log)、[06-new-contract-tests.log](u2_cov_app/06-new-contract-tests.log)（新規契約テスト 51 件）、[07-lib-unit-tests.log](u2_cov_app/07-lib-unit-tests.log)（`--lib` 395 件）。
