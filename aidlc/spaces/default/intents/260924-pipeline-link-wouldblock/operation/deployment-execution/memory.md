<!-- INVARIANT: examples are single-line HTML comments so a fresh template parses to total=0 (MEMORY_EMPTY). Do NOT un-comment or split across lines. t100 guards this. -->
> This file is kept up to date automatically while the stage runs. Add observations at the review step, not by editing here directly.

## Interpretations
- 2026-09-24T14:26:37Z — 改訂 1 のコミットは、本体の作業ツリーではなく前回の作業用チェックアウト（修正ブランチの worktree）で作った; 本体の作業ツリーにはスモーク用のローカル変更（`.claude/settings.json`・`.codex/hooks.json`）があり、PR に混ぜないため。2 つのチェックアウトの対象ファイルが同一であることを diff で確かめてから積んだ。
<!-- example: 2026-05-29T10:14:32Z — chose REST over GraphQL; the consuming team only needs CRUD, revisit if subscriptions land -->

## Deviations
<!-- example: 2026-05-29T10:14:32Z — skipped the optional caching layer the stage prose suggested; the dataset is small enough that it adds risk -->

## Tradeoffs
- 2026-09-24T14:26:37Z — push 前の確認は fmt と再現テスト 1 本だけにした; 全体テストとカバレッジは Build and Test でやり直し済みで、macOS ローカルの SIGKILL で確定できないため、PR の CI（Linux）を判定の場にした。
<!-- example: 2026-05-29T10:14:32Z — picked TDD over BDD this run; the team is unit-first and the domain is well-understood -->

## Open questions
<!-- example: 2026-05-29T10:14:32Z — confirm the retention window with compliance before the next stage hardens the schema -->
