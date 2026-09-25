# CD 構成 — Issue #134 の修正を main へ届ける経路

## 前提と上流の成果物

- bugfix scope では CI Pipeline と Infrastructure Design を実行しない。そのため上流の `ci-config`（`construction/ci-pipeline/ci-config.md`）、`quality-gates`（`construction/ci-pipeline/quality-gates.md`）、`infrastructure-specification`・`cicd-pipeline`（`construction/infrastructure-design/`）はどれも存在しない。scope の設計どおりの不在である。
- 代わりに、ワークスペースに実在する構成を根拠にする: `.github/workflows/ci.yml`、`.github/workflows/review-thread-resolution.yml`、リポジトリ設定（GitHub API）。
- **パイプライン自体は変えない。** この文書は、既存の経路にこの修正を載せる方法を記録する。

## 配布物とデプロイ先

- amadeus-ng は CLI（`aidlc` のマルチコールバイナリ）の Rust ワークスペースである。サーバーや常駐プロセスは無い。
- 「デプロイ」は main へのマージを指す。main に入った後の配布（`cargo install`、タグ）はこの修正の射程外。stage-1 の安定タグは手順書 §2-7 の別作業で扱う。

## 経路（既存）

```text
作業ブランチ fix/134-replace-pipeline-immediate
  -> PR（main 宛て）
     -> CI: pull_request イベント（aidlc-distribution / check / quint / coverage / audit / review-thread-resolution / ci-success）
  -> merge queue（CI の緑を確かめてから進行役が入れる）
     -> CI: merge_group イベント（必須チェック CI Success）
  -> main へ squash
```

<!-- Text fallback: 作業ブランチから main 宛ての PR を出すと、pull_request イベントで CI の全ジョブが走る。CI の緑を確かめた進行役が merge queue へ入れると merge_group イベントで CI が再び走り、必須チェック CI Success が通れば main へ squash でマージされる。 -->

## この修正での設定値

| 項目 | 値 | 出典 |
| --- | --- | --- |
| 作業ブランチ | `fix/134-replace-pipeline-immediate`（main から切る） | `org.md` Way of Working（短命の作業ブランチ） |
| PR に含めるもの | `modules/core/read-model-updater/src/orchestration/journal_reader_impl.rs` の 1 ファイルだけ | 質問 Q1 = A |
| 含めないもの | ワークフローの記録（`aidlc/spaces/default/intents/`）、コード知識ベース（`aidlc/spaces/default/codekb/`）、スモーク用のローカル変更（`.claude/settings.json`、`.codex/hooks.json` の削除）、`aidlc/spaces/default/.claudian/`・`.obsidian/` | 質問 Q1 = A、スモークの手順書 |
| マージ方式 | squash | `org.md` Way of Working |
| 改訂 1 の届け方 | PR #154 の同じブランチに追加のコミットとして積んで push する。force push はしない（main へは squash で 1 コミットになる） | 質問 Q3 = A |
| merge queue への投入 | 進行役。PR の必須チェック（`check` / `quint` / `coverage` / `CI Success`）が緑（`mergeStateStatus: CLEAN`）になってから、進行役の判断したタイミングで入れる | 質問 Q2 = X（利用者の指示「CI greenならマージいいよ。タイミング任せる」） |
| 投入の方法 | `gh api graphql` の `enqueuePullRequest(input:{pullRequestId})`。auto-merge がリポジトリ設定で無効（`allow_auto_merge: false`）なので、`gh pr merge` は使えない。投入後は `pullRequest(number:){ state isInMergeQueue }` で `MERGED` まで見届ける | リポジトリ設定、過去の PR（#129〜#133）での実績 |
| 私（進行役）の担当 | 改訂 1 のコミットと push、CI の結果の確認、merge queue への投入、マージの見届けと報告 | 質問 Q2 = X、Q3 = A |

## 品質ゲート（既存の CI が担う）

| ゲート | ジョブ | この修正での見どころ |
| --- | --- | --- |
| 書式・静的検査・lint・全テスト | `check` | ローカルの macOS で起きた子プロセスの SIGKILL が、Linux の CI で出ないこと |
| カバレッジ 90% の床 | `coverage` | NFR3。ローカルでは計測できなかった（`construction/build-and-test/test-results.md`）。#134 の元の症状はこのジョブで出ていた（NFR2） |
| upstream ゴールデン | `aidlc-distribution` | NFR4 |
| 形式モデル・依存監査 | `quint` / `audit` | 変更の影響は無い見込み |
| 集約 | `ci-success` | merge queue の必須チェック |
