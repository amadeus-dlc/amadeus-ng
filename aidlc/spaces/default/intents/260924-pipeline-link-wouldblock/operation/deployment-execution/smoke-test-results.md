# スモークテストの結果 — Issue #134 の修正

## 上流の成果物

- `operation/deployment-pipeline/cd-config.md`: デプロイ先は main。スモークは main の先端で行う。
- `operation/deployment-pipeline/deployment-strategy.md`: 「デプロイ後の確認（スモーク）」の 2 点をここで確かめる。
- `construction/build-and-test/test-results.md`（`build-test-results`）: ローカルでは全体テストとカバレッジが macOS の SIGKILL で確定しなかった。その分は CI の結果で補う。
- `environment-inventory` は、bugfix scope では Environment Provisioning を実行しないので存在しない（scope の設計どおり）。

## 実施した確認

main の先端 `065ab78f`（PR #154 の squash コミット）を、本体の作業ツリーとは別のチェックアウト（detached）に取り出して実行した。本体の作業ツリーには、スモーク用のローカル変更があるため使っていない。

| # | 確認 | コマンド・証拠 | 結果 |
| --- | --- | --- | --- |
| 1 | 再現テストが main の先端で緑 | `cargo test -p core-read-model-updater --lib orchestration::journal_reader_impl::tests::replace_pipeline_waits_for_a_write_lock_held_by_another_connection -- --exact` | 1 passed（0.28 秒） |
| 2 | RMU クレートの単体テスト全体が緑 | `cargo test -p core-read-model-updater --lib` | 343 passed / 0 failed |
| 3 | main に入った内容の CI が緑 | merge queue の `merge_group` run 36016133595（マージされた内容そのもの） | 全ジョブ success |

## 判定

スモークは合格。修正（`replace_pipeline` を IMMEDIATE で始める）と、ロック待ちの観測を確かめる再現テストが main で動いている。
