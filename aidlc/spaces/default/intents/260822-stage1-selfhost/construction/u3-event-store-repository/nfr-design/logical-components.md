# logical-components — U3 SQLite EventStore と WorkflowExecutionRepository（`u3-event-store-repository`）

> NFR Design（Construction 3.3）成果物（Unit: U3、kind: library、Bolt: B5）。出典: `../functional-design/functional-spec.md`（§1 配置、§7 テスト）、`../nfr-requirements/
> security-requirements.md`（NFR2.x / NFR3.x）、`security-design.md`（同ディレクトリ）。

> ## ⚠ 部分失効（2026-08-27 / ADR-010・Bolt B6 — event-store-adapter-rs v2.0.0 へ乗り換え）
>
> 自前ストア（`event_store_impl` / `schema` / `wire` / `memory`）を前提にした行は失効した。
> **NFR4.1（依存最小化）は再検討が要る** — `chrono` と `serde` がドメイン層に入り、自前 ISO 8601 整形を
> 撤去したため（ADR-010 が明記）。**NFR3.5（登録簿の直列化）は未決へ差し戻し**（`within_write_transaction`
> が削除され代替が未定 — U7 で裁定）。`busy_timeout` も本家接続には設定できず**未決（U7）**。

## 1. コンポーネント一覧

| コンポーネント | 層 / クレート | 責務 | 依存 |
|---|---|---|---|
| `orchestration::ports`（`WorkflowExecutionRepository` / ~~`EventStore`~~ / `JournalReader`） | use-case `core-use-case` | ポート trait（C3。~~u64~~ → **`usize`**、2026-08-27 / ADR-010 で本家に復帰。ローカル `EventStore` は削除 — 正本は本家 crate） | `core-domain`、`event-store-adapter-rs` |
| `orchestration::errors`（`RepositoryError` / ~~`EventStoreError`~~ → `JournalReadError` / `CorruptCause`） | use-case | 材料のみのエラー（2026-08-27 / ADR-010: 改称。`CorruptCause` は 6 → 4 分類） | — |
| `orchestration::{GlobalSeqNr, ProjectionName}` | use-case | 値オブジェクト | — |
| ~~`orchestration::event_store_impl`（+ `schema`）~~ → **削除**（2026-08-27 / ADR-010） | adapter `core-interface-adapter` | ~~C6 の 3 テーブル、Tx、`within_write_transaction`、open / 初期化~~ → 本家 `EventStoreForSqlite` / `EventStoreForMemory` が担う | — |
| `orchestration::journal_reader_impl`（**新設** 2026-08-27） | adapter | 本家 `journal` を別接続で読む横断カーソル（rowid）、我々の `amadeus_projection_checkpoint` 表、スキーマガード | `rusqlite`、`core-use-case` |
| ~~`orchestration::wire`（`event_wire` / `state_wire`）~~ → **削除**（2026-08-27 / ADR-010） | adapter | ~~符号化・復号（3 段検査の 1・2 段）~~ → payload は本家が serde で書く。復号側の検査点は集約の `from_state()`（3 段検査の 3 段目）だけが残る | — |
| `orchestration::store_path` | adapter | `StorePath::for_space` | `core-domain::workspace::SpaceName` |
| `orchestration::workflow_execution_repository_impl` | adapter | `find_by_id(&self)` / `store(&mut self)`（~~`EventStoreImpl`~~ → **本家ストア `S`** を単一所有。2026-08-27 / ADR-010） | `event-store-adapter-rs` |
| ~~`orchestration::memory::{in_memory_event_store, workflow_execution_repository}`~~ → **削除**（2026-08-27 / ADR-010） | adapter（`memory/`） | ~~テストダブル（同じ契約）~~ → `WorkflowExecutionRepositoryImpl::in_memory()` が本家 memory バックエンドを内包する（テストダブルではない） | — |
| `clock`（既存） | adapter 機構モジュール | ~~`updated_at` の供給~~ → **現在利用者なし**（2026-08-27 / ADR-010: 時刻はイベントの `occurred_at` から来る。ユースケース着手時の注入シームとして残置）、Fake | — |
| `core-domain::orchestration::{IntentId(UUIDv7), WorkflowExecutionState, StateError}` | domain | 是正・改名 | — |
| `core-domain::workspace::IntentDirName` | domain | 新設 | — |
| `formal/orchestration/journal_protocol.qnt` + `tests/conformance/fixtures/journal_protocol/` | formal | 協定モデルと ITF fixture | Quint 0.32.0 |
| `modules/core/interface-adapter/tests/journal_protocol_conformance.rs` | adapter tests | ITF 再生（~~InMemory~~ → **`WorkflowExecutionRepositoryImpl` + `JournalReaderImpl`**、2026-08-27 / ADR-010 + フェイク投影）。**モデルは 1 文字も変えずに通った** | — |
| `scripts/quint-gate.sh` / `scripts/coverage.sh` | scripts | ゲート更新（journal_protocol / TOLERANCE 0.01） | — |
| `tools/lint`（`reap-decision-locality` 削除） | lint | ルール表・README 同期 | — |

## 2. 境界と隔離

- ~~ドメインは serde / rusqlite / tokio を知らない（`core-domain` の依存は不変）。ワイヤ構造体は adapter に閉じ、`pub(crate)`。~~ → **部分失効（2026-08-27 / ADR-010）**: 本家 trait が `Serialize` / `Deserialize` / `DateTime<Utc>` を境界に要求するため、**ドメインは serde と chrono を持つ**（rusqlite / tokio は依然として知らない）。ワイヤ構造体は削除。ただし集約の復号は memento を経由するので検査点は 1 か所のまま。**NFR4.1（依存最小化）の再検討が要る** — 自前 ISO 8601 整形の存在意義が変わった（ADR-010 が明記）。
- ユースケース層はポートと値だけ（実装依存なし — `core-use-case` の `Cargo.toml` に `core-interface-adapter` は無い: E0432 で機械強制）。
- Repository 実装は ~~`EventStoreImpl`~~ → **本家のイベントストア（型引数 `S`）** を**単一所有**する（2026-08-27 / ADR-010）。可変操作は `&mut self`、読取は `&self` であり、内部可変性は持たない（正本 `coding-rules/interior-mutability.md` / `command-query-separation.md`、オーナー裁定 2026-08-23。本家 `EventStore` のレシーバとそのまま揃う）。
- Clock は機構（Gateway ではない）。ProcessProbe は退役。（2026-08-27 補足: ストアが Clock を持たなくなったため現在利用者はいない）

## 3. 障害ドメインとブラストラディウス

| 障害 | 影響範囲 | 封じ込め |
|---|---|---|
| ストア破損 | 当該 space の全 intent（1 DB） | `Corrupt` で中断。投影は再生成可能（U4）。DB のバックアップ / 復元は利用者運用 |
| 競合 | 当該コマンド 1 回 | rollback + Conflict、再試行はユースケース |
| Busy 超過 | 当該コマンド 1 回 | `Io(WouldBlock)` で中断。**2026-08-27 / ADR-010: 本家の接続に `busy_timeout` を設定できないため、待たずに即 `SQLITE_BUSY` になる**（従来は 5000ms 待った）— BR2.1 の実質的な後退であり、単一プロセス前提の現状は受容して **U7 の並行モデルと併せて再裁定**する |
| 依存の脆弱性（~~rusqlite~~ / tokio / **event-store-adapter-rs**） | ビルド全体 | `cargo audit`（CI）、固定版（2026-08-27: `event-store-adapter-rs` は `=2.0.0` の**完全固定** — 本家スキーマに結合しているため。`rusqlite` は `JournalReaderImpl` の別接続用に残る） |

## 4. テストの配置（NFR2.x）

| 種別 | 場所 | 内容 |
|---|---|---|
| ユニット（インライン `#[cfg(test)]`） | use-case 値・エラー、domain `IntentId` / `IntentDirName` / `WorkflowExecutionState`、adapter ~~`wire`~~ / `store_path` / ~~`schema`~~ → `journal_reader_impl`（2026-08-27 / ADR-010） | parse 受理・拒否、Display の材料、~~DDL 突合（`PRAGMA table_info`）~~ → **本家 DDL のスキーマガード突合** |
| 契約テスト（ジェネリック） | adapter `tests/workflow_execution_repository_contract.rs` | ~~InMemory / SQLite 両実装~~ → **本家 memory / SQLite の両バックエンド**（2026-08-27 / ADR-010。実装コードは同一）: ラウンドトリップ・NotFound・Conflict・Corrupt 各原因・events_after 順序・checkpoint 単調性・~~within_write_transaction 直列化（2 接続）~~ → **失効**（口ごと削除。U7 で裁定） |
| PBT | ~~adapter `wire`（インライン）~~ → **domain（集約の serde 往復と改竄拒否）**（2026-08-27 / ADR-010: ワイヤ構造体ごと削除） | encode→decode 恒等、~~バイト決定性~~（本家の serde なので我々の検証対象ではない）、`PROPTEST_RNG_SEED` 固定 |
| クラッシュ再構成 | adapter tests | store 後に接続を捨て、新接続で find_by_id → 同値 |
| ITF 準拠 | adapter `tests/journal_protocol_conformance.rs` | fixtures ≥ 6、全アクション網羅 |
| 既存スイート | domain / adapter | IntentId リテラル置換・State 改名の追随のみ（engine_loop ITF / ゴールデン / WorkflowDefinitionRepository） |
| ゲート | `scripts/quint-gate.sh`（journal_protocol）、`scripts/coverage.sh`（90% 床、TOLERANCE 0.01）、`cargo lint` 自己テスト、`cargo audit` | CI 4 ジョブ + CI Success |

## 5. Infrastructure Design への橋渡し

- 本 Unit は CLI ライブラリ層。インフラ設計（配布・CI）は U10 で扱い済み。composition root（U7）が `StorePath::for_space` と Clock を配線する。
