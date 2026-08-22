# code-generation-questions — U10 CI ガバナンス（`u10-ci-governance`）

> Code Generation（Construction 3.5）の質問票（Unit: U10、kind: packaging、Bolt: B2）。出典: `../nfr-design/security-design.md`、
> `../nfr-requirements/{security-requirements,tech-stack-decisions}.md`、`../../../inception/delivery-planning/bolt-plan.md`
> （B2 = U10、2026-08-23 改訂）、`aidlc/spaces/default/memory/team.md`（PR 直列運用、Bolt = PR、squash-merge）、実地確認
> （PR #24 は CI 3 ジョブ緑・mergeStateStatus CLEAN・未マージ）。

## 質問

### Q1. Bolt B2 のブランチを切るタイミング

PR #24（Bolt B1）は CI 緑でマージ可能な状態ですが未マージです。PR は直列運用のため、B2 のブランチは `main` に B1 が
入ってから切るのが素直です（stack すると squash-merge 後に rebase が要る）。

- A. **#24 をマージしてから** `main` から `bolt/b2-u10-ci-governance` を切る。計画承認は今行い、実装の委任はマージ後に開始 — 推奨
- B. いま `bolt/b1-u1-canon-json-goldens` の上に B2 ブランチを積み（stack）、#24 マージ後に `main` へ rebase する
- X. Other (please specify)

[Answer]: A

## Plan Approval

対象: `code-generation-plan.md`（§4 の Step 0〜11、埋め込み Testing Contract `sha256:303d9bb7b5d777d54a6761be9ed154d85d5bb3f2d6b9cce02f71f4ed1b3a4ff3`）と
`unit-test-instructions.md`（`scripts/governance/verify-ci-governance.sh` / `cargo test --manifest-path tools/lint/Cargo.toml` の Unit 限定コマンド）。

[Approval Fingerprint]: sha256:7f0e1353ae14399ae2c8a4f8aa147ebe38a5376db0641c6f90bc1a24414f3c75

- Approve Plan — この計画で実装に進む（委任は Q1 のタイミングで）
- Request Changes — 計画・テスト手順を修正する

[Answer]: Approve Plan
