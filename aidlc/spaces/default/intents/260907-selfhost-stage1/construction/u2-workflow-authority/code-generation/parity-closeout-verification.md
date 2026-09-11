# U2 parity 着地の検証（`u2_parity_closeout`）

2026-09-11。前セッションの 2 担当（`u2_classic_parity` / `u2_hook_parity`）が途中まで進めた変更を 1 担当で直列に引き取り、`cargo test --workspace --no-fail-fast` の 25 件失敗（[ベースライン](step9-logs/workspace-test-2026-09-10-resume-baseline.log)）を 0 件へ着地させた記録である。生ログは [parity-closeout-logs/](parity-closeout-logs/)（先頭にコマンドと UTC 時刻、末尾に `# exit=<code>`）。

方法論は承認済み Testing Contract の TDD（red → green → refactor）。既存テストの期待を変えた箇所は、その根拠（コーパスの該当ファイル・契約の条項・裁定の Q 番号）を各行に記す。テストの削除・ignore・弱体化はしていない。

## 1. 引き継ぎ項目ごとの状態

### classic 側（Step 8 の是正、裁定 Q1 = A、4 ケース固定、R7、R9）

| 項目 | 状態 | 根拠ログ / 変更 |
| --- | --- | --- |
| D1 作業なしの `next --scope classic` が `print`（error ではない） | 完了（前担当） | [classic-parity-logs/green-1](classic-parity-logs/green-1-d1-no-active-intent.log)。本セッションでも `classic_corpus_contract` 6 件成功（[15](parity-closeout-logs/15-green-classic-corpus-consumes-conditional.log)、[16](parity-closeout-logs/16-green-q1-execute-jump-spelling.log)） |
| D2 Greenfield で reverse-engineering を SKIP | 完了（前担当）＋追従 | [classic-parity-logs/green-2](classic-parity-logs/green-2-d2-intent-create-state.log)。D2 の帰結で「最初の run-stage が reverse-engineering」を前提にしていた fixture 10 件（`upstream_271_contract` 6、`task_update_contract` 2、`learnings_contract` 1、`runtime_graph_contract` 1）は一時ワークスペースにソース 1 つを置いて Brownfield にした（本家 `aidlc-utility.ts:5895-5904`。採取元 `stop-values.json` / `runtime-sync-observations.json` は reverse-engineering が EXECUTE の記録で観測されている）。red [06](parity-closeout-logs/06-red-upstream-271-greenfield-fixture.log) / [08](parity-closeout-logs/08-red-task-update-greenfield-fixture.log) / [12](parity-closeout-logs/12-red-learnings-runtime-graph-greenfield-fixture.log) → green [07](parity-closeout-logs/07-green-upstream-271-brownfield-fixture.log) / [09](parity-closeout-logs/09-green-task-update-brownfield-fixture.log) / [13](parity-closeout-logs/13-green-learnings-runtime-graph-brownfield-fixture.log) |
| D3 `Request` 欄の `/aidlc ` 前置 | 完了（前担当） | [classic-parity-logs/green-3](classic-parity-logs/green-3-d3-intent-create-audit-request.log) |
| D4 `bundle` の `sha256:` 接頭辞 | 完了（前担当） | [classic-parity-logs/green-4](classic-parity-logs/green-4-d4-load-steering-bundle.log) |
| D6 承認待ちへの再報告時の根拠再検証 | 完了（本 build が gate-start で持つ根拠の範囲） | 本 build の gate-start 前提は pipeline 根拠（`require_pipeline_for_report`）であり、`commit_verdict_use_case.rs` `attempt` は `AlreadyAwaiting` の no-op を返す前にそれを再検証する。`pipeline_link_contract::an_open_gate_cannot_reuse_receipts_after_the_handoff_changes` が「開いたゲートへの再報告が handoff 変更後に拒否される」ことを固定している。**本家 `verifyGateOpeningGuards` の残り 3 種（成果物の存在・要約確認・レビュアー受領証の present-approval-gate 形）は本 build の gate-start に初回から無い**。再報告だけの差ではないので §3 に記録した |
| Q1 = A print directive の本家逐語化 | 完了 | `EngineCommand::cli_spelling` を配布入口形 `bun .claude/tools/<tool>.ts …` へ揃えた（`ResolveJump` → `ExecuteJump { stage, direction, scope }`、`Unpark`、`ChangeScope`、`ChangeConfig`、`ReportSkipped`）。`wording::resolve_jump` → `execute_jump`（本家 `:6640-6642` 逐語）、`resume_redo`（本家 `:7528` 逐語）。red [14](parity-closeout-logs/14-red-classic-corpus-consumes-and-jump.log) → green [16](parity-closeout-logs/16-green-q1-execute-jump-spelling.log)、[18](parity-closeout-logs/18-green-next-branches-execute-jump.log)。例外は composer ディスパッチ（§3 F-C1） |
| Q1 = A `next --stage` が方向と scope を解決して `execute` を名指す | 完了 | 方向は `read_next_jump.outcome`（集約 `jump_resolve` の答え）を運ぶ。`cli/next/stage-jump-print/stdout.json` と 1 バイトも違わない（[16](parity-closeout-logs/16-green-q1-execute-jump-spelling.log)） |
| 4 ケース固定（`classic_corpus_contract.rs`） | 完了 | 6 テスト（`no-active-intent`、`intent-create` の状態・監査、`next/start`、`continue/load-steering` 全文、`stage-jump-print`）すべて成功。`continue/load-steering` の run-stage 全文一致には `consumes[].conditional_on` の絞り込み（§2-2）が必要だった |
| R7 旧ピン・逸脱台帳を根拠にする注記の是正 | 完了 | `wording.rs`（`stage_graph_not_readable`・`report` 逐語の見出し・`state_field_not_found`）、`cli/mod.rs`、`runtime.rs` 2 箇所、`intent_lifecycle.rs`、`read_only_verb.rs`、`engine_command.rs` の module doc、`workflow_definition.rs`、`compiled_definition_repository_impl.rs` 2 箇所、RMU `wording.rs` 3 箇所、`audit_events.rs`、`projection.rs`（削除済み `upstream-3c3146cf/README.md` の引用）。挙動の変更は無い。`mod.rs` の規約「旧ピン 3c3146cf の個別引用は当時の実装根拠」に従う `ピン 3c3146cf file:line` 形の引用は据え置いた |
| R9 `jump_contract.rs` の fixture を `tests/golden/selfhost-stage1/` へ | 完了 | `tests/golden/selfhost-stage1/jump-direct-execute.json`（intent 記録の `jump-logs/questions-observations.json` の写し）を `include_str!` する。[17](parity-closeout-logs/17-green-r7-r9-jump-lifecycle-golden.log) |

### hook 側（是正 5 件、裁定 Q1 = A、reviewer-scope 残課題 3・4）

| 項目 | 状態 | 根拠ログ / 変更 |
| --- | --- | --- |
| 監査値の `<project-dir>` 置換（描画側 1 箇所） | 完了（前担当） | [hook-parity-logs/green-1](hook-parity-logs/green-1-audit-project-dir-redaction.log)、[green-1a](hook-parity-logs/green-1a-rmu-lib-audit-redaction.log)、[green-1b](hook-parity-logs/green-1b-regression-error-logged-redaction.log) |
| 状態欄読取りの `[ \t]*` | 完了（前担当） | [hook-parity-logs/green-2b-3b](hook-parity-logs/green-2b-3b-dispatch-rules-contract.log)（`the_current_stage_is_read_with_a_tab_or_no_separator_after_the_colon`） |
| `Layout::shared` の lone-intent 後退 | 完了（1 点を調整） | カーソルが実在しない記録を指すときは無視して唯一の記録へ後退する（本家 case 79 `dangling`、[hook-parity-logs/green-2-3](hook-parity-logs/green-2-3-unit-field-and-layout.log)）。前担当は「実在」を本家どおり `aidlc-state.md` の有無で判定したが、それは U2 で先に着地していた投影の復元（`intent_lifecycle::next_restores_missing_projection_files_without_repeating_audit` — 状態ファイルを失っても `next` がジャーナルから描き直す）と両立しない。本 build では状態ファイルは投影なので、**カーソルが名指すディレクトリが在れば記録として選び、復元に委ねる**形へ調整した（`layout.rs` `shared`、テスト `a_cursor_naming_a_directory_without_a_state_file_is_still_the_record`）。唯一記録の数え方（状態ファイルの無いディレクトリは数えない）は本家 `listIntentDirs` のまま。red [03](parity-closeout-logs/03-red-intent-lifecycle-restoration.log) → green [04](parity-closeout-logs/04-green-intent-lifecycle-restoration.log)。§3 F-H1 に裁定材料として記す |
| Stop フックの台帳 flush-all | 完了（前担当） | [hook-parity-logs/green-4](hook-parity-logs/green-4-stop-flush-all.log)、[green-4a](hook-parity-logs/green-4a-fold-usage-unit.log) |
| 差し向け記録の `stage` / `unit` の逐語強制（裁定 Q1 = A） | 完了（前担当） | [hook-parity-logs/green-5a](hook-parity-logs/green-5a-domain-reviewed-types.log)、[green-5b](hook-parity-logs/green-5b-review-guards-contract.log)。clippy `-D warnings` に掛かった `reviewer_scope_block.rs` のテストの添字を `get(2..4)` へ直した（[20](parity-closeout-logs/20-check-fmt-clippy-lint-2.log) → [22](parity-closeout-logs/22-check-fmt-clippy-lint-4.log)） |
| reviewer-scope 残課題 3（use case 単体テスト 3 件） | 完了（前担当） | [hook-parity-logs/green-6a](hook-parity-logs/green-6a-use-case-unit-tests.log)（4 件）。テストモジュールに `#![allow(clippy::panic)]` を足した（[21](parity-closeout-logs/21-check-fmt-clippy-lint-3.log)） |
| reviewer-scope 残課題 4（`Glob` / 不正 JSON stdin の子プロセス面） | 完了（前担当） | [hook-parity-logs/green-6b](hook-parity-logs/green-6b-glob-and-malformed-stdin.log) |

## 2. 本セッションで実装・是正したもの

### 2-1. lone-intent 後退に追従したテスト fixture

- `directive_drawing.rs` `layout_with_record`: カーソルの先に記録ディレクトリを作る（red [01](parity-closeout-logs/01-red-lib-lone-intent-tests.log) → green [02](parity-closeout-logs/02-green-lib-lone-intent-tests.log)）。
- `turn.rs` テストの `forget_cursor` → `forget_record`: カーソルを消すだけでは唯一の記録へ後退するので、記録の状態ファイルも消して「記録 0」にする。
- `turn.rs` `a_cost_clause_needs_all_four_columns`: 前担当が足した `greenfield_cost_*` 4 列（一時ワークスペースは Greenfield と判定される）も同時に欠かせる。
- `next_branches.rs` `an_invalid_active_space_names_the_cursor_file_to_fix`: テストの意図「record は解決できるが空間名が通らない」どおり、カーソルの先のディレクトリを作る（[05](parity-closeout-logs/05-green-next-branches-invalid-space.log)）。
- `claude_hook_contract.rs` `write_audit_hook_projects_created_updated_and_drop_via_events`: 前担当が足した `aidlc-state.md` を外した（状態ファイルが在ると監査の書込先がクローン別シャードになり、fixture の `audit/shard.md` を読む検査と食い違う。調整後の `Layout` では状態ファイル無しでも記録として解決する）。red [10](parity-closeout-logs/10-red-claude-hook-artifact-created.log) → green [11](parity-closeout-logs/11-green-claude-hook-artifact-created.log)。

### 2-2. `consumes[].conditional_on` の絞り込み（`continue/load-steering` 全文一致）

本家 `resolveConsumes`（`aidlc-orchestrate.ts:2519-2534`）は状態ファイルの `Project Type` と食い違う `conditional_on` の宣言を落とし、種別が読めなければ全宣言を残す。本 build は種別を問わず全宣言を載せていたので、Greenfield の classic で `practices-discovery` の `consumes` に codekb 6 パスが出ていた。

- RMU `read_run_stage` に `consumes_brownfield_rel` / `consumes_greenfield_rel` を足し（`READ_SCHEMA_VERSION` 5 → 6）、`RunStageView` と DAO の列写像を 26 列にした。
- `directive_drawing::run_stage` は `project_kind` で列を選ぶ。`Turn::draw_run_stage` は state binding → 実行 → `read_intent.project_type`（鋳造時の走査結果の投影で、状態ファイルの `Project Type` と同じ値）を小文字にして種別を読む（本家 `projectTypeFrom` と同じ）。
- red [14](parity-closeout-logs/14-red-classic-corpus-consumes-and-jump.log) → green [15](parity-closeout-logs/15-green-classic-corpus-consumes-conditional.log)。

### 2-3. 品質検査

[19](parity-closeout-logs/19-check-fmt-clippy-lint-1.log) → [22](parity-closeout-logs/22-check-fmt-clippy-lint-4.log) で `cargo fmt --all --check`、`cargo clippy --workspace --all-targets -- -D warnings`、`cargo lint` がすべて exit 0。`bun scripts/goldens/verify-corpus.ts tests/golden/upstream-a277af21` と `bun scripts/aidlc-sync.ts --check` は [24](parity-closeout-logs/24-verify-corpus-and-sync-check.log) で成功。

## 3. 契約や裁定で決まらず記録に留めた差（切替条件 2 の判定材料）

| # | 差 | 本家 2.7.1 | 本 build | 扱い |
| --- | --- | --- | --- | --- |
| F-H1 | カーソルが名指す**実在ディレクトリ**に `aidlc-state.md` が無いとき | `activeIntent` は無視して唯一記録へ後退（`existsSync(join(dir, raw, "aidlc-state.md"))`） | 記録として選び、`next` がジャーナルから状態ファイルを復元する | 本 build の投影復元（承認済み Step 3）と本家の判定が両立しない 1 点。復元を残す側へ倒した。人間の裁定で本家側に倒す場合は `layout.rs` `shared` の `is_dir()` を `is_record()` へ戻し、`intent_lifecycle` の復元テスト 2 件を見直す |
| F-C1 | `next --compose` の print directive | `composeDispatchDirective`（`aidlc-orchestrate.ts:1684-1740`）— 差し向け先エージェント・`creationDescription`・`--report` / `--new-scope`・提案の形・表示契約を含む多段の散文 | `Dispatch the composer: run \`aidlc-composer detect\`.` | 綴りだけでなく文面の構造が違う。2.7.1 コーパスに compose の採取は無く、スモーク経路の外（`memory/project.md` Forbidden「スモークで踏まない既存課題を追加しない」）なので、Q1 = A の逐語化から除外して記録する |
| F-C2 | gate-start（`report --result awaiting-approval`）の根拠検証 | `verifyGateOpeningGuards`: 成果物の存在・要約確認・pipeline リンク・レビュアー受領証（`Cannot present "<slug>" for approval: …`） | pipeline リンクのみ（初回も再報告も同じ） | D6 は「再報告時に再検証するか」の差で、再検証の対象は本 build の実装範囲で揃っている。残り 3 種は初回の gate-start にも無い別件 |
| F-X1 | `pipeline_link_contract::concurrent_duplicate_completions_persist_only_one_receipt` | — | 高負荷時に勝者側の投影読取りが `io: WouldBlock`（SQLite busy）で落ちることがある | 既存の負荷依存の揺れ（本セッションの変更と無関係、単体・スイート再実行で成功）。読取り側の busy 待ちを入れるかは別件 |
| F-C3 | Brownfield の `consumes` の解決先 | `resolveConsumePath` は生産者の置き場（reverse-engineering の産物は `aidlc/spaces/<space>/codekb/<repo>/…`）へ解決する | `<record>/<artifact>`（語彙名を record に前置） | 今回の 4 ケースは Greenfield で宣言が落ちるため観測されない。Brownfield の run-stage 全文一致は未検証 |

## 4. 変更ファイル一覧（本セッション）

- `modules/app/aidlc/src/layout.rs`（`shared` の実在判定、テスト 1 件の置換）
- `modules/app/aidlc/src/directive_drawing.rs`（`run_stage` に `project_kind`、テスト fixture）
- `modules/app/aidlc/src/turn.rs`（`recorded_project_kind`、`ExecuteJump` 配線、テスト fixture）
- `modules/app/aidlc/src/wording.rs`（`execute_jump`、`resume_redo`、R7 注記）
- `modules/app/aidlc/src/runtime.rs`、`modules/app/aidlc/src/cli/mod.rs`（R7 注記）
- `modules/app/aidlc/tests/{classic_corpus_contract,claude_hook_contract,intent_lifecycle,jump_contract,learnings_contract,next_branches,runtime_graph_contract,task_update_contract,upstream_271_contract}.rs`
- `modules/core/query/use-case/src/orchestration/engine_command.rs`、`read_only_verb.rs`、`port/read_view/run_stage_view.rs`、`modules/core/query/use-case/tests/read_model_use_case_test.rs`
- `modules/core/query/interface-adapter/src/run_stage_columns.rs`
- `modules/core/read-model-updater/src/read_tables/{run_stage_row,sql}.rs`、`src/workspace/{projection,wording}.rs`、`tests/journal_reader_impl_test.rs`（読取り版のリテラル 5 → 6）
- `modules/core/command/domain/src/orchestration/reviewer_scope_block.rs`、`src/workflow_definition/workflow_definition.rs`、`src/workspace/audit_events.rs`
- `modules/core/command/use-case/src/orchestration/guard_reviewer_scope_use_case.rs`
- `modules/core/command/interface-adapter/src/orchestration/compiled_definition_repository_impl.rs`
- `tests/golden/selfhost-stage1/jump-direct-execute.json`（新規）

承認済みの `code-generation-plan.md` / `unit-test-instructions.md` / `code-generation-questions.md`、`.claude/` 配下の配布資産は触っていない。git のコミット・push・stash は行っていない。

## 5. 最終検査

| 検査 | 結果 | ログ |
| --- | --- | --- |
| `cargo test --workspace --no-fail-fast`（1 回目） | 3,121 件成功・2 件失敗 | [23](parity-closeout-logs/23-workspace-test-round-1.log)。失敗は (a) `journal_reader_impl_test::a_store_left_on_the_old_read_schema_is_rebuilt_from_the_journal` — 読取り版のリテラル `5` が §2-2 の版上げ (6) に追従していなかった → テストのリテラルを 6 へ（[25](parity-closeout-logs/25-green-schema-v6-and-pipeline-link-rerun.log) で成功）、(b) `pipeline_link_contract::concurrent_duplicate_completions_persist_only_one_receipt` — 2 プロセス同時の `link` で勝者側の投影読取りが SQLite の `WouldBlock` を受けた。load average 20〜31（他セッションの並走）下の再現で、単体 3 回・スイート 2 回の再実行はすべて成功（[26](parity-closeout-logs/26-pipeline-link-suite-rerun.log)）。本セッションの変更はこの経路（`aidlc-log link` の投影）に触れておらず、ベースラインでも成功していた。負荷依存の既存の揺れとして §3 F-X1 に記す |
| `cargo test --workspace --no-fail-fast`（最終） | **101 スイート・3,123 件成功・0 件失敗（exit 0）** | [workspace-test-final.log](parity-closeout-logs/workspace-test-final.log) |
| `cargo fmt --all --check` / `cargo clippy --workspace --all-targets -- -D warnings` / `cargo lint` | すべて exit 0 | [22](parity-closeout-logs/22-check-fmt-clippy-lint-4.log) |
| `bun scripts/goldens/verify-corpus.ts tests/golden/upstream-a277af21` | 成功 | [24](parity-closeout-logs/24-verify-corpus-and-sync-check.log) |
| `bun scripts/aidlc-sync.ts --check` | 同期済み（コピー 0、削除 0） | 同上 |

カバレッジ（`bash scripts/coverage.sh --base main`）、Quint ゲート、release バイナリ契約、`cargo audit` は Step 9 で親が 1 回行う前提のため、ここでは実行していない。
