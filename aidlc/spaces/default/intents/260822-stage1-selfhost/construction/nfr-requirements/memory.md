<!-- INVARIANT: examples are single-line HTML comments so a fresh template parses to total=0 (MEMORY_EMPTY). Do NOT un-comment or split across lines. t100 guards this. -->
> This file is kept up to date automatically while the stage runs. Add observations at the review step, not by editing here directly.

## Interpretations
<!-- example: 2026-05-29T10:14:32Z — chose REST over GraphQL; the consuming team only needs CRUD, revisit if subscriptions land -->
- 2026-08-22T11:48:53Z — [u1-canon-json-goldens] 質問ゼロで要約確認だけを取った; U1 は純粋ライブラリで適用 NFR（NFR1/NFR2/NFR4）の数値・方針は先行ステージと ADR 0001 で確定済みのため、構築フェーズの『質問は例外』方針に従い前提 4 点（技術選定・セキュリティ・品質・性能）の確認に置き換えた。kind = library のため成果物は security-requirements / tech-stack-decisions / traceability の 3 つ

## Deviations
<!-- example: 2026-05-29T10:14:32Z — skipped the optional caching layer the stage prose suggested; the dataset is small enough that it adds risk -->

## Tradeoffs
<!-- example: 2026-05-29T10:14:32Z — picked TDD over BDD this run; the team is unit-first and the domain is well-understood -->

## Open questions
<!-- example: 2026-05-29T10:14:32Z — confirm the retention window with compliance before the next stage hardens the schema -->
- 2026-08-22T11:55:53Z — [u1-canon-json-goldens] レビュー Minor 2 件（再帰深さ上限 128 の upstream 互換影響 — 契約 JSON の実測最大深さの棚卸しか意図的非互換の明示 / STRIDE の Repudiation 行の『該当なし』と来歴記述の食い違い）は終端受領後のため未反映; nfr-requirements のステージゲート（unit-major 末尾）で提示し、code-generation の計画で深さの棚卸しを吸収する
- 2026-08-22T11:53:15Z — [u1-canon-json-goldens] unit-major では Current Stage が functional-design のままなので、PostToolUse のセンサーが nfr-requirements の成果物を functional-design の consumes / BR 契約で評価して SENSOR_FAILED（upstream-coverage 2+4 件、traceability 54 件）を出した; 成果物側の欠陥ではなくフックのステージ解決の限界（directive.stage ではなく Current Stage を見る）。upstream 報告候補。本ステージでは誤検知として扱う
