# tech-stack-decisions — U3 イベントストアと IntentExecutionRepository（`u3-event-store-repository`）

> NFR Requirements（Construction 3.2）成果物（Unit: U3、kind: library）。**改訂履歴**: 初版 2026-08-23（Bolt B5 着手前 — 自前 SQLite ドライバ + ローカル
> `EventStore` trait 前提）→ **再走 2026-09-07（本版、Modify）**: ADR-010（B6 / B7、2026-08-27〜29）で本家 `event-store-adapter-rs` へ乗り換えた現行を
> `origin/main` = `f2b6b6a9` で実測して全面更新した。
> 出典: `Cargo.toml`（`[workspace.dependencies]` / `[workspace.lints]`）、`Cargo.lock`（実測版）、`modules/core/command/{domain,use-case,interface-adapter}/Cargo.toml`、
> `rust-toolchain.toml`、`.github/workflows/ci.yml`、`security-requirements.md`（NFR2.x / NFR4.x）、`../functional-design/rules.md`（BR2.1 / BR2.5 / BR2.7 / BR3.3 / BR3.5）、
> `../functional-design/functional-spec.md`（§1 所有表・§4 永続化表現）、ADR-010、確認事項 `nfr-requirements-questions.md`（P7〜P12）。

## 1. 選定

| 項目 | 決定（現行） | 根拠 | 代替案（不採用の理由） |
|---|---|---|---|
| イベントストア | 本家 `event-store-adapter-rs = "=3.0.0"`（完全固定、`sqlite` feature）。`IntentExecutionRepositoryImpl<S>` の型引数 `S` が `EventStoreForSqlite`（`open`）/ `EventStoreForMemory`（`in_memory`）を選ぶ | ADR-010（Conformist — 借り物の契約を 1 文字も変えない、upstream-contracts.md）。v3 は `Event` / `Aggregate` trait を廃し `EventEnvelope` / `SnapshotEnvelope` の payload 純化。我々のドメイン型が実装する本家 trait は `AggregateId` のみ | ローカル同形 trait（ADR-006 = 初版の選定）: 同形性の手動維持が劣化した実例（`usize` → `u64` 具体化 — upstream-contracts.md「失敗の実例」）。自前ストア: B6 で 5,480 行を削除 |
| SQLite ドライバ | `rusqlite = 0.40.2`（`bundled`）— 本家と同じ crate・同じ feature を共有し、本 Unit では `store_failure`（本家が `Box<dyn Error>` に包む SQLite の失敗を開けて `ErrorCode` を分類する）にだけ使う | 同梱 SQLite 本体が 1 つに統合され、実行環境の libsqlite3 の版差が入らない（`interface-adapter/Cargo.toml` のコメント、実測） | 自前の DDL・Tx・PRAGMA を持つ理由が無い（接続は本家所有） |
| SQLite 設定 | 本家の既定に従う。我々は PRAGMA を発行しない（実測 `interface-adapter/src/` に `PRAGMA` / `busy_timeout` 0 件）。`busy_timeout` は本家接続に設定できず、RMU 側の別接続（`JournalReaderImpl::open_with_busy_timeout`、既定 5000ms、U4 所有）にだけある | Conformist。別プロセスの並行書込は `SQLITE_BUSY` → `Io { kind: WouldBlock }`（NFR4.7） | 初版 P2（`synchronous` / `journal_mode` / `busy_timeout` の自前設定）: 接続を持たないので実現不能 — 失効 |
| async | ポートは `async fn`（AFIT）。`tokio`（`rt`, `macros`）は use-case / adapter の **dev-dependency**（`#[tokio::test]`）。実行時ランタイムは合成ルート（U7）の `current_thread`。Repository 内で `spawn` / `spawn_blocking` はしない（await 直列、`Send` 不要） | ワンショット CLI、ms 単位のブロッキング。内部可変性なし（`store` は `&mut self` — interior-mutability.md） | `dyn` ポート: use-case-rules §2（静的束縛が既定） |
| 永続化表現 | `serde` / `serde_json`（`preserve_order` / `float_roundtrip`）はアダプタのみ。DTO 3 型（`IntentExecutionDto` / `IntentExecutionEventDto` / `IntentExecutionAggregateKeyDto`、`dto/` に 1 型 1 ファイル）。payload のバイトは本家の `serde_json::to_vec` が書く — **canon-json は使わない**（契約 JSON の射程は upstream 観測面のみ、BR2.5） | domain-persistence-neutrality.md（`core-command-domain` の `Cargo.toml` に serde / ESA が無い — 機械強制、実測） | 初版「payload は canon-json でバイト決定」: 本家が書くため我々は呼んでいない — 失効 |
| 時刻・識別子 | `chrono 0.4`（`DateTime<Utc>`、`std` / `clock` / `serde`）— 封筒の `occurred_at` は適用後集約から組む（BR2.6）。`uuid 1.26`（`v7`、domain）— `IntentExecutionId` / `IntentExecutionEventId` | ADR-010 決定 3、aggregate-commands.md（イベント ID は集約のコマンド内で採番） | Repository が Clock を持つ（初版）: 時刻を再採番しない |
| 形式検証 | Quint 0.32.0（Node 22 経由）— `formal/orchestration/journal_protocol.qnt`（不変条件 8 / witness 4）、ITF fixture 8 トレース `tests/conformance/fixtures/journal_protocol/`、再生 `modules/app/aidlc/tests/journal_protocol_conformance.rs`（`SnapshotStrategy::every(1)` 明示 — 実測 `:226-227`） | ADR 0003、BR3.3 / BR3.5 | — |
| テスト | 契約テスト（`intent_execution_repository_contract.rs`、両バックエンドに同一関数群、22 件）+ 実装固有（`intent_execution_repository_impl_test.rs`、23 件）+ クラッシュ再構成（`modules/app/aidlc/tests/crash_reconstruction_test.rs`、5 件）— いずれも実測 PASS。`tempfile 3`（dev）で一時 DB。PBT はアダプタに置かない（`proptest` は domain の値オブジェクトのみ） | NFR2.1 / NFR2.2、BR2.7 | 自作 HashMap のインメモリダブル: 禁止（gateway-taxonomy §5 — `in_memory()` が唯一のインメモリ形） |
| ツールチェーン | `rust-toolchain.toml` = `1.95.0`（`rustfmt` / `clippy` / `llvm-tools`、profile minimal） | FR9.2 / NFR4（固定）。CI は `scripts/governance/toolchain-inputs.sh` で読む | floating stable: CI が突然赤になる（team.md） |

## 2. 依存の差分

本再走の Bolt では **依存を変えない**（NFR4.1）。現行の依存（実測 2026-09-07）を記録する。

| クレート | `[dependencies]`（実測） | 用途 |
|---|---|---|
| workspace `Cargo.toml` | `event-store-adapter-rs = "=3.0.0"`、`rusqlite = { version = "0.40.2", features = ["bundled"] }`、`serde 1`（derive）、`serde_json 1`（preserve_order / float_roundtrip）、`chrono 0.4`、`uuid 1.26`、`tokio 1`（rt / macros）、`proptest 1` | 一元管理 |
| `core-command-domain` | `chrono`、`uuid`（v7）。**serde / event-store-adapter-rs なし** | 永続化知識からの中立（Cargo.toml が機械強制） |
| `core-command-use-case` | `core-command-domain`、`chrono`。dev: `tokio` | ポート定義（AFIT）。アダプタは dev にも書けない（DIP のクレート分離） |
| `core-command-interface-adapter` | `core-command-use-case` / `core-command-domain` / `core-infrastructure`、`rusqlite`、`serde` / `serde_json`、`chrono`、`event-store-adapter-rs`（`sqlite`）。dev: `tempfile 3`、`tokio` | Repository 実装・DTO・エラー分類 |
| `tools/lint` | 依存変更なし（detached、独立 `Cargo.lock` は `cargo audit --file` の対象） | — |

`cargo audit` は CI `audit` ジョブ（workspace + `tools/lint`）で走る。**advisory**（`ci-success` の `needs` 外 — 実測 `ci.yml`）。

## 3. 未決（後続で確定 — いずれも U7 の裁定事項）

- 本家接続の `busy_timeout` が設定できない前提での**複数プロセスの並行モデル**（現状は単一プロセス前提、NFR4.7 — contract-summary §4）。
- 登録簿 `intents.json` の直列化（旧 `within_write_transaction` は退役。ADR-010 は「登録簿を SQLite へ移す」を筋と書く — 未決）。
- `SnapshotStrategy` の既定値 10 の確定（`snapshot_strategy.rs:31`「実際の値は合成ルート (U7) が確定する」。app 側の配線は現状無い — 実測 `grep SnapshotStrategy modules/app/aidlc/src/` 0 件）。

## 4. 前版からの変更（2026-09-07 再走）

- §1 を「自前 rusqlite ドライバ + ローカル trait + tokio 導入 + canon-json payload + md5 除去」から、本家 `event-store-adapter-rs = "=3.0.0"` を中核とする現行へ全面差し替え。代替案列を追加。
- §2 を「依存を変えない」+ 現行依存の実測表へ。旧「rusqlite / tokio の固定版は B5 着手時に確定」は解消（`Cargo.lock` に記録済み）。
- §3 の未決を U7 の 3 件へ更新（旧「PRAGMA 追加は非目標」は §1 の SQLite 設定行へ畳んだ）。
