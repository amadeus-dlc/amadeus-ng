# code-generation-questions — U9 正本・仕様の canon 追従（`u9-canon-docs`）

> Code Generation（Construction 3.5）の質問票（Unit: U9、Bolt: B4、規模 S）。出典: `code-generation-plan.md`、`unit-test-instructions.md`、
> `../functional-design/*.md`（pending-revision 含む）、`../nfr-requirements/*.md`、`../nfr-design/*.md`、`../../../inception/delivery-planning/bolt-plan.md`（B4）。
>
> **質問なし。** ブランチ / PR / 記録コミットの運用は B1〜B3 で確定済み（`origin/main` から Bolt ブランチ（作成済み）、記録コミット → 文書コミット、
> PR は 1 本直列、squash-merge、レビューボット全件対応）。前提 P1〜P3 を確認のうえ、計画承認（Plan Approval）を求める。

## 前提（確認事項）

- P1. 本 Unit はコードを書かない（`modules` / `tools` / `scripts` / `.github` / `Cargo.*` / `docs/specs/research` の diff ゼロ）。TDD の赤→緑は
  「受入検査（sentinel grep / diff / README 行数 / 表整形）を先に走らせて赤を記録し、改訂で緑にする」と読み替える。
- P2. 機能設計・NFR 要求・NFR 設計の pending-revision（BR2.5 の 12 号 5 箇所、BR1.5 §1b 再構成、BR5.1 の grep 範囲と sentinel 7 語、diff スコープ
  Cargo.* まで、StageGraphReader の扱い）は**計画に取り込んで**実施する（正本の本文はステージゲートの Request Changes で同期）。
- P3. 委任は 2 本（委任 1 = coding-rules / components.md / deviations.md、委任 2 = 仕様 01 / 10 / 11 / 12 号）、所有ファイルが重ならないため並行。
  モデルは両方 Opus。開発エージェントは計画・検査手順・本質問票を書き換えず、`developer-report-<n>.md` に報告する。仕様の規範名は U2 の承認済み
  改名（`WorkflowExecutionSnapshot` → `WorkflowExecutionState`、B5 で改名）を採り、現行コード名を括弧で注記する。

## Plan Approval

`code-generation-plan.md`（埋め込みの Testing Contract を含む）と `unit-test-instructions.md` を確認し、文書改訂に進んでよいか。

[Approval Fingerprint]: sha256:819fec3a8e7f27641e799263135410c49c4a0de2261c53ea4d32ad5813e61c23

- Approve Plan — 計画どおり文書改訂に進む
- Request Changes — 計画を修正する

[Answer]: Approve Plan
