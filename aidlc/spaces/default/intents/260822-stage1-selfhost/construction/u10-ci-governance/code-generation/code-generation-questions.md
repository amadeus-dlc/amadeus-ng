# code-generation-questions — U10 CI ガバナンス（`u10-ci-governance`）

> Code Generation（Construction 3.5）の質問票（Unit: U10、kind: packaging、Bolt: B2）。出典: `../nfr-design/security-design.md`、
> `../nfr-requirements/{security-requirements,tech-stack-decisions}.md`、`../../../inception/delivery-planning/bolt-plan.md`
> （B2 = U10、2026-08-23 改訂）、`aidlc/spaces/default/memory/team.md`（PR 直列運用、Bolt = PR、squash-merge）、実地確認
> （PR #24 は CI 3 ジョブ緑・mergeStateStatus CLEAN・未マージ）。

## 以前の質問（2026-08-22の記録）

Q1のブランチ作成タイミングは履歴として保持する。現在は既存の実装を確認し記録を是正する再作業であり、旧Boltブランチの作成・
ruleset適用・PR作成を再実行しない。今回の対象は末尾のPlan Approvalに示す。

### Q1. Bolt B2 のブランチを切るタイミング

PR #24（Bolt B1）は CI 緑でマージ可能な状態ですが未マージです。PR は直列運用のため、B2 のブランチは `main` に B1 が
入ってから切るのが素直です（stack すると squash-merge 後に rebase が要る）。

- A. **#24 をマージしてから** `main` から `bolt/b2-u10-ci-governance` を切る。計画承認は今行い、実装の委任はマージ後に開始 — 推奨
- B. いま `bolt/b1-u1-canon-json-goldens` の上に B2 ブランチを積み（stack）、#24 マージ後に `main` へ rebase する
- X. Other (please specify)

[Answer]: A

## Plan Approval

2026-09-06の対象: `code-generation-plan.md` のStep 1〜6と、そのTesting Contract、および `unit-test-instructions.md`。

ワークスペースのCI設定は変更しない。現行設定を改訂済み要件・設計へ照合し、Unit限定コマンド（検査20項目・`tools/lint` 自己テスト）と受入の
実測（カバレッジ2回測定・`cargo audit` 2件・unsafe不適合例の拒否）を実行して記録する。`code-summary.md` を現行の事実で書き直し（旧版は
履歴として保存済み）、`traceability.json` の15件を実在ファイルのパス単体へ対応付け、`source-manifest.json`（変更パスなし）を作る。
不一致が判明した場合は、検査項目を先に追加する変更案を返して計画を改訂する。

計画準備時の `verify-ci-governance.sh --with-ruleset` は20項目成功であった。これは全CI実行・キュー完走・依存監査・カバレッジ再測定の
成功を意味しない。

[Approval Fingerprint]: sha256:73fb6047d771f21ad6fa75a7cb9179c25d20dd34e637e9e3e0a03a60a4defe45

- Approve Plan — この計画で実コード生成（検証と記録の是正）に進む
- Request Changes — 計画・テスト手順を修正する

[Answer]: Approve Plan
