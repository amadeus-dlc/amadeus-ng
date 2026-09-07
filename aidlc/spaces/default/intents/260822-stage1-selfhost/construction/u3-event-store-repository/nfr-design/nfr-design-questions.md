# nfr-design-questions — U3 イベントストアと IntentExecutionRepository（`u3-event-store-repository`）

> NFR Design（Construction 3.3）の質問票（Unit: U3、kind: library）。**改訂履歴**: 初版 2026-08-23（Bolt B5 着手前、P1〜P5、Looks correct）→
> **再走 2026-09-07（本版、Modify）**: unit-major 再走で既存成果物 3 点（security-design / logical-components / traceability）を現行コード
> （作業ツリー `e7191c0d`、`origin/main` = `f2b6b6a9` と同一のプロダクトコード）で実測し、検査点・依存・ピン・論理コンポーネント名・
> テスト配置・NFR ID 集合が失効していたためオーナー回答（会話、AskUserQuestion）**Modify** で書き直す。前提 P1〜P5 は履歴として残し、
> 現行の前提を P6〜P12 として追記した。
>
> 出典: `../nfr-requirements/security-requirements.md`（NFR1.1〜1.3 / NFR2.1〜2.5 / NFR3.1〜3.5 / NFR4.1〜4.7、STRIDE、データ分類、
> 末尾レビュー R-01〜R-06 — 2026-09-07 再走 READY）、`../nfr-requirements/tech-stack-decisions.md`（§1 選定・§3 未決）、
> `../functional-design/functional-spec.md`（§1 所有表・§2 契約・§3 保存と再構成・§4 永続化表現・§5 検証モデル・§7 検証範囲 —
> 2026-09-05 是正版・2026-09-07 再レビュー READY）、`../functional-design/rules.md`（BR1.1〜BR5.2）、
> `../../../inception/contract-design/contract-summary.md`（C3 / C6 — 2026-09-07 現行化）、実コード
> `modules/core/command/{domain,use-case,interface-adapter}/`、`modules/app/aidlc/tests/`、`formal/orchestration/journal_protocol.qnt`、
> `Cargo.toml` / `Cargo.lock` / `.github/workflows/ci.yml`（実測）。performance / scalability / reliability / observability の要求・設計は
> kind = library のため存在せず、本ステージの成果物は `security-design.md` / `logical-components.md` / `traceability.json` の 3 つ。
>
> **質問なし。** 基盤選択（本家 event-store-adapter-rs・スナップショット + 差分再生・ロック退役・DTO の所有）は ADR-001 / 007 / 010 と
> 2026-08-30 / 2026-09-05 のオーナー裁定で確定済みで、本 Unit の未決 3 件（`busy_timeout` / 複数プロセス並行・登録簿の直列化・
> `SnapshotStrategy` 既定値）は U7 の裁定事項として tech-stack-decisions §3 に載っている。次の前提を確認して成果物へ進む。

## 前提（2026-08-23 初版 — 履歴）

- P1. ~~**検査点の設計**: 信頼しない入力からドメイン型へ至る経路に 3 段（ワイヤ復号 / Domain Primitive parse / 集約不変条件）の検査を置き、
  どの段の失敗も `Corrupt { cause }` へ写し panic しない。~~ → **失効**（ADR-010 でワイヤ型と `CorruptCause` は削除、2026-08-30 裁定で
  再構成のクラッシュ境界が導入）。現行は P6。
- P2. **障害ドメイン 4 種**（Io / Conflict / Corrupt・Schema / Busy 超過）。→ **一部有効**（`Schema` 変種は削除、Busy は「超過」ではなく
  即時 `WouldBlock`）。現行は P8。
- P3. ~~**論理コンポーネント**: use-case `orchestration`（ポート + エラー + 値）、adapter `orchestration::{sqlite_event_store, wire, store_path,
  workflow_execution_repository_impl, memory}`、Clock は機構モジュール。~~ → **失効**（自前ストア・ワイヤ・memory ダブルは削除、
  クレートは `core-command-*` に改名、`JournalReaderImpl` は RMU へ移転）。現行は P7。
- P4. **退役の安全手順**（削除 → ビルド → grep 0 件 → 既存テスト緑、ロック系を先に 1 コミット）。→ **完了**（B6 / B7 で実施済み。
  grep 0 件の維持は NFR1.2 の合格基準として残る）。
- P5. **Quint DoD**（不変条件ごとに 1 変異モデル、状態遷移レベルの不変条件、in-module witness、ITF fixture の `#meta` 正規化）。
  → **有効**（適用範囲の限定を P9 で現行化）。

## 再走の前提（2026-09-07 — 現行）

- P6. **検査点は四層 + 書込前検査**（NFR3.2 / NFR4.3 / NFR4.4、実測 `intent_execution_repository_impl.rs:331-438`）:
  (1) ストア復号 — 本家 serde の失敗 `EventStoreReadError::DeserializationError` → `Corrupt(StoreDeserialization)`。
  (2) DTO → ドメイン — `IntentExecutionDto::to_domain` / `IntentExecutionEventDto::to_domain` が綴り・文法（`DtoDecodeError::Malformed
  { field, value }`）と不変条件（`InvariantViolation`）を検査し、基底は検査付き完全コンストラクタ `IntentExecution::new`（cursor / parked_at
  の範囲・`seq_nr ≥ 1`・`check_invariants` — 実測 `intent_execution.rs:290-345`）を必ず通る → `Corrupt(Undecodable(e))`。
  (3) 差分行の Repository 検査 — 通番の連続（`SequenceGap`）・manifest `intent-execution-event/1`（`ForeignManifest`）・payload の
  `aggregate_id` ≠ 要求 ID（`Undecodable(InvariantViolation)`）。基底欠落 + journal あり（`MissingSnapshot`）。
  (4) `IntentExecution::replay` / `apply_event` のクラッシュ境界 — DTO として成立した壊れた遷移（未知ステージ・不変条件違反）は panic
  （`# Panics` を持つ公開 API は誕生変換 `From<(Started, DateTime<Utc>)>` を含む 3 か所、正本 `intent_execution.rs:40-41`）。
  書込側は (0) 本家を呼ぶ**前**の `event.aggregate_id() != aggregate.id()` → `Corrupt(WriteContract)`（実測 `:447-452`）と、本家の
  `SerializationError` / `ContractViolation` → `Corrupt(WriteContract)`。分類はポート契約に載せず、`RepositoryError::Corrupt { id, seq_nr,
  source }` の `source` にアダプタ私有 `CorruptDetail` 6 変種で運ぶ（BR1.5）。
- P7. **論理コンポーネント（現行）**: use-case `orchestration/port/{intent_execution_repository, repository_error}.rs`；adapter
  `orchestration/{intent_execution_repository_impl, snapshot_strategy, store_failure, kinds_codec}.rs` + `dto/`（32 エントリ — 型引数 3 型
  `IntentExecutionDto` / `IntentExecutionEventDto` / `IntentExecutionAggregateKeyDto`、イベント変種 DTO 群、`DtoDecodeError`、`tests.rs`）。
  mod は private、公開は `orchestration/mod.rs` の `pub use` ファサードのみ（実測 `:28-41`）；domain `orchestration/intent_execution.rs`
  （`new` / `replay` / `with_version`）・`workspace/{store_path, intent_dir_name}.rs`；formal `journal_protocol.qnt` + fixture 8 トレース；
  app/aidlc tests `journal_protocol_conformance.rs` / `crash_reconstruction_test.rs`。RMU（U4）の `journal_reader_impl.rs`（別接続、
  `busy_timeout` 既定 5000ms — 実測 `:82`）は U3 の外。adapter の `Clock` / `SystemClock` / `FakeClock` は U3 の経路では使わず（封筒の
  `occurred_at` は集約の `last_updated_at`、BR2.6）、他クレートからの利用も `modules/` 配下 0 件（実測 grep）。`tools/lint` 7 ルールのうち
  U3 に直接効くのは `port-naming` と `command-side-io`。
- P8. **障害ドメイン（現行）**: (a) `Io { kind, path }` — `store_failure::io_kind` が rusqlite の code を `ErrorKind` に写す（`DatabaseBusy` /
  `DatabaseLocked` → `WouldBlock`、`CannotOpen` / `NotFound` → `NotFound`、`PermissionDenied` / `ReadOnly` / `AuthorizationForStatementDenied`
  → `PermissionDenied`、`DatabaseCorrupt` / `NotADatabase` → `InvalidData`、`OperationInterrupted` → `Interrupted`、他 `Other` — 実測
  `store_failure.rs:20-34`）。呼出側へ返し再試行しない。(b) `Conflict { expected, actual }` — `actual` は `stored_version` の再読取（読めなければ
  0、材料のみ）。ユースケースが再水和して 1 回再試行（C1「2 回続いたら exit 1」、U5）。(c) `Corrupt` 6 原因 — 中断、部分集約を返さない。
  (d) クラッシュ境界 — panic でプロセス終了、ストアは不変（書込時に検査済みの歴史だけが入る前提）。(e) `WouldBlock` — 本家接続に
  `busy_timeout` を設定できないため待たずに返す。単一プロセス前提で、複数プロセスと `reopened()` が生む同一プロセス複数ハンドル（直列化の
  実体は本家の楽観 version CAS — nfr-requirements R-01）は U7 の並行モデル裁定へ繰り延べる。
- P9. **Quint / ITF の適用範囲**: モデルは 1 集約・writer 2・投影 1・`every(1)` 限定（不変条件 8 / witness 4、BR3.3）。fixture 8 トレースを
  `journal_protocol_conformance.rs` が `every(1)` 明示で `IntentExecutionRepositoryImpl` + `JournalReaderImpl` + 実 RMU 投影に再生（実測
  `:129,:226-227`）。既定 10 / 任意 N の間欠更新と差分再生は実装固有テスト（`the_first_store_always_writes_the_snapshot_base` /
  `the_strategy_refreshes_the_snapshot_every_n_events` / `a_stale_snapshot_plus_delta_matches_the_freshest_state`）が担い、モデルの証明範囲に
  含めない。「モデルは 1 文字も変えずに通った」は履歴であり現在の合格証拠にしない（functional-spec §5）。
- P10. **サプライチェーン（現行）**: 依存を変えない（`event-store-adapter-rs = "=3.0.0"` / `rusqlite 0.40.2` bundled / `serde` / `serde_json`
  / `chrono` — adapter 所有）。推移依存 `thiserror 2.0.20` は本家経由で入るが我々は直接使わない（手実装エラー enum の様式を維持 —
  実測 `cargo tree`）。`core-command-domain` / `core-command-use-case` の `Cargo.toml` に ESA なし（実測、domain は serde もなし）。
  `unsafe_code = "forbid"` は workspace lints。`cargo audit` は CI `audit` ジョブで advisory。
- P11. **テスト配置（現行、実測）**: 契約 11 関数 × memory / SQLite = 22（`intent_execution_repository_contract.rs`）、実装固有 23
  （`intent_execution_repository_impl_test.rs`）、本家適合 5 関数 × 2 バックエンド（`upstream_event_store_conformance.rs`）、インライン
  `intent_execution_repository_impl.rs` 10 / `store_failure.rs` 4 / `snapshot_strategy.rs` 2 / `dto/tests.rs` 29、クラッシュ再構成 5 と
  ITF 8 トレース（`modules/app/aidlc/tests/`）。CI 7 ジョブ（aidlc-distribution / check / quint / coverage / audit / review-thread-resolution /
  ci-success）のうち `ci-success` の `needs` は `audit` を除く。
- P12. **旧レビュー節・所見の扱い**: 旧 READY レビュー節（2026-08-23、Minor 3）は `review-history-20260823.md` へ逐語退避し、本文に要旨表を
  残す。traceability は現行の 20 ID（NFR1.1〜1.3 / 2.1〜2.5 / 3.1〜3.5 / 4.1〜4.7）。nfr-requirements 再走レビュー R-01〜R-06（CAS が
  直列化の実体・`# Panics` 3 か所・DTO 32 エントリ・検出範囲の限定・proptest 3 クレート）と、上流 `requirements.md:133-135`（NFR3 の
  `audit_lock.qnt` / `WorkflowExecution`）が失効している事実（R-03、上流改訂はオーナー裁定待ち）を設計に反映する。

## Consolidated Summary Confirmation

- セキュリティ設計 = 検査点の四層 + 書込前検査（P6）、障害ドメイン 5 種の扱い（P8）、サプライチェーン「依存を変えない」（P10）、
  決定性と協定の維持（P9 — モデルは `every(1)` 限定、間欠更新は実装テスト）、退役の維持（P4 完了 → grep 0 件の維持）。
- 論理コンポーネント = 現行の 3 クレート + formal + app tests の配置と境界（P7）、障害ドメインとブラストラディウス（P8）、テスト配置
  （P11）、U7 への橋渡し（`StorePath::for_space` / `SnapshotStrategy` / 並行モデル 3 件の未決）。
- 成果物は security-design.md / logical-components.md / traceability.json（構造 §1〜§9 は維持、内容を現行化、前版からの変更節を追加、
  旧レビュー節は退避 — P12）。

Does this all look correct before I generate the artifact?

- Looks correct
- Request changes

[Answer]: Looks correct
