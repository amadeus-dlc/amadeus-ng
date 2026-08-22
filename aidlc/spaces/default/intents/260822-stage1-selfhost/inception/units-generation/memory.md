<!-- INVARIANT: examples are single-line HTML comments so a fresh template parses to total=0 (MEMORY_EMPTY). Do NOT un-comment or split across lines. t100 guards this. -->
> This file is kept up to date automatically while the stage runs. Add observations at the review step, not by editing here directly.

## Interpretations
<!-- example: 2026-05-29T10:14:32Z — chose REST over GraphQL; the consuming team only needs CRUD, revisit if subscriptions land -->
- 2026-08-22T09:47:16Z — NFR1 の target は U7（最終の互換面）に一本化し、U1/U4 での検収は story-map の備考に書いた; traceability の target は単一 Unit ID でなければ突合できないため
- 2026-08-22T09:47:16Z — U9（正本修正 FR8.1）→ U3 の『着手前に済んでいるのが望ましい』関係は辺にせず申し送りにした; Q6 = A（コンパイル/テスト依存のみを辺にする）に従い、レビューで強制される正本準拠は運用上の前提として 2.9 へ渡す
- 2026-08-22T09:40:07Z — 後方ジャンプ完了後の再入: Q1〜Q8 の回答は再質問せず引き継ぎ、要約確認のみ改めて取った; 入力（requirements.md 改訂版・ADR-005 完全移動）は Unit 境界の選択（ハイブリッド・7〜10 Unit）に影響しないと判断
- 2026-08-22T08:41:07Z — FR1.2（ロック区間との結合）と ADR-007（ロック退役）の矛盾を検出し Q9 で人間に裁定を求めた; ADR-007 自身が requirements 改訂の必要性を注記しているが後方ジャンプは未実施のため、読み替え/ジャンプ/除外の 3 択にした
- 2026-08-22T08:41:07Z — user-stories がスキップ済みのため traceability.json は FR ID（FR1.1 等のサブ項目単位）で列挙し、story-map は「FR → Unit」対応表として書く; ステージ定義の「stories.md が無ければ FR を列挙」分岐に従った

## Deviations
<!-- example: 2026-05-29T10:14:32Z — skipped the optional caching layer the stage prose suggested; the dataset is small enough that it adds risk -->
- 2026-08-22T09:48:20Z — traceability センサーが SENSOR_FAILED（81 件）を出したが、原因は センサー実装（aidlc-sensor-traceability.ts storyAssignments）が story-map の行を US ID でしか認識せず、stories.md 不在で FR を列挙する経路でも FR 行を対応として数えないこと; ステージ定義の「stories.md が無ければ FR を列挙」と噛み合っていない upstream 側の限界と判断し、成果物は FR → Unit 対応のまま維持した（助言センサーのため承認は妨げない。手動で 43 ID と story-map の一致を確認済み）
- 2026-08-22T08:49:51Z — Q9 でオーナーが「改訂しないとまずい」と判断したため、ステージ途中で requirements-analysis へ後方ジャンプする（ステージ定義は Unit 成果物生成まで一気通貫だが、入力矛盾を未解決のまま Unit に FR1.2 を割り当てると構築フェーズの合格基準が二重になるため）; Q1〜Q8 の回答は本ファイルと questions.md に保持し、再入時は再質問せず確認のみ行う

## Tradeoffs
<!-- example: 2026-05-29T10:14:32Z — picked TDD over BDD this run; the team is unit-first and the domain is well-understood -->
- 2026-08-22T09:47:16Z — Unit 数は 10（Q2 の帯 7〜10 の上限）にした; FR4（CLI）と FR5（フック 4 本）を 1 Unit にまとめ（フックはサブコマンド — 同じ CliDispatcher の behaviour）、FR8 のコード分（PlanAction 完全移動・畳み込み移設）は ADR-002 の集約 ES 化と同じドメインコア Unit（U2）に寄せた。代替は 11〜12 Unit（フック独立・FR8 コード独立）だったが、PR 直列運用のオーバーヘッドを優先して束ねた

## Open questions
<!-- example: 2026-05-29T10:14:32Z — confirm the retention window with compliance before the next stage hardens the schema -->
- 2026-08-22T09:48:20Z — upstream へ: traceability センサーの units-generation 経路が FR-only の story-map を『対応なし』と誤判定する（US 行しか読まない）。FR 行も対応として読む修正を提案するか（O5 と同様に報告要否を判断）
- 2026-08-22T08:41:07Z — delivery-planning 2.9 へ: ADR-007 による requirements.md FR1.1/FR1.2 の文言改訂（後方ジャンプ）をいつ行うか
