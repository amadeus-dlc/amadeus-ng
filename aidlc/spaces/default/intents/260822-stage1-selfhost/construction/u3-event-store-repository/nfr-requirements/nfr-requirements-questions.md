# nfr-requirements-questions — U3 イベントストアと IntentExecutionRepository（`u3-event-store-repository`）

> NFR Requirements（Construction 3.2）の質問票（Unit: U3、kind: library）。**改訂履歴**: 初版 2026-08-23（Bolt B5 着手前、P1〜P6、Looks correct）→
> **再走 2026-09-07（本版、Modify）**: unit-major 再走で既存成果物 3 点（security-requirements / tech-stack-decisions / traceability）を現行コード
> `origin/main` = `f2b6b6a9` で実測し、内容の大半が失効していたためオーナー回答（会話、AskUserQuestion）**Modify** で書き直す。前提 P1〜P6 は
> 履歴として残し、現行の前提を P7〜P12 として追記した。
>
> 出典: `../functional-design/functional-spec.md`（§1 責務・§2 契約・§3 保存と再構成・§5 検証モデル・§7 検証範囲、2026-09-05 是正版・
> 2026-09-07 再レビュー READY）、`../functional-design/rules.md`（BR1.1〜BR5.2）、`../../../inception/requirements-analysis/requirements.md`
> （NFR1〜NFR5、FR1.2 / FR1.3）、`../../../inception/contract-design/contract-summary.md`（C3 / C6 — 2026-09-07 現行化）、
> `Cargo.toml` / `Cargo.lock` / `rust-toolchain.toml` / `.github/workflows/ci.yml` / `scripts/coverage.sh`（実測）、`aidlc/spaces/default/memory/team.md`
> （Testing Posture / CI ゲート / サプライチェーン裁定）。
>
> **質問なし。** 基盤選択（本家 event-store-adapter-rs への乗り換え・スナップショット + 差分再生・ロック退役・DTO の所有）は ADR-001 / 007 / 010 と
> 2026-09-05 のオーナー裁定で確定済みであり、本 Unit に残る未決（`busy_timeout` / 複数プロセス並行・登録簿の直列化・`SnapshotStrategy` 既定値の確定）は
> いずれも U7（合成ルート）の裁定事項として contract-summary §4 に載っている。次の前提を確認して成果物へ進む。

## 前提（2026-08-23 初版 — 履歴）

- P1. ~~**依存追加は 2 つ**: `rusqlite`（`bundled`）と `tokio`（`rt` + `macros`）。`md5` は退役と同時に除去。~~ → **失効**（ADR-010 / B6・B7、2026-08-27〜29）。
  自前 SQLite ストアは削除され、本家 `event-store-adapter-rs` に乗り換えた。現行は P7。
- P2. ~~**安全側の SQLite 設定**: `synchronous` 既定、`journal_mode` 既定、`busy_timeout` 5000ms、`foreign_keys` 不要。~~ → **失効**。本家のイベントストア接続は
  我々が PRAGMA を発行できない（実測: `interface-adapter/src/` に `PRAGMA` / `busy_timeout` は 0 件）。`busy_timeout` 5000ms は RMU 側の別接続
  （`JournalReaderImpl::open_with_busy_timeout`、U4）にだけ残る。現行は P8。
- P3. **脅威モデルはローカル単一ユーザ**: ストアは `.gitignore` 配下で、改竄・欠損は `Corrupt` として検出して拒否する（panic しない）。暗号学的完全性は
  要求しない。→ **有効**（分類の中身は P9 で現行化）。
- P4. ~~**品質ゲート**: TDD、PBT（`PROPTEST_RNG_SEED` 固定）、ITF 準拠（fixture ≥ 6）、クラッシュ再構成テスト、カバレッジ 90% 床、clippy 全ルール deny、
  `cargo lint` 自己テスト（`reap-decision-locality` 削除後）。~~ → **一部失効**。PBT はアダプタには無い（`proptest` の利用は `core-command-domain` の
  workflow_definition 値オブジェクトのみ — 実測）。`reap-decision-locality` は退役済み。現行は P10。
- P5. **性能は非目標**（NFR5）: 数値目標なし。→ **有効**（P11 で現行の上限を更新）。
- P6. **ログ・秘密情報**: ストア層はログ出力を持たない。ペイロードの人間入力は逐語のまま保存。環境変数・資格情報を読まない。→ **有効**（P12 で実測を添える）。

## 再走の前提（2026-09-07 — 現行）

- P7. **依存は変えない**: 本 Unit の永続化中核は `event-store-adapter-rs = "=3.0.0"`（完全固定、`sqlite` feature で `EventStoreForSqlite`、
  `in_memory()` は `EventStoreForMemory` — 実測 `Cargo.toml:34` / `interface-adapter/Cargo.toml`）。`rusqlite = 0.40.2`（`bundled`）は本家と同じ
  crate を共有し、本家が `Box<dyn Error>` に包む SQLite の失敗を開けて分類する用途（`store_failure`）にだけ使う。`serde` / `serde_json` / `chrono`
  はアダプタが所有し、`core-command-domain` の `Cargo.toml` には serde / ESA が現れない（domain-persistence-neutrality の機械強制 — 実測）。
  `tokio` は use-case / adapter の dev-dependency（`#[tokio::test]`）。本再走で `Cargo.toml` / `Cargo.lock` は 1 行も変えない。
- P8. **Tx・CAS・接続・DDL は本家が所有**: `store` は `persist_event` / `persist_event_and_snapshot` に委譲し、期待版は `aggregate.version()`
  （BR1.3）。我々は接続も Tx も PRAGMA も持たない。別プロセスの並行書込は本家接続の `SQLITE_BUSY` になり、単一プロセス前提を受容する
  （複数プロセスの並行モデルと登録簿 `intents.json` の直列化は U7 の裁定 — contract-summary §4）。
- P9. **脅威モデルの現行化**: 破損・欠損・順序違反・別実行 payload・foreign manifest・ストア復号失敗は `RepositoryError::Corrupt { id, seq_nr, source }`
  で拒否し、原因はアダプタ私有 `CorruptDetail` 6 変種（MissingSnapshot / ForeignManifest / SequenceGap / Undecodable / StoreDeserialization /
  WriteContract — 実測 `intent_execution_repository_impl.rs`）を `Error::source` 連鎖で運ぶ（ポート契約には分類を載せない — BR1.5）。DTO として
  成立した壊れた遷移（未知ステージ等）は `IntentExecution::replay` のクラッシュ境界であり、`Corrupt` に畳むとは約束しない（aggregate-commands 裁定
  2026-08-30）。書込前 ID 照合（`event.aggregate_id() != aggregate.id()` → `Corrupt(WriteContract)`）は 2026-09-05 R-07 是正で追加済み。
- P10. **品質ゲートの現行化**: TDD（契約テスト 22 件を memory / SQLite の両バックエンドに同一に課す + 実装固有 23 件 = 45 件、実測 PASS）、
  ITF 適合（`journal_protocol_conformance.rs`、`SnapshotStrategy::every(1)` 明示、fixture 8 トレース — 実測 `ls tests/conformance/fixtures/journal_protocol`）、
  クラッシュ再構成（`crash_reconstruction_test.rs` 5 件、実測 PASS）、Quint 不変条件 8 / witness 4、カバレッジ絶対 90% 床 + 相対ゲート
  `TOLERANCE=0.01`（`PROPTEST_RNG_SEED=20260823` 固定 — 実測 `scripts/coverage.sh:40-43`、旧 pending-revision 1 は解消済み）、workspace lints deny
  （`unwrap_used` / `expect_used` / `indexing_slicing` / `panic` / `print_stdout` / `dbg_macro` … 実測 `Cargo.toml:39-69`、旧 pending-revision 2 は解消済み）、
  `cargo lint`（`port-naming` / `command-side-io` / `no-public-fields` / `one-public-type` ほか）。CI 7 ジョブのうち `audit` は advisory
  （`ci-success` の `needs` 外 — 実測 `ci.yml`、旧 pending-revision 3 は注記として反映）。
- P11. **性能は非目標**: 設計上の上限は「コマンド 1 回 = 本家ストアの Tx 1 回（journal 追記 + 必要なら snapshot 更新）」で、`find_by_id` の差分再生は
  既定 `SnapshotStrategy::every(10)` のもと最大 9 イベント（基底は seq_nr 1 と 10 の倍数で更新）。数値目標は立てない。
- P12. **ログ・秘密情報・環境変数**: `interface-adapter/src/` に `std::env` / `println!` / `eprintln!` / `tracing::` は 0 件（実測 grep）。`print_stdout` deny と
  `command-side-io` lint（`*_repository_impl.rs` 以外の I/O を所見）で機械強制。ペイロードの人間入力は逐語で保存（加工しない）。

## Consolidated Summary Confirmation

- NFR1（upstream 互換）= 逸脱台帳 #4 は ESA v3 まで登録済み（達成済みの維持）、ロック dir 非生成（旧ロック機構の grep 0 件）、本家スキーマへの非干渉。
  NFR2（品質ゲート）= P10（TDD 45 件 / ITF 8 トレース / Quint 8+4 / カバレッジ 90% + 0.01 / lints deny / `cargo lint`）。
  NFR3（監査完全性）= 再構成の決定性（最新スナップショット + 差分）、健全性検査（`Corrupt` 6 原因 + クラッシュ境界の明示）、原子性と楽観 version
  （Tx / CAS は本家）、書込前 ID 照合（新設）、チェックポイント・投影は U4 へ所有移転。
  NFR4（セキュリティ / サプライチェーン）= 依存を変えない（ESA `=3.0.0` / rusqlite 0.40.2 / domain は serde・ESA 非依存）、`unsafe_code = "forbid"`
  workspace、panic しない（deny 4 本 + `replay` クラッシュ境界の例外を明記）、改竄は `Corrupt` 検出（暗号学的完全性は非要求）、ログ・環境変数なし、
  パスは `aidlc/spaces/<space>/intents/.aidlc-store.sqlite`、単一プロセス前提（並行は U7）。NFR5 = 非目標の明示。
- 旧 pending-revision 3 件（TOLERANCE 0.01 / `indexing_slicing`・`panic` deny / `audit` advisory 注記）は本版で閉じる。旧 Review 節（2026-08-23 READY）は
  本文の「Review 履歴」節に要旨を残し、全文は `review-history-20260823.md` へ退避する。
- 成果物は security-requirements.md / tech-stack-decisions.md / traceability.json（構造 §1〜§5 は維持、内容を現行化、§6 前版からの変更を追加）。

Does this all look correct before I generate the artifact?

- Looks correct
- Request changes

[Answer]: Looks correct
