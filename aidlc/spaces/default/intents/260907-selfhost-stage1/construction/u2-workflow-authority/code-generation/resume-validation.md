# 再開時の独立検証

## 対象と結果

2026-09-09（日本時間）、U2の実装計画承認後に実装担当とは別に実行した。Rustの変更後の全体検証・CI・実地スモークの代わりにはしない。

| コマンド | 結果 |
| --- | --- |
| `mise trust` | 終了0。未信頼設定なし |
| `bun scripts/aidlc-sync.ts --check` | 終了0。コピー0、削除0、保持設定の確認0、同期済み |
| `bun scripts/goldens/verify-corpus.ts tests/golden/upstream-a277af21` | 終了0。保存済みコーパスの検証成功 |
| 下記の配布回帰テスト | 終了0。95成功、失敗0、642 assertions |
| 下記の採取・比較テスト | 終了0。47成功、失敗0、229 assertions |

```bash
bun test ./scripts/aidlc-sync.test.ts ./scripts/aidlc-harness.test.ts ./scripts/aidlc-unit-lifecycle.test.ts ./scripts/aidlc-plan-progress.test.ts ./scripts/aidlc-traceability.test.ts ./.codex/hooks/aidlc-codex-adapter.test.ts
bun test ./scripts/goldens/capture-source.test.ts ./scripts/goldens/capture-corpus.test.ts ./scripts/goldens/compare-corpus.test.ts ./scripts/goldens/capture-doctor.test.ts ./scripts/goldens/capture-learnings.test.ts
```

実行ログはローカルの `/tmp/amadeus-stage1-resume-distribution-tests.log` と `/tmp/amadeus-stage1-resume-corpus-tests.log`。一時ファイルのため恒久的な証拠とは扱わない。上記の対象ファイルを変更した場合、この結果は変更後の成功を示さない。

## 中断点の照合

`tdd-evidence.md` と `construction-inventory.md` はArtifactSavedの通常RMU統合前で止まっている。一方、監査記録の2026-09-08T18:36:59Zは統合完了、19:09〜19:10:50Zはnextの名詞入力・カーソル観測の接続を報告している。実装担当も通常RMU統合の現物を確認した。古い作業メモだけを根拠に再実装せず、現コードと後続検証で残作業を判断する。

`hook-runtime-diagnostic-questions.md` の保存失敗診断の扱いは再開時点で未回答だったため、別途裁定を求めた。計画承認からこの例外の承認を推定しない。

## next引数修正後の独立検証

実装担当の修正後に `cargo test -p aidlc --test next_input_contract --test next_branches` を別途実行し、終了0を確認した。`next_branches` は44件、`next_input_contract` は2件が成功し、失敗・無視・フィルタ除外は各0件。後者は固定本家21入力の全文比較と、自由文中の名詞をコマンドへ変換しない境界を検証する。ログは `/tmp/amadeus-stage1-resume-next-tests.log`。

`git diff --check` も終了0だった。Rust workspace全体、カバレッジ、CIと実地スモークの完了を示す結果ではない。

## 保存失敗診断の裁定後の独立検証

`hook-runtime-diagnostic-questions.md` Q1への「推奨でいいよ」を記録した後、実装担当が追加した次のテストを別途実行し、終了0、1件成功、失敗0、8件フィルタ除外を確認した。

```bash
cargo test -p aidlc --test claude_hook_contract heartbeat_directory_failure_preserves_public_files_and_reports_eisdir
```

元の観測にEISDIR・open・採取時の対象パスが含まれること、Rustの標準エラー1行と末尾LF、終了コード1、空の標準出力、既存初期ファイルのバイト不変、heartbeatディレクトリの保持とdrop等の無生成を検証している。内部SQLite/read表を本家のファイル集合と同一だとは扱わない。ログは `/tmp/amadeus-stage1-eisdir-independent.log`。この時点では停止フック・補助更新・U2全体の検証は未完了。

## 停止フックの3境界の独立検証

`cargo test -p aidlc --test upstream_271_contract <test-name>` で以下を1件ずつ実行し、各終了0、成功1件、失敗0、ほか56件はフィルタ除外だった。

- `stop_blocks_pending_work_once_then_releases_without_workflow_progress`
- `stop_limit_honors_the_numeric_prefix_and_reentrant_initial_count`
- `stop_allows_an_open_human_gate_without_starting_a_no_progress_streak`

初回差止めと同進捗での解除、上限指定・再入、開いた人間承認ゲートを対象にする。未回答文書・会話のみのターン・背景処理・障害回復等の未完了境界を代替しない。ログは `/tmp/amadeus-u2-stop-independent.log`。

## 依存追加後の脆弱性監査

`cargo audit` は129依存を検査し、終了0だった。対象 `Cargo.lock` のSHA-256は `9e2ab79cf4533986823f7f86afaf1e365e175c5dd67d735f29a8c7a225fc458f`。ログは `/tmp/amadeus-u2-current-dependency-audit.log`。この後に依存を変更した場合は再検査が必要であり、CI上の監査ジョブの結果を代替しない。

## 2026-09-09 今回の変更を揃えた後の最終確認

全担当の編集を閉じ、cargo fmt --allで整形した後の親担当の検査である。

| 検査 | 結果 |
| --- | --- |
| workspace全target Clippy（-D warnings） | 終了0 |
| cargo lint | 終了0 |
| cargo fmt --all --check | 終了0 |
| git diff --check | 終了0 |
| 下記8つのapp結合target | 終了0、175件成功、失敗0、無視0 |
| source_baseline_publication_contract | 4件成功 |
| projection_golden_test | 16件成功、2件失敗。センサー監査3種類の未裁定差を残した |

```bash
cargo test -p aidlc --test intent_lifecycle --test session_hooks_contract --test session_compaction_contract --test directive_context_contract --test plan_runtime_contract --test pipeline_link_contract --test review_receipt_contract --test jump_contract
```

内訳はdirective_context 3、intent_lifecycle 123、jump 7、pipeline_link 12、plan_runtime 5、review_receipt 5、session_compaction 4、session_hooks 16。ログを実測して集計し、テスト内部のコーパス観測数をテスト件数へ加算していない。ログはtest-evidence/resume-final-*.log。

別に、診断記録API7件、Query DAO34件、SQL規約13件、検証根拠の本家22観測等が成功している。これらと上記175件を重複を考慮せず全体のテスト総数として合算しない。

## 未完了の責任と再開点

U2とStep 9は未完了。全workspaceテスト、Quint/ITF、90%床/相対カバレッジ、release、CI、B1統合、最終成果物・独立レビューは残る。

人間裁定待ちはsensor-audit-comparison-questions.md、jump-contract-questions.md、task-update-contract-questions.mdの3つ。質問のdecisionを記録しasyncで提示済みだが、回答は届いていない。実装計画の承認から例外裁定を推定しない。

レビュー受領は本文・ソース・追記境界の照合まで。Markdown構文の完全互換、challengeの結果Query、retry回数制限、recovery、summary受領、ゲート前鮮度は次のsliceへ残る。TaskUpdate/runtime compile/learnings persist等はruntime-sync-contract.mdとUnit直下remaining-step7-inventory.mdを参照。
