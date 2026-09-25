<!-- INVARIANT: examples are single-line HTML comments so a fresh template parses to total=0 (MEMORY_EMPTY). Do NOT un-comment or split across lines. t100 guards this. -->
> This file is kept up to date automatically while the stage runs. Add observations at the review step, not by editing here directly.

## Interpretations
- 2026-09-24T13:47:01Z — 作業中に届いた利用者の発言「CI greenならマージいいよ。タイミング任せる」を、Q2（merge queue へ入れる主体）の回答の変更と解釈した; Q2 を A から X に改め、要約の確認を取り直した。merge queue への投入は PR #154 に限った事前承認として扱い、戻しの PR には広げていない。
<!-- example: 2026-05-29T10:14:32Z — chose REST over GraphQL; the consuming team only needs CRUD, revisit if subscriptions land -->

## Deviations
- 2026-09-24T03:34:41Z — CD の新設・変更が要らない bugfix だが、スモーク手順書の 6 ゲート条件を満たすため、人の判断で実行した; 中身は既存の PR→CI→merge queue→main の経路と戻し方の記録にとどめた。
<!-- example: 2026-05-29T10:14:32Z — skipped the optional caching layer the stage prose suggested; the dataset is small enough that it adds risk -->

## Tradeoffs
<!-- example: 2026-05-29T10:14:32Z — picked TDD over BDD this run; the team is unit-first and the domain is well-understood -->

## Open questions
<!-- example: 2026-05-29T10:14:32Z — confirm the retention window with compliance before the next stage hardens the schema -->
