# 成果物保存監査の連続更新・再投影

2026-09-09。Step 7のSessionAudit実装中に見つけた懸念を、親担当が既存の公開フックテストで再検証した。Step 6の完了後に検出した追加不具合である。

## 再現した問題

既存テストはWrite→Editの監査に作成行・更新行が「含まれる」ことだけを確認して成功していた。作成行の件数を1件とする検査を加えると、実際は2件でRedになった。

監査を差分ではなく過去全履歴から描いていた。さらに、構造化投影は同じ集約IDの全観測を個別行に変換し、同じ主キーへINSERTしていた。監査ファイルに更新行が見えても、構造化投影の確定に失敗し、Queryが古い値のままになる経路があった。

## 修正

- 監査へ追加するArtifactSavedを、公開チェックポイントより新しい事実に限定した。
- 構造化行は `ArtifactAuditRow::project` で対象ごとにArtifactAuditを再構成し、集約の最新観測から一行を作る。投影側に別の状態判断を作らない。先頭の欠落をMissingGenesisで拒否し、途中の通番矛盾は集約の再構成契約に従う。
- auditだけの更新では既存行を保持して更新し、古いas_ofによる巻戻しを拒否する。ほかの回答・進行行は消さない。
- 既存recordとnative開始recordの両方で、Write→Edit後の監査件数、最新のQuery、監査失敗記録がないことを検査した。

## 検証結果

- 修正前の既存公開テストは成功。件数の厳密な検査で作成行2対1のRedを取得し、修正後に成功した。
- 履歴先頭の欠落を拒否するテストもRed→Green。テスト記述時のString型違いと不正なfixtureディレクトリ名は、対象不具合のRedに数えていない。
- 公開Claudeフック10件、構造化投影41件、公開障害回復33件が成功。
- SQLの再投影・後続更新・旧as_of再投影を順に実行する1件が成功。最新値、行数1、無関係な回答行の保持を確認した。
- 定期cargo lintは24回目まで成功。全体Clippyの最終結果は、並行作業の検査が揃った時点で追記する。

証跡は `test-evidence/artifact-repeat-*.log`、`artifact-history-red.log`、`artifact-read-tables.log`、`artifact-hook-regression.log`、`artifact-recovery-regression.log`。変更は `read_tables.rs`、`read_tables/artifact_audit_row.rs`、`read_tables/sql.rs`、`orchestration/read_model_updater.rs`、公開フックテストと投影テストである。

以前の `step6-source-checkpoint.json` は、追加不具合の是正前に採った断面として保持する。本書の修正後コードまでその旧SHAで確認済みとは扱わない。U2最終source-manifestは全実装終了後に確定する。
