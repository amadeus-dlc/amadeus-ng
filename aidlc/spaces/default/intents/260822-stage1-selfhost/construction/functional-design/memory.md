<!-- INVARIANT: examples are single-line HTML comments so a fresh template parses to total=0 (MEMORY_EMPTY). Do NOT un-comment or split across lines. t100 guards this. -->
> This file is kept up to date automatically while the stage runs. Add observations at the review step, not by editing here directly.

## Interpretations
<!-- example: 2026-05-29T10:14:32Z — chose REST over GraphQL; the consuming team only needs CRUD, revisit if subscriptions land -->
- 2026-08-22T11:47:37Z — [u1-canon-json-goldens] レビュー Major 1（C7 のゴールデン受入表スキーマが 2 フィールド省略表記）は ADR 0001 が正本で C7 側の省略誤りと判断し、contract-summary.md を in-place 訂正して監査台帳に Change Request を残した（上流矛盾は人間裁定というルールの例外 — オーナー承認済み ADR が一方を明確に裏付ける場合）。ゲートで提示する
- 2026-08-22T11:36:16Z — [u1-canon-json-goldens] 構築フェーズの質問は本当の空白だけに絞り 2 問（ゴールデンの非決定フィールドの正規化、CLI ゴールデンのシナリオ範囲）にした; 3 プロファイル・キー順・数値表記は ADR 0001 で確定済みなので問い直さない。canon-json の公開 API の形（プロファイル enum + 1 関数群）はアーキテクトの設計判断として functional-spec に書く
- 2026-08-22T11:36:16Z — [u1-canon-json-goldens] user-stories が Skip のため traceability.json の upstream_ids は AC ではなく U1 の FR（FR7.1/7.2/7.3）と NFR1 にし、target は rules.md の BR ID にする; センサーは AC を期待するため誤検知し得るが、story-map と同じ FR 連鎖で一貫させる

## Deviations
<!-- example: 2026-05-29T10:14:32Z — skipped the optional caching layer the stage prose suggested; the dataset is small enough that it adds risk -->
- 2026-08-22T11:42:39Z — [u1-canon-json-goldens] traceability センサーが『missing_from_upstream_ids: 34』を報告したが、これは stories.md 不在時にセンサーが requirements.md の全 FR を各 Unit の upstream_ids に要求する実装上の限界（Unit に割り当てられた FR だけを列挙するステージ定義と噛み合わない）; Unit スコープの FR7.x のみを列挙する方針を維持し、誤検知として扱う（units-generation と同じ上流センサーの限界。O5 と同様に upstream 報告候補）

## Tradeoffs
<!-- example: 2026-05-29T10:14:32Z — picked TDD over BDD this run; the team is unit-first and the domain is well-understood -->

## Open questions
<!-- example: 2026-05-29T10:14:32Z — confirm the retention window with compliance before the next stage hardens the schema -->
- 2026-08-22T11:47:37Z — [u1-canon-json-goldens] レビュー Minor 2（W2 に用途→ダイジェスト族の対応表: バンドル digest / directiveHash / route hash = compact-raw、approval fingerprint / contract_sha256 = canonical-prefixed）と Minor 3（integer_value の i64/u64 判別: 非負は u64 優先、それ以外 i64）は終端受領後のため未反映。functional-design のステージゲートで Request Changes か、code-generation の計画で吸収するかをオーナーが判断
- 2026-08-22T17:10:00Z — [Interpretations] U10（packaging）は functional-design の成果物が適用外で、エンジンは「成果物なしで充足」と判定するが summary-confirmation の受領証は要求する（questions file 無しで error）; ゼロ質問 + 前提 P1/P2 の質問票を作り checkpoint を記録した。エンジンの per-unit 歩行は依存バッチ順（u1 → u10 → u2 → u9）で bolt-plan と異なる — オーナー裁定で U10 を B2 に前倒し（bolt-plan.md 改訂）
- 2026-08-22T23:50:00Z — [Interpretations] U2: ADR-002 ④「next_decision は集約のクエリ（21 分岐ラダー）」を字義どおり全分岐と読むと、集約が存在しない birth 前の分岐やフラグだけの分岐が集約クエリにならない — 分担（状態依存のみ集約 / 全部純関数 / 最小）を Q1 として人間裁定へ。StageIndex（B-2 繰延）と非ゲート完了イベント名も Q2 / Q3 に
