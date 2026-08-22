<!-- INVARIANT: examples are single-line HTML comments so a fresh template parses to total=0 (MEMORY_EMPTY). Do NOT un-comment or split across lines. t100 guards this. -->
> This file is kept up to date automatically while the stage runs. Add observations at the review step, not by editing here directly.

## Interpretations
<!-- example: 2026-05-29T10:14:32Z — chose REST over GraphQL; the consuming team only needs CRUD, revisit if subscriptions land -->
- 2026-08-22T04:43:46Z — リード草案は Walking Skeleton を skeleton: off と提案（brownfield かつ Quint/ITF/ゴールデンパリティで疎通実証済みという根拠）; チーム意思としての最終確定はインタビュー質問に回した。
- 2026-08-22T04:43:46Z — Deployment 節は org.md 既定（deploy on merge → staging）を当てはめず、CLI 配布（cargo install 計画）という実態の事実記述に留めた; Web サービス前提の既定はこのプロジェクトに不適合の可能性と明記。
- 2026-08-22T05:07:33Z — Walking Skeleton の質問で「省けないと思っていた」という逆質問を受け、スコープ既定（classic は on）とチーム実践による上書き可否を説明のうえ off が確定; 説明後の選択なので裁定として堅い。
- 2026-08-22T05:07:33Z — practices-event（PRACTICES_DISCOVERED）は委任エージェントからガードで拒否されるため conductor が実行; ライフサイクル発行の conductor 専有はフレームワーク設計どおりと解釈。

## Deviations
<!-- example: 2026-05-29T10:14:32Z — skipped the optional caching layer the stage prose suggested; the dataset is small enough that it adds risk -->

## Tradeoffs
<!-- example: 2026-05-29T10:14:32Z — picked TDD over BDD this run; the team is unit-first and the domain is well-understood -->
- 2026-08-22T04:43:46Z — オーナー明言（t_wada 流 TDD・Bolt 単位 PR・Issue=intent）を第一級証拠としてリード起草の入力に指定; 会話の外にいる委任エージェントには届かない情報のため、ブリーフで明示供給する方式を選んだ。
- 2026-08-22T05:07:33Z — 規則本文の部分複製をやめ coding-rules 正本ファイル名参照へ統一（開発者レビュー採択）; DRY と読みやすさのトレードオフで DRY を優先。

## Open questions
<!-- example: 2026-05-29T10:14:32Z — confirm the retention window with compliance before the next stage hardens the schema -->
