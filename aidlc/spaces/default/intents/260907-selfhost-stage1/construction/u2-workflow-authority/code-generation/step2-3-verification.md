# Step 2–3の完了確認

## 判定と範囲

2026-09-09、ユーザーが現計画での続行を選択した後、過去の成功記録だけで判断せず、現在のコードで報告・保存・投影・結果取得・障害回復の対象テストを実行した。Step 2–3の完了条件を満たすため、計画の該当チェックを完了へ更新した。

この判定は報告事実と結果取得の仕組みに対するもの。開始時や工程完了時の追加監査情報、主要フック、補助更新、2.7.1との残る観測差はStep 4–8、全体の最終回帰・品質ゲートはStep 9に残る。U2全体を完了とは扱わない。

## 確認した契約

- 更新ユースケースの成功は `Result<(), CommitError>`。旧 `CommitOutcome` は残っていない。
- 成功する遷移とno-opを単一のReportedとして保存し、呼出側のReportIdを保持する。
- 拒否では報告成功を保存せず、競合は1回だけ再試行する。再試行は最初の対象を保持する。
- ReportIdによる結果取得は、別報告や最新結果へ置き換えない。不在・壊れたschemaはエラーにする。
- 投影位置と結果行の整合、保存後に投影だけ失敗した場合の回復、反復公開の監査重複防止を確認した。

## 検証結果

| コマンド | 結果 |
| --- | --- |
| `cargo test -p core-command-use-case --lib orchestration::commit_verdict_use_case::tests::` | 32件成功 |
| `cargo test -p core-command-interface-adapter --test report_result_contract --test commit_verdict_use_case_wiring_test` | 1件＋1件成功 |
| `cargo test -p core-read-model-updater --test report_result_projection_contract` | 5件成功 |
| `cargo test -p core-query-interface-adapter --test report_result_dao_contract` | 5件成功 |
| `cargo test -p core-read-model-updater --test publication_recovery_contract` | 33件成功 |
| `cargo test -p aidlc --test intent_lifecycle` | 118件成功 |
| `cargo clippy -p aidlc --test intent_lifecycle -- -D warnings` | 終了0 |
| 対象ファイルのrustfmt・git diff検査 | 終了0 |

## CLIテストの古い前提の是正

初回のCLI結合検査は116成功・2失敗だった。製品を期待に合わせて戻さず、次の古い前提を実装済み契約に合わせた。

1. 復旧用のnextも指示発行の事実を保存する。全ジャーナル件数が不変という旧期待を、次の呼出しごとにDirectiveIssuedだけが1件増える検査へ変更した。Reportedや遷移を再保存しないこと、公開監査の重複がないこと、状態が不変なことは維持・明示した。保存イベントの種類と件数、投影位置をそれぞれ検査している。
2. decision/answerは接続済みなので、引数なし呼出しは必須stageの不足を拒否する。旧「未配線」の期待を変更した。linkはStep 5の未完了項目であり、現時点の拒否検査を維持している。

変更したソースは `modules/app/aidlc/tests/intent_lifecycle.rs` のみ。製品コードはこの確認作業では変更していない。ログは `test-evidence/step2-3-*.txt`。
