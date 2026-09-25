<!-- INVARIANT: examples are single-line HTML comments so a fresh template parses to total=0 (MEMORY_EMPTY). Do NOT un-comment or split across lines. t100 guards this. -->
> This file is kept up to date automatically while the stage runs. Add observations at the review step, not by editing here directly.

## Interpretations
<!-- example: 2026-05-29T10:14:32Z — chose REST over GraphQL; the consuming team only needs CRUD, revisit if subscriptions land -->

## Deviations
- 2026-09-24T07:19:42Z — PR #154 のレビューで、再現テストのホルダが主スレッドの呼び出し前に解放しうる（修正前でも通ってしまう）と指摘され、人の判断で差し戻した; 解放のタイマーを「呼ぶ直前」の合図から数え、所要時間が待ちを含むことも確かめる形に改める。ロック取得の試行そのものを通知する口は本番コードに無く、テストのためだけに足すのは規則に反するため、所要時間で観測する。
<!-- example: 2026-05-29T10:14:32Z — skipped the optional caching layer the stage prose suggested; the dataset is small enough that it adds risk -->

## Tradeoffs
- 2026-09-24T01:25:12Z — Testing Contract は test-after（org 既定）だが、要件 FR3.1 は「修正前に失敗すること」の証拠を求める; 順序は契約どおり実装→テストにし、テストを書いた後に修正を一時的に戻して赤を記録する検証手順を計画に入れる方針にした。TDD に読み替えると契約から外れるため。
<!-- example: 2026-05-29T10:14:32Z — picked TDD over BDD this run; the team is unit-first and the domain is well-understood -->

## Open questions
<!-- example: 2026-05-29T10:14:32Z — confirm the retention window with compliance before the next stage hardens the schema -->
