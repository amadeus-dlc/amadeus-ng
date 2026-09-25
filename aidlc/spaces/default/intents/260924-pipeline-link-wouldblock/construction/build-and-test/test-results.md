# テスト結果 — Issue #134（`replace_pipeline` を IMMEDIATE に揃える）

## 検証した状態

- 作業用のチェックアウト（`git worktree add --detach` で `main` の先端 `5a2b51d3` を取り出し、`journal_reader_impl.rs` の変更 1 件だけを載せたもの）。`git status --short` の差分はこの 1 ファイルだけ。
- ビルドの出力先は別の `CARGO_TARGET_DIR`。本体の `target/`（フックが使う `target/release/aidlc`）には触れていない。
- 環境: macOS（Apple Silicon）、Rust 1.95.0。

## ビルド

| コマンド | 結果 |
| --- | --- |
| `cargo build --workspace --all-targets` | 成功 |
| `cargo fmt --all -- --check` | 成功（終了コード 0） |
| `cargo clippy --workspace --all-targets -- -D warnings` | 成功（終了コード 0） |
| `cargo lint` | 成功（終了コード 0） |

## 単体テスト（`unit-test-instructions.md` のコマンド、各 1 回）

| コマンド | 結果 |
| --- | --- |
| `cargo test -p core-read-model-updater --lib orchestration::journal_reader_impl::tests::a_write_lock_held_by_another_connection_is_reported_as_would_block -- --exact` | 1 通過 |
| `cargo test -p core-read-model-updater --lib orchestration::journal_reader_impl::tests::replace_pipeline_waits_for_a_write_lock_held_by_another_connection -- --exact` | 1 通過（0.28 秒。ロックを握る 200ms を待ってから成功） |
| `cargo test -p aidlc --test pipeline_link_contract -- --exact a_publication_failure_preserves_the_receipt_for_recovery a_failed_pipeline_projection_preserves_the_prior_query_result_until_recovery concurrent_duplicate_completions_persist_only_one_receipt` | 3 通過 |

修正前に赤になることの証拠は Code Generation が記録している（`construction/code-generation/code-summary.md` の「修正前の赤」: DEFERRED に戻すと 0.05 秒で `Err(Io { kind: WouldBlock, .. })`）。

## 既存スイート全体（`cargo test --workspace --no-fail-fast`）

- テストバイナリ 140 本、**通過 4058・失敗 1・無視 0**
- 失敗: `pipeline_link_contract::public_link_inputs_and_results_match_fixed_upstream`（`pipeline_link_contract.rs:198`、終了コード `None` = シグナルによる終了）
  - 同じテストを単独で 5 回走らせると、5 回とも通過した。
- 本体の作業ツリーで落ちていた `engine_hook_wiring_contract::the_binding_definition_counts_registrations_apart_from_hook_names` は、作業用のチェックアウトでは通過した。原因は、スモーク用のコミットしない `.claude/settings.json` の変更だったと確定した。

### 子プロセスの SIGKILL（`unix_wait_status(9)`）の切り分け

`cargo test -p aidlc --test pipeline_link_contract`（14 本）を同じ環境で繰り返した結果:

| 状態 | 回数 | 全通過 | 1〜2 本が SIGKILL で失敗 |
| --- | --- | --- | --- |
| 修正あり（最初の 3 回） | 3 | 0 | 3 |
| 修正なし（`git stash` で修正を外した HEAD） | 4 | 3 | 1 |
| 修正あり（続けて 4 回） | 4 | 3 | 1 |

- 失敗の形はいつも同じで、子プロセスが `unix_wait_status(9)` で終わり、stdout と stderr は空になる。落ちるテストは回ごとに入れ替わる（`reports_require_the_complete_current_pipeline_before_opening_or_approving_a_gate`、`single_receipts_do_not_replace_the_main_pipeline_chain`、`public_link_inputs_and_results_match_fixed_upstream` など）。
- 修正を外しても同じ頻度で起きる。#134 の症状（終了コード 1 と `WouldBlock` の文言）とも形が違う。今回の変更に起因するものではない。Issue #134 のコメントにある PR #138 の観測（macOS ローカルで 1 件だけ落ちる）と同じものとみられる。

## カバレッジ（`scripts/coverage.sh`、90% の床）

| 試行 | 結果 |
| --- | --- |
| 1 回目 | 計測の途中で `artifact_reuse_contract::a_receipt_for_a_defined_stage_appends_exactly_one_audit_row_without_moving_the_state` が SIGKILL で失敗し、`coverage.json` が作られず中断（終了コード 1） |
| 2 回目 | 計測の途中で `jump_contract` の 4 本が SIGKILL で失敗し、同じく中断（終了コード 1） |

計測付きの実行は遅いので、SIGKILL が起きやすい。ローカルでは率を得られなかった。閾値と除外の設定には触れていない。

## Target Verification Matrix（確定）

| Target ID | Source | Expected | Actual | Evidence | Owning Stage | Verdict |
| --- | --- | --- | --- | --- | --- | --- |
| TC-SF-1 | Testing Contract `obligations.scope_floor[0]` | 不具合を狙った回帰テストがあり、修正後に通り、修正前に落ちる | 修正後 1 通過（0.28 秒）、修正前は 0.05 秒で `WouldBlock` | 上の単体テスト表、`code-generation/code-summary.md` の「修正前の赤」 | build-and-test | Met |
| TC-SF-2 | Testing Contract `obligations.scope_floor[1]` / FR4.2 | `cargo test --workspace` の失敗 0 | 4058 通過・1 失敗（修正前にも起きる SIGKILL） | 上の「既存スイート全体」と切り分け表 | build-and-test | Not Met |
| TC-SV-1 | Testing Contract `obligations.strategy_volume` | 要件ごとに検証できるテストが最も狭い水準に 1 本ある | FR1〜FR3・NFR1・NFR4・NFR5 に対応するテストがある（`traceability.json`） | `code-generation/traceability.json`、`cross-unit-traceability.md` | build-and-test | Met |
| NFR1 | `requirements.md` NFR1 | busy timeout 5000ms を変えない | `DEFAULT_BUSY_TIMEOUT` に差分なし | `git diff` の差分は `replace_pipeline` のトランザクション開始とテストだけ | build-and-test | Met |
| NFR2 | `requirements.md` NFR2 | 修正 PR の CI（`check`・`coverage`）が再実行なしで緑 | 未計測（PR はまだ無い） | — | deployment-execution | Unverified |
| NFR3 | `requirements.md` NFR3 / `scripts/coverage.sh` | 行カバレッジ 90% 以上 | 計測できず（2 回とも SIGKILL で中断） | 上の「カバレッジ」表 | build-and-test | Unverified |
| NFR4 | `requirements.md` NFR4 | 出力契約・監査行の形が変わらない | 固定本家との比較テスト `public_link_inputs_and_results_match_fixed_upstream` は単独で 5/5 通過。全体実行では SIGKILL で 1 回落ちた | 上の「既存スイート全体」 | build-and-test | Met |
| NFR5 | `requirements.md` NFR5 | fmt・clippy・lint が通る | 3 つとも終了コード 0 | 上の「ビルド」表 | build-and-test | Met |

## 失敗時の手順の記録

- **失敗の判定**: TC-SF-2 が Not Met、NFR2・NFR3 が Unverified なので、この段は失敗である。
- **段 1（この段の中での修正、2 回まで）**: 環境の問題として切り分けた。
  - ローカル設定の混入は、作業用のチェックアウトで解消した（1 回目）。
  - カバレッジを再計測した（2 回目）。
  - SIGKILL は、この段で直せる設定や足場の問題ではなかった。
- **段 2（分類と影響の見積もり）**: 原因は生成コードにも、Code Generation の方針にもない。修正を外しても同じ頻度で起き、落ちるテストも回ごとに入れ替わる。Code Generation に戻って直せる候補は無い。
- **段 3（自律的な差し戻し）**: 当たらない。`Construction Autonomy Mode` は unset（gated として扱う）。差し戻して直す候補も無い。
- **段 4（人に判断を仰ぐ）**: 修正の候補が無い版の問いで、人の判断を仰ぐ。

## 人の判断

- **選択**: `Accept failure`（2026-09-24）
- **扱い**: TC-SF-2（Not Met）と NFR3（Unverified）は、失敗として記録したまま承認ゲートへ進む。ワークスペース全体の緑とカバレッジ 90% は、修正 PR の CI（Linux の `check`・`coverage` ジョブ）で確かめる。NFR2 の判定と同じ場所になる。

## 改訂 1（Code Generation への差し戻しの後、2026-09-24）

PR #154 のレビュー指摘を受けて再現テストの同期を改訂したので、作業用のチェックアウト（ブランチ `fix/134-replace-pipeline-immediate` の `1ec94b6f` に、改訂後の `journal_reader_impl.rs` を載せたもの。差分はテストの部分だけ）で確かめ直した。

| 検査 | 結果 |
| --- | --- |
| `cargo build --workspace --all-targets` | 成功 |
| `cargo fmt --all -- --check` / `cargo clippy --workspace --all-targets -- -D warnings` / `cargo lint` | 3 つとも終了コード 0 |
| 単体テストのコマンド（`unit-test-instructions.md`、各 1 回） | `a_write_lock_held_by_another_connection_is_reported_as_would_block` 1 通過、再現テスト 1 通過（0.30 秒）、契約テスト 3 本通過 |
| `cargo test --workspace --no-fail-fast` | テストバイナリ 140 本、**通過 4058・失敗 1** |

- 失敗した 1 件は `learnings_contract::entries_become_candidates_and_open_questions_stay_parked`（`learnings_contract.rs:341`）。子プロセスが `unix_wait_status(9)` で終わり、出力は空だった。1 回目と同じ macOS ローカルの SIGKILL である。単独で再実行すると通過した。今回の変更（テストだけ）とは関係しない。
- カバレッジは、1 回目に同じ SIGKILL で 2 回とも計測が中断した。この段での修正の試行は 2 回まで（失敗時の手順の段 1）なので、ローカルでは計測し直していない。1 回目のコミット `1ec94b6f` の PR の CI では、行カバレッジ 97.44% で床（90%）を満たしている。改訂 1 のコミットでも、PR の CI の `coverage` ジョブで確かめる。

### Target Verification Matrix（改訂 1 で確定）

| Target ID | Expected | Actual | Evidence | Owning Stage | Verdict |
| --- | --- | --- | --- | --- | --- |
| TC-SF-1 | 回帰テストがあり、修正後に通り、修正前に落ちる | 修正後は 209〜257ms 待って成功、修正前は 58〜125µs で `WouldBlock` | `code-generation/code-summary.md` の「改訂 1」、上の表 | build-and-test | Met |
| TC-SF-2 | `cargo test --workspace` の失敗 0 | 4058 通過・1 失敗（修正と関係しない SIGKILL） | 上の表 | build-and-test | Not Met |
| TC-SV-1 | 要件ごとに検証できるテストが 1 本ある | 変更なし | `code-generation/traceability.json` | build-and-test | Met |
| NFR1 | busy timeout 5000ms を変えない | 差分なし | `git diff` | build-and-test | Met |
| NFR2 | 修正 PR の CI が再実行なしで緑 | 1 回目のコミットでは全ジョブ緑（再実行なし）。改訂 1 のコミットはこれから | PR #154、run 35954181150 | deployment-execution | Unverified |
| NFR3 | 行カバレッジ 90% 以上 | ローカルでは計測できず。1 回目のコミットの CI では 97.44% | PR #154 の `coverage` ジョブ | deployment-execution | Unverified |
| NFR4 | 出力契約・監査行の形が変わらない | 変更はテストだけ。本体は 1 回目のまま（1 回目の CI の `aidlc-distribution` は緑） | 上の表 | build-and-test | Met |
| NFR5 | fmt・clippy・lint が通る | 3 つとも通過 | 上の表 | build-and-test | Met |

### 人の判断（改訂 1）

- **選択**: `Accept failure`（2026-09-24、改訂 1）
- **扱い**: TC-SF-2 は Not Met のまま記録して承認ゲートへ進む。NFR2・NFR3 は PR #154 の CI で確かめる。

## 訂正（2026-09-25）— FR1.2 の検証

FR1.2（並行した 2 本のうち、もう一方は `already completed` と終了コード 1 で拒否される）を `OK` / `Met` としていたのは誤りだった。`concurrent_duplicate_completions_persist_only_one_receipt` は成功した起動の数と監査の件数しか確かめておらず、拒否側の文言と終了コードは検査していなかった（#155 での CodeRabbit の指摘）。

- `code-generation/traceability.json` の FR1.2 を `Deferred` に改めた。
- 拒否側の検査は PR #156（https://github.com/amadeus-dlc/amadeus-ng/pull/156）で足す。これがマージされるまで、FR1.2 は検証済みとして扱わない。TC-SV-1 の `Met` も FR1.2 については同じ扱いとする。

### 解消（2026-09-25）

PR #156（https://github.com/amadeus-dlc/amadeus-ng/pull/156、main の `d1c9caef`）がマージされ、並行テストが拒否側の終了コード 1 と `already completed this attempt` を確かめるようになった。`code-generation/traceability.json` の FR1.2 を `OK` に戻した。FR1.2 と TC-SV-1 は検証済みとして扱ってよい。
