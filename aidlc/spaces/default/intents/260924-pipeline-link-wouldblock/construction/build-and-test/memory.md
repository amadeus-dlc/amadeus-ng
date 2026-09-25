<!-- INVARIANT: examples are single-line HTML comments so a fresh template parses to total=0 (MEMORY_EMPTY). Do NOT un-comment or split across lines. t100 guards this. -->
> This file is kept up to date automatically while the stage runs. Add observations at the review step, not by editing here directly.

## Interpretations
- 2026-09-24T03:04:29Z — 既存スイートの緑は、本体の作業ツリーではなく HEAD + 修正 1 ファイルだけを載せた作業用チェックアウト（別の CARGO_TARGET_DIR）で確かめることにした; 本体にはスモーク用のコミットしない settings.json 変更があり、フック配線の契約テストがそれを読んでずれるため。本体の target/release/aidlc はフックが使うので触らない。
<!-- example: 2026-05-29T10:14:32Z — chose REST over GraphQL; the consuming team only needs CRUD, revisit if subscriptions land -->

## Deviations
<!-- example: 2026-05-29T10:14:32Z — skipped the optional caching layer the stage prose suggested; the dataset is small enough that it adds risk -->

## Tradeoffs
<!-- example: 2026-05-29T10:14:32Z — picked TDD over BDD this run; the team is unit-first and the domain is well-understood -->

## Open questions
- 2026-09-24T03:18:11Z — macOS ローカルの子プロセス SIGKILL で、全体テストとカバレッジの計測が緑にならなかった; 修正の有無で頻度が同じ（4 回中 1 回）で、落ちるテストも毎回入れ替わる。生成コードに戻して直せる候補が無いので、修正候補の無い版の halt-and-ask にした。
<!-- example: 2026-05-29T10:14:32Z — confirm the retention window with compliance before the next stage hardens the schema -->
