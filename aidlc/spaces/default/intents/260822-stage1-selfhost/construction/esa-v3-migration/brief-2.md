# B7 委任ブリーフ 2 — v3 乗り換えの仕様同期（doc-sync）

Conversation language: 日本語

## 目的

B7 のコード変更（`b30a294` + nit 修正）で生じた仕様・記録とコードの語彙ドリフトを解消する。
コードは**一切変更しない**。読むだけ。

## 必読

- `construction/esa-v3-migration/developer-report-1.md`（特に §1 の削除/新設表と §4-(f) の
  ドリフト 2 点）と `brief-1.md`（固定裁定 9 点 + 2026-08-29 改訂ブロック）
- 同期スタイルの前例: B6 の doc-sync（`docs/specs/11-workspace.md` などにある
  「~~旧記述~~ → **新記述**（2026-08-27 / ADR-010・Bolt B6）」形式の日付付き失効注記）。
  同じ様式で「（2026-08-29 / ADR-010・Bolt B7）」を使うこと。**過去の記録を書き換えず、
  失効注記を重ねる**のが原則（ADR・contract は追記式）。ただし「現在形の仕様文」
  （docs/specs の地の文）は現在の姿に直してよい — 大改訂になる場合のみ失効注記形式。

## 対象と作業内容

1. **`docs/specs/11-workspace.md`** — C6 の memento 属性数（17 → 16、`version` 列の除去）、
   `schema_version` に触れる記述 → journal `manifest` 列（`workflow-execution-event/1`）へ、
   Repository / JournalReader の署名に触れる箇所（`find_by_id` → `RehydratedWorkflowExecution`、
   `store(event, aggregate, expected_version)`、`events_after` → `Vec<JournalEntry>`）、
   `=2.0.0` ピンへの言及 → `=3.0.0`。
2. **`docs/specs/10-orchestration.md`** — `WorkflowExecutionRepository` 行と
   `WorkflowExecutionEventId` / 旧封筒への言及を実態へ（ドメインイベント 12 語彙は不変）。
3. **`docs/specs/01-domain-model.md` ほか docs/specs 全体** — `grep -rn
   "WorkflowExecutionEventId\|schema_version\|set_version\|EventStoreImpl\|=2\.0\.0"
   docs/specs/` で見つかる残骸をすべて処理（意味が変わっていない BR 番号参照は触らない）。
4. **`inception/contract-design/contract-summary.md`** — C3 の trait 全文と 2026-08-28 注記の
   直後に B7 実施の追記（trait 移動は U4 のままだが、署名が v3 形へ変わったこと）。C5 の
   `schema_version` 予約フィールド → manifest 後継の追記。C6 の封筒列（occurred_at ナノ秒 /
   manifest）追記。
5. **`inception/domain-design/decisions.md`** — ADR-010 へ日付付き追記 1 ブロック:
   本家 v3.0.0（2026-08-28 リリース）が要望書の 4 設計質問すべてに回答する形で出たこと、
   B7 で `=3.0.0` へ乗り換えたこと、version は集約の外（`RehydratedWorkflowExecution`）を
   持ち回る形になったこと（brief-1 改訂ブロックの TOCTOU 経緯を 2〜3 行で要約）、更新も
   `persist_event_and_snapshot` であること、`manifest` が `schema_version` の後継であること。
6. **`upstream-request-esa-event-envelope.md`** — 冒頭に結果注記: 本家 v3.0.0 が
   2026-08-28 に本要望の方向で実装・リリースされ、B7 で採用済み。
7. **`construction/u3-event-store-repository/functional-design/functional-spec.md`** —
   B6 失効注記がある箇所のうち B7 でさらに変わった行（ポート署名・`check_preconditions` の
   消滅・ITF 再生先）へ B7 注記を重ねる。

## 所有ファイル（これ以外に書くな）

- `docs/specs/**`
- `aidlc/spaces/default/intents/260822-stage1-selfhost/inception/contract-design/contract-summary.md`
- `aidlc/spaces/default/intents/260822-stage1-selfhost/inception/domain-design/decisions.md`
- `aidlc/spaces/default/intents/260822-stage1-selfhost/upstream-request-esa-event-envelope.md`
- `aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u3-event-store-repository/functional-design/functional-spec.md`
- 報告書: `aidlc/spaces/default/intents/260822-stage1-selfhost/construction/esa-v3-migration/doc-sync-report.md`

**禁止**: `modules/**`・`formal/**`・`Cargo.*`・coding-rules・memory への変更。push 禁止。

## 検収

- `grep -rn "WorkflowExecutionEventId\|schema_version\|EventStoreImpl" docs/specs/` が
  0 件（失効注記の中の取り消し線表記 `~~...~~` 内は許容）
- 固定トークン（BR/FR/C 番号、`READY`、YAML キー等）は英語のまま（org.md 保存トークン規則）
- 報告書に: 変更ファイル一覧、各ファイルの変更要点 1 行、grep 検収の実行結果
- コミットは 1 本、メッセージは「b7: 仕様同期 — ...」
