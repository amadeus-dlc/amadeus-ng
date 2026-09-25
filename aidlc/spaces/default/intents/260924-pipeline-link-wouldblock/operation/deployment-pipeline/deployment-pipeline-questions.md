# Deployment Pipeline — 確認したいこと

この修正を main へ届ける経路を決めるための質問です。パイプライン自体（`.github/workflows/ci.yml`）は変えません。

すでに決まっていて、ここでは尋ねないこと:

- ブランチ戦略は trunk-based。短命の作業ブランチから main へ squash でマージする（`org.md` の Way of Working）。
- CI は `pull_request` と `merge_group` で走り、merge queue の必須チェックは `CI Success`。
- リポジトリの設定では auto-merge が無効（`allow_auto_merge: false`）。そのため、merge queue へは人が入れる。

## Question 1: PR に含めるもの

今回の作業では、コードのほかに、ワークフローの記録（`aidlc/spaces/default/intents/260924-pipeline-link-wouldblock/`）とコード知識ベース（`aidlc/spaces/default/codekb/amadeus-ng/`）ができています。今の main は、`aidlc/spaces/default/` のうち `knowledge/` と `memory/` だけを管理していて、`intents/` と `codekb/` は 1 つも入っていません。PR に何を含めますか？

A. コードの変更だけ（`journal_reader_impl.rs` の 1 ファイル）。記録とコード知識ベースはローカルに残す
B. コードの変更に加え、この intent の記録とコード知識ベースも含める（stage-1 スモークの証拠として main に残す）
C. コードの変更だけの PR と、記録・コード知識ベースだけの PR の 2 本に分ける
X. Other (please specify)

[Answer]: A

## Question 2: PR を main に入れるまでの流れ

A. 私が作業ブランチを作って PR を出し、CI の緑を確かめる。merge queue へ入れるのはオーナー
B. A に加え、CI が緑になったら私が merge queue へ入れる（`gh pr merge --squash`）
X. Other (please specify)

[Answer]: X. CI greenならマージいいよ。タイミング任せる

（2026-09-24 に利用者の指示で A から改めた。進行役が PR の CI が緑であることを確かめたうえで、自分の判断したタイミングで merge queue へ入れる。）

## Question 3: 改訂 1 の届け方（差し戻し後の追加）

1 回目のコミット `1ec94b6f` は PR #154 に載っており、CI は全ジョブ緑、レビューの指摘が 1 件残っています。改訂 1（再現テストの同期を強めた変更）をどう届けますか？

A. 同じブランチに追加のコミットとして積んで push する（履歴を書き換えない。main へは squash で 1 コミットになる）
B. 1 回目のコミットに畳み込み、force push する
X. Other (please specify)

[Answer]: A

## Consolidated Summary Confirmation

- PR に含めるもの（Q1）: コードの変更だけ（`modules/core/read-model-updater/src/orchestration/journal_reader_impl.rs` の 1 ファイル）。ワークフローの記録とコード知識ベースはローカルに残す。
- main に入れるまでの流れ（Q2）: 私が PR を出し、CI の緑を確かめたうえで、私の判断したタイミングで merge queue へ入れる（利用者の指示「CI greenならマージいいよ。タイミング任せる」）。
- 改訂 1 の届け方（Q3）: 同じブランチ `fix/134-replace-pipeline-immediate` に追加のコミットとして積んで push する。履歴は書き換えない（main へは squash で 1 コミットになる）。
- 決定済みの前提: trunk-based、main へ squash、CI は `pull_request` と `merge_group`、パイプライン自体は変えない。

Does this all look correct before I generate the artifact?

- Looks correct
- Request changes

[Answer]: Looks correct
