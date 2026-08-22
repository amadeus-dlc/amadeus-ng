<!-- INVARIANT: examples are single-line HTML comments so a fresh template parses to total=0 (MEMORY_EMPTY). Do NOT un-comment or split across lines. t100 guards this. -->
> This file is kept up to date automatically while the stage runs. Add observations at the review step, not by editing here directly.

## Interpretations
<!-- example: 2026-05-29T10:14:32Z — chose REST over GraphQL; the consuming team only needs CRUD, revisit if subscriptions land -->
- 2026-08-22T11:56:32Z — [u1-canon-json-goldens] 質問ゼロで前提 3 点（セキュリティ設計・論理コンポーネント・障害ドメイン）の確認のみ; kind = library で performance/scalability/reliability/observability の要求が無く、成果物は security-design / logical-components / traceability。NFR 要求レビューの Minor 1（深さ上限 128 の互換影響）は P1 に『実測深さの棚卸し』として引き取った

## Deviations
<!-- example: 2026-05-29T10:14:32Z — skipped the optional caching layer the stage prose suggested; the dataset is small enough that it adds risk -->

## Tradeoffs
<!-- example: 2026-05-29T10:14:32Z — picked TDD over BDD this run; the team is unit-first and the domain is well-understood -->

## Open questions
<!-- example: 2026-05-29T10:14:32Z — confirm the retention window with compliance before the next stage hardens the schema -->

### U1 レビュー結果（2026-08-22）

- aidlc-architecture-reviewer-agent、iteration 1、advisory: **READY**（Critical 0 / Major 0 / Minor 2）。
- Minor 1: `inception/domain-design/components.md` の CanonJson `external_dependencies: []` が
  sha2 / serde / serde_json の実依存と食い違う — code-generation 計画の棚卸し項目に追加し、実体化後に
  components.md を更新する（繰り延べ）。
- Minor 2: security-design.md §4 の serde_json 直接呼び出し禁止の記述が「直列化関数」に限定され、
  BR1.7 の「契約経路の `to_value`」への言及が抜けている — レビュー凍結後のため本文は触らず、
  code-generation 計画に禁止範囲（直列化関数 + 契約経路の to_value）を明記して引き取る（繰り延べ）。
- 凍結（review-freeze）後に成果物は変更していない。
- 2026-08-23T01:15:00Z — [Interpretations] U10（packaging）の nfr-design: produces は security-design + traceability の 2 つ（logical-components は produces 外）— 論理コンポーネント（CI ジョブ・スクリプト・ruleset の境界と障害ドメイン）は security-design 内の節に置く。`audit` ジョブを required checks に含めない判断（外部 advisory DB の一時障害で全マージが止まるのを避ける）を前提 P1 として人間確認へ
- 2026-08-23T01:30:00Z — [Interpretations] proptest 1.11.0 のソース（config.rs:40 `PROPTEST_RNG_SEED`、`RngSeed::Fixed(u64)`）で環境変数によるシード固定が実在することを確認 — NFR2.4 の決定化は `scripts/coverage.sh` と CI で `PROPTEST_RNG_SEED` を固定値に設定するだけで足り、テストコードの変更は不要（security-design §4 の第一候補 (a) が成立）
- 2026-08-23T01:45:00Z — [Open questions] U10 nfr-design レビュー READY（Minor 2）: (1) カバレッジ除外 regex の表記が tech-stack-decisions（`^` アンカー + 相対パス基準）と設計で逐語不一致 → code-generation 計画で正本を 1 つに確定（`modules/app/aidlc/src/main\.rs$`、cargo llvm-cov の相対パス基準） (2) ruleset 冪等スクリプトは規則タイプの有無だけでなく required コンテキスト集合の一致で収束判定する → 計画に反映
