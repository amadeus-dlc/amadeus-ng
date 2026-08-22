<!-- INVARIANT: examples are single-line HTML comments so a fresh template parses to total=0 (MEMORY_EMPTY). Do NOT un-comment or split across lines. t100 guards this. -->
> This file is kept up to date automatically while the stage runs. Add observations at the review step, not by editing here directly.

## Interpretations
<!-- example: 2026-05-29T10:14:32Z — chose REST over GraphQL; the consuming team only needs CRUD, revisit if subscriptions land -->
- 2026-08-22T09:09:34Z — ゲート差し戻し（指摘は是正して）: レビュー所見 Major 1（FR8.1 に ADR-006 指示の正本修正 2 点を同梱）と Minor 1（FR3.3 合格基準の具体化）を反映した; ADR が後続修正の担い手を別 FR に割り当てたときは、要求改訂時にその FR も同時に更新しないと要求単体で矛盾が残る
- 2026-08-22T09:01:16Z — 改訂ラウンド: 既存の `## Review`（product-lead READY, 05:24）は再レビュー前に削除した; 履歴は監査台帳にあり、改訂後の成果物に旧判定を残すと改訂分を覆っているように読めるため
- 2026-08-22T05:23:10Z — Q2（条件3の受入 = 実地1本）と Q4（DoD = 実地スモーク）は同一のスモーク実行に収斂すると解釈し、requirements では1つの DoD に統合; 矛盾ではなく同一検収の二面と判定。
- 2026-08-22T05:23:10Z — 0b の初回質問が「実行時採取」という圧縮語のままで人間に通じず差し戻しを受けた; 本家ツールを実行して正解データを採る作業だと平易に説明してから確定。術語は質問文の中で注釈するという stage 規約（gloss）を自分の質問にも適用すべきだった。

## Deviations
<!-- example: 2026-05-29T10:14:32Z — skipped the optional caching layer the stage prose suggested; the dataset is small enough that it adds risk -->
- 2026-08-22T09:01:16Z — 改訂ラウンド: units-generation からの後方ジャンプで再入し、Q1〜Q5 を再質問せず Q6（改訂範囲）1 問だけ追加して Modify した; domain-design の ADR-001/003/004/007 が FR1.2 の合格基準（ロック区間・audit_lock.qnt 原版）と矛盾していたため。後続設計で要求の合格基準が覆った場合は、設計ステージの ADR に『要求改訂が必要』と書くだけでなく同セッション内で後方ジャンプまで行うべきだった

## Tradeoffs
<!-- example: 2026-05-29T10:14:32Z — picked TDD over BDD this run; the team is unit-first and the domain is well-understood -->
- 2026-08-22T05:23:10Z — 質問を5件に絞った（Standard の下限）; Issue #7 が切替条件・クリティカルパス・スコープ外まで明文化済みで、境界が動く点だけ確認すれば足りると判断。

## Open questions
<!-- example: 2026-05-29T10:14:32Z — confirm the retention window with compliance before the next stage hardens the schema -->
