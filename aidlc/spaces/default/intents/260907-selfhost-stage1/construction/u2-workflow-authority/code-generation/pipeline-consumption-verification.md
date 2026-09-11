# pipeline受領の消費側の検証

## 対象と現在地

承認済み[U2計画](code-generation-plan.md)のStep 5について、通常の単一root repositoryのpipeline受領を検証した。固定比較元は本家2.7.1、`a277af218f0df7f325d3b8be7b6d90fce2c5bd40`。

[以前の棚卸し](link-verification.md)に残っていた「nextのcompletedが空」「reportのpipeline完了前提が未接続」は、今回の作業開始時のディスクでは既に接続されていた。既存実装を重複して書き換えず、追加済み11契約を実行した後、SQLite投影失敗と復旧の契約を1件追加した。今回、新たな製品実装は加えていない。

## 確認した経路

- `PipelineHistory` が現在の試行境界・受領順序・handoffの内容とmtimeを照合する。
- `PipelineTables` がその判断を通常/単独実行別の行へ投影し、`JournalReaderImpl::replace_pipeline` が実行単位でSQLiteへ保存する。
- `PipelineProgressDaoImpl` と `PipelineProgressUseCase` が、実行ID・stage・singleの複合条件で保存済み完了列を読む。`turn.rs::draw_run_stage` が `PipelineDirective.completed` へ描画する。Queryは受領を判断し直さない。
- `IntentExecution::require_pipeline_for_report` を `CommitVerdictUseCase` が呼び、承認待ち・承認・修正完了に必要な現試行の全linkを検査する。`require_pipeline_single` は単独試行の開始境界と専用受領を検査する。
- handoffの変更は新しいlinkイベントがなくても再投影される。承認待ちへ入った後のhandoff変更も、再報告・承認で古い受領を使えない。

本家の `checkPipelineLinkEvidence` / `verifyPipelineLinkPrecondition` と、固定採取済みの `pipeline-progress-human-revision.json` 14観測を照合した。比較テストはnextのpipelineフィールドとreportのstdout/stderr/exitを対象とする。この14観測をnext全体や状態・監査全文の一致と読み替えない。

## 今回追加した検査

`modules/app/aidlc/tests/pipeline_link_contract.rs` の `a_failed_pipeline_projection_preserves_the_prior_query_result_until_recovery`。

1. 公開CLIでdeveloper受領を保存し、通常RMUの投影をQuery APIから読む。
2. 別の実行ID・stageは不在、単独実行の完了列は空であることを確認する。
3. 一時SQLite内に失敗を注入するtriggerを置き、handoffを変更してnextを実行する。
4. nextがerrorを返し、投影のDELETE/INSERTがロールバックされてQueryから以前の行を読めることを確認する。
5. triggerを除去して再実行し、失効した完了列が空として保存・取得されることを確認する。

Repository・RMU・Queryは本番実装を使う。障害注入と合成人間入力は一時workspace内のみである。実作業の承認・受領・状態は更新していない。

## 検査証跡と限界

- [契約検査ログ](pipeline-consumption-logs/contract.log): `cargo test -p aidlc --test pipeline_link_contract`、12件成功・失敗0、終了0。
- [Clippyログ](pipeline-consumption-logs/clippy.log): `cargo clippy -p aidlc --test pipeline_link_contract -- -D warnings`、終了0。
- [独自lintログ](pipeline-consumption-logs/lint.log): `cargo lint`、終了0（正常時は出力なし）。
- 追加ファイルをrustfmtで整形し、対象差分の空白エラーを確認した。

今回の新規テストは既存実装に対する追加検証として初回から成功した。Red→Greenを新たに実施した記録ではなく、前回の消費側実装のRed証跡をこのログで補ったとは扱わない。開始時に既に存在した製品差分の著者やTDDの順序も、この検証だけでは証明しない。

複数repo、review受領の内容・ソースfingerprint結合、全workspace検査、90%床と相対条件、release実行、実際のClaude一周は、この限定検証の完了条件に含めていない。Step 5全体・U2全体の完了を宣言しない。

次はこの検証を統合検査へ含めることを推奨する。比較対象を広げる場合は、残るreview受領の内容・ソース結合を別途検証する。
