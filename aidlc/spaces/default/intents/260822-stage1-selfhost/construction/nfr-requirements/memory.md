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
- 2026-08-22T17:12:00Z — [Interpretations] U10（packaging）: 実地 `gh api` で `main` に ruleset「main」（active: deletion / non_fast_forward / merge_queue SQUASH ALLGREEN）が既にあり required_status_checks だけ無いことを確認（practices-discovery 時点の「protection 404 / rules []」から変化）。FR9.1 は ruleset への required checks 追加で満たし、merge queue が `merge_group` イベントで検査を要求するため `ci.yml` に `merge_group` トリガを足す必要がある（足さないと queue が詰まる）— 前提 P1/P2 として人間確認へ
- 2026-08-22T17:15:00Z — [Interpretations] U10 前提 P1〜P8 を Looks correct で確認。成果物は security-requirements（NFR2.x 品質ゲート要求も同居 — packaging に固有ファイルが無く、品質ゲートは「機械強制」という意味でセキュリティ/ガバナンス要求と同じ文書に置く）/ tech-stack-decisions / traceability の 3 つ
- 2026-08-22T17:21:00Z — [Open questions] U10 nfr-requirements レビュー READY（Minor 3）: (1) Dependabot への言及なし（SHA ピン留め見送りとの非対称）→ nfr-design / U10 計画で「見送り・後続 intent」と明記 (2) NFR4.2 の合格基準に実測基準と運用規範が混在 → 繰り延べ（文面の分離） (3) NFR2.1/2.2 に正常系（緑 PR が merge queue で squash-merge される）の実地確認を追加 → U10 code-generation の受入手順に入れる。凍結後のため本文は触らない
