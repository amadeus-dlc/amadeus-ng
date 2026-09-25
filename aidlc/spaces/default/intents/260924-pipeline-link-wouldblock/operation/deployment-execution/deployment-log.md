# デプロイの記録 — Issue #134

上流の成果物: `operation/deployment-pipeline/cd-config.md`（経路と設定値）、`operation/deployment-pipeline/deployment-strategy.md`（昇格と中止の条件）、`construction/build-and-test/test-results.md`（ローカルの結果）。`environment-inventory` は、bugfix scope では Environment Provisioning を実行しないので存在しない（scope の設計どおり）。デプロイ先の環境は無く、main へのマージが「デプロイ」に当たる。

## 1 回目（2026-09-24）

| 手順 | 結果 |
| --- | --- |
| 作業用のチェックアウト（`main` = `5a2b51d3` + 修正 1 ファイル）で作業ブランチを作成 | `fix/134-replace-pipeline-immediate` |
| コミット | `1ec94b6f`（`journal_reader_impl.rs` の 1 ファイルだけ） |
| push と PR 作成 | [#154](https://github.com/amadeus-dlc/amadeus-ng/pull/154)（本文に `Closes #134`、射程外の #151・#152・#153 を参照） |
| PR の CI（run 35954181150、`pull_request`） | 全ジョブ success、再実行なし。`check` 28 分 13 秒、`coverage` 21 分 41 秒（行カバレッジ 97.44%、床 90%）、`aidlc-distribution`・`quint`・`audit`・`CI Success` が success |
| レビューのやり取りのゲート（`Check unresolved comments`） | **失敗**。CodeRabbit の未解決の指摘 1 件（再現テストの同期） |

### レビューの指摘と判断

- **指摘**: 再現テストのホルダは「握った」合図の直後から 200ms を数える。主スレッドが `replace_pipeline` を呼ぶ前に COMMIT してしまうと、修正前の DEFERRED でもテストが通り、修正の効果を見分けられなくなる。
- **判断（人）**: `Fix the test`。Code Generation へ差し戻し、計画を改めて承認したうえで、テストが実際にロック待ちを観測したことを確かめる形に強める。Build and Test と PR の CI もやり直す。
- 差し戻しの回数: 1/3（`construction/build-and-test/test-results.md` の `## Loop-Back Log` とは別に、この段からの人の判断による差し戻し）。

## 2 回目（2026-09-24、改訂 1）

前提: `operation/deployment-pipeline/cd-config.md` の Q2（利用者の指示「CI greenならマージいいよ。タイミング任せる」で、進行役が merge queue へ入れる）と Q3（同じブランチに追加のコミット）。

| 手順 | 結果 |
| --- | --- |
| Code Generation で改訂 1 を承認し、Build and Test をやり直した | 再現テストが、主スレッドが呼ぶ直前の合図から HOLD を数え、所要時間 100ms 以上でロック待ちを観測したことを確かめる形になった |
| push 前の確認（作業用チェックアウト） | `cargo fmt --all -- --check` 緑、再現テスト 1 本 緑 |
| コミットと push | `163f9c06`（`journal_reader_impl.rs` の 1 ファイル、テストだけの変更）を `fix/134-replace-pipeline-immediate` に積んで push。force push はしていない |
| レビューの指摘 | CodeRabbit のスレッドに対応内容を返信し（[返信](https://github.com/amadeus-dlc/amadeus-ng/pull/154#discussion_r4094736564)）、進行役が resolve した（質問 Q3 = A） |
| PR の CI（run 36012811015、`pull_request`） | 必須チェックがすべて success、再実行なし。`check` 25 分 54 秒、`coverage` 24 分 45 秒（行カバレッジ 97.44%、床 90%）、`aidlc-distribution`・`quint`・`audit`・`CI Review Thread Gate`・`CI Success` が success。`Check unresolved comments` も success |
| 取り消しになった実行 | `Review Thread Resolution`（`pull_request_review` など）の 3 件が cancelled。スレッドを resolve したときの重複起動が後の実行に置き換えられたもので、最新の実行（run 36013957981）は success。必須チェックではない |
| merge queue への投入 | 2026-09-24T14:53:29Z、GraphQL `enqueuePullRequest` で進行役が投入（1 番目） |
| merge queue の CI（run 36016133595、`merge_group`） | 全ジョブ success（`check` 約 29 分、`coverage` 約 19 分）。`CI Review Thread Gate` は `pull_request` でだけ走る条件なので skipped |
| main へのマージ | 2026-09-24T15:23:15Z、squash コミット `065ab78f`。Issue #134 は `Closes #134` で自動で閉じた |

NFR2（CI の安定性）: PR の CI も merge queue の CI も、`check` と `coverage` が再実行なしで緑になった。フレークの頻度は低いので、2 回の緑も十分な証拠ではない。その後の main の CI で `projection: read: io: WouldBlock` が再発しないかは、運用の中で見続ける（`deployment-strategy.md`）。
