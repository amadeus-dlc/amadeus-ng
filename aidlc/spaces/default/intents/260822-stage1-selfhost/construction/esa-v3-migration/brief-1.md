# B7 委任ブリーフ 1 — event-store-adapter-rs v3.0.0（EventEnvelope API）への乗り換え

Conversation language: 日本語

## 背景

- 本家 v3.0.0（2026-08-28 リリース）は、我々の要望書
  [`upstream-request-esa-event-envelope.md`](../../upstream-request-esa-event-envelope.md)
  の 4 設計質問すべてに回答する形で `Event` / `Aggregate` trait を廃止し、
  `EventEnvelope<AID, P>` / `SnapshotEnvelope<A>` に置き換えた。
- 移行の正本: 本家 `docs/MIGRATION_GUIDE_v3.ja.md` / `docs/DATABASE_SCHEMA.ja.md`
  （ローカル参照コピー: `/private/tmp/claude-501/-Users-j5ik2o-orca-workspaces-amadeus-ng-docs/0acf7966-8195-4d7d-9c6a-a934d785fa76/scratchpad/esa-v3/docs/`）。
- 方針は B6 と同じ **Conformist**（腐敗防止層なし・後方互換なし — オーナー裁定）。
  v3 は v2 の保存データを読まないが、本番データは存在しないため移行処理は書かない
  （テスト・フィクスチャは作り直す）。

## 着手前の必読

- `aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/`（全 16 規則、README から。
  特に upstream-contracts / gateway-taxonomy / field-visibility / factory-naming /
  ubiquitous-language / cqrs-boundaries / interior-mutability / error-handling）
- ADR-009 / ADR-010（`inception/domain-design/decisions.md`。2026-08-28 追記
  = RMU 二層裁定・アンカー照合の前倒し導入を含む）

## 固定済みの設計裁定（変更するな。疑義があれば止めて報告）

1. **ピン**: `event-store-adapter-rs = "=3.0.0"`（`sqlite` feature）。MSRV 1.94.1 ≤
   固定ツールチェーン 1.95.0 で問題なし。
2. **ドメインイベントの payload 純化**:
   - 自前封筒（現 `WorkflowExecutionEvent` struct の id / schema_version /
     occurred_at フィールド）と `WorkflowExecutionEventId` 型を**削除**。
   - 12 変種 enum `WorkflowExecutionEventPayload` を **`WorkflowExecutionEvent` に改名**
     — これがドメインイベントの正体（ubiquitous-language。「Payload」は輸送の語で
     ドメインの語ではない）。serde（`Serialize`/`Deserialize`）は維持。
   - `impl Event` / `impl Aggregate` は本家から trait が消えたので削除。
3. **seq_nr / 連続性検証はドメイン責務のまま**（v3 は「採番・連続性はドメイン側」と
   明文化 — 我々の現行設計と一致）:
   - 集約は `seq_nr: usize` / `last_updated_at` フィールドを維持。
   - `apply_event(seq_nr: usize, occurred_at: DateTime<Utc>, event: &WorkflowExecutionEvent)`
     へシグネチャ変更。SequenceGap / SequenceExhausted / 不変条件検査は現行どおり。
   - `commit` は適用後に payload（と呼ばない — イベント）を返すだけ。**封筒はドメインで
     作らない**。Repository が `EventEnvelope::new(intent_id, aggregate.seq_nr(),
     occurred_at, event).with_manifest(MANIFEST)` を組む（commit 成功後の
     `aggregate.seq_nr()` = そのイベントの通番）。
4. **version フィールドを集約と memento から削除**: 楽観ロック版数は
   `SnapshotEnvelope::version()` が正本（オーナー裁定「seq_nr と version を混ぜない」の
   完成形 — ストア採番トークンがストア側に閉じた）。genesis の `set_version(1)` ハックは
   消滅。`WorkflowExecutionState`（memento）から version 列を除去し、ITF・テストを
   追従させる。

   > **2026-08-29 改訂（委任者裁定 — 実装担当の矛盾指摘 選択肢 A を採用）**: 初稿の
   > 「更新は `persist_event(envelope, snapshot.version())`」は、store 内で snapshot を
   > 読み直す形になり TOCTOU で楽観ロックを無効化する（memory バックエンドには
   > `(aid, seq_nr)` 一意制約が無く黙って二重書込になる）ため撤回。本家移行ガイド §3 の
   > 持ち回り形に確定する: `find_by_id` は**再水和レコード**（集約 + ストア採番 version、
   > private フィールド + アクセサ）を返し、`store` は `expected_version` を引数に取る。
   > version は集約の**外**を通るので「集約と memento から削除」は完全に維持される。
   > expected_version は newtype（`StoreVersion` 等、genesis は関連定数）を推奨 —
   > seq_nr との取り違えを型で塞ぐ。既存契約テスト `a_write_from_a_stale_version_conflicts`
   > は「並行書込後に握り直さないと競合する」趣旨へ書き換え、sqlite / memory 両方で
   > 検出が成立することを証明する。
   >
   > **同日追記（実装担当の Quint 実測に基づく確定）**: 更新も
   > `persist_event_and_snapshot(envelope, aggregate, expected_version)` を使う —
   > v3 の `persist_event` は snapshot の seq_nr を進めないため、モデル不変条件
   > `snapshot_tracks_journal`（snapSeq == journalLen、スナップショット毎書込の設計契約）
   > を破る。genesis / 更新の分岐は `event.seq_nr == 1` で導出（`is_created` の消滅に
   > 整合、本家 v3 と同型）。expected_version は usize のまま（不透明トークンの旨を
   > doc 明記。newtype 化は U5/U6 実装時の境界強化候補として報告書に記録）。
5. **manifest 定数** `workflow-execution-event/1` — 旧 `schema_version` の後継。
   Repository が書き、`JournalReaderImpl` は不一致・欠落を `Corrupt(UndecodablePayload)`
   で拒否（旧 #466 検査の後継。payload 内メタ照合 #500 は二重化ごと消滅）。
6. **`JournalReader` ポートの戻り値**: `(GlobalSeqNr, WorkflowExecutionEvent)` では
   集約識別が失われるため、**自前の読取レコード `JournalEntry`** を `core-use-case` に
   新設して返す — フィールドは global_seq / intent_id (`IntentId` へ parse、失敗は
   Corrupt) / seq_nr / occurred_at / event。private フィールド + アクセサ
   （field-visibility）。**本家の `EventEnvelope` 型をポートから出すな** — この trait は
   U4 で RMU クレート所有へ移り、RMU はライブラリ型に依存できない（ADR-009 2026-08-28
   追記）。
7. **不変**: rowid カーソル、`amadeus_projection_checkpoint` 表、(aid, seq_nr) アンカー
   照合（B6 で導入済み）、busy_timeout、`Connection::open_with_flags`（CREATE なし —
   #511）。スキーマガードは v3 DDL（`occurred_at` ナノ秒 + `manifest TEXT NOT NULL
   DEFAULT ''` 列、`(aid, seq_nr)` UNIQUE index）へピンを張り替え、`SELECT` に
   occurred_at / manifest 列を追加。
8. **v3 の新契約への追従**: `with_keep_snapshot_count` が `Result` に、
   `EventStoreWriteError::ContractViolation` の match 腕追加、不在集約への更新は一律
   `OptimisticLockError`（`RepositoryError::Conflict` 写像は現行踏襲）。
9. **Quint モデルは変更しない**。ITF 準拠テスト・統合テストは新シグネチャへ追従。

## 所有ファイル（これ以外に書くな）

- `Cargo.toml` / `Cargo.lock`
- `modules/**`（core-domain / core-use-case / core-interface-adapter とそのテスト）
- `tests/**`（あれば）
- 報告書: `aidlc/spaces/default/intents/260822-stage1-selfhost/construction/esa-v3-migration/developer-report-1.md`

**禁止**: `docs/specs/**`・`aidlc/**`（報告書以外）・`formal/**`・`.github/**` への変更。
本家リポジトリ（j5ik2o/event-store-adapter-rs）への issue/PR/コメント等の接触は一切禁止。

## 受入基準（全部を自分で実行して報告書に結果を貼る）

1. `cargo fmt --all --check`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo lint`
4. `cargo test --workspace` 全緑
5. `bash scripts/quint-gate.sh` 緑
6. `bash scripts/coverage.sh --base origin/main` — 絶対 90% 床と相対ゲート（base −0.01pp）
   の**両方 PASS**。新設エラー枝には到達テストを書いてから測ること（B6 の教訓:
   後追いでゲート回復に 3 コミット要した）。
7. プロダクトコードに `unwrap`/`expect` なし、`#[allow]` には理由コメント必須。

## 報告書に必ず書くこと

- 変更概要（削除された型/フィールドの一覧、行数増減）
- 固定裁定 1〜9 それぞれの実施箇所（file:line）
- 受入基準 1〜6 の実行ログ末尾
- 判断に迷って独自解釈した点（あれば正直に列挙 — 空欄可）
