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

## 2026-08-23T00:30Z — U10 nfr-requirements-questions.md を確認済みバイトへ復元
- PR #25 レビュー指摘の引き取り（ecb2307）で人間確認済みの質問ファイルを書き換えてしまい、エンジンが「確認後に変更」を検出してステージ完了を拒否した。
- 人間が確認したバイト（0f3a151）へ復元。訂正内容（FR9.6 は U9 の責務 / 日付 UTC 表記）は `u10-ci-governance/code-generation/superseding-decisions.md`（#6 ほか）と本日誌が正本。
- 学習候補: 人間確認済みの questions ファイルは訂正対象にせず、訂正は superseding-decisions / 日誌へ書く。
- 2026-08-23T00:32:00Z — [Interpretations] [u2-domain-es-core] kind = library のため produces は security-requirements / tech-stack-decisions / traceability の 3 つ（performance / scalability / reliability / observability は service/ui のみ）。U2 固有の NFR 質問は無し — 適用 NFR（NFR1 engine_loop 契約維持・NFR2 TDD + 決定的 PBT・NFR3 集約側 replay 決定性・NFR4 依存追加なし）は ADR / 機能設計 BR / team.md で確定済み。前提 P1〜P6 を人間確認へ
- 2026-08-23T00:40:00Z — [Tradeoffs] [u2-domain-es-core] STRIDE の Tampering は暗号学的完全性（署名 / HMAC）を要求せず、seq_nr 飛び・不変条件違反の検出（NFR3.2）で再水和を拒否する設計に留めた — ローカル単一ユーザのワークスペースで真実源は git 管理外の SQLite（C6）。必要になれば後続 intent
- 2026-08-23T00:40:30Z — [Interpretations] [u2-domain-es-core] NFR1 は集約に観測可能面が無いため「engine_loop 契約の維持（ITF）・ゲート判定の upstream 一致・イベント語彙の契約安定性」の 3 つに限定し、逐語一致・ゴールデンは U4 / U6 / U7 の NFR1 へ送った
- 2026-08-23T00:45:00Z — [Open questions] [u10-ci-governance] 回復レビュー（iteration 2）READY、Major 2 / Minor 3: (1) NFR2.4 の合格基準が `TOLERANCE=0.01` / 差 0.00pp のままで実装（暫定 0.05、残差 0.0175pp）と不一致 (2) NFR2.1 が「3 コンテキスト」のまま（実地は check/quint/coverage/CI Success の 4 つ）、NFR4.4「ジョブ個別の昇格なし」が `review-thread-resolution` ジョブの個別権限（checks/statuses: write 等）と矛盾、§1/§3 に外部再利用ワークフロー（SHA 固定）の信頼境界が無い (3)〜(5) iteration 1 の Minor 再掲（NFR4.2 基準の混在 / NFR2.1 正常系 / Dependabot）。回復受領は終端のため本文は触らず、nfr-requirements のステージゲートで所見を提示し Request Changes で修正経路に入れる（事実は superseding-decisions.md #11 に記録）
- 2026-08-23T01:00:00Z — [Deviations] [u2-domain-es-core] レビュー iteration 1 は NOT-READY（Major 3 / Minor 2、すべて事実誤認で機械検証可能）: (1) tech-stack「依存なし」は誤り — core-domain の runtime 依存は audit-events / directive-schema / message-catalog の 3 つ、dev は proptest + serde_json（ベースラインを実測に直し「追加 0」を再定義） (2) NFR1.1 の ITF 書換対象は engine_loop_conformance.rs 1 本のみ（audit_lock_conformance.rs は U3 の FR1.2） (3) NFR3.3 の snapshot 列挙に plan / conditional が欠落 (4) ドメインクレート単独カバレッジの実測なし (5) clippy deny は 48（rust 5 + rustdoc 1 + clippy 42、team.md の「rust 4」は数え漏れ）。オーナーの WorkflowDefinitionId 裁定（集約＝エンティティには ID が要る）と併せて 1 回で修正し、回復レビュー（iteration 2）に出す方針
- 2026-08-23T01:00:30Z — [Open questions] [u2-domain-es-core] オーナー指摘: WorkflowDefinition は集約ルート（12 号 §2.1）なのに識別子が無い。提案 = 内容アドレス ID（canon-json hash-canonical の sha256）を Repository が付与、Started.definition_id で間接参照、next_decision で DefinitionMismatch 検査。裁定待ち（U2 機能設計側は回復レビュー済みのためゲートの Request Changes 経路で反映）
- 2026-08-23T01:20:00Z — [Interpretations] [u2-domain-es-core] オーナー裁定（集約はエンティティ → 不変 ID + 内容版 revision、内容アドレス ID は却下、後方互換 find() は残さない）を ADR-008 / C4 / C5 / U2 機能設計 BR2.6 / NFR3.4 に反映し、レビュー所見 1〜5 も同時に是正（依存ベースライン実測 / ITF 対象 1 本 / snapshot 列挙 / カバレッジ根拠 / lint 48）。回復レビュー（iteration 2）へ
- 2026-08-23T01:20:30Z — [Open questions] team.md Code Style の「workspace lints 計47ルール（rust 4）」は実測 48（rust 5）— §13 の学習で訂正候補（memory は直接編集しない）
- 2026-08-23T01:30:00Z — [Deviations] [u2-domain-es-core] 回復レビュー（iteration 2）READY、Major 1: BR2.6 / NFR3.4 が `start` にも id 検査を要求していたが start は静的コンストラクタで比較対象が無く戻り値型も StartError — 検査は next_decision のみ（start は記録のみ）に訂正（rules BR2.6 / entities / functional-spec / NFR3.4 を同期）。回復受領後の編集のため受領は再び無効 — nfr-requirements のステージゲートで Request Changes の修正経路により再レビューする（オーナー指示「修正してレビューは是正して」を優先）
- 2026-08-23T01:40:00Z — [Deviations] [u2-domain-es-core] bash/python で書いた produces をエンジンが「要約確認後のネイティブ書込記録なし」として完了拒否 — Write/Edit で再保存して通した。学習候補: produces 配下の成果物はハーネスのネイティブ書込（Write/Edit）で書く（フックが ARTIFACT_* を記録する経路でないと完了受理されない）
