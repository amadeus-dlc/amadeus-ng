# pending-revision — U2 functional-design（ステージゲートの Request Changes で適用する改訂案）

> 回復レビュー枠は消費済みのため、functional-design ステージゲートで Request Changes を選んだ直後に適用してレビュアーを再実行する。
> ADR-008（BR2.6）の反映は実施済み。以下は Bolt B3 実装（developer-report-2.md §4）で判明した設計の訂正事項。

1. `entities.md` WorkflowExecution.stages: `list<StageSlug>` → `list<StageEntry>`（phase を再水和でも保持 — BR1.3 のゲート判定に必要）。
   `WorkflowExecutionSnapshot.state` の注記も同期。plan / conditional は独立列のまま（from_snapshot が StageEntry との整合を検査）。
2. `entities.md` IntentId: 「`<kebab-slug>-<id8>`（id8 = 16 進 8 桁）」→「kebab 表記（`[a-z0-9]+` を `-` で連結）。実データは `<YYMMDD>-<slug>`
   （intents.json の dirName、例 `260822-stage1-selfhost`）で `-<id8>` サフィックスを持たない」。
   - **廃止（2026-08-23）**: 本項目の「kebab 表記」は項目 8（`IntentId` = UUIDv7、Q2 = A）で上書きされた。kebab の記録ディレクトリ名は別の値 `IntentDirName` として規定する（01 号 §3.3 / 11 号 §2.2）。
3. `entities.md` payloads Started: `depth?` / `test_strategy?` を残し（C5 どおり）、「呼出側がフラグ上書き or scope metadata 既定を解決して渡す素通し値」と注記。
   `functional-spec.md` §2 / W1: `start(intent_id, &def, &StartRequest{scope, request, depth?, test_strategy?}, occurred_at)` と `start_with_entries`（ITF 用）。
4. `functional-spec.md` W3: `apply_event(Started)` は genesis 専用（既存集約への適用は InvariantViolation）。seq_nr = 1 からの全イベント再構成が要るなら
   U3 の設計で `from_started` 相当の入口を足す旨を注記。
5. `rules.md` BR3.1 / `functional-spec.md` W4: `EngineSignal` の導出で UnparkThenResume / ResumeMenu / NewWorkRouting は Done に畳む（Quint 語彙外）。
6. `entities.md` / `rules.md` BR5.2: 公開面に `WorkflowExecutionEventPayload` / `WorkflowExecutionSnapshotBuilder` / `IntentIdError` / `StartRequest` を追加。
7. 所見 20 / BR2.2: 「索引 0 は EXECUTE」を独立ガード（cursor_in_scope の初期条件）として明記。
8. （オーナー裁定 2026-08-23、U9 FD Q2 = A）`IntentId` の正本は **UUIDv7**（`intents.json` の `uuid`、01 号 §3）。項目 2 の「一般 kebab」は
   記録ディレクトリ名用の別の値（`IntentDirName`）として書き分ける。U2 の `IntentId::parse` を UUIDv7 形式に改める是正は Bolt B5（U3 —
   `aggregate_id` を SQLite に書く最初の Unit）で行う。entities.md の IntentId 行を「UUIDv7（`intents.json` の uuid）」に、`IntentDirName` を新設。
9. （オーナー質問・了承 2026-08-23「その改名案よさそう」）`WorkflowExecutionSnapshot` の名前が C6 の永続化テーブル `snapshot` と同じで紛らわしい — B5（U3）で `WorkflowExecutionState`（memento）へ
   改名する（オーナー了承済み）（責務は変えない: serde なし、`snapshot()` / `from_snapshot()` = 状態の写しと不変条件つき復元）。entities / spec の用語も同期。
   - 追記（B4 統合時、2026-08-23）: 改名の目的は「ドメイン API から `snapshot` の語を除き、ES のスナップショット（C6 `snapshot` テーブル）との
     混同を避ける」ことなので、メソッドも `state()` / `from_state()` へ改名する（10 号 §2.1 の規範と一致）。B5 の計画で確定し、ゲートで
     オーナー確認（開発エージェントの設計質問 1）。
