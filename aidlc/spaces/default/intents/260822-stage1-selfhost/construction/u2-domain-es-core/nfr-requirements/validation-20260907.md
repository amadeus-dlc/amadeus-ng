# U2 NFR 要求改訂の検証記録（2026-09-07 再走）

2026-09-07（UTC 2026-09-06 深夜）、利用者の要約確認（P7〜P12、Looks correct）後に security-requirements.md・tech-stack-decisions.md・
traceability.json を改訂した。旧 READY レビュー節（2026-08-23）は `security-requirements-review-history-2026-08-23.md` へ退避した。

## 実行結果

| 検査 | 結果 | 確認範囲 |
|---|---|---|
| `PROPTEST_RNG_SEED=20260823 cargo llvm-cov --package core-command-domain --summary-only` | 終了コード 0。TOTAL: リージョン 98.69%（21718 中 285 未到達）、関数 98.20%（1773 中 32）、行 98.66%（13823 中 185） | ドメインクレート単独の基準値（NFR2.3）。ワークスペース全体の床 90% とは別の参考値 |
| `aidlc-sensor-required-sections.ts --stage nfr-requirements`（security-requirements.md） | pass、H2 5 本、所見 0 | 必須見出し |
| `aidlc-sensor-required-sections.ts --stage nfr-requirements`（tech-stack-decisions.md） | pass、H2 3 本、所見 0 | 必須見出し |
| `aidlc-sensor-traceability.ts --stage nfr-requirements` | pass、gaps / orphans / missing / invalid すべて空 | NFR1〜NFR5 の被覆と派生 ID |
| `git diff --check`（nfr-requirements 配下） | PASS | 空白エラー |
| 依存ベースライン（`modules/core/command/domain/Cargo.toml` 実測） | runtime = chrono / uuid（v7）/ core-infrastructure、dev = proptest / serde_json | NFR4.1 の再ベースライン |
| workspace lints（`Cargo.toml` `[workspace.lints]` 実測） | rust 5 + rustdoc 1 + clippy 44 = 50 | NFR2.4（旧 48 は失効） |
| 時計・乱数の利用箇所（`src/` の grep） | `uuid::Uuid::now_v7` は `*EventId::generate` のみ。`std::time` / `std::env` / `rand` の利用なし | NFR3.1 |
| `resolve_review_policy` の不一致エラー名（`intent.rs` 実測） | `IntentReviewError::DefinitionMismatch` | NFR3.4。質問票 P10 の括弧書き「LineageMismatch」は誤記であり、成果物は実装名で記載した（質問票は確認済みバイトのため訂正せず、ここに記録する） |

## 検証の限界

CI 全体の実行、`cargo audit`、Quint ゲート、ITF 準拠テストの再実行は本改訂では行っていない（U10 の 2026-09-06 実測を引用）。
FCC 化・`next_decision` の Result 化は設計であり、現行コードには未反映（functional-spec §9 の引継ぎ）。本要求書の合格基準は
U2 の code-generation 再走で実測する。機能設計の advisory レビュー所見 R-01〜R-03 は凍結中の設計本文に対するもので、
NFR2.5 と tech-stack-decisions §3 の未決に引き継いだ。
