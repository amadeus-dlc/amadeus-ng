# logical-components — U3 SQLite EventStore と WorkflowExecutionRepository（`u3-event-store-repository`）

> NFR Design（Construction 3.3）成果物（Unit: U3、kind: library、Bolt: B5）。出典: `../functional-design/functional-spec.md`（§1 配置、§7 テスト）、`../nfr-requirements/
> security-requirements.md`（NFR2.x / NFR3.x）、`security-design.md`（同ディレクトリ）。

## 1. コンポーネント一覧

| コンポーネント | 層 / クレート | 責務 | 依存 |
|---|---|---|---|
| `orchestration::ports`（`WorkflowExecutionRepository` / `EventStore` / `JournalReader`） | use-case `core-use-case` | ポート trait（C3、u64） | `core-domain` |
| `orchestration::errors`（`RepositoryError` / `EventStoreError` / `CorruptCause`） | use-case | 材料のみのエラー | — |
| `orchestration::{GlobalSeqNr, ProjectionName}` | use-case | 値オブジェクト | — |
| `orchestration::event_store_impl`（+ `schema`） | adapter `core-interface-adapter` | C6 の 3 テーブル、Tx、`within_write_transaction`、open / 初期化 | `rusqlite`、`canon-json`、`clock` |
| `orchestration::wire`（`event_wire` / `state_wire`） | adapter | 符号化・復号（3 段検査の 1・2 段） | `serde` / `serde_json`、`core-domain` |
| `orchestration::store_path` | adapter | `StorePath::of` | `core-domain::workspace::SpaceName` |
| `orchestration::workflow_execution_repository_impl` | adapter | `find_by_id(&self)` / `store(&mut self)`（`EventStoreImpl` を直接所有） | 上記 |
| `orchestration::memory::{in_memory_event_store, workflow_execution_repository}` | adapter（`memory/`） | テストダブル（同じ契約） | — |
| `clock`（既存） | adapter 機構モジュール | `updated_at` の供給、Fake | — |
| `core-domain::orchestration::{IntentId(UUIDv7), WorkflowExecutionState, StateError}` | domain | 是正・改名 | — |
| `core-domain::workspace::IntentDirName` | domain | 新設 | — |
| `formal/orchestration/journal_protocol.qnt` + `tests/conformance/fixtures/journal_protocol/` | formal | 協定モデルと ITF fixture | Quint 0.32.0 |
| `modules/core/interface-adapter/tests/journal_protocol_conformance.rs` | adapter tests | ITF 再生（InMemory + フェイク投影） | — |
| `scripts/quint-gate.sh` / `scripts/coverage.sh` | scripts | ゲート更新（journal_protocol / TOLERANCE 0.01） | — |
| `tools/lint`（`reap-decision-locality` 削除） | lint | ルール表・README 同期 | — |

## 2. 境界と隔離

- ドメインは serde / rusqlite / tokio を知らない（`core-domain` の依存は不変）。ワイヤ構造体は adapter に閉じ、`pub(crate)`。
- ユースケース層はポートと値だけ（実装依存なし — `core-use-case` の `Cargo.toml` に `core-interface-adapter` は無い: E0432 で機械強制）。
- Repository 実装は `EventStoreImpl` を**直接所有**する。可変操作は `&mut self`、読取は `&self` であり、内部可変性は持たない（正本 `coding-rules/interior-mutability.md` / `command-query-separation.md`、オーナー裁定 2026-08-23）。
- Clock は機構（Gateway ではない）。ProcessProbe は退役。

## 3. 障害ドメインとブラストラディウス

| 障害 | 影響範囲 | 封じ込め |
|---|---|---|
| ストア破損 | 当該 space の全 intent（1 DB） | `Corrupt` で中断。投影は再生成可能（U4）。DB のバックアップ / 復元は利用者運用 |
| 競合 | 当該コマンド 1 回 | rollback + Conflict、再試行はユースケース |
| Busy 超過 | 当該コマンド 1 回 | `Io(WouldBlock)` で中断 |
| 依存の脆弱性（rusqlite / tokio） | ビルド全体 | `cargo audit`（CI）、固定版 |

## 4. テストの配置（NFR2.x）

| 種別 | 場所 | 内容 |
|---|---|---|
| ユニット（インライン `#[cfg(test)]`） | use-case 値・エラー、domain `IntentId` / `IntentDirName` / `WorkflowExecutionState`、adapter `wire` / `store_path` / `schema` | parse 受理・拒否、Display の材料、DDL 突合（`PRAGMA table_info`） |
| 契約テスト（ジェネリック） | adapter `tests/workflow_execution_repository_contract.rs` | InMemory / SQLite 両実装: ラウンドトリップ・NotFound・Conflict・Corrupt 各原因・events_after 順序・checkpoint 単調性・within_write_transaction 直列化（2 接続） |
| PBT | adapter `wire`（インライン） | encode→decode 恒等、バイト決定性、`PROPTEST_RNG_SEED` 固定 |
| クラッシュ再構成 | adapter tests | store 後に接続を捨て、新接続で find_by_id → 同値 |
| ITF 準拠 | adapter `tests/journal_protocol_conformance.rs` | fixtures ≥ 6、全アクション網羅 |
| 既存スイート | domain / adapter | IntentId リテラル置換・State 改名の追随のみ（engine_loop ITF / ゴールデン / WorkflowDefinitionRepository） |
| ゲート | `scripts/quint-gate.sh`（journal_protocol）、`scripts/coverage.sh`（90% 床、TOLERANCE 0.01）、`cargo lint` 自己テスト、`cargo audit` | CI 4 ジョブ + CI Success |

## 5. Infrastructure Design への橋渡し

- 本 Unit は CLI ライブラリ層。インフラ設計（配布・CI）は U10 で扱い済み。composition root（U7）が `StorePath::of` と Clock を配線する。
