<!-- INVARIANT: examples are single-line HTML comments so a fresh template parses to total=0 (MEMORY_EMPTY). Do NOT un-comment or split across lines. t100 guards this. -->
> This file is kept up to date automatically while the stage runs. Add observations at the review step, not by editing here directly.

## Interpretations
<!-- example: 2026-05-29T10:14:32Z — chose REST over GraphQL; the consuming team only needs CRUD, revisit if subscriptions land -->
- 2026-08-22T05:23:10Z — Q2（条件3の受入 = 実地1本）と Q4（DoD = 実地スモーク）は同一のスモーク実行に収斂すると解釈し、requirements では1つの DoD に統合; 矛盾ではなく同一検収の二面と判定。
- 2026-08-22T05:23:10Z — 0b の初回質問が「実行時採取」という圧縮語のままで人間に通じず差し戻しを受けた; 本家ツールを実行して正解データを採る作業だと平易に説明してから確定。術語は質問文の中で注釈するという stage 規約（gloss）を自分の質問にも適用すべきだった。

## Deviations
<!-- example: 2026-05-29T10:14:32Z — skipped the optional caching layer the stage prose suggested; the dataset is small enough that it adds risk -->

## Tradeoffs
<!-- example: 2026-05-29T10:14:32Z — picked TDD over BDD this run; the team is unit-first and the domain is well-understood -->
- 2026-08-22T05:23:10Z — 質問を5件に絞った（Standard の下限）; Issue #7 が切替条件・クリティカルパス・スコープ外まで明文化済みで、境界が動く点だけ確認すれば足りると判断。

## Open questions
<!-- example: 2026-05-29T10:14:32Z — confirm the retention window with compliance before the next stage hardens the schema -->
