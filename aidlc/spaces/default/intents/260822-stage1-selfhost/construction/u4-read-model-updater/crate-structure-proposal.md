# U4（ReadModelUpdater）クレート構成案 — B8 着手前のオーナー確認用

Conversation language: 日本語
前提裁定: ADR-009（CQRS クレート境界 + 2026-08-28 改訂 = RMU が JournalReader を呼ぶ二層構造）、
ADR-010（v3 乗り換え済み）、unit-of-work.md U4 原文（ジャーナル読取→投影→checkpoint 前進、冪等）。

## 1. クレート配置（提案）

```text
modules/read-model-updater/        パッケージ名: read-model-updater
  src/
    journal_reader.rs              ← core-use-case から trait を移動（呼ぶ者が港を所有）
    journal_entry.rs               ← 同（global_seq / intent_id / seq_nr / occurred_at / event）
    journal_read_error.rs          ← 同（Io / Corrupt / CheckpointRegression）+ ProjectionName / GlobalSeqNr
    updater.rs                     RMU コンポーネント = 取得ループ（checkpoint 読取 → events_after
                                   → 投影核 → advance_checkpoint）。&mut self
    projection.rs                  純粋投影核 fn project(entries: &[JournalEntry], read_model: &mut ReadModel)
    state_file.rs / audit_shard.rs 投影ライタ（aidlc-state.md / 監査シャード 86 語彙の逐語互換描画。
                                   旧 render_audit_block / state_writers の転生 — 11-workspace §投影）
```

- **依存**: `core-domain`（ドメインイベント語彙）+ `chrono` + `serde`（描画のみ）。
  **`core-use-case`・`core-interface-adapter`・event-store-adapter-rs には依存しない**
  （cqrs-boundaries: RMU の Cargo.toml にコマンド側ポートが現れたら違反）。
- `core-use-case` は読取語彙（JournalReader / JournalEntry / GlobalSeqNr / ProjectionName /
  JournalReadError）を**失う**。コマンド側ユースケース（U5/U6）はもともと 1 度も呼ばないので
  呼出側修正は発生しない。
- `core-interface-adapter` は `read-model-updater` に依存を**追加**し、`JournalReaderImpl` が
  移動後の trait を実装する（下の論点 A）。

## 2. 依存グラフ（cqrs-boundaries 判定表）

```text
core-domain        ← 共有（イベント語彙）
core-use-case      → core-domain                        （コマンド側 — RMU 依存なし ✓）
read-model-updater → core-domain                        （橋 — コマンド側ポート依存なし ✓）
core-interface-adapter → core-domain, core-use-case, read-model-updater, esa
                                                        （実装層 — 両側の契約を実装してよい ✓ 対象外条項）
app/aidlc (U7)     → 上記全部                            （合成ルート — RMU の起動のみ）
```

## 3. オーナー裁定が要る論点

### 論点 A — `JournalReaderImpl`（SQLite 実装）の置き場所

- **(a) 推奨: `core-interface-adapter` に留める**。アダプタ層は両側のポートを実装してよい
  （cqrs-boundaries 対象外条項が既に明文）。SQLite の同輩（Repository 実装・スキーマガード・
  アンカー照合テスト）と同居が保て、新クレート不要。
- (b) `read-model-updater` に feature 付きで同居 — RMU が rusqlite を抱えストレージ都合で汚れる
- (c) クエリ側アダプタクレートを新設 — 最も厳密だがクレート +1 の複雑さ

### 論点 B — 純粋投影核の入力

- **(a) 推奨: `JournalEntry` 列**（`fn project(entries: &[JournalEntry], read_model: &mut ReadModel)`）。
  監査行の描画には occurred_at（タイムスタンプ列）と intent_id・seq_nr が**逐語互換に必須**で、
  JournalEntry はそれらをドメイン語彙だけで運ぶ読取レコード（ライブラリ型なし）。
  「投影核はドメインイベント（とその発生文脈）だけを受け取る」の範囲内と考える。
- (b) 素のイベント列 + メタデータ別引数 — 引数が並ぶだけで JournalEntry の再発明になる

### 論点 C — Bolt 分割

- **(a) 推奨: B8 一本**（クレート新設 + 語彙移動 + 取得ループ + 投影本体 + ゴールデン検収）。
  U4 は M サイズ想定で、移動だけの Bolt を挟むと PR 直列の待ちが増える。
- (b) B8a（構造移動のみ）/ B8b（投影本体）に分割 — レビュー単位は小さくなる

## 4. 検収（B8 の受入基準になるもの）

- FR1.1: 投影出力（状態ファイル・監査シャード）が 0a 逐語契約に一致（U1 ゴールデンとの突合）
- NFR3: ジャーナル → 投影の再生成が冪等（同じ checkpoint から何度流しても同一バイト）
- 監査シャード横断の位置付き読取（timestamp ソート + バッファ位置 tiebreak — FR1.1）
- 既存 ITF 準拠（journal_protocol）のフェイク投影を実 RMU に差し替えても緑
- CI 3 ジョブ + カバレッジ相対ゲート + `cargo lint`（TDD、テストピラミッド配分）
