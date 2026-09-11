# カバレッジ向上の共通 brief（U2 Step 9）

作業ディレクトリ `/Users/j5ik2o/orca/workspaces/amadeus-ng/stage1`（git worktree。外へ出ない。コミット・push・stash 禁止）。

## 目的

`bash scripts/coverage.sh --base main` の相対ゲート（`head >= base - 0.01`、base = 99.15%）を通す。現状 head 94.77%、未カバー 4,532 行。利用者の裁定は「A. テストを足してゲートを通す」（[coverage-gap.md](../coverage-gap.md)）。閾値・除外・seed（`scripts/coverage.sh`）は変更しない。

## 規律

- 承認済み Testing Contract（TDD）に従う。新しいテストは「その行が実行される振る舞い」を検証する契約として書く。`assert!(true)` や実装と同じ計算を写すだけのアサート、命中させるためだけの無意味な呼び出しは禁止。
- 未カバー行の大半は失敗経路・拒否経路・DTO の変換・Display である。それぞれ「不正入力を拒否する」「壊れた保存物を成功に丸めない」「文言が本家逐語である」といった契約として検証する。
- **プロダクトコードは変更しない**（テスト追加のみ）。到達不能と判断した dead code は削除せず、報告書に「dead code 候補」として根拠（参照 0 件の grep 結果）を列挙する。テスト容易化のための公開 API 追加もしない。
- テストは所有 crate 内（`src` の `#[cfg(test)]` モジュール、または `tests/` の新規・既存ファイル）に置く。他担当の crate のファイルには触れない。
- `#[cfg(test)]` 内の `unwrap` / `expect` は `clippy.toml` の許容範囲。`cargo clippy --workspace --all-targets -- -D warnings` と `cargo lint` を節目で通す。
- 計測: 自分の crate だけを `cargo llvm-cov --no-report -p <crate> --tests` → `cargo llvm-cov report --json --output-path <tmp>` で測り（`--ignore-filename-regex '(^|/)modules/app/aidlc/src/main\.rs$'`）、対象ファイルの未カバー行を JSON の `files[].segments` か `cargo llvm-cov report --text` で確認して狙う。workspace 全体の `coverage.sh` は親が最後に 1 回行う。
- 子プロセスを `env_clear()` で起動するテストは `tests/support/coverage_profile_env.rs` の `coverage_profile_env()` を `.envs()` で渡す（計測下で `.profraw` が cwd に落ちるのを防ぐ）。
- ログ: `…/code-generation/coverage-logs/<担当名>/NN-<slug>.log`（先頭にコマンドと UTC 時刻、末尾 `# exit=<code>`）。少なくとも「着手前の crate 計測」「着地後の crate 計測」「対象テストの実行」「fmt/clippy/lint」を残す。
- 承認済み `code-generation-plan.md` / `unit-test-instructions.md` / `code-generation-questions.md`、`.claude/` 配下は編集しない。`aidlc next` や `.claude/tools/aidlc-orchestrate.ts` の実走行はガードで拒否されるので試みない。

## 完了条件

1. 担当の対象ファイルの未カバー行の合計を **少なくとも 85% 減らす**（到達不能で残る行は dead code 候補として報告）。
2. 所有 crate の `cargo test -p <crate>` 0 失敗、fmt / clippy / lint 成功。
3. 報告書 `…/code-generation/coverage-logs/<担当名>-verification.md`（日本語）: 着手前後の crate 行カバレッジ、追加したテスト（ファイル・件数・検証した契約）、dead code 候補、残った未カバー行と理由。

最後に §11 形式（## Subagent Summary / ### Produced / ### Key Decisions / ### Issues / Concerns / ### Next Steps）で簡潔に返す。
