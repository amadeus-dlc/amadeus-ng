# lone-intent 判定を本家に合わせた検証（`u2_lone_intent_upstream`）

2026-09-11。裁定 **F-H1 = B「本家に合わせる」**（[lone-intent-fallback-questions.md](lone-intent-fallback-questions.md)）の実装記録。前担当 [parity-closeout-verification.md](parity-closeout-verification.md) §1「`Layout::shared` の lone-intent 後退」と §3 F-H1 を起点にする。ログは [lone-intent-upstream-logs/](lone-intent-upstream-logs/)（各ファイル先頭にコマンドと UTC 時刻、末尾に `# exit=<code>`）。

## 1. 変更内容

| ファイル | 変更 |
| --- | --- |
| `modules/app/aidlc/src/layout.rs` | `shared` のカーソル実在判定を `is_dir()` から `is_record()`（`aidlc-state.md` の有無）へ戻した。本家 `aidlc-lib.ts` `activeIntent` の `existsSync(join(dir, raw, "aidlc-state.md"))` と同じ。doc コメントを裁定 B の内容に書き換え、復元経路が残る条件を明記した。単体テスト `a_cursor_naming_a_directory_without_a_state_file_is_still_the_record` を `…_is_not_a_record` へ置換（カーソル無視 → `None`、さらに状態ファイルを持つ記録が 1 つ在れば唯一記録へ後退） |
| `modules/app/aidlc/tests/intent_lifecycle.rs` | 復元テスト 2 件の前提を見直し（§3） |
| `modules/app/aidlc/tests/next_branches.rs` | `an_invalid_active_space_names_the_cursor_file_to_fix` の fixture: 逃がし先の記録に `aidlc-state.md` を置く（テストの意図「記録は解決できるが空間名が通らない」を本家の判定で成立させる） |
| `modules/app/aidlc/tests/claude_hook_contract.rs` | `write_audit_hook_projects_created_updated_and_drop_via_events` の fixture: 記録に `aidlc-state.md` を置き（本家の意味での記録にする）、投影先のシャードを `audit/*.md` から実測で探す補助関数 `projected_shard` を追加。状態ファイルが在るので監査はクローン別シャードへ投影され、hook-health の drop は記録配下 `.aidlc-hooks-health/` に落ちる |
| `modules/app/aidlc/tests/dispatch_rules_contract.rs` | case 79 の強化版 `a_cursor_naming_a_directory_without_a_state_file_falls_back_to_the_lone_record` を追加（ディレクトリだけ在って状態ファイルの無いカーソル先も無視して `rec-0001` を読む） |

承認済み `code-generation-plan.md` / `unit-test-instructions.md` / `code-generation-questions.md`、`.claude/` 配下の配布資産は編集していない。git のコミット・push・stash は行っていない。

## 2. Red / Green の根拠

| 段 | ログ | 結果 |
| --- | --- | --- |
| Red（単体） | [01](lone-intent-upstream-logs/01-red-layout-cursor-without-state-file.log) | 置換した `a_cursor_naming_a_directory_without_a_state_file_is_not_a_record` が `left: Some(…/half-made)`, `right: None` で失敗（exit 101） |
| Green（単体） | [02](lone-intent-upstream-logs/02-green-layout-cursor-without-state-file.log) | `is_dir()` → `is_record()` で `layout::tests` 9 件成功 |
| Red（統合） | [03](lone-intent-upstream-logs/03-red-aidlc-crate-after-is-record.log) | 判定変更だけを入れた `cargo test -p aidlc --no-fail-fast` で失敗はちょうど 4 件: `next_restores_missing_projection_files_without_repeating_audit`、`next_rejects_a_corrupt_restoration_plan_without_recreating_files`（intent_lifecycle）、`an_invalid_active_space_names_the_cursor_file_to_fix`（next_branches）、`write_audit_hook_projects_created_updated_and_drop_via_events`（claude_hook_contract）。前担当が §1 で予告した 2 件に加え、`is_dir()` に追従していた fixture 2 件 |
| Green（統合） | [04](lone-intent-upstream-logs/04-green-revised-tests.log) | 上記 4 件と case 79 系 2 件（既存 + 追加）がすべて成功 |
| 品質検査 | [05](lone-intent-upstream-logs/05-check-fmt-clippy-lint.log) → [06](lone-intent-upstream-logs/06-check-fmt-clippy-lint-2.log) | 05 は `claude_hook_contract.rs` の整形差 1 箇所で fmt が exit 1（clippy / lint は exit 0）。`cargo fmt --all` 後の 06 で `cargo fmt --all --check` / `cargo clippy --workspace --all-targets -- -D warnings` / `cargo lint` すべて exit 0 |

追加した統合テスト（dispatch_rules_contract）は単体 Red（01）で契約の失敗を確認した後に足したもので、単独の Red は採っていない。テストの削除・`#[ignore]`・弱体化は行っていない。

## 3. 復元経路が残る条件（承認済み Step 3 との両立）

本家の判定では記録の実在は `aidlc-state.md` の有無で決まり、これは `Layout::shared` のカーソル判定だけでなく、唯一記録の数え方（`lone_record` = 本家 `listIntentDirs`）と、セッション binding（`bound_selection` は従来から `state_file().exists()` を要求）にも共通している。したがって:

- **残る経路**: 記録が状態ファイルを持って解決できたうえで、監査シャード（`<record>/audit/*.md`）や memory の投影だけが失われている場合。`next`（および読取前に `catch_up` を通る全動詞）が `restore_missing_files` でジャーナルから描き直し、監査を繰り返さない。`next_restores_missing_projection_files_without_repeating_audit` の前半（`audit/` を消して `next` を 2 回、状態・監査ともバイト一致）と、`next_rejects_a_corrupt_restoration_plan_without_recreating_files`（`audit/` を消し、破損した復旧計画を注入 → 全入口が `projection restoration: read: io: InvalidData at …` で止まり、`audit/` を作り直さず、状態ファイル・ジャーナル・公開位置を変えない）が固定する。
- **届かない経路**: `aidlc-state.md` を失った記録。カーソルが名指していても本家どおり無視され、唯一記録にも数えられないので `next` は「状態なし」（`No workflow state found (no active intent). …`）を答え、ファイルを作り直さず、ジャーナルにも触れない。`next_restores_missing_projection_files_without_repeating_audit` の後半が固定する。依頼文の「カーソル無し・唯一記録の場合に限られる」は、本家の唯一記録が状態ファイルを持つものだけを数える以上、状態ファイル自体の復元を `next` から届かせる条件にはならない（本家には状態ファイルが正本でこの分岐が無い、という前担当 §3 F-H1 の記述どおり）。
- 例外的に状態ファイル不在のまま `catch_up` に到達するのは、`Layout::for_record` で記録を名指す経路（`intent-create` 直後の初回投影、`layout_for_artifact_only` のカーソル直読み、`intent_location` / `plan_approval` / `learnings` の明示指定）だけである。これらは本セッションで変更していない。

## 4. 本家 case 79 `dangling` との突き合わせ

- 採取条件（[stage-rules-logs/closeout-workspaces.sh](stage-rules-logs/closeout-workspaces.sh) L98–101）: 記録 `rec-0001`（`- **Current Stage**: code-generation`）だけが実在し、`active-intent` は存在しない `gone` を指す。本家の観測はカーソルを無視して唯一記録へ後退し `code-generation` の規則束を配る（[stage-rules-verification.md](stage-rules-verification.md) §表 6 行目）。期待値は本家採取のまま、手修正していない。
- 本 build: 単体 `layout::tests::a_cursor_that_names_an_absent_record_falls_back_to_the_lone_record` と統合 `dispatch_rules_contract::a_cursor_naming_an_absent_record_falls_back_to_the_lone_record` が同じ条件で `rec-0001` / `code-generation` を選ぶ（[04](lone-intent-upstream-logs/04-green-revised-tests.log)）。
- 本家判定の残り 1 点（ディレクトリは在るが `aidlc-state.md` が無い `half-made` を指すカーソル）は 2.7.1 コーパスに採取が無いので、本家コード `existsSync(join(dir, raw, "aidlc-state.md"))` を根拠に、単体 `…_is_not_a_record` と統合 `…_without_a_state_file_falls_back_to_the_lone_record` で同じ結果（無視して `rec-0001`）を固定した。前担当 §3 F-H1 の差はこれで解消し、切替条件 2 の判定材料からは外れる。

## 5. 最終検査

| 検査 | 結果 | ログ |
| --- | --- | --- |
| `cargo test -p aidlc --no-fail-fast` | 29 スイート、0 失敗（exit 0） | [07](lone-intent-upstream-logs/07-green-aidlc-crate-final.log) |
| `cargo test --workspace --no-fail-fast` | **101 スイート・3,124 件成功・0 件失敗（exit 0）** | [workspace-test-final.log](lone-intent-upstream-logs/workspace-test-final.log) |
| `cargo fmt --all --check` / `cargo clippy --workspace --all-targets -- -D warnings` / `cargo lint` | すべて exit 0 | [06](lone-intent-upstream-logs/06-check-fmt-clippy-lint-2.log) |
