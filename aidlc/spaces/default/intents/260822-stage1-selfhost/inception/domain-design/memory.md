<!-- INVARIANT: examples are single-line HTML comments so a fresh template parses to total=0 (MEMORY_EMPTY). Do NOT un-comment or split across lines. t100 guards this. -->
> This file is kept up to date automatically while the stage runs. Add observations at the review step, not by editing here directly.

## Interpretations
<!-- example: 2026-05-29T10:14:32Z — chose REST over GraphQL; the consuming team only needs CRUD, revisit if subscriptions land -->
- 2026-08-22T09:19:27Z — 承認済み requirements.md の FR8.3 は後方ジャンプせず文言訂正で済ませた; 変更は ADR-005 の移行方式（re-export の有無）だけで要求 ID・合格基準の構造に影響せず、再ジャンプ（RA→US→RM→DD 再実行）は釣り合わないと判断。合格基準は完全移動を検証できる形に具体化した
- 2026-08-22T09:16:44Z — 後方ジャンプ（requirements-analysis 改訂）後の再入で成果物を Keep した; 改訂は本ステージの ADR-001/003/004/007 に要求側を合わせたもので、components.md / decisions.md / traceability.json の内容に変更理由が無い。アーキテクチャレビューは改訂後 requirements.md との整合確認として 1 回通す
- 2026-08-22T06:30:25Z — オーナーが Q2 の議論で統一ルールを宣言:「集約は FSM。状態としてのデータと状態遷移のための振る舞いは同じ型に閉じ込める。横展開する考え方」。遷移は &mut self コマンド（typestate 不採用）、導出はクエリメソッド、ユースケースは進行管理・フロー制御のみでビジネスロジック禁止。coding-rules 正本への追加候補（学びの儀式で提示する）。
- 2026-08-22T08:12:44Z — 「マルチクローン交換」と説明なしの術語で答えてオーナーから叱責; 直前の学び（質問文の術語は注釈する）を自分の回答にも適用すべきだった。以後、説明文でも初出術語は必ず平易な言い換えを添える。
- 2026-08-22T08:27:55Z — オーナー訂正: intent 粒度は n Issue = 1 intent（1:1 は誤り）。project.md Corrections に上書き行を永続化。team.md Way of Working の 1:1 記述の是正は次回 practices-promote か B束文書修正で実施、requirements C3 の文言是正は FR1 合格基準の後方ジャンプ改訂に同梱する。

## Deviations
<!-- example: 2026-05-29T10:14:32Z — skipped the optional caching layer the stage prose suggested; the dataset is small enough that it adds risk -->
- 2026-08-22T09:31:25Z — 再入の手順を redo ジャンプでやり直した; Keep/Modify 後にレビューを先に通したため、必須の要約確認（summary-confirmation）→成果物保存→レビューの順序を満たせずゲートが拒否された。redo で試行を仕切り直し、確認→ネイティブ保存→レビュー→学び→ゲートの順で再実行。Keep/Modify の人間判断（ARTIFACT_REUSED 記録済み）は再質問しない
- 2026-08-22T09:19:27Z — Keep 判定の直後にオーナー裁定（利便再エクスポートはどこでも禁止）が届き、ADR-005 を re-export 併用から完全移動へ改訂して decisions.md / components.md を Modify した; 同じ前提を写していた requirements.md FR8.3（承認済み）と design-audit R1 も同時に訂正し、監査台帳に Change Request として記録
- 2026-08-22T08:12:44Z — 初回生成後にオーナーとの根本設計議論（ES vs WAL+投影）が発生し、成果物を全面書き直しへ; 当初の「監査先行 WAL + 同期プロジェクション」「集約がイベント列を返す」案は 1コマンド1イベント規律違反としてオーナーが棄却、event-store-adapter-rs 前提の正統 ES（SQLite ストア + RMU + チェックポイント）で確定。レビュー NOT-READY 所見（YAML 破損・対称性）も書き直しで同時解消する。

## Tradeoffs
<!-- example: 2026-05-29T10:14:32Z — picked TDD over BDD this run; the team is unit-first and the domain is well-understood -->

## Open questions
<!-- example: 2026-05-29T10:14:32Z — confirm the retention window with compliance before the next stage hardens the schema -->
