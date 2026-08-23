# functional-design-questions — U10 CI ガバナンス（`u10-ci-governance`）

> Functional Design（Construction 3.1）の質問票（Unit: U10、kind: packaging）。出典:
> `../../../inception/units-generation/unit-of-work.md`（U10 — CI 3 ジョブの branch protection 機械強制、`cargo audit`、
> `rust-toolchain.toml`、`unsafe_code` workspace lint 昇格、`permissions: contents: read`、`tools/lint` の CI 3 ステップ、
> coding-rules エラー規則の正本化）、`../../../inception/requirements-analysis/requirements.md`（NFR2 / NFR4）、
> `aidlc/spaces/default/memory/team.md`（Testing Posture / Code Style の確定アクション）。
>
> **質問なし。** U10 は packaging（CI・ガバナンス設定）の Unit で、エンティティ・業務規則・ワークフローを持たない。
> 機能設計の成果物（entities / rules / functional-spec）は kind = packaging には適用されず、ワークフローエンジンも
> 本 Unit の functional-design を「成果物なしで充足」と判定している。設計上の前提だけを確認して次のステージへ進む。

## 前提（確認事項）

- P1. U10 の設計対象は `.github/workflows/ci.yml`・`Cargo.toml`（`[workspace.lints.rust] unsafe_code = "forbid"`）・
  `rust-toolchain.toml`・`scripts/coverage.sh`（composition root 除外、PBT シード固定）・`tools/lint` の CI 組込み・
  branch protection（`gh api`）であり、ドメインモデルや API を持たない。（訂正 2026-08-22 UTC: 当初ここに挙げていた
  「coding-rules のエラーハンドリング規則」は FR9.6 = U9 の責務で U10 の対象外 — PR #25 レビュー指摘の引き取り）
- P2. 機能要件の追跡は FR 側に対象がなく、NFR2（品質ゲート）/ NFR4（サプライチェーン）を nfr-requirements 以降で扱う。

## Consolidated Summary Confirmation

- U10 に機能設計の質問はなし（packaging kind — エンティティ・規則・ワークフローを持たない）
- 設計対象（P1）: CI ワークフロー・workspace lint・ツールチェーン固定・カバレッジ設定・`tools/lint` の CI 組込み・
  branch protection（エラー規則は U9 — 訂正）
- 追跡（P2）: FR 対象なし、NFR2 / NFR4 は nfr-requirements 以降で扱う

Does this all look correct before I generate the artifact?

- Looks correct
- Request changes

[Answer]: Looks correct
