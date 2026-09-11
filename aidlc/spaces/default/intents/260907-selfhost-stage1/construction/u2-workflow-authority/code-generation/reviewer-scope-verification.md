# 通常の reviewer-scope（`aidlc hook reviewer-scope`）の着地確認記録

担当 `u2_reviewer_scope_closeout`。対象は承認済み[実装計画](code-generation-plan.md) Step 7 の
「通常の reviewer-scope」— [remaining-step7-inventory.md](../remaining-step7-inventory.md) の
「reviewer-scope」行と、[review-guards-verification.md](review-guards-verification.md) が残課題 2 に
挙げた「越境拒否・`REVIEWER_SCOPE_BLOCKED` の保存・差し向け記録の TTL 掃除」である。
前担当 `u2_reviewer_scope` が実装と Green までを進めて記録の手前で停止したため、本書はその実装を
本家 2.7.1（固定コミット `a277af218f0df7f325d3b8be7b6d90fce2c5bd40`）と突き合わせ、証跡を整え、
着地を記録する。実装の書き換えは、突き合わせで本家との差を実測した 1 件だけを TDD（失敗する
テストを先に走らせてから最小実装する進め方）で行った。生の実行出力は
[reviewer-scope-logs/](reviewer-scope-logs/) にある（既存ログは上書きしていない）。

用語: 「reviewer-scope」は、per-unit（作業単位ごと）のレビュアーが兄弟 Unit の
`construction/<other-unit>/` を読もうとする呼出しを、ツール実行前（PreToolUse）に拒否する保護である。
「差し向け記録（dispatch record）」は指揮者がレビュアーを呼ぶ直前に置く
`<record>/.aidlc-reviewer-dispatch.json`（誰を・どのステージ・どの Unit・何を許すか）。
「TTL（鮮度の窓）」は差し向け記録を信じる上限時間。「fail-open」は材料が無い・読めないときに
人間の作業を止めずに通す設計。「稼働記録（heartbeat）」はフックが動いた事実を残す
`.aidlc-hooks-health/<hook>.last`、「drop」は通したが記録に値する事情を残す `<hook>.drops`。
「parity（同一性検査）」は本家の採取と本 build の出力を同じ入力で比べる検査。

## 依頼文と現物の差（最初に記す）

突き合わせの前提を誤らないため、依頼文の記述と実測が違った点を先に書く。読み替えはしていない。

| 依頼文 | 実測 |
| --- | --- |
| 「イベント列挙 `intent_execution_event.rs`（29 変種。`REVIEWER_SCOPE_BLOCKED` を含む）」「`intent_execution_event/reviewer_scope_blocked.rs`」 | `IntentExecutionEvent` は 30 変種で `ReviewerScopeBlocked` 変種は**無く**、`intent_execution_event/reviewer_scope_blocked.rs` も**存在しない**。`REVIEWER_SCOPE_BLOCKED` の保存は `workspace::SessionAudit` 集約（`EventType::ReviewerScopeBlocked`、`session_audit_record.rs:41-44`、`audit_events.rs:119`）であり、前段の `REVIEW_FREEZE_BLOCKED` と同じ経路である。したがって本書での「イベント列挙の所有」は `session_audit_record.rs` の reviewer-scope 分岐を指す。 |
| 本家採取「`tests/golden/selfhost-stage1/`（`review-receipts.json` 等）」 | `review-receipts.json` はレビュー受領証（`request` / `retry-first` / `retry-second`）の採取で reviewer-scope ではない。reviewer-scope の採取は `tests/golden/upstream-a277af21/reviewer-scope/cases.json`（165 件、来歴 `source.commit = a277af21…`、`files: 277`、`manifest: 282b17c5…`）である。 |
| `runtime.rs` の `run_hook` 配線「1047 行、1078 行付近」 | 実測 `runtime.rs:1046-1047`（名前の一覧）と `:1075-1079`（分岐）。**編集していない**。 |

## 実装の要約（責任分割）

上流の 1 ファイル（`aidlc-reviewer-scope.ts` 932 行）を、既存の層と規則
（[coding-rules/](../../../../knowledge/aidlc-shared/coding-rules/)）に沿って 5 層へ分けてある。

| 層 | 責任 | ファイル（行数） |
| --- | --- | --- |
| 言語拡張 `harness-infrastructure` | reviewer-scope の字句規則でシェルを語と実行位置へ切る（本家 `shellTokens:384` / `shellSegments:430`）。**シェルを実行しない**。review-freeze 側の `RedirectionFreeWords` とは字句規則が違う別移植で、両 doc に差分表を明記 | `reviewer_scope_segments.rs`（282）、`reviewer_scope_words.rs`（67） |
| 封筒 `harness-claude` | PreToolUse の JSON から工具・呼び手・`cwd`・`scoped_registration`・経路候補を読む（本家 `candidateStrings:116`、`REVIEW_AGENT_RE:748`）。差し向け記録の読取り（本家 `parseDispatchRecord:673`） | `reviewer_scope_envelope.rs`（307）、`reviewer_dispatch_record.rs`（107） |
| 判定 `core-command-domain` | 差し向けと基点から読取り範囲を組み、候補ごとに越境を判定する純粋な規則（本家 `prepareScope:278` / `judgeOccurrence:227` / `judgeCommandText:591` / `evaluateReviewerScope:629`）。拒否 1 件の材料と監査項目 | `reviewer_scope.rs`（469）、`reviewer_scope_paths.rs`（289、公開型なし）、`reviewer_scope_block.rs`（102）、`reviewer_scope_verdict.rs`（20）、`reviewer_scope_candidate.rs`（19）、`reviewer_scope_candidates.rs`（60）、`reviewer_dispatch.rs`（102）、`exempt_paths.rs`（56）、`inspected_tool.rs`（110）、`inspected_command.rs`（59）、`inspected_command_step.rs`（57）、`scope_token.rs`（48）、`unit_name.rs`（50、review-freeze と共用） |
| 監査語彙 `core-command-domain::workspace` | `REVIEWER_SCOPE_BLOCKED` の必須 4 項目（`Tool` / `Target` / `Stage` / `Unit`。凍結側と違い `Unit` を任意にしない） | `session_audit_record.rs:36-44`、`audit_events.rs:119`、契約 `tests/session_audit_contract.rs` |
| 更新ユースケース `core-command-use-case` | 判定し、拒否のときだけ `SessionAudit` 集約に 1 件保存する。集約は読まない（進行状態は材料に入らない）。記録に失敗しても拒否の材料を運んで戻す（本家「an audit failure never changes the block decision」） | `guard_reviewer_scope_use_case.rs`（117）、`reviewer_scope_request.rs`（64）、`reviewer_scope_error.rs`（42）、`reviewer_scope_cause.rs`（20） |
| 実行入口 `aidlc` | 停止スイッチ → 稼働記録 → 封筒 → 記録不在の助言 → TTL 掃除 → 読取り → 身元 → 判定 → 保存と投影 → 逐語の拒否理由で exit 2（本家 `run:756-932` と同じ順序）。拒否文言は `wording::reviewer_scope_blocked`（本家 `blockReason:692`） | `runtime/review_guards.rs`（336、reviewer-scope 部分は `:137-253`）、`wording.rs:1311-1319` |
| 契約テスト | 実行バイナリを子プロセスで呼ぶ 12 件（本工程で 1 件追加）と、本家採取 165 件との parity 1 件 | `tests/review_guards_contract.rs`（890、reviewer-scope 部分は `:353-697`）、`harness/claude/tests/reviewer_scope_parity.rs`（105） |

`REVIEWER_SCOPE_BLOCKED` の保存経路は **コマンド（`GuardReviewerScopeUseCase`）→ `SessionAudit` の
イベント → SQLite（`SessionAuditRepositoryImpl`）→ RMU の `catch_up` が監査シャードへ描く**
（`read_model_updater.rs:462` 付近が `history.sessions()` を `render_audit_block` で描く）である。
実行入口は保存後に `catch_up` を呼び、描けなくても拒否は変えない。

## 本工程で行った変更（TDD、1 件）

本家フックの実走行（下記「本家との突き合わせ (2)」）で **封筒に `cwd` が無いときの基点**が
本家と違うことを実測した。本家は `cwd: … ? cwdField : projectDir`（`run:912-915`。claimed-checkout 側も `:799-806` で同じ）で
project dir へ倒すが、本 build は `None` のままだったため、相対の探索根 `aidlc`（project dir から
見て `construction/` の上）を掃く根だと見抜けず通していた。

| 段 | コマンド | 結果 | ログ |
| --- | --- | --- | --- |
| Red | `cargo test -p aidlc --test review_guards_contract a_call_without_a_working_directory` | 1 failed（exit 0 ≠ 2 の不一致。テストは実装より先に置いた） | [red-6-cwd-fallback.log](reviewer-scope-logs/red-6-cwd-fallback.log) |
| Green | 同上 | 1 passed | [green-14-cwd-fallback.log](reviewer-scope-logs/green-14-cwd-fallback.log) |
| Green（全件） | `cargo test -p aidlc --test review_guards_contract` | 24 passed / 0 failed | [green-15-closeout-review-guards-contract-after-fix.log](reviewer-scope-logs/green-15-closeout-review-guards-contract-after-fix.log) |

変更は `review_guards.rs:193-199`（`envelope.cwd()` が無ければ `layout.project_dir()`）と、
契約テスト `a_call_without_a_working_directory_resolves_relative_roots_against_the_project_dir`
（`review_guards_contract.rs:675-697`）の 2 箇所である。Refactor は不要だった（式 1 つの追加）。

## Red の再現（前担当のログの評価）

前担当のログを内容から読み、red-first の順序が守られていたかを判断した。判断できない点は
そのまま書く。時刻はログファイルの更新時刻である。

| # | ログ | 時刻 | 失敗の内容 | 順序の判断 |
| --- | --- | --- | --- | --- |
| 1 | [red-1-lexer.log](reviewer-scope-logs/red-1-lexer.log) | 12:26 | `reviewer_scope_segments` の 10 件が「空の結果」で失敗（1 件は空入力の検査で自明に成功）。コンパイラが `parse(command)` の引数未使用を警告しており、本体が空返しのスタブだったことを示す | テストが実装本体より先に存在したことはログから読める。「テストを書いてから最初に走らせた」のか「実装後に空返しへ戻した」のかは**ログからは判別できない** |
| 2 | [red-2-parity-stub.log](reviewer-scope-logs/red-2-parity-stub.log) | 12:43 | 本家採取 165 件のうち 7 件（`case-154`〜`159`、`162`: `rg -g ''`、`cd` を伴う `Bash`、`Grep` の `glob` 単独、ディレクトリ `Read`）が本家と違う | スタブではなく**部分実装**に対する red。parity テストが判定の完成より先に走っていたことは読めるが、最初の red かは判別できない（ログは末尾のみで先頭が無い） |
| 3 | [red-3-audit-vocabulary.log](reviewer-scope-logs/red-3-audit-vocabulary.log) | 12:53 | `SessionAuditRecord::new(EventType::ReviewerScopeBlocked, …)` が `InvalidRecord` で拒否（語彙を足す前） | テスト先行と読める（語彙が無ければ構築できない、という意図どおりの失敗） |
| 4 | [red-4-wording.log](reviewer-scope-logs/red-4-wording.log) | 13:07 | 拒否文言が `""`（空返しのスタブ）で本家採取の `block_reasons` と不一致 | red-1 と同じくスタブに対する失敗。順序は判別できない |
| 5 | [red-5-enforcement-stub.log](reviewer-scope-logs/red-5-enforcement-stub.log) | 13:24 | 子プロセス 3 件が exit 0（拒否されない）で失敗。コンパイラが `return Completion::silent(); // RED-STUB` の**後ろに本体がある**（unreachable statement）と警告 | **実装後にスタブで戻して取り直した red** である。green-5（13:17、23 件成功）が red-5 より先にあることとも整合する。前担当はこれを記録に書けずに停止したので、ここに明記する |
| 6 | [red-6-cwd-fallback.log](reviewer-scope-logs/red-6-cwd-fallback.log) | 本工程 | 上記「本工程で行った変更」 | テストを先に置き、実装を触る前に走らせた |

いずれも「テストが見つからない」「依存不足」「構文不正」ではなく、意図した振る舞いの不一致で
失敗している。red-1 と red-4 に対応する Green は前担当のログに無かったため、本工程で同じ
コマンドを再実行して補った（下表 green-1 / green-4。**着地時点の再実行**であり、当時の直後の
Green ではない）。

## Green 一覧

### 前担当のログ（そのまま保存）

| ログ | 時刻 | 内容 |
| --- | --- | --- |
| [green-2-parity-165.log](reviewer-scope-logs/green-2-parity-165.log) | 12:49 | parity 1 passed（当時の harness-claude には他担当の未使用警告 8 件が出ている。本担当のファイルではない） |
| [green-3-audit-vocabulary.log](reviewer-scope-logs/green-3-audit-vocabulary.log) | 12:57 | `session_audit_contract` 12 passed |
| [green-5-review-guards-contract.log](reviewer-scope-logs/green-5-review-guards-contract.log) | 13:17 | 契約テスト 23 passed（294.66 秒。同時ビルドの影響） |
| [lint-1-one-public-type.log](reviewer-scope-logs/lint-1-one-public-type.log) | 13:53 | `cargo lint` が `reviewer_scope_error.rs` の 2 公開型（`ReviewerScopeError` + `ReviewerScopeCause`）を所見。その後 `reviewer_scope_cause.rs` へ分割された（現物で確認） |
| [green-6-cargo-lint.log](reviewer-scope-logs/green-6-cargo-lint.log) | 13:53 | 出力なし（所見 0 件。終了状態はログに無い — 本工程の check-3 で終了 0 を確認） |
| [green-6-clippy-app.log](reviewer-scope-logs/green-6-clippy-app.log) / [green-6-clippy-core.log](reviewer-scope-logs/green-6-clippy-core.log) | 13:53 | `Checking … Finished`（コマンド行と除外指定はログに無い） |

### 本工程の再実行（着地時点）

| 検査 | コマンド | 結果 | ログ |
| --- | --- | --- | --- |
| 字句解析 | `cargo test -p harness-infrastructure --lib -- reviewer_scope` | 11 passed | [green-1-lexer.log](reviewer-scope-logs/green-1-lexer.log) |
| 逐語文言 | `cargo test -p aidlc --lib -- wording::tests::the_reviewer_scope` | 1 passed（本家採取 `block_reasons` 3 文と一致） | [green-4-wording.log](reviewer-scope-logs/green-4-wording.log) |
| parity 165 | `cargo test -p harness-claude --test reviewer_scope_parity` | 1 passed（165 件すべて一致） | [green-7-closeout-parity-165.log](reviewer-scope-logs/green-7-closeout-parity-165.log) |
| 判定の部品 | `cargo test -p core-command-domain --lib -- reviewer_scope exempt_paths inspected_command inspected_tool` | 13 passed（`reviewer_scope_paths` 9、`inspected_tool` 3、`reviewer_scope_block` 1） | [green-8-closeout-domain.log](reviewer-scope-logs/green-8-closeout-domain.log) |
| 差し向け・綴り・Unit 名 | `cargo test -p core-command-domain --lib -- reviewer_dispatch scope_token unit_name` | 9 passed | [green-13-closeout-domain-dispatch.log](reviewer-scope-logs/green-13-closeout-domain-dispatch.log) |
| 監査語彙 | `cargo test -p core-command-domain --test session_audit_contract` | 12 passed | [green-9-closeout-audit-vocabulary.log](reviewer-scope-logs/green-9-closeout-audit-vocabulary.log) |
| 更新ユースケース | `cargo test -p core-command-use-case --lib -- reviewer_scope` | **0 tests**（`GuardReviewerScopeUseCase` に単体テストが無い。下記「残課題」） | [green-10-closeout-use-case.log](reviewer-scope-logs/green-10-closeout-use-case.log) |
| 封筒・記録読取り | `cargo test -p harness-claude --lib -- reviewer_scope reviewer_dispatch` | 12 passed | [green-11-closeout-harness-claude.log](reviewer-scope-logs/green-11-closeout-harness-claude.log) |
| 契約テスト（修正前） | `cargo test -p aidlc --test review_guards_contract` | 23 passed | [green-12-closeout-review-guards-contract.log](reviewer-scope-logs/green-12-closeout-review-guards-contract.log) |
| 契約テスト（修正後） | 同上 | 24 passed | [green-15-closeout-review-guards-contract-after-fix.log](reviewer-scope-logs/green-15-closeout-review-guards-contract-after-fix.log) |

`reviewer_scope.rs`（判定本体）と `guard_reviewer_scope_use_case.rs` には `#[cfg(test)]` が無い。
判定本体は parity 165 件と子プロセス 12 件が固定しており、ユースケースは子プロセス面だけが
固定している。

## 本家との突き合わせ

### (1) 純関数の parity — 165 ケース、差分 0

`differential-upstream-165.tsv` と `green-2-parity-165.log` を読み、何を比較しているかを確かめた。

- **採取物**: `tests/golden/upstream-a277af21/reviewer-scope/cases.json`。
  `scripts/goldens/capture-reviewer-scope.ts` が固定コミットの `dist/claude` を**実際に読み込み**、
  `evaluateReviewerScope(tool, input, DISPATCH, CONTEXT)` を 165 入力
  （`scripts/goldens/reviewer-scope-corpus.json`）で呼んだ結果である。差し向けは
  `unit: u1-alpha`、許可経路 3 件（`construction/` を通らない 1 件、兄弟の 1 ファイルを絶対で 1 件、
  同じことを相対で 1 件）、基点は `recordRoot: /r`、`cwd: /w` に固定してある。
- **比較しているもの**: 入力（工具・`tool_input`）に対する **拒否したか（`block`）と、拒否した綴り
  （`target`）** の 2 つだけ。内訳は Bash 97、Read 32、Glob 11、LS 9、Grep 9、NotebookRead / Write /
  Edit / MultiEdit / NotebookEdit / Task / WebFetch 各 1。拒否 114、許可 51。
- **本 build 側**: `reviewer_scope_parity.rs` が同じ差し向けと基点で `ReviewerScopeEnvelope::parse`
  → `ReviewerScope::of(...).judge(...)` を通し、165 件を**全件**回して `block` と `target` を
  バイトで比べる（`observations.len() >= 165` を主張し、1 件でも違えば全差分を列挙して失敗する）。
  着地時点の再実行は [green-7](reviewer-scope-logs/green-7-closeout-parity-165.log) で成功 —
  **165 件、差分 0**。
- `differential-upstream-165.tsv`（165 行: 工具・入力・`ALLOW`/`BLOCK`・綴り）は `cases.json` を
  平坦化した表で、本工程で `cases.json` と照合して 0 差を確認した。parity が緑である以上、本 build
  の出力も同じ表になる。
- **この parity が比べていないもの**: 拒否文言（別途 `block_reasons` 3 文を `wording` のテストが
  固定 — green-4）、標準エラー・終了コード・監査行・稼働記録・drop（フック全体の観測面）。
  これらは下記 (2) で本家フックを実走行させて比べた。

### (2) フック全体の実走行 — 32 ケース、修正前 12 件差 → 修正後 11 件差

parity は純関数の比較なので、**本家フックそのもの**を固定コミットの実バイトで走らせて、
標準入出力・終了コード・監査行・稼働記録・drop・目印・差し向け記録の残存を同じ入力で比べた。
治具と全採取物は [reviewer-scope-logs/hook-differential/](reviewer-scope-logs/hook-differential/)
（手順は同ディレクトリの `README.md`）。

- 本家: `git -C vendor/aidlc-workflows archive a277af21… dist/claude` を一時ディレクトリへ展開
  （`hooks/aidlc-reviewer-scope.ts` の sha256 は `git show` の実バイトと一致
  `b4ef0550…548af`）。`AIDLC_PROJECT_DIR=<一時 project>` で本家自身の記録解決
  （`activeIntent` は記録ディレクトリに `aidlc-state.md` を要求する）を通し、
  `bun <dist>/.claude/hooks/aidlc-reviewer-scope.ts < 入力` で実行。`aidlc-orchestrate.ts` は
  走らせていない。
- 本 build: 契約テストと同じ手順で一時ワークスペースを作り（`intent-create`）、
  `target/debug/aidlc hook reviewer-scope < 入力`。
- 正規化は project dir・記録名・ISO 時刻の 3 つだけ（`<P>` / `<REC>` / `<TS>`）。

| ケース | 入力の要点 | 本家 | 本 build（修正後） | 判定 |
| --- | --- | --- | --- | --- |
| c01 | `Read` 兄弟ファイル（絶対） | exit 2、`REVIEWER_SCOPE_BLOCKED` | 同じ。監査 `Target` の綴りだけ差（A） | DIFF(A) |
| c02 | `Read` 現 Unit | exit 0、監査なし、`.last` あり | 同じ | SAME |
| c03 | `Read` 許可ファイル | exit 0 | 同じ | SAME |
| c04 | `LS` 許可ファイルの親ディレクトリ | exit 2（完全一致でないので越境） | 同じ。監査 `Target` の綴り差 | DIFF(A) |
| c05 | `Grep` 経路なし | exit 2、`Target: .` | 同じ（バイト一致） | SAME |
| c06 | `Glob` `construction/*/design.md` | exit 2 | 同じ | SAME |
| c07 | `Glob` `construction/u1-x/**/*.md` | exit 0（現 Unit へ絞っている） | 同じ | SAME |
| c08 | `Bash` `cat <兄弟>` | exit 2、`Tool: Bash` | 同じ。監査 `Target` の綴り差 | DIFF(A) |
| c09 | `Bash` `rg foo .` | exit 2、`Target: .` | 同じ | SAME |
| c10 | `Write` 兄弟 | exit 2、`Tool: Write` | 同じ。監査 `Target` の綴り差 | DIFF(A) |
| c11 | 別のサブエージェント | exit 0 | 同じ | SAME |
| c12 | `agent_type` なし | exit 0 | 同じ | SAME |
| c13 | `agent_type` なし + `scoped_registration: true` | exit 2 | 同じ。監査 `Target` の綴り差 | DIFF(A) |
| c14 | `Task`（見ない工具） | exit 0 | 同じ | SAME |
| c15 | 記録なし、レビュアーが `construction/` へ | exit 0、目印と drop 1 行 | 同じ（drop の文面バイト一致） | SAME |
| c16 | 記録なし、開発者エージェント | exit 0、目印なし | 同じ | SAME |
| c17 | 記録に `exempt` が無い | exit 0、drop「malformed」 | 同じ | SAME |
| c18 | 記録が 7 時間前 | exit 0、drop「orphaned」、記録が消える | 同じ | SAME |
| c19 | `AIDLC_DISABLE_REVIEWER_SCOPE_HOOK=1` | exit 0、`.last` なし | 同じ | SAME |
| c20 | 標準入力 `not json` | exit 0、`.last` あり | 同じ | SAME |
| c21 | 標準入力 `[]` | exit 0 | 同じ | SAME |
| c22 | **`cwd` なし**、`Grep path: aidlc` | exit 2、`Target: aidlc` | 修正前 exit 0（B）→ **修正後 exit 2 で一致** | SAME |
| c23 | `cwd` あり、同上 | exit 2 | 同じ | SAME |
| c24 | 記録 `stage: "Functional Design"` | exit 2、`Stage` 逐語 | exit 0、drop「malformed」（C） | DIFF(C) |
| c25 | 記録 `unit: "a/b"` | exit 2、`Unit: a/b` | exit 0、drop「malformed」（C） | DIFF(C) |
| c26 | `Read` 相対 `construction/u2-y/…` | exit 2 | 同じ（バイト一致） | SAME |
| c27 | `NotebookRead` 兄弟 | exit 2 | 同じ。監査 `Target` の綴り差 | DIFF(A) |
| c28 | `Read` 相対で許した兄弟ファイル | exit 0 | 同じ | SAME |
| c29 | `LS` その親 | exit 2 | 同じ。監査 `Target` の綴り差 | DIFF(A) |
| c30 | `Bash` `cd <REC>/construction && cat u2-y/…` | exit 2、`Target: <REC>/construction` | 同じ。監査 `Target` の綴り差 | DIFF(A) |
| c31 | 標準入力が空 | exit 0 | 同じ | SAME |
| c32 | `cwd` なし、`Read` 兄弟（絶対） | exit 2 | 同じ。監査 `Target` の綴り差 | DIFF(A) |

修正前の表は [differential-hook-32-before-cwd-fix.tsv](reviewer-scope-logs/hook-differential/differential-hook-32-before-cwd-fix.tsv)、
修正後は [differential-hook-32-after-cwd-fix.tsv](reviewer-scope-logs/hook-differential/differential-hook-32-after-cwd-fix.tsv)。
残る差は 2 種類で、**どちらも本工程では直していない**（理由は各項）。

**A. 監査行 `Target` の project dir 置換（9 件）**

本家 `aidlc-audit.ts:512-534` `renderAuditBlock` は、監査行の**すべての値**に
`redactProjectDirPrefix(value, projectDir)`（`aidlc-lib.ts:22167-22203`）を掛け、project dir
（実パス・`\` 変換を含む）で始まる部分を逐語 `<project-dir>` に置き換える。本 build の RMU
`workspace/audit_block.rs:47-72` `render_audit_block` は値をそのまま描く。よって絶対経路を名指した
拒否では、本家が `**Target**: <project-dir>/aidlc/spaces/default/intents/<REC>/construction/u2-y/contract.md`
と書くのに対し本 build は `**Target**: /…/workspace/aidlc/spaces/…/contract.md` と書く
（見出し・`Timestamp`・`Event`・項目の順序・`Stage`・`Unit`・標準エラーの文面・exit は一致）。
相対の綴り（c05 / c06 / c22 / c26）は置換の対象にならないので一致する。

直していない理由: 置換は監査シャードの**描画規則**であり、reviewer-scope 固有ではない。同じ
`render_audit_block` を通る `REVIEW_FREEZE_BLOCKED` の `Target`（review-freeze、絶対経路）も同じ差を
持つはずで（本工程では未実測）、本家では `ERROR_LOGGED` 等の全イベントに掛かる。本 build には
`runtime/log_failure.rs:58` に同じ規則の `redact(value, layout)`（テスト付き）が private で既にあり、
是正は (a) それを共有可能な場所へ昇格して両フックの入口で `Target` を置換してから保存する、
(b) RMU の描画に project dir を渡して本家と同じ場所で置換する、のどちらかになる。どちらも
本担当の所有範囲（`log_failure.rs` は他担当、RMU の汎用描画は reviewer-scope 固有でない）の外なので、
親の裁定に回す。なお本家の置換は監査行だけで、標準エラーの拒否理由は生の綴りのままである
（c01 の `stderr` はバイト一致）。

**B. `cwd` 欠落時の基点（1 件、本工程で是正済み）**

上記「本工程で行った変更」。c22 は修正後に本家とバイト一致した。

**C. 差し向け記録の文法（2 件）**

本家 `parseDispatchRecord:673-690` は `reviewer` と `unit` の非空、`stage` が文字列、`exempt` が
文字列配列、だけを見る。`stage: "Functional Design"` や `unit: "a/b"` もそのまま受け、`Stage` / `Unit` を
監査と拒否文言へ逐語で載せる（c24 / c25）。本 build の `parse_reviewer_dispatch` は `StageSlug`
（`/^[a-z][a-z0-9-]*$/`）と `UnitName`（区切りを含まない）を通すので、文法外の記録は「読めない」
として **fail-open（通す）** へ倒れ、drop に「malformed」を残す。前担当はこれを意図した差として
`reviewer_dispatch_record.rs:11-18` の doc に書き、`stage-protocol-reviewer.md:155`
（記録には現在のステージ slug を書く）を根拠にしている。

直していない理由: 記録の文法をどこまで信じるかは設計の裁定である。本家に合わせるなら
`ReviewerDispatch` の `stage` / `unit` を生の文字列で持ち直す（`StageSlug` / `UnitName` を外す）
ことになり、`ReviewerScopeBlock` の `Stage` / `Unit` 型と `session_audit_record.rs` の
必須項目に波及する。差の向きは「規約違反の記録に対して本家は強制し、本 build は通す」であり、
**保護が弱まる側**である点を明記して裁定を求める（「Assumptions & Open Questions」Q2）。

### ソース読解で突き合わせた観測（(1)(2) で踏んでいない分岐を含む）

| 観測 | 本家の出典 | 本 build |
| --- | --- | --- |
| 停止スイッチは `=1` の一致だけ | `aidlc-reviewer-scope.ts:758` | `disabled()` 同じ（c19） |
| 稼働記録は判断より先で、失敗しても判断を変えない | `:760-769` | `observe_hook_health` の戻り値を捨てる（c20 で `.last` の中身 = ISO 秒、改行なし、が一致） |
| 標準入力が読めない・JSON がオブジェクトでない・見ない工具は通す | `:771-783`、`isClaudeCodeHookInput` は `isPlainObject` だけ | 同じ（c14 / c20 / c21 / c31）。`hook_event_name` は両方とも見ない |
| 記録が無ければ通し、レビュー専用 2 エージェントが `construction/` へ触れたときだけ 10 分に 1 度助言 | `:829-861`（`REVIEW_AGENT_RE.test:840`、窓 `:846`）、`REVIEW_AGENT_RE:748` | 同じ文面・同じ窓（`ADVISORY_WINDOW` = 10 分）。目印 `reviewer-scope.missing-record.last`（c15 / c16、契約テスト 2 件） |
| 記録の mtime が `REVIEWER_DISPATCH_TTL_MS` を超えたら削除して通す | `:864-881`（判定 `:866`）、`aidlc-lib.ts:14645`（`6 * 60 * 60 * 1000`） | `DISPATCH_TTL` = 6 時間、`elapsed > TTL` で削除（c18、契約テスト `an_orphaned_dispatch_record_is_ignored_and_cleaned_up`）。mtime が読めなければ古いとみなさない |
| 記録が読めない・形が違えば drop して通す | `:882-890` | 同じ文面（c17） |
| 身元: 名乗るなら差し向けの当人と一致、名乗らないなら `scoped_registration` | `:906-908` | `ReviewerDispatch::is_dispatched`（c11 / c12 / c13） |
| 拒否は `Tool` / `Target` / `Stage` / `Unit` の 4 項目を常に載せる | `emitReviewerScopeBlocked:703-746` | `ReviewerScopeBlock::audit_fields`（順序も同じ。`session_audit_contract` が必須 4 項目を固定） |
| 監査ファイルが無ければ行を書かないが拒否はする | `:715` `if (!existsSync(auditFilePath(projectDir))) return;` | **未実測**。本 build は `SessionAudit` へ保存して RMU が描くので、シャードが無い状態では投影がシャードを作る可能性がある（下記「範囲外・未駆動」） |
| 監査ロック競合時は行を落として拒否は保持 | `:716-723` | 保存失敗を `ReviewerScopeError` で運び、drop に残して拒否を保持（経路は同じ向き。競合そのものは未駆動） |
| 拒否は標準エラーへ逐語 + 改行、exit 2、標準出力は空 | `:931-932`、`blockReason:692-701` | 同じ（c01 等でバイト一致） |
| `Grep` の `pattern` は内容の正規表現なので見ない | `:108-115` | 同じ（`raw_texts` の doc） |
| `candidateStrings` の鍵と順序（`file_path` → `notebook_path` → `path` → `paths[]`、`LS`/`Glob`/`Grep` の探索根、`Bash` の `command`） | `:116-150` | `raw_texts` 同じ（parity の Read/LS/Glob/Grep/Bash 全件） |
| `..` の畳み込み、大小の畳み込み、グロブ成分の `construction` 一致、掃く根の判定 | `:164-243` | `reviewer_scope_paths.rs` / `crosses_at`（parity） |
| `cd` が続く実行位置の基点を動かす、`grep` / `rg` / `find` / `ls` / `cat` 族 / その他の読み方 | `:461-627` | `judge_command` 以下（parity の Bash 97 件、c30） |
| team unit ownership の claimed-checkout（`unitScope`） | `:786-826` | **未移植**（`review_guards.rs` の doc に明記）。ゼロ Unit の本 build では到達しない |

## 確認項目の表（依頼 3 の (a)〜(f)）

「固定」は子プロセス（実行バイナリ）の契約テスト。「parity」は純関数の 165 件。「実走行」は上記 (2)。

| 項目 | 実装 | 公開境界の固定 | 本家との一致 |
| --- | --- | --- | --- |
| (a) 記録があるときの兄弟 Unit への読取り拒否と `REVIEWER_SCOPE_BLOCKED` の保存（コマンド → イベント → SQLite → RMU） | 実装済み | `Read`: `a_dispatched_reviewer_reaching_a_sibling_unit_is_refused_and_recorded`（見出し・`Event`・`Tool`・`Target`・`Stage`・`Unit` を主張）。`Grep`: `a_pathless_search_that_would_sweep_every_sibling_is_refused`（`Target: .`）。`Bash`: `a_shell_call_reaching_a_sibling_unit_is_refused_through_the_same_path`（`Tool: Bash`）。`LS`: `the_exempt_carve_out_is_exact_so_browsing_its_directory_still_crosses`。**`Glob` の子プロセス面は無い**（parity 11 件と実走行 c06 / c07 で確認） | 語彙・項目・順序は一致。絶対経路の `Target` だけ `<project-dir>` 置換の差（A） |
| (b) `exempt` の通過 | 実装済み（絶対・相対の両形、完全一致のみ） | `the_current_unit_and_an_exempt_sibling_file_stay_open`（許可 3 形、監査行なし） | 一致（c03 / c28 / c29、parity） |
| (c) 記録不在時の 1 回だけの助言と heartbeat | 実装済み | `the_reviewer_scope_hook_records_its_heartbeat_and_advises_once_on_a_missing_record`（2 回呼んで drop 1 行）、`a_non_reviewer_call_leaves_no_advisory` | 一致（c15 / c16、drop 文面バイト一致） |
| (d) 差し向け記録の TTL 掃除 | 実装済み（6 時間、超過で削除 + drop） | `an_orphaned_dispatch_record_is_ignored_and_cleaned_up`（7 時間前の mtime） | 一致（c18、本家 `REVIEWER_DISPATCH_TTL_MS = 6h`） |
| (e) 停止スイッチ | 実装済み | `the_reviewer_scope_off_switch_stops_every_observation`（`.last` も残さない）、`the_off_switch_stops_enforcement_as_well_as_observation` | 一致（c19） |
| (f) 不正 JSON / 無関係入力の素通し | 実装済み | 子プロセス面: `only_the_dispatched_reviewer_is_enforced_against`（指揮者・他エージェント）、`a_malformed_dispatch_record_skips_enforcement_and_records_why`。**標準入力が不正 JSON のときの reviewer-scope の子プロセス面は無い**（封筒の単体テストと実走行 c20 / c21 / c31 で確認。凍結側は `a_read_only_call_and_malformed_input_are_allowed_without_a_record` がある） | 一致（c11 / c12 / c14 / c17 / c20 / c21 / c31） |
| 追加: `cwd` 欠落時の基点 | 本工程で実装 | `a_call_without_a_working_directory_resolves_relative_roots_against_the_project_dir` | 一致（c22、修正後） |

## 範囲外・未駆動の分岐

- **ゼロ Unit の非適用分岐**: 棚卸し行のとおり、本 build の CLI は Unit を持つ状態を作れない
  （`aidlc-bolt start` 未配線、`ReviewPolicy::per_unit` は Q1 = B の配線のみ）。差し向け記録は
  規約上**指揮者が直接書く**ファイルなので、契約テストと実走行はそれを直接置いて越境拒否を駆動した。
  「Unit 有りの状態から記録が置かれるまで」の指揮者側の流れは本 build に無く、本工程の対象外。
- **team unit ownership の claimed-checkout**（本家 `:786-826`）: 未移植。`Unit Ownership: team` を
  本 build は作れないので到達しない。
- **監査シャードが無い状態**（本家 `:715`）: 未実測。`intent-create` 後は必ずシャードがあるため、
  一時ワークスペースでは作れなかった（作るには生成物を消す必要がある）。本 build がその状態で
  シャードを新規に作るかどうかは確認していない。
- **監査ロックの競合**（本家 `:716-723`）: 未駆動。
- **Kiro CLI の `scoped_registration`**: 封筒の欄と身元判定だけ（c13）。Kiro 側の登録は本 build にない。
- **`redactProjectDirPrefix` の実パス・`\` 変種**: A の是正時に `log_failure.rs::redact` が
  既に扱っている（同ファイルのテスト `project_redaction_also_recognizes_the_resolved_directory`）。
- **字句規則の差**（`2>&1` の `&` で区切る等）: 本家 `shellTokens` を逐語で写しており、review-freeze
  側の是正パッチは**持ち込んでいない**（別の上流関数のため）。`reviewer_scope_segments.rs` の
  テスト `a_descriptor_duplication_splits_on_its_ampersand` が固定している。
- **`Glob` の子プロセス面**と**不正 JSON 標準入力の子プロセス面**（reviewer-scope）は上表のとおり
  未固定。実装は実走行で一致を確認済み。

## 検査結果表

| 検査 | コマンド | 結果 | ログ |
| --- | --- | --- | --- |
| 整形 | `rustfmt --edition 2024 --config-path rustfmt.toml --check <所有 24 ファイル>` | 差分なし（終了 0） | [check-1-rustfmt.log](reviewer-scope-logs/check-1-rustfmt.log) |
| 静的検査（依頼の 5 クレート） | `cargo clippy -p aidlc -p harness-claude -p harness-infrastructure -p core-command-domain -p core-command-use-case --all-targets --no-deps -- -D warnings` | **失敗（終了 101）**。`aidlc` lib の 10 件はすべて他担当が編集中の `modules/app/aidlc/src/usage_ledger/**`（`dead_code` 7、`missing_const_for_fn` 2、`manual_let_else` 1、`collapsible_match` 1）。所有ファイルの所見は 0 | [check-2-clippy.log](reviewer-scope-logs/check-2-clippy.log) |
| 静的検査（4 クレート単独） | `cargo clippy -p harness-claude -p harness-infrastructure -p core-command-domain -p core-command-use-case --all-targets --no-deps -- -D warnings` | 所見なし（終了 0） | [check-2b-clippy-four-crates.log](reviewer-scope-logs/check-2b-clippy-four-crates.log) |
| 静的検査（`aidlc`、他担当の分類だけ除外） | `cargo clippy -p aidlc --all-targets --no-deps -- -D warnings -A dead_code -A clippy::missing_const_for_fn -A clippy::manual_let_else -A clippy::collapsible_match` | 所見なし（終了 0）。除外 4 種は `usage_ledger` の失敗分類そのもので、所有ファイルはこの除外に依存していない（除外なしの実行でも所有ファイルは名指しされていない） | [check-2c-clippy-app-with-foreign-exclusions.log](reviewer-scope-logs/check-2c-clippy-app-with-foreign-exclusions.log) |
| 独自検査 | `cargo lint` | 所見なし（出力なし、終了 0） | [check-3-cargo-lint.log](reviewer-scope-logs/check-3-cargo-lint.log) |
| 契約テスト | `cargo test -p aidlc --test review_guards_contract` | 24 passed / 0 failed | green-15 |

ビルド時の `aidlc` lib の警告 53 件（green-15 の出力）もすべて `usage_ledger/**` で、所有ファイルは
0 件（`-->` 行で集計）。`cargo test --workspace` / `cargo clippy --workspace` / カバレッジ / Quint / ITF
は本工程では実行していない（承認済み[テスト手順](unit-test-instructions.md)が B1 統合前の共通検査と
位置づけているため）。

## 残課題

1. **A: 監査行の `<project-dir>` 置換**（reviewer-scope と review-freeze の `Target`）。是正の置き場
   （`log_failure.rs::redact` の昇格 or RMU 描画）を親が決める。決まれば TDD で 1 スライス。
2. **C: 差し向け記録の文法**（本家は逐語で強制、本 build は fail-open）。裁定待ち（Q2）。
3. `GuardReviewerScopeUseCase` の単体テスト 0 件。契約（各コンポーネント 5〜8 件）に対し、この
   コンポーネントは子プロセス面の 12 件だけで固定されている。use-case の `#[cfg(test)]` フェイク
   （`test_support.rs`）で「拒否のとき 1 件保存・許可のとき保存なし・保存失敗時に拒否を運ぶ」の 3 件を
   足すのが最小。
4. `Glob` と不正 JSON 標準入力の子プロセス面（reviewer-scope）。実装は実走行で一致済みなので
   固定だけが未了。
5. 監査シャードが無い状態と監査ロック競合の分岐の実測。
6. フック全体の差分採取（32 ケース）はゴールデン化していない一度きりの採取である。固定化するなら
   `capture-reviewer-scope.ts` と同じ来歴規則に沿って `tests/golden/upstream-a277af21/` へ載せる。
7. `cargo test --workspace` / `cargo clippy --workspace` は他担当の `usage_ledger` が落ち着いた
   時点で親が 1 回行う。

## Sources

- 本家（固定コミット `a277af218f0df7f325d3b8be7b6d90fce2c5bd40` の `git show` 実バイト）
  `dist/claude/.claude/hooks/aidlc-reviewer-scope.ts:108-150,164-243,278-330,384-460,461-627,629-701,703-746,748,756-932`、
  `tools/aidlc-lib.ts:551-584,1633-1653,14418-14421,14630-14645,16163-16186,22078-22091,22167-22203`、
  `tools/aidlc-audit.ts:112,242,487,512-534,591-610`、
  `aidlc-common/protocols/stage-protocol-reviewer.md:96-98,155`。
- 本家採取 `tests/golden/upstream-a277af21/reviewer-scope/cases.json`、
  `scripts/goldens/capture-reviewer-scope.ts`、`scripts/goldens/reviewer-scope-corpus.json`。
- 現コード（上記「実装の要約」の各ファイル）と `modules/app/aidlc/src/runtime.rs:1046-1047,1075-1079`、
  `modules/core/read-model-updater/src/orchestration/read_model_updater.rs:462`、
  `modules/core/read-model-updater/src/workspace/audit_block.rs:47-72`、
  `modules/app/aidlc/src/runtime/log_failure.rs:58`、`modules/app/aidlc/src/layout.rs:205`。
- 前担当のログ [reviewer-scope-logs/](reviewer-scope-logs/)（red-1〜5、green-2/3/5/6、lint-1、
  `differential-upstream-165.tsv`）と本工程の採取（green-1/4/7〜15、red-6、check-1〜3、
  `hook-differential/`）。
- 関連記録 [review-guards-verification.md](review-guards-verification.md)、
  [shell-write-targets-verification.md](shell-write-targets-verification.md)、
  [step7-divergence-questions.md](step7-divergence-questions.md)、
  [switchover-condition2-findings.md](switchover-condition2-findings.md)（F1: `.aidlc-execution` 依存 —
  本フックは実行カーソルを読まないので依存を増やしていない）、
  [remaining-step7-inventory.md](../remaining-step7-inventory.md)。
- 承認済み [実装計画](code-generation-plan.md)、[テスト手順](unit-test-instructions.md)、
  設計規則 [coding-rules/](../../../../knowledge/aidlc-shared/coding-rules/)。

## Assumptions & Open Questions

- Q1（親の裁定）: 監査行の `<project-dir>` 置換（差 A）をどこで是正するか — `log_failure.rs::redact`
  を共有化して両フックの入口で置換するか、RMU の `render_audit_block` に project dir を渡すか。
  どちらも本担当の所有範囲外なので実装していない。
- Q2（人間の裁定）: 差し向け記録の `stage` / `unit` が文法外のとき、本家どおり逐語で強制するか、
  現状どおり fail-open で通すか。現状は保護が弱まる側の差である。
- 仮定: 実走行の比較で project dir・記録名・ISO 時刻を正規化したことは、本家の `<project-dir>`
  置換の有無を隠していない（置換後の文字列は正規化の対象外で、差 A として顕在化している）。
- 仮定: 本家側の一時 project には `aidlc-state.md` を最小内容で置いた。`isTeamUnitOwnership` は
  それを team と読まないため、claimed-checkout 分岐は両側とも通っていない。
