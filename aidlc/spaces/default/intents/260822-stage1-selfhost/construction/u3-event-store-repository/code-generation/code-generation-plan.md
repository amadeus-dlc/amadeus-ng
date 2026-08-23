# code-generation-plan — U3 SQLite EventStore と WorkflowExecutionRepository（`u3-event-store-repository`）

> Code Generation（Construction 3.5）の計画（Unit: U3、kind: library、Bolt: B5、規模 L）。出典: `../functional-design/{entities,rules,functional-spec}.md`（BR1.1〜BR5.2、
> レビュー所見 1〜3 反映済み）と `pending-revision.md`、`../nfr-requirements/{security-requirements,tech-stack-decisions}.md` と `pending-revision.md`（項目 1 TOLERANCE、
> 2 lint 昇格、3 audit advisory）、`../nfr-design/{security-design,logical-components}.md`、`../../../inception/contract-design/contract-summary.md`（C3 / C6）と
> `pending-revision.md`（C3 u64）、`../../../inception/units-generation/unit-of-work.md`（U3）、`../../../inception/delivery-planning/bolt-plan.md`（B5）、
> `../../u2-domain-es-core/functional-design/pending-revision.md`（項目 8 / 9）、Bolt B3 実装（`modules/core/domain/src/orchestration/`）、`code-generation-questions.md`
> （Q1 = A: `indexing_slicing` / `panic` の lint 昇格）。

## 1. 前提と範囲

- **ブランチ / PR**: `bolt/b5-u3-event-store-repository`（`origin/main` db6c0a1 起点、作成済み・記録コミット持越し済み）。PR は 1 本直列、squash-merge、コミット名 = Bolt slug。
  記録コミット → コードコミットの順。CodeRabbit の指摘は全件返信 + 修正 + resolve（review-thread gate）。
- **実測基線（2026-08-23、`origin/main`）**: テスト約 335、カバレッジは `scripts/coverage.sh` の base（着手時に採取）、`indexing_slicing` / `panic` 警告 120（索引 118 +
  スライス 2、`panic!` 0 — `-W` 実測。うち `lock_protocol.rs` 13 と `audit_lock_conformance.rs` 5 は退役で消える）。
- **範囲（FD BR）**: BR1.x ポート、BR2.x SQLite ストア + Repository 実装 + InMemory、BR3.x ロック退役 + Quint モデル `journal_protocol.qnt` + ITF、BR4.x U2 是正
  （`IntentId` UUIDv7 / `IntentDirName` / `WorkflowExecutionState` 改名 — メソッドも `state()` / `from_state()`）、BR5.x 仕様・正本の同期と合格条件。
- **取り込む pending-revision**: NFR 要求 1（`scripts/coverage.sh` TOLERANCE 0.05 → 0.01 + コメント更新）、2（Q1 = A: `clippy::indexing_slicing` / `clippy::panic` を
  `[workspace.lints.clippy]` に deny 追加し既存コードを是正 — テストは `#![allow]` を file / mod 単位で）、3（`cargo audit` は advisory ジョブ — 緑を確認して
  code-summary に記録）、contract-design（C3 の数値型は u64 — Rust trait が正本）、FD（`entities.md` の `## Review` 履歴はゲートで処理）。
- **数値型**: seq_nr / version / GlobalSeqNr は `u64`。rusqlite への受け渡しは `i64` に明示変換（`u64::try_from` / `i64::try_from`、失敗は `Corrupt`）。
- **設計に無い判断**: 推測で進めず `developer-report-<n>.md` の「設計質問」に書く（B3 / B4 の運用）。

## 2. 公開 API（設計の写し — 実装の契約）

- `core_use_case::orchestration`: `trait WorkflowExecutionRepository { async fn find_by_id(&self, &IntentId) -> Result<WorkflowExecution, RepositoryError>; async fn store(&mut self, &WorkflowExecutionEvent, &WorkflowExecution) -> Result<(), RepositoryError>; }`（2026-08-23 改訂: `&self` → `&mut self`。オーナー裁定、正本 `coding-rules/command-query-separation.md`）、
  `trait EventStore<AID, A, E> { async fn persist_event(&mut self, &E, version: u64); async fn persist_event_and_snapshot(&mut self, &E, &A); async fn get_latest_snapshot_by_id(&self, &AID) -> Result<Option<A>, _>; async fn get_events_by_id_since_seq_nr(&self, &AID, seq_nr: u64) -> Result<Vec<E>, _>; }`（戻りは `EventStoreError`）、
  `trait JournalReader { async fn events_after(&self, GlobalSeqNr) -> Result<Vec<(GlobalSeqNr, WorkflowExecutionEvent)>, _>; async fn checkpoint(&self, &ProjectionName) -> Result<GlobalSeqNr, _>; async fn advance_checkpoint(&mut self, &ProjectionName, GlobalSeqNr) -> Result<(), _>; }`、
  `RepositoryError { NotFound { intent_id }, Conflict { expected, actual }, Io { kind, path }, Corrupt { aggregate_id, seq_nr, cause } }`、
  `EventStoreError { Conflict, Io, Corrupt { aggregate_id: String, .. }, Schema { found, supported }, CheckpointRegression { projection, current, requested } }`、
  `CorruptCause { MissingSnapshot, UndecodablePayload, UnknownEventType, SchemaVersion, InvariantViolation, SequenceGap }`、`GlobalSeqNr(u64)`（`ZERO`）、`ProjectionName`（kebab ≤ 64）。
- `core_interface_adapter::orchestration`: `EventStoreImpl::open(StorePath, C: Clock) -> Result<Self, EventStoreError>`、`within_write_transaction(&mut self, f)`、
  `StorePath::for_space(&Path, &SpaceName)`、`WorkflowExecutionRepositoryImpl { store: EventStoreImpl<C> }`（直接所有）、`memory::{InMemoryEventStore, InMemoryWorkflowExecutionRepository}`。
- `core_domain`: `orchestration::{IntentId（UUIDv7）, WorkflowExecutionState, WorkflowExecutionStateBuilder, StateError}`、`WorkflowExecution::{state, from_state}`、`workspace::IntentDirName`。
- 意味論は FD functional-spec §3（store / find_by_id / 差分読取 / 登録簿直列化 / open）、ワイヤは §4、モデルは §5。

## 3. 規則の実装方針（BR → ステップ）

| BR | 方針 | ステップ |
|---|---|---|
| BR3.1 / BR3.2 | 退役は 1 コミットで一括削除 → build → grep 0 → 既存スイート緑。後方互換の残置なし | 1 |
| BR4.1 / BR4.2 / BR4.3 | IntentId UUIDv7（Red: parse テスト）、IntentDirName 新設（Red）、Snapshot → State 改名（refactor、旧名 0 件） | 2 |
| BR1.1〜BR1.5 | ポート / エラー / 値をユースケース層に（Red: 値型 parse・エラー Display） | 3 |
| BR2.5 / BR2.7 | ワイヤ（PBT ラウンドトリップ）、InMemory 2 本、契約テスト（ジェネリック）を先に赤で | 4〜5 |
| BR2.1〜BR2.4 / BR2.6 / BR2.8 | SQLite ストア（DDL 逐語、BEGIN IMMEDIATE、楽観 version、within_write_transaction、Clock）、Repository 実装（`EventStoreImpl` を直接所有）、StorePath、依存追加 | 6〜8 |
| BR3.3 / BR3.4 / BR3.5 | journal_protocol.qnt（8 不変条件 + 4 witness、mutation 表）、ITF fixture ≥ 6、conformance（InMemory + フェイク投影）、quint-gate 更新 | 9〜11 |
| BR5.1 | 仕様・正本の同期（10 / 11 / 01 号、deviations # 4、coding-rules） | 12 |
| NFR 要求 pending 1 / 2 | coverage TOLERANCE 0.01、lint 昇格 + 是正 | 1（TOLERANCE）/ 13（lint） |
| BR5.2 | 受入（テスト・カバレッジ・quint・audit・grep・lint・CI） | 14〜15 |

## 4. 棚卸し（code-summary に記録する事項）

- 固定した `rusqlite` / `tokio` の版、`cargo audit` の結果、依存差分。
- mutation 表（不変条件 × 変異 × 検出）、ITF fixture の seed と網羅アクション。
- grep（BR3.1 / BR3.2 / BR4.3）の結果、lint 昇格後の是正件数（src / tests）。
- カバレッジ（base → head、TOLERANCE 0.01 で相対ゲート緑）、テスト数。
- 設計質問と裁定。

## 5. 実装ステップ（TDD、レイヤーごとに Red → Green → Refactor）

Testing Contract の層: 「Data model」= 値型・エラー型・IntentId / IntentDirName / State 改名（use-case / domain）、「Repository」= ストアと Repository 実装
（adapter）、「Business logic」= ワイヤと検査点 / Quint 協定、「API」= 契約テスト両実装・ITF・クラッシュ再構成・ゲート。Frontend は該当なし。各 Red で失敗コマンド出力を
`developer-report-<n>.md` に記録してから Green に進む。

### 5.0 コンダクタ（承認後・委任前）

- [ ] Step 0. `bun .claude/tools/aidlc-bolt.ts start --name B5 --batch 1`、aidlc 記録を 1 コミット、基線採取（`cargo test --workspace` 数、`scripts/coverage.sh` base、
      lint 警告 120）。

### 5.1 委任 1 — 退役 + U2 是正（開発エージェント Opus、所有: `modules/core/{domain,use-case,interface-adapter}/src/**`、`modules/core/domain/tests/**`、`modules/core/interface-adapter/tests/**`、`modules/infra-io/src/**`、`tools/lint/**`、`formal/workspace/**`、`tests/conformance/fixtures/audit_lock/**`、`scripts/quint-gate.sh`、`scripts/coverage.sh`、`modules/core/interface-adapter/Cargo.toml`（md5 除去のみ）、`aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/{tell-dont-ask,README,gateway-taxonomy}.md`）

- [ ] Step 1. **退役（1 コミット分）**: 削除 — use-case `workspace/`（mod ごと）、adapter `workspace/fs_workspace_lock.rs` / `process_probe.rs`（`workspace/mod.rs` と
      `lib.rs` の `pub use` 整理、`state_file_io.rs` は維持）、domain `workspace/{lock_protocol,lock_identity}.rs` と `pub use`（`LockProtocol` / `LockIdentity` /
      `reap_eligible` / `LockError`）、infra-io `process_probe.rs`、tests `fs_workspace_lock_test.rs` / `audit_lock_conformance.rs`、`formal/workspace/audit_lock.qnt`、
      `tests/conformance/fixtures/audit_lock/`、`tools/lint` の `reap-decision-locality`（ルール・HELP・`mentions_reap_state`・赤例テスト・`main.rs` の登録）、adapter
      `Cargo.toml` の `md5`。`scripts/quint-gate.sh` から audit_lock の typecheck / run / witness を除去（journal_protocol は委任 4 が追加）。`scripts/coverage.sh`
      `TOLERANCE=0.01` + 冒頭コメント更新。coding-rules: `tell-dont-ask.md` の reap 例を「履歴（退役済み、ADR-007）」注記に、`README.md` の tell-dont-ask 行の機械強制
      を `cargo lint`（checkbox-vocabulary）に、`gateway-taxonomy.md` §1 の機構モジュール例から `process_probe` を外す。検査: `cargo build --workspace`、
      `grep -rnE 'WorkspaceLock|FsWorkspaceLock|LockProtocol|LockIdentity|reap_eligible|OwnerStamp|AcquireBudget|LockGuard|LockError|process_alive|ProcessProbe|audit_lock|reap-decision-locality' modules tools scripts formal .github Cargo.toml` = 0、
      `cargo test --workspace` 緑、`cargo test --manifest-path tools/lint/Cargo.toml` 緑、`cargo fmt` / `clippy -D warnings`（`.aidlc-lock` grep も 0）。
- [ ] Step 2. **U2 是正（1 コミット分）**: Red — `IntentId::parse` の UUIDv7 受理 / 拒否テスト（大文字・v4・variant 不正・長さ・空、`IntentIdError` 5 変種）、
      `IntentDirName::parse` の受理 / 拒否（`260822-stage1-selfhost` / `-2` サフィックス / 先頭 6 桁なし / 大文字 / 65 字）。Green — 実装（標準ライブラリのみ、正規表現
      クレートなし）。Refactor — `WorkflowExecutionSnapshot` → `WorkflowExecutionState`（`workflow_execution_state.rs`）、`…Builder`、`SnapshotError` → `StateError`
      （`state_error.rs`）、`snapshot()` → `state()`、`from_snapshot()` → `from_state()`、rustdoc、`mod.rs` の `pub use`（旧名 0 件）。既存テスト・ITF・ゴールデンの
      IntentId リテラル（`260822-stage1-selfhost` / `itf-engine-loop` / `u2`）を UUIDv7（例 `01a02785-1bd8-76eb-aeea-5aa303ebd5b6` — intents.json 実データ、他は
      任意の有効 v7）に置換。検査: `grep -rn 'Snapshot' modules/core/domain/src/orchestration` = 0、`cargo test --workspace` 緑。
      `developer-report-1.md`。

### 5.2 委任 2 — ポート・値・エラー / InMemory / ワイヤ / 契約テスト（Opus、所有: `modules/core/use-case/src/orchestration/**`（既存 `workflow_definition_repository.rs` は読取のみ）、`modules/core/use-case/src/lib.rs`、`modules/core/interface-adapter/src/orchestration/{memory/in_memory_event_store.rs,memory/workflow_execution_repository.rs,memory/mod.rs,wire/**,mod.rs}`、`modules/core/interface-adapter/tests/{support/**,workflow_execution_repository_contract.rs}`、両 `Cargo.toml` の dev-dependency `tokio` 追加）

- [ ] Step 3. Data model — Red: `GlobalSeqNr` / `ProjectionName`（parse 受理・拒否）、`RepositoryError` / `EventStoreError` / `CorruptCause` の Display（材料のみ）と
      `Error` 実装、`EventStoreError → RepositoryError` 写像（各 5〜8 本）。Green / Refactor — use-case `orchestration/{event_store,journal_reader,workflow_execution_repository,repository_error,event_store_error,global_seq_nr,projection_name}.rs` と `pub use`。
- [ ] Step 4. Business logic（ワイヤ）— Red: `wire/event_wire.rs` / `wire/state_wire.rs` の encode → decode ラウンドトリップ PBT（全 12 変種・16 属性の生成器、
      `PROPTEST_RNG_SEED` 固定）、未知 `type` / 未知フィールド / schema_version ≠ 1 / 型不一致の拒否（`Corrupt` 原因別）、正準 JSON のバイト決定性（canon-json
      `to_value` → `serialize`）。Green / Refactor — serde 構造体は `pub(crate)`、Domain Primitive の parse で検査段 2（security-design §2）。
- [ ] Step 5. API（契約テスト）— Red: `tests/support/contract.rs` にジェネリック契約テスト関数群（ラウンドトリップ（start → 数コマンド → store × n → 新インスタンス
      find_by_id → `state()` 同値）、NotFound、Conflict（2 再水和の競合）、Corrupt（MissingSnapshot / UndecodablePayload / SchemaVersion — 実装が行を直接いじれる
      フックを支援として持つ）、events_after の順序と欠落なし、checkpoint 未登録 = ZERO、advance の単調性 / CheckpointRegression、genesis（expected 0）の store）。
      `InMemoryEventStore`（BTreeMap journal / snapshot / checkpoint、同じ Conflict 規則、within_write_transaction 相当はクロージャ実行のみ）と
      `InMemoryWorkflowExecutionRepository { store: InMemoryEventStore }`（直接所有） で緑に。`developer-report-2.md`。

### 5.3 委任 3 — SQLite ストア + Repository 実装（Opus、所有: `modules/core/interface-adapter/src/orchestration/{sqlite_event_store.rs,schema.rs,store_path.rs,workflow_execution_repository_impl.rs}`、`modules/core/interface-adapter/tests/{sqlite_event_store_test.rs,workflow_execution_repository_impl_test.rs,crash_reconstruction_test.rs}`、`Cargo.toml`（workspace deps: rusqlite / tokio）、`modules/core/interface-adapter/Cargo.toml`、`Cargo.lock`。`mod.rs` の `pub use` 追記は委任 2 完了後に本委任が行う）

- [ ] Step 6. Repository — Red: 既存契約テスト群を SQLite 実装で実行（`tempfile` の一時 dir に `intents/.aidlc-store.sqlite`）、追加テスト: open / 初期化（user_version
      0 → 1、1 → OK、2 → Schema、親 dir 欠落 → Io NotFound）、`PRAGMA table_info` で C6 の列・型・制約突合、BEGIN IMMEDIATE の 2 接続直列化（busy_timeout 内 / 超過
      → Io WouldBlock）、`within_write_transaction` の rollback（f が Err）、クラッシュ再構成（store 後に接続 drop → 新接続 find_by_id 同値）、rusqlite Error → 写像。
- [ ] Step 7. Repository — Green: `schema.rs`（C6 DDL 定数）、`EventStoreImpl`（open / pragmas / Tx 手順 BR2.3 / JournalReader / within_write_transaction / Clock）、
      `StorePath`、`WorkflowExecutionRepositoryImpl`（`EventStoreImpl` を直接所有、BR1.2 / BR1.3 の手順 — `expected = aggregate.version()` = `event.seq_nr() − 1` の前提検査、replay 後に
      `with_version(last)`）。依存: workspace `rusqlite = { version = "<latest 0.3x>", features = ["bundled"] }`、`tokio = { version = "1", features = ["rt", "macros"] }`。
- [ ] Step 8. Repository — Refactor: エラー写像の一本化、rustdoc（`# Errors`）、索引アクセスなし（`get` / イテレータ）、`cargo audit` 実行（結果を報告）、
      `cargo clippy -D warnings` 緑。`developer-report-3.md`。

### 5.4 委任 4 — Quint モデル + ITF + quint-gate（Opus、所有: `formal/orchestration/journal_protocol.qnt`、`tests/conformance/fixtures/journal_protocol/**`、`modules/core/interface-adapter/tests/journal_protocol_conformance.rs`、`scripts/quint-gate.sh`（journal_protocol ステップ追加のみ）、`formal/README*`（あれば））

- [ ] Step 9. Business logic（協定）— `journal_protocol.qnt`: 定数 WRITERS = 2、var / action / invariant 8 / witness 4（FD BR3.3）、prev 状態スナップショット方式
      （engine_loop v2 / 旧 audit_lock v2 と同型）。`quint typecheck` → `quint run --invariants …`（seed 固定・max-samples 明示）緑。
- [ ] Step 10. mutation: 不変条件ごとに 1 変異モデルを一時作成して violation を確認（表: invariant / 変異 / 結果）。witness 4 本を負形式 run で経路実在を確認。
- [ ] Step 11. API（ITF）— `quint run … --out-itf` で seed 6 本以上採取、`#meta` 正規化（engine_loop の採取手順 — `tests/conformance/` の既存規約に従う）、
      `journal_protocol_conformance.rs`（InMemoryEventStore + フェイク投影、lastAction × lastActor 駆動、全アクション網羅の assert）。`scripts/quint-gate.sh` に
      typecheck / invariants run / witness のステップを追加。`developer-report-4.md`。

### 5.5 委任 5 — 仕様・正本の同期（Sonnet、所有: `docs/specs/{01-domain-model,10-orchestration,11-workspace,deviations}.md`）

- [ ] Step 12. BR5.1: 10 号 §6 I14 と 11 号 §6 W1〜W5 → journal_protocol の J1〜J6（conflict_rejected / snapshot_tracks_journal / checkpoint_monotone / projection_idempotent /
      truth_is_journal / no_lost_update）と E4 定義名、11 号 §2.2 `LockIdentity` 行 → 退役、§3 / §4 の `ProcessProbe` → 退役、§8 の Quint 記録に journal_protocol への
      改訂経緯、§10 未決 2 件を Q1 / Q2 の裁定で確定（stage-0/1 併用期の相互排他は「担保しない — 単一クローン運用」と明記、`intents.json` は
      `within_write_transaction`）、10 号 §3 / 11 号 §3 の実装欄に `EventStoreImpl`、01 号 §3.3 代表不変条件 + §6 第一陣を協定モデルへ、`deviations.md` # 4 のパス
      `aidlc/spaces/<space>/intents/.aidlc-store.sqlite` 確定（「相当」除去）。出典注記つき、逐語契約には触れない。`developer-report-5.md`。

### 5.6 委任 6 — lint 昇格 + 既存是正（Sonnet、所有: `Cargo.toml`（`[workspace.lints.clippy]` のみ）、`modules/**`（委任 1〜3 完了後、索引の是正に限る）、`clippy.toml`（必要なら））

- [ ] Step 13. `indexing_slicing = "deny"` / `panic = "deny"` を追加 → `cargo clippy --workspace --all-targets -- -D warnings` の違反（基線 120、退役で −18）を是正:
      プロダクトコードは `get()` / イテレータ / `split_at_checked` 等に書き換え（挙動不変、テスト緑のまま）、テストコードは file / mod 単位の
      `#![allow(clippy::indexing_slicing)]`（理由コメント 1 行）。是正件数（src / tests）を `developer-report-6.md` に。

### 5.7 コンダクタ（統合）

- [ ] Step 14. 受入（FD BR5.2 / `unit-test-instructions.md`）: `cargo test --workspace` 全緑、`scripts/coverage.sh` 90% 床 + 相対ゲート（TOLERANCE 0.01）、
      `bash scripts/quint-gate.sh` 緑、`cargo audit` 緑、grep（BR3.1 / BR3.2 / BR4.3）0 件、`cargo lint` + `tools/lint` 自己テスト緑、`cargo fmt` / `clippy` 緑。
      `code-summary.md` / `traceability.json`。
- [ ] Step 15. advisory レビュー → PR（本文に受入の実測）→ CodeRabbit 全件対応 → CI 緑 → merge queue → `aidlc-bolt.ts complete --name B5 --batch 1`。

## 6. トレーサビリティ（要求 → ステップ）

| 要求 | BR | ステップ |
|---|---|---|
| FR1.2 | BR1.3, BR2.3, BR2.4, BR3.x | 1, 6〜11 |
| FR1.3 | BR1.1, BR1.2, BR1.4, BR1.5, BR2.1, BR2.2, BR2.5〜2.8 | 3〜8 |
| NFR3 | BR1.2, BR3.3, BR3.5, BR5.2 | 5〜11, 14 |
| NFR1.1 / 1.2 | BR5.1, BR3.2 | 1, 12 |
| NFR2.x | BR5.2 + pending 1 / 2 | 1, 13, 14 |
| NFR4.x | BR1.5, BR2.8, 依存差分 | 7, 8, 13, 14 |
| U2 pending 8 / 9 | BR4.1〜4.3 | 2 |

## 7. 委任の形

- 直列: 委任 1 → 委任 2 → {委任 3 ∥ 委任 4 ∥ 委任 5} → 委任 6 → 統合。並行する委任は所有ファイルが重ならない（委任 4 の conformance は `tests/` 配下の新規ファイル、
  委任 5 は `docs/specs/` のみ）。委任 3 が `orchestration/mod.rs` の `pub use` を追記する（委任 2 完了後なので衝突しない）。
- 開発エージェントは計画・`unit-test-instructions.md`・本質問票を書き換えない。進捗・設計質問・検査結果は `developer-report-<n>.md`。`git commit` / `git add` はコンダクタ
  （委任 1 の 2 コミットはコンダクタが区切って行う — 委任 1 は「退役」と「是正」を順に進め、それぞれ完了時点で報告する）。
- モデル: 委任 1〜4 = Opus（退役の波及・契約の実装・Quint）、委任 5〜6 = Sonnet（文書同期・機械的是正）。
- 新規コードは最初から `indexing_slicing` / `panic` を生まない（委任 6 の是正対象は既存コード）。`unused_async` が trait 実装で発火した場合は設計質問に上げる
  （`#[allow]` で握りつぶさない）。

## Testing Contract

> 注: 本 Unit はライブラリ層（ドメイン是正 + ユースケース層ポート + アダプタ実装）。Frontend 層は該当なし。ITF 準拠・ゴールデン・Quint は TDD の外側の受入ゲート。

```json
{
  "version": 1,
  "methodology": "tdd",
  "source": "team",
  "ordering": "新規プロダクションコードはレイヤーごとに red-green-refactor",
  "scope": "classic",
  "test_strategy": "standard",
  "project_type": "brownfield",
  "applicable_notes": [
    {
      "layer": "org",
      "text": "We treat tests as a first-class deliverable in every Bolt. The specific\nmethodology (TDD, BDD, ATDD, or classic test-after) is affirmed at\npractices-discovery and recorded in `team.md` under this heading with explicit\n`Methodology` and `Ordering` fields; Code Generation resolves those fields\nindependently from coverage, tooling, and scope notes.\n\nWhen no posture has been affirmed, our default per scope is:\n- **Methodology**: test-after\n- **Ordering**: implement each applicable testable layer, then write and run\n  that layer's tests.\n- `mvp`, `enterprise`, `feature`, `infra`, `classic` add an 80% line-coverage\n  floor and CI execution before merge.\n- `bugfix`, `security-patch` add a targeted regression for the specific\n  bug/vulnerability and require the existing suite to remain green.\n- `express` uses the Minimal strategy: requirement-driven unit tests (one per\n  requirement, with a happy-path floor per component); existing tests remain\n  green.\n- `poc`, `refactor`, `workshop` add no extra new-test floor and require the\n  existing suite to remain green.\n\nThe active `Test Strategy` still applies in every scope and determines test\nvolume/types. Scope floors are additive; they never reduce or replace the\nselected strategy.\n\nAffirm a stricter posture in `team.md` if the team commits to one."
    },
    {
      "layer": "team",
      "text": "- **Methodology**: tdd\n- **Ordering**: 新規プロダクションコードはレイヤーごとに red-green-refactor\n  （失敗するテストを先に書く）で実装する。Quint モデル検査・ITF 準拠テスト・\n  ゴールデンパリティは TDD サイクルの外側の受け入れゲートとして維持し、\n  TDD の red を代替しない。（インタビュー Q2、選択肢 A で確定——品質レビュー\n  の自己完結化置換案どおり）\n\nテストピラミッド（ユニット層を厚く、結合・E2E層を薄く）を意識した配分とする\n（オーナー明言）。比率は**定性のみ**とし、数値目標は定めない（インタビュー\nQ3、選択肢 A）: 単体テスト優位・統合テストは境界ごと・E2E は最小、という\n配置規則で充足する。\n\nこのプロジェクトは TDD の上に **3層の品質保証** を重ねている点が特徴的で、\nそれぞれ役割が異なる（`code-quality-assessment.md` §品質保証の全体像より）:\n\n1. **Quint 形式検証**（毎 PR）— 決定論コアの状態機械契約そのものを検証。\n   不変条件 run 27本・到達性 witness 12本の反転判定・決定的シナリオ。\n   モデルの検査力自体も mutation テストで証明済み（engine_loop 3/3、\n   audit_lock 10/10 + witness 7/7、stop_hook 7/7）。\n2. **ITF 準拠テスト**（`modules/core/domain/tests/`、engine_loop / audit_lock\n   の2モデル・2ファイル）— Quint モデルのトレースを集約に再生し状態射影を\n   突き合わせることで、モデルと実装の乖離を検出。TDD の「テストを先に書く」\n   対象は実装コードだが、契約の正本は Quint 側にあるため、ITF 準拠テストは\n   実装後に契約適合を機械確認する位置づけ（TDD サイクルの red-green-refactor\n   そのものではなく、その外側のゲート）。なお stop_hook は ITF 準拠テストが\n   未整備（既知の穴、`evidence.md` インタビュー未確定事項 (e) 参照）。\n3. **PBT（proptest）+ ゴールデンパリティ**— upstream 配布実バイト33ノードの\n   全数 load パリティを固定し、upstream 互換の逸脱を検出。\n\nしたがって TDD サイクルは主にユニットテスト層（インライン `#[cfg(test)]`、\n実測**40ファイル**——集計方法: `modules/` 配下・`tests/` ディレクトリを除いた\nインライン `#[cfg(test)]` 数。`tests/` 配下6本（ITF準拠2 + 統合4）を含めると\n46、`tools/lint/src/check.rs` を含めても47であり、いずれの集計でも48には\nならない。開発者レビュー指摘どおり40へ訂正した）に適用し、ITF 準拠テスト・\nゴールデンパリティはレイヤー横断の受け入れ確認として TDD サイクルの外側に\n位置づける。\n\n- **カバレッジ**: 絶対ゲート90%床 + PR 相対ゲート（head が base を下回ったら\n  fail、許容誤差 0.5pp。PBT のシード非固定に起因するノイズ較正値であり、\n  stage-1 スコープで**シード固定により 0.01 へ引き締める**——インタビュー\n  Q7、選択肢 A/B。除外設定は現状無いが、**composition root（`main.rs` の\n  配線部分）のみカバレッジ除外を許可**し、それ以外は床を維持する\n  （インタビュー Q5、選択肢 B。除外設定は `scripts/coverage.sh` への確定\n  アクション、`evidence.md` 参照）。実測 94.87〜95.29%（`scripts/coverage.sh`）。\n- **ツーリング**: `cargo test --workspace`（234テスト全緑、実測）、\n  `cargo-llvm-cov`、Quint 0.32.0（Node 22 経由）。\n- **テスト種別**: ユニット（インライン `#[cfg(test)]`）、PBT（proptest、集約\n  本体同居）、ITF 準拠（`modules/core/domain/tests/` 2本）、統合（\n  `modules/core/interface-adapter/tests/` 4本 — ゴールデンパリティ・FS ロック・\n  Repository 実装・シンボリックリンク防御）。\n- **CI ゲート**（`main` へのマージ条件、実測）: `check` ジョブ（`cargo fmt\n  --all --check` → `cargo clippy --workspace --all-targets -- -D warnings` →\n  `cargo lint` → `cargo test --workspace`）、`quint` ジョブ\n  （`scripts/quint-gate.sh`）、`coverage` ジョブ（`scripts/coverage.sh`、\n  絶対90%床 + PR 相対ゲート）の3ジョブすべてを緑にする。この3ジョブは\n  **stage-1 スコープで branch protection の required status checks として\n  機械強制する**（インタビュー Q4、選択肢 A——現状は運用規律のみで機械強制が\n  無いという品質レビューの重大指摘を受けての裁定。設定作業は\n  `evidence.md` の確定アクションに記載）。\n- **スコープ注記**: `tools/lint`（`cargo lint` の実装クレート）は workspace\n  非メンバーの detached クレートであり、CI の fmt/clippy/test がまだ届いて\n  いない（設計監査 C27）。**stage-1 スコープに含める**: `tools/lint` への\n  CI 3ステップ（fmt/clippy/自己テスト）追加（インタビュー Q7、選択肢 A）。\n  macOS CI ジョブ追加・`main` への push トリガー追加は本 intent には\n  含めず、後続 intent へ繰り延べる（インタビュー Q7、選択肢 E 相当の一部\n  不採択）。"
    }
  ],
  "obligations": {
    "strategy": "standard",
    "strategy_volume": [
      "Five to eight tests per component.",
      "Unit tests plus integration tests for key boundaries.",
      "Add E2E, performance, or security tests when requirements demand them."
    ],
    "scope_floor": [
      "Keep the existing test suite green.",
      "This scope adds no extra new-test floor beyond the selected test strategy."
    ],
    "combination_rule": "Apply every selected-strategy obligation and every scope-floor obligation; neither replaces the other, and a targeted scope regression may add the narrowest necessary test type beyond the strategy default."
  },
  "plan_profile": {
    "methodology": "tdd",
    "runner_step": "Verify the existing test runner/configuration and record the exact unit-scoped command.",
    "runner_ready_before_first_test": true,
    "testable_layers": [
      "Data model / database behavior",
      "Repository / data access",
      "Business logic",
      "API / endpoint",
      "Frontend behavior"
    ],
    "steps": [
      "Project structure and production configuration skeleton.",
      "Verify the existing test runner/configuration and record the exact unit-scoped command.",
      "Data model / database behavior - Red: write the failing tests and record the failing command output.",
      "Data model / database behavior - Green: implement only enough behavior to pass.",
      "Data model / database behavior - Refactor: improve the implementation while tests stay green.",
      "Repository / data access - Red: write the failing tests and record the failing command output.",
      "Repository / data access - Green: implement only enough behavior to pass.",
      "Repository / data access - Refactor: improve the implementation while tests stay green.",
      "Business logic - Red: write the failing tests and record the failing command output.",
      "Business logic - Green: implement only enough behavior to pass.",
      "Business logic - Refactor: improve the implementation while tests stay green.",
      "API / endpoint - Red: write the failing tests and record the failing command output.",
      "API / endpoint - Green: implement only enough behavior to pass.",
      "API / endpoint - Refactor: improve the implementation while tests stay green.",
      "Frontend behavior - Red: write the failing tests and record the failing command output.",
      "Frontend behavior - Green: implement only enough behavior to pass.",
      "Frontend behavior - Refactor: improve the implementation while tests stay green.",
      "Environment/build configuration.",
      "Documentation and traceability."
    ]
  },
  "input_sha256": "sha256:e4f36aa113753d3604df570f5ec3a0cb465d4b29d82a17a16efbb2ea8b993111",
  "contract_sha256": "sha256:303d9bb7b5d777d54a6761be9ed154d85d5bb3f2d6b9cce02f71f4ed1b3a4ff3"
}
```

