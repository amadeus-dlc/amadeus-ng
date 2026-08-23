# pending-revision — U2 functional-design（ステージゲートの Request Changes で適用する改訂案）

> 回復レビュー枠は消費済みのため、functional-design ステージゲートで Request Changes を選んだ直後に適用してレビュアーを再実行する。
> ADR-008（BR2.6）の反映は実施済み。以下は Bolt B3 実装（developer-report-2.md §4）で判明した設計の訂正事項。

1. `entities.md` WorkflowExecution.stages: `list<StageSlug>` → `list<StageEntry>`（phase を再水和でも保持 — BR1.3 のゲート判定に必要）。
   `WorkflowExecutionSnapshot.state` の注記も同期。plan / conditional は独立列のまま（from_snapshot が StageEntry との整合を検査）。
2. `entities.md` IntentId: 「`<kebab-slug>-<id8>`（id8 = 16 進 8 桁）」→「kebab 表記（`[a-z0-9]+` を `-` で連結）。実データは `<YYMMDD>-<slug>`
   （intents.json の dirName、例 `260822-stage1-selfhost`）で `-<id8>` サフィックスを持たない」。
3. `entities.md` payloads Started: `depth?` / `test_strategy?` を残し（C5 どおり）、「呼出側がフラグ上書き or scope metadata 既定を解決して渡す素通し値」と注記。
   `functional-spec.md` §2 / W1: `start(intent_id, &def, &StartRequest{scope, request, depth?, test_strategy?}, occurred_at)` と `start_with_entries`（ITF 用）。
4. `functional-spec.md` W3: `apply_event(Started)` は genesis 専用（既存集約への適用は InvariantViolation）。seq_nr = 1 からの全イベント再構成が要るなら
   U3 の設計で `from_started` 相当の入口を足す旨を注記。
5. `rules.md` BR3.1 / `functional-spec.md` W4: `EngineSignal` の導出で UnparkThenResume / ResumeMenu / NewWorkRouting は Done に畳む（Quint 語彙外）。
6. `entities.md` / `rules.md` BR5.2: 公開面に `WorkflowExecutionEventPayload` / `WorkflowExecutionSnapshotBuilder` / `IntentIdError` / `StartRequest` を追加。
7. 所見 20 / BR2.2: 「索引 0 は EXECUTE」を独立ガード（cursor_in_scope の初期条件）として明記。
