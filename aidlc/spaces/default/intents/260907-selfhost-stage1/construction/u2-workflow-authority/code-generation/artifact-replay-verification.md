# 成果物監査の再構成の是正

## 検出した問題と修正

`ArtifactAudit::replay` が更新コマンドの `record` を呼び、そこで生成したイベントを捨てていた。プロジェクト規則 `aggregate-commands.md` の「再構成はイベントを作らない」「replay/applyはResultを返さない」に反していた。

保存済みイベントを直接反映する `apply_event` へ状態反映を集めた。通常の `record` は拒否条件を確認して1イベントを生成し、そのイベントを同じapplyへ渡す。`replay` は新イベントを生成せず、保存された値を順にapplyしてSelfを返す。壊れた履歴は規則どおりpanicし、各APIへ条件を記載した。唯一のRepository呼出側も、保存済みversionを保持して新しい戻り値を受け取る形へ合わせた。

## TDDと契約検証

- 通番の飛び、別集約のイベント、同じ集約IDを名乗る別対象の観測、の3ケースは旧実装でpanicせずRedとなった。
- 是正後に3ケースが成功。通常の保存・再構成、空の差分、保存された値とversionの保持、通番上限での拒否と状態不変を含む7件が成功した。
- 実SQLiteのRepository検査では、スナップショットだけを古い時点へ戻し、保存済みの後続イベントから最新の全状態を復元した。versionとジャーナルを保持し、読取りで履歴が増えないことを確認した。

## 最終検証

| コマンド | 結果 |
| --- | --- |
| `cargo test -p core-command-domain --test artifact_audit_contract` | 7件成功 |
| `cargo test -p core-command-interface-adapter --test artifact_audit_repository_contract` | 差分再構成を含む1件成功 |
| `cargo test -p aidlc --test claude_hook_contract` | 10件成功 |
| `cargo clippy -p core-command-interface-adapter --all-targets -- -D warnings` | 終了0 |
| 拡充後のRepositoryテスト対象Clippy | 終了0 |
| 対象のrustfmt・git diff検査 | 終了0 |

ユーザーの「tools/lint定期的に実行せよ」に従い、実装と検証の区切りで `cargo lint` を実行した。`.cargo/config.toml` のこの別名は `tools/lint/Cargo.toml` を起動する。今回の定期実行01〜04はすべて終了0で、違反はなかった。以降も各修正の区切りで実行する。

ログは `test-evidence/artifact-replay-*.txt` と `test-evidence/periodic-lint-*.txt`。

## 変更ファイル

- `modules/core/command/domain/src/workspace/artifact_audit.rs`
- `modules/core/command/domain/tests/artifact_audit_contract.rs`
- `modules/core/command/interface-adapter/src/orchestration/artifact_audit_repository_impl.rs`
- `modules/core/command/interface-adapter/tests/artifact_audit_repository_contract.rs`
