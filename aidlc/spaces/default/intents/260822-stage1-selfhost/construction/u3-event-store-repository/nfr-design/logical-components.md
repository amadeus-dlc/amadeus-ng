# logical-components — U3 イベントストアと IntentExecutionRepository（`u3-event-store-repository`）

> NFR Design（Construction 3.3）成果物（Unit: U3、kind: library）。**改訂履歴**: 初版 2026-08-23（B5 着手前）→ 部分失効注記 2026-08-27 →
> **再走 2026-09-07（本版、Modify）**: 現行コード（`modules/core/command/{domain,use-case,interface-adapter}/`、`modules/app/aidlc/tests/`、
> `formal/`）を実測して全面更新した。出典: `../functional-design/functional-spec.md`（§1 所有表、§7 検証範囲）、
> `../nfr-requirements/security-requirements.md`（NFR2.x / NFR3.x / NFR4.x）、`../nfr-requirements/tech-stack-decisions.md`（§3 未決）、
> `security-design.md`（同ディレクトリ）、確認事項 `nfr-design-questions.md`（P7 / P8 / P11、Looks correct）。
> 用語: **RMU** = read-model-updater（U4）、**DTO** = 保存・復元用の写し、**ファサード** = `mod.rs` の `pub use` 列挙（それ以外は private）。

## 1. コンポーネント一覧

| コンポーネント | 層 / クレート | 責務 | 依存（実測 `Cargo.toml`） |
|---|---|---|---|
| `orchestration::port::intent_execution_repository`（`IntentExecutionRepository` — `find_by_id(&self)` / `store(&mut self)`、AFIT） | use-case `core-command-use-case` | ポート trait（C3。版と通番は `usize`） | `core-command-domain`、`chrono`。dev: `tokio` |
| `orchestration::port::repository_error`（`RepositoryError<Id>` — `NotFound` / `Conflict` / `Io` / `Corrupt { id, seq_nr, source }`） | use-case | 材料のみのエラー 1 本（`PartialEq` なし、`Corrupt` の分類は `source` 連鎖） | — |
| `orchestration::intent_execution_repository_impl`（`IntentExecutionRepositoryImpl<S>`、`open` / `in_memory` / `with_snapshot_strategy` / `reopened` / `path`、私有 `CorruptDetail` 6 変種、`envelope` / `read_error` / `write_error` / `stored_version`） | adapter `core-command-interface-adapter` | 本家ストア `S` の単一所有、基底 + 差分の再構成、書込前 ID 照合、エラー写像 | `event-store-adapter-rs`（`sqlite`）、`rusqlite`、`serde` / `serde_json`、`chrono`。dev: `tempfile`、`tokio` |
| `orchestration::snapshot_strategy`（`SnapshotStrategy::every(NonZeroUsize)` / `wants_snapshot`、既定 10） | adapter | 基底の書き直し間隔（値の確定は U7） | — |
| `orchestration::store_failure`（`io_kind` / `io_kind_of_source`、`pub(crate)`） | adapter | rusqlite の code → `std::io::ErrorKind` の写像 1 か所。RMU にも同名の写像があるが**意図的な複製**（共有すると command 側が RMU を `Cargo.toml` に書くことになり cqrs-boundaries 違反） | `rusqlite` |
| `orchestration::dto`（32 エントリ — 型引数 3 型 `IntentExecutionDto` / `IntentExecutionEventDto` / `IntentExecutionAggregateKeyDto`、イベント変種 DTO 16 種、`DtoDecodeError`、`dto_vocabulary`、`tests.rs`。同ディレクトリの `Intent*` / `WorkflowDefinition*` DTO は兄弟 Repository の所有） | adapter | 永続化表現の正本（本家 serde が格納）、`to_domain` で検査付きドメイン構築へ変換 | `serde`、`core-command-domain` |
| `orchestration::kinds_codec` | adapter | `produces_kinds` の順序保存 JSON map codec。利用者は `WorkflowDefinitionDto`（兄弟 Repository の DTO、実測 `workflow_definition_dto.rs:92`）で U3 の DTO は使わない — 同じ `dto/` に同居する共有機構として記す | `serde` |
| `orchestration::mod.rs` ファサード | adapter | 実装 mod は private、公開は `pub use`（`IntentExecutionRepositoryImpl` / `SnapshotStrategy` / DTO 群 / `IntentExecutionSqliteStore` / `IntentExecutionMemoryStore` 別名） | — |
| `orchestration::intent_execution`（`IntentExecution::new` / `replay` / `apply_event` / `with_version` / `version` / `seq_nr`） | domain `core-command-domain` | 検査付き完全コンストラクタ（Err）と再生（panic 境界）。永続化知識なし | `chrono`、`uuid v7`、`core-infrastructure`。**serde / ESA なし**（機械強制） |
| `workspace::store_path`（`StorePath::for_space`）/ `workspace::intent_dir_name`（`IntentDirName`） | domain | ストアの場所の導出（生の `PathBuf` を受けない）、intent ディレクトリ名の文法 | `workspace::SpaceName` |
| `formal/orchestration/journal_protocol.qnt` + `tests/conformance/fixtures/journal_protocol/`（8 トレース） | formal | 協定モデル（1 集約・writer 2・投影 1・every(1)、不変条件 8 / witness 4）と ITF fixture | Quint 0.32.0（Node 22） |
| `modules/app/aidlc/tests/journal_protocol_conformance.rs` | app tests | ITF 再生 — `IntentExecutionRepositoryImpl`（every(1) 明示）+ RMU `JournalReaderImpl` + 実 RMU 投影の結合（書込側と読取側を同じ DB で突合できる唯一の層） | 両クレート |
| `modules/app/aidlc/tests/crash_reconstruction_test.rs`（5 件） | app tests | 接続を捨てて新接続で再構成、COMMIT 前クラッシュの無痕、版の持ち越し、開閉の反復 | — |
| `scripts/quint-gate.sh` / `scripts/coverage.sh`（`TOLERANCE=0.01`、`PROPTEST_RNG_SEED`） | scripts | CI ゲート（現行値。本 Unit で変更しない） | — |
| 本家 `event-store-adapter-rs = "=3.0.0"`（`EventStore` / `EventEnvelope` / `SnapshotEnvelope`、`EventStoreForSqlite` / `EventStoreForMemory`） | 上流の境界づけられたコンテキスト | `journal` / `snapshot` の DDL・Tx・CAS・版の採番 | Conformist（契約を 1 文字も変えない） |

U3 の外にある隣接コンポーネント（参照のみ）: RMU `core-read-model-updater::orchestration::{journal_reader, journal_reader_impl}`
（別接続、`busy_timeout` 既定 5000ms、チェックポイント表、スキーマガード — U4）、adapter の `Clock` / `SystemClock` / `FakeClock`
（U3 の経路では使わない — 封筒の `occurred_at` は集約の `last_updated_at`。他クレートからの利用も `modules/` 配下 0 件、実測 grep）、
合成ルート `modules/app/aidlc/src/runtime.rs`（`StorePath::for_space` と `IntentExecutionRepositoryImpl::open` の配線 — U7）。

## 2. 境界と隔離

- **ドメインは永続化知識から中立**: `core-command-domain` の `Cargo.toml` に serde / ESA が無く、DTO と `to_domain` はアダプタに閉じる。
  検査付き構築（`IntentExecution::new`）はドメインの公開 API であり、アダプタはドメインの非公開表現に触れない（BR2.8、
  domain-persistence-neutrality / abstract-data-type）。旧「domain に serde と chrono が入った（B6）」は B12 改訂 9 で解消済み。
- **ユースケース層はポートと値だけ**: `core-command-use-case` の `Cargo.toml` に `core-command-interface-adapter` は無い（dev にも書けない —
  DIP のクレート分離）。単体テスト専用の `pub(crate)` trait フェイク 4 つは use-case の `#[cfg(test)]` に住み、公開のインメモリ実装は
  `IntentExecutionRepositoryImpl::in_memory()`（本家 memory バックエンド、テストダブルではない）だけである（C3 現行化、gateway-taxonomy §5）。
- **Repository は本家ストアを単一所有**: 可変操作は `&mut self`、読取は `&self`、内部可変性なし（interior-mutability.md）。`reopened()` は
  同じストアを指す別ハンドルであり、直列化の実体は本家 CAS（security-design §3）。
- **コマンド側と RMU は相互独立**: コマンド側クレートの `Cargo.toml` に RMU は現れない。`store_failure` の写像は両側に意図的に複製する。
  RMU は専用 DTO と `JournalBatch` を使い、コマンド側の永続化 DTO を共有しない（functional-spec §4）。
- **モジュール可視性**: 実装ファイルの mod は private、公開はファサードの `pub use` のみ（module-visibility、`unreachable_pub` deny）。
  フィールドは private + アクセサ（`cargo lint` `no-public-fields`）。

## 3. 障害ドメインとブラストラディウス

| 障害 | 影響範囲 | 封じ込め |
|---|---|---|
| ストア破損（6 原因） | 当該 space の全 intent（1 DB `.aidlc-store.sqlite`） | `Corrupt` で当該コマンドを中断。投影は U4 がジャーナルから冪等に再生成。DB のバックアップ / 復元は利用者運用 |
| クラッシュ境界（壊れた歴史の再生） | 当該コマンド 1 回（プロセス終了） | ストアは不変。書込時検査済みの歴史だけが入る前提の破れとして調査 |
| 競合 | 当該コマンド 1 回 | 本家 CAS で拒否 + `Conflict`。再試行はユースケース（1 回）、2 回続けば exit 1（C1） |
| Busy / Locked | 当該コマンド 1 回 | `Io(WouldBlock)` で即時中断。待たない（本家接続に `busy_timeout` 不可）。並行モデルは U7 |
| 親 dir 欠落・権限・ディスク | 当該コマンド 1 回 | `Io { kind, path }`。親 dir は作らない |
| 依存の脆弱性（ESA / rusqlite / serde / chrono / 推移 thiserror・libsqlite3-sys） | ビルド全体 | `cargo audit`（CI advisory）、完全固定 `=3.0.0`、`Cargo.lock` |
| RMU 側の障害（投影・チェックポイント） | U4 の所有 | 本 Unit の写像・テストに含めない（ITF 適合テストが結合面だけを突合） |

## 4. テストの配置（NFR2.x、実測 2026-09-07）

| 種別 | 場所 | 内容 | 件数 |
|---|---|---|---|
| 契約テスト（ジェネリック、`contract_tests!`） | adapter `tests/intent_execution_repository_contract.rs` | 両バックエンド同一関数群: 独立オープン・reopen の反映・ラウンドトリップ・NotFound・genesis 版採番・genesis 二重 Conflict・並行再水和 Conflict・古い版の Conflict・再水和版の成功・genesis 版 ≠ 0 の契約違反・別実行イベントの書込前拒否 | 11 × 2 = 22 |
| 実装固有 | adapter `tests/intent_execution_repository_impl_test.rs` | 基底 + 差分（初回必須・間隔 N・古い基底・版維持）、破損境界（基底欠落・payload / state 改竄・未知型・別実行 payload・foreign manifest・通番飛び・不変条件違反・閉集合外の綴り）、クラッシュ（未知ステージ）、I/O（親 dir・壊れたファイル・表欠落・パス報告） | 23 |
| 本家適合（両バックエンド） | adapter `tests/upstream_event_store_conformance.rs` | DTO が本家契約を満たし、snapshot + replay の往復でドメインへ戻る | 5 × 2 |
| インライン `#[cfg(test)]` | adapter `intent_execution_repository_impl.rs` / `store_failure.rs` / `snapshot_strategy.rs` / `dto/tests.rs` | impl 10 件 = `CorruptDetail` の表示・連鎖（2）、揮発ストアに path が無い、封筒の材料が適用後集約から来る、第 2 集約の封筒識別、基底が journal 先頭でないこと、読取 / 書込失敗の kind 別写像（2）、Conflict の actual、reopened の共有 / `store_failure` 4 = rusqlite code の写像・非 SQLite・boxed の分類 / `snapshot_strategy` 2 = 既定 10 と任意間隔 / `dto/tests.rs` 29 = DTO の往復・拒否 | 10 / 4 / 2 / 29 |
| クラッシュ再構成 | app `tests/crash_reconstruction_test.rs` | 新接続で同値、ジャーナル全読、放棄 Tx の無痕、開閉の反復、版の持ち越し | 5 |
| ITF 適合 | app `tests/journal_protocol_conformance.rs` | fixture 8 トレース、every(1) 明示、load / store_ok / store_conflict / catchup / crash / idle を駆動し journal 長・snapshot 版 / 通番・checkpoint を照合 | 8 トレース |
| ゲート | `scripts/quint-gate.sh`（journal_protocol typecheck / 不変条件 8 / witness 4）、`scripts/coverage.sh`（90% 床、TOLERANCE 0.01、シード固定）、`cargo lint`、`cargo audit`（advisory） | CI 7 ジョブ（aidlc-distribution / check / quint / coverage / audit / review-thread-resolution / ci-success）。`ci-success` の `needs` は `audit` を除く | — |

配置規則: ユニット層を厚く（インライン + 実装固有）、結合層は境界ごと（契約・本家適合）、E2E は最小（app の ITF・クラッシュ）— team.md の
テストピラミッド。新規テストは red → green の順で PR のコミット列に残す（NFR2.1）。

## 5. Infrastructure Design への橋渡し（U7）

- 本 Unit は CLI ライブラリ層。配布・CI は U10 で扱い済み。合成ルート（U7、`runtime.rs`）が `StorePath::for_space(aidlc_root, space)` で
  場所を導き、`IntentExecutionRepositoryImpl::open` をコマンド単位で 1 回開く（現行 8 か所）。`SnapshotStrategy` は既定 10 のまま
  配線されていない（`grep SnapshotStrategy modules/app/aidlc/src/` 0 件）。
- U7 の裁定事項（tech-stack-decisions §3 / contract-summary §4）: (1) 複数プロセスの並行モデル（本家接続に `busy_timeout` 不可、
  `WouldBlock` 即時）と `reopened()` 複数ハンドルの扱い、(2) 登録簿 `intents.json` の直列化（旧 `within_write_transaction` は退役、
  ADR-010 は登録簿を SQLite へ移す案を筋と記す）、(3) `SnapshotStrategy` 既定値の確定。本 Unit はいずれも先取りしない。

## 6. 前版からの変更（2026-09-07 再走）

- §1 を旧クレート名（`core-use-case` / `core-interface-adapter` / `core-domain`）・旧型（`WorkflowExecutionRepository` / `CorruptCause` /
  `wire` / `memory` / `event_store_impl` / adapter 内 `journal_reader_impl`）から、現行の `core-command-*` 3 クレート + formal + app tests へ
  全面差し替え。`store_failure` / `kinds_codec` / DTO 32 エントリ / ファサードを追加。RMU と Clock を「U3 の外」に明示。
- §2 の「domain は serde と chrono を持つ」を撤回（現行は serde なし・機械強制）。use-case の trait フェイク 4 つを明記。
- §3 に クラッシュ境界・RMU 側障害の行を追加、ピンを `=3.0.0` へ。
- §4 を実測件数（22 / 23 / 5×2 / 10+4+2+29 / 5 / 8）と現行の置き場（ITF は app）へ。PBT 行を削除（アダプタに無い）。CI 4 → 7 ジョブ。
- §5 に U7 裁定 3 件と配線の実測（open 8 か所、SnapshotStrategy 未配線）を追加。
