# U4（ReadModelUpdater）クレート構成案 — B8 着手前のオーナー確認用

Conversation language: 日本語
前提裁定: ADR-009（CQRS クレート境界 + 2026-08-28 改訂 = RMU が JournalReader を呼ぶ二層構造）、
ADR-010（v3 乗り換え済み）、unit-of-work.md U4 原文、
**2026-08-29 オーナー裁定「interface-adapter / use-case はコマンド側とクエリ側に分割する。
JournalReaderImpl は RMU クレート」**（初稿の論点 A は両選択肢とも却下 — 本改訂版が正）。

## 1. クレート配置（改訂版）

現行の `core-use-case` / `core-interface-adapter` を**側で完全分割**し、クエリ側の実体は
すべて RMU クレートへ吸収する。amadeus-ng のクエリ側は「リードモデル（状態ファイル・
監査シャード）を書く」側であり、読取 API のユースケース層はまだ存在しないため、
クエリ側 = RMU クレート 1 つで完結する。

**命名（2026-08-29 オーナー裁定）**: `core-{command,query}-` 接頭辞で統一する。

```text
modules/core/command/use-case/          パッケージ名: core-command-use-case（旧 core-use-case の残部）
  workflow_execution_repository.rs / workflow_definition_repository.rs /
  rehydrated_workflow_execution.rs / repository_error.rs / corrupt_cause.rs（コマンド側専用に）

modules/core/command/interface-adapter/ パッケージ名: core-command-interface-adapter（旧 core-interface-adapter の残部）
  workflow_execution_repository_impl.rs / workflow_definition_repository_impl.rs /
  memory/ / store_failure.rs（コマンド側写像）

modules/core/query/read-model-updater/  パッケージ名: core-query-read-model-updater（クエリ側の全実体）
  journal_reader.rs / journal_entry.rs / journal_read_error.rs /
  global_seq_nr.rs / projection_name.rs      ← 旧 core-use-case から移動（読取語彙）
  journal_reader_impl.rs                      ← 旧 core-interface-adapter から移動（オーナー裁定 —
                                                RMU の仕事はジャーナル読取そのものであり、
                                                SQLite 依存は RMU の本質的な結合）
  updater.rs                                  取得ループ（checkpoint → events_after → 投影核 → advance）
  projection.rs                               純粋投影核 project(entries: &[JournalEntry], read_model)
  state_file.rs / audit_shard.rs              投影ライタ（state_file_io.rs の転生 + 監査 86 語彙）
```

- `core-domain` は共有のまま（両側が依存してよい唯一の層）。
- **infrastructure 層（2026-08-29 オーナー裁定追加）**: `modules/core/infrastructure` =
  `core-infrastructure`（旧 `infra-io` の改名 — `atomic` / `append_only` / `fs_meta`。言語拡張系
  のみを置き、RPC クライアント・DB アクセスは置かない —
  `coding-rules/infrastructure-layer.md`）。`modules/harness/infrastructure` =
  `harness-infrastructure` は harness 文脈の言語拡張の受け皿として憲章 doc 付きで新設
  （実体は U7 以降）。依存方向: infrastructure はどの層も知らない・どの層からも使ってよい。

## 2. 依存グラフ（cqrs-boundaries 判定表・改訂）

```text
core-domain                    ← 共有（イベント語彙・集約）
core-infrastructure            ← 言語拡張（旧 infra-io。どの層も知らない）
core-command-use-case          → core-domain
core-command-interface-adapter → core-domain, core-command-use-case, event-store-adapter-rs(sqlite)
core-query-read-model-updater  → core-domain, core-infrastructure, audit-events,
                                 message-catalog, rusqlite, serde_json, chrono
                                 （共有層・外部ライブラリのみ — 側のクレートはゼロ。実装実測）
app/aidlc (U7)                 → 両側（合成ルートだけが両側を知る — RMU の起動のみ）
```

- **コマンド側とクエリ側は互いの Cargo.toml に現れない**（相互独立が物理強制）。
- 旧「アダプタ層は両側の契約を実装してよい」（cqrs-boundaries 対象外条項）は本裁定で**失効**
  — アダプタ層も側で分割する。B8 で cqrs-boundaries.md / ADR-009 を改訂する。

## 3. 共有部品の行き先（B8 で確定する設計点 — 委任者推奨付き）

| 部品 | 現状 | 行き先（推奨） | 理由 |
|---|---|---|---|
| `CorruptCause` | 両エラー型が共有 | **側ごとに専用 enum に分割**（コマンド側: MissingSnapshot 等 / クエリ側: CheckpointAnchorMismatch 等） | 相互独立 > DRY。現に片側にしか意味の無い変種が両方にある |
| `store_failure.rs`（io_kind 写像） | 両実装が共有 | 側ごとに複製 | 同上（30 行程度の写像） |
| `StorePath` | 両実装が共有 | `core-domain`（workspace 文脈）へ移動 | 「space → ストアの場所」はワークスペースの語彙 |
| `EVENT_MANIFEST` | interface-adapter 内 | `core-domain`（イベント enum の隣） | 直列化版の型判別子はイベント語彙の Published Language — 書く側と検める側が同じ正本を見る |

## 4. 検収（B8 の受入基準になるもの）

- FR1.1: 投影出力（状態ファイル・監査シャード）が 0a 逐語契約に一致（U1 ゴールデンとの突合）
- NFR3: ジャーナル → 投影の再生成が冪等（同じ checkpoint から何度流しても同一バイト）
- 監査シャード横断の位置付き読取（timestamp ソート + バッファ位置 tiebreak — FR1.1）
- 既存 ITF 準拠（journal_protocol）のフェイク投影を実 RMU に差し替えても緑
- CI 3 ジョブ + カバレッジ相対ゲート + `cargo lint`（TDD、テストピラミッド配分）
