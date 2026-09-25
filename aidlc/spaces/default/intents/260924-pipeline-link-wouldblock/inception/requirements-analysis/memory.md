<!-- INVARIANT: examples are single-line HTML comments so a fresh template parses to total=0 (MEMORY_EMPTY). Do NOT un-comment or split across lines. t100 guards this. -->
> This file is kept up to date automatically while the stage runs. Add observations at the review step, not by editing here directly.

## Interpretations
- 2026-09-24T01:18:28Z — Issue #134 本文とコメントを要件の出典として扱った; 期待する振る舞い 3 点と受入条件 2 点がすでに明記されているので、質問は修正方針・射程・再現テストの置き場の 3 点に絞った（Minimal 深度）。upstream には投影の後段が無く、この不具合は native 固有なので観測互換との衝突は無いと判断した。
<!-- example: 2026-05-29T10:14:32Z — chose REST over GraphQL; the consuming team only needs CRUD, revisit if subscriptions land -->

## Deviations
<!-- example: 2026-05-29T10:14:32Z — skipped the optional caching layer the stage prose suggested; the dataset is small enough that it adds risk -->

## Tradeoffs
<!-- example: 2026-05-29T10:14:32Z — picked TDD over BDD this run; the team is unit-first and the domain is well-understood -->

## Open questions
<!-- example: 2026-05-29T10:14:32Z — confirm the retention window with compliance before the next stage hardens the schema -->
