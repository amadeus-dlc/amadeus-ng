<!-- INVARIANT: examples are single-line HTML comments so a fresh template parses to total=0 (MEMORY_EMPTY). Do NOT un-comment or split across lines. t100 guards this. -->
> This file is kept up to date automatically while the stage runs. Add observations at the review step, not by editing here directly.

## Interpretations
<!-- example: 2026-05-29T10:14:32Z — chose REST over GraphQL; the consuming team only needs CRUD, revisit if subscriptions land -->
- 2026-08-22T11:32:08Z — unit-major を選んだ（Q7 = A）ため `aidlc-state.ts set-construction-iteration unit-major` を記録した; Bolt = PR 直列運用と動くコードの早期確保に合致。infrastructure-design は SKIP 済みなので per-unit 設計ステージは functional-design / nfr-requirements / nfr-design / code-generation の 4 つ
- 2026-08-22T11:32:08Z — Q2 の回答『quintは使いたい』は WSJF（順序の点数モデル）への回答ではなく Quint（形式検証）維持の意思表示と解釈し、Q2a で Bolt 計画上の位置づけ（毎 PR ゲート維持 + U2/U3 でモデル改訂同梱）として確定した; 質問文の『形式的なスコアリングモデル』が『形式検証』と読めてしまう曖昧さがあった

## Deviations
<!-- example: 2026-05-29T10:14:32Z — skipped the optional caching layer the stage prose suggested; the dataset is small enough that it adds risk -->

## Tradeoffs
<!-- example: 2026-05-29T10:14:32Z — picked TDD over BDD this run; the team is unit-first and the domain is well-understood -->
- 2026-08-22T11:32:08Z — 根 4 Unit の順を U1 → U2 → U9 → U10 にした; U10（CI 硬化）を最初に置く案と比べ、ES 化の学び（心配 A）と互換オラクル（心配 B）を優先した。U10 は main.rs を触る B9 より前（B6）に置いて FR9.5 の除外設定が先に入るようにした

## Open questions
<!-- example: 2026-05-29T10:14:32Z — confirm the retention window with compliance before the next stage hardens the schema -->
- 2026-08-22T11:32:08Z — L 規模 Bolt（B2/B4/B8/B9）が 1 PR に収まらないときの扱い（Unit 再分割 vs 同 Bolt 内 2 PR の例外許可）は Bolt 着手時のオーナー裁定に委ねた（risk R1）
