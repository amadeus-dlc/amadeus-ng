# security-design — U3 イベントストアと IntentExecutionRepository（`u3-event-store-repository`）

> NFR Design（Construction 3.3）成果物（Unit: U3、kind: library）。**改訂履歴**: 初版 2026-08-23（Bolt B5 着手前 — 自前 SQLite ストア・
> ワイヤ型・3 段の検査点・`within_write_transaction` 前提、レビュー READY Minor 3）→ 部分失効注記 2026-08-27 / 29（ADR-010 v2.0.0、B12 分割）
> → **再走 2026-09-07（本版、Modify）**: 現行コード（作業ツリー `e7191c0d`、プロダクトコードは `origin/main` = `f2b6b6a9` と同一）を実測して
> 全面更新した。旧 READY レビュー節は `review-history-20260823.md` へ逐語退避し、要旨を末尾の「Review 履歴」に残す。
>
> 出典: `../nfr-requirements/security-requirements.md`（NFR1.1〜1.3 / NFR2.1〜2.5 / NFR3.1〜3.5 / NFR4.1〜4.7、§3 STRIDE、§4 データ分類、
> 末尾レビュー R-01〜R-06）、`../nfr-requirements/tech-stack-decisions.md`（§1 選定、§3 未決 3 件）、`../functional-design/functional-spec.md`
> （§1 所有表、§2 契約、§3 保存と再構成、§4 永続化表現、§5 検証モデルの適用範囲、§7 検証範囲）、`../functional-design/rules.md`
> （BR1.1〜BR5.2）、`../../../inception/contract-design/contract-summary.md`（C1 のリトライ規約、C3 / C6）、実コード
> `modules/core/command/interface-adapter/src/orchestration/{intent_execution_repository_impl,store_failure,snapshot_strategy}.rs`・`dto/`、
> `modules/core/command/domain/src/orchestration/intent_execution.rs`、`modules/core/command/domain/src/workspace/store_path.rs`、
> `modules/app/aidlc/tests/{journal_protocol_conformance,crash_reconstruction_test}.rs`、`Cargo.toml` / `Cargo.lock`（実測）、
> 確認事項 `nfr-design-questions.md`（P1〜P5 = 2026-08-23 履歴、**P6〜P12 = 2026-09-07**、Looks correct）。
>
> 設計ステージの制約に従い、コードは ≤15 行の例示のみ。用語: **DTO** = アダプタ層が保存・復元に使う写し（Data Transfer Object）、
> **CAS** = 比較して一致したときだけ書く楽観ロック（Compare-And-Set、本家が snapshot 行の `version` で行う）、**RMU** = read-model-updater
> （U4、ジャーナルを読んで upstream 互換ファイルへ投影する側）、**ITF** = Quint が出力するトレース形式（実装に再生して突合する）、
> **manifest** = ジャーナル行に載る「型名/読み方の版」札（`intent-execution-event/1`）。

## 1. 設計方針

(a) **ストアは信頼しない入力** — ドメイン型に至るまでに四層の検査点（§2）を置き、分類できる破損は `Corrupt`（材料のみ、panic なし）で
止める。DTO として成立したのに壊れている遷移（未知ステージ・不変条件違反）だけは `replay` のクラッシュ境界で止める（オーナー裁定
2026-08-30 — 記録された歴史は書込時に検査済みであり、壊れていたら回復せず止まるのが正）。(b) **書込は本家の Tx 1 回に閉じる** —
`store` は本家を呼ぶ前に ID を照合し、Tx と CAS は本家に委ね、競合はストアの状態を変えずに `Conflict`。(c) **失敗は材料だけを運ぶ** —
`RepositoryError<Id>` 4 変種、再試行の政策はユースケース（`Conflict` のみ再水和 + 1 回、C1「2 回続いたら exit 1」）。(d) **退役は維持する**
— ロック機構・自前 DDL・接続露出の再導入なし、grep 0 件を合格基準に残す。(e) **協定は Quint で検証し ITF で実装に縫い付ける** — ただし
モデルは `every(1)` 限定で、間欠更新は実装テストが担う。(f) **依存を変えず、ドメインは永続化知識から中立** — `core-command-domain` の
`Cargo.toml` に serde / ESA が無いことが機械強制。

## 2. 検査点（NFR3.2 / NFR3.4 / NFR4.3 / NFR4.4）

読取経路 `find_by_id`（実測 `intent_execution_repository_impl.rs:331-439`）と書込経路 `store`（`:441-481`）の検査を、上流から順に置く。
分類は**ポート契約に載せず**、`RepositoryError::Corrupt { id, seq_nr, source }` の `source` にアダプタ私有 `CorruptDetail`
（`MissingSnapshot` / `ForeignManifest` / `SequenceGap` / `Undecodable(DtoDecodeError)` / `StoreDeserialization` / `WriteContract`、
`:101-113`）を `Error::source` 連鎖で運ぶ（BR1.5）。

| 層 | 場所 | 検査 | 失敗の写像 | 固定するテスト（実測 PASS） |
|---|---|---|---|---|
| (0) 書込前 | `store` 冒頭（`:447-452`） | `event.aggregate_id() != aggregate.id()` — 同じ Rust 型で別実行のイベントを渡せるため、本家を呼ぶ**前**に拒む（BR1.3） | `Corrupt { seq_nr: Some(aggregate.seq_nr()), source: WriteContract }`、I/O なし | 契約 `an_event_from_another_execution_is_rejected_before_writing`（memory / SQLite。genesis 双方 NotFound 維持・更新拒否後の元状態維持） |
| (0') 本家の書込契約 | `write_error`（`:277-303`） | 本家の `SerializationError` / `ContractViolation`（`seq_nr == 0`、genesis と `expected_version ≠ 0` の対応崩れ — 我々が封筒を組み違えたときだけ出る） | `Corrupt(WriteContract)` | 契約 `a_genesis_with_a_non_zero_version_is_a_contract_violation` |
| (1) ストア復号 | `read_error`（`:245-265`） | 本家 serde が payload を DTO に復号できない（`EventStoreReadError::DeserializationError`） | `Corrupt { seq_nr: None, source: StoreDeserialization }` | 実装固有 `a_tampered_snapshot_payload_is_corrupt` / `a_journal_row_with_an_unknown_event_type_is_corrupt`（source 文字列 `store deserialization failed`） |
| (2) DTO → ドメイン | `IntentExecutionDto::to_domain`（`dto/intent_execution_dto.rs:208-293`）、`IntentExecutionEventDto::to_domain` | 綴り・文法（閉集合外の語、Domain Primitive の文法外 → `DtoDecodeError::Malformed { field, value }`）と不変条件（`InvariantViolation`）。基底は検査付き完全コンストラクタ `IntentExecution::new`（cursor / parked_at の範囲、`seq_nr ≥ 1`、`check_invariants` — `intent_execution.rs:290-345`）を**必ず**通る | `Corrupt(Undecodable(e))`（基底は `seq_nr: None`、差分行は `Some(seq_nr)`） | 実装固有 `a_tampered_snapshot_state_is_corrupt` / `a_snapshot_that_breaks_the_aggregate_invariants_is_corrupt` / `a_replayed_row_whose_spelling_is_outside_the_closed_set_is_refused`（`undecodable payload`）、DTO インライン `dto/tests.rs` 29 件 |
| (3) 差分行の整合 | `find_by_id` の差分ループ（`:378-438`） | 基底欠落 + journal あり（`MissingSnapshot`）、通番の連続（`checked_add` で枯渇も `SequenceGap`）、manifest = `intent-execution-event/1`（`ForeignManifest` — 本家は manifest を検証しないので我々が拒む、RMU `decode_entry` と対称）、payload の `aggregate_id` ≠ 要求 ID（`Undecodable(InvariantViolation)`） | `Corrupt { seq_nr: Some(row) }`、部分集約を返さない | 実装固有 `a_journal_without_a_snapshot_is_corrupt_not_missing` / `a_gap_in_the_delta_rows_is_corrupt_not_a_crash` / `a_journal_row_with_a_foreign_manifest_is_refused_before_replay` / `a_delta_row_whose_payload_names_another_execution_is_corrupt` |
| (4) クラッシュ境界 | `IntentExecution::replay` → `apply_event`（`:352` / `:1514`） | 通番 = 現在値 + 1、イベントのステージ slug が `slots` に存在、適用後の不変条件 | **panic**（`# Panics` 明記、`missing_panics_doc` 緑）。`# Panics` を持つ公開 API は誕生変換 `From<(Started, DateTime<Utc>)>`（`:2373`）を含む **3 か所**（正本 `intent_execution.rs:40-41`、nfr-requirements R-02） | 実装固有 `a_replayed_event_naming_a_stage_outside_the_plan_crashes_reconstruction`（`#[should_panic(expected = "apply_event: corrupted history")]`） |

- **検出範囲の限定（NFR4.4、R-05）**: 検出できるのは不変条件・通番・manifest・ID を破る改竄に限る。不変条件を満たす形に書き換えた改竄
  （別の妥当なステージ・判定への差し替え）は検出せずそのまま真実として再生する。暗号学的完全性（署名 / HMAC）は要求しない（ローカル
  単一ユーザ、git 管理外 — 必要なら後続 intent）。
- **末尾欠落は検出しない**（BR1.2）: 戻された差分の途中欠落は (3) で捕まるが、末尾の行が消えた場合は別の終端記録が無いため検出できない
  （`a_foreign_manifest_before_the_snapshot_base_is_not_read` / `a_snapshot_alone_is_a_sufficient_rehydration_base` が範囲を示す）。
  これはジャーナル削除を許容した承認ではない。
- **panic の射程は (4) だけ**（NFR4.3）: `unwrap_used` / `expect_used` / `indexing_slicing` / `panic` は workspace lints で deny（実測
  `Cargo.toml:39-44`）。例外は `intent_execution.rs` の 3 か所と `SnapshotStrategy::default` の `unwrap_or(NonZeroUsize::MIN)`（panic
  しない形）。アダプタは `Option` / `Result` だけで組む（`checked_add`、`let-else`）。
- **NFR3.4（書込前 ID 照合）** は (0) が担う。照合は ID の一致だけを保証し、同 ID の任意イベントと状態の意味的対応は証明しない。

```text
// find_by_id の検査経路（例示）
snapshot = store.get_latest_snapshot_by_id(key)?         // (1) DeserializationError → StoreDeserialization
none → journal(since 1) が空なら NotFound、あれば Corrupt(MissingSnapshot)
base  = snapshot.aggregate().to_domain()?                  // (2) Malformed / InvariantViolation → Undecodable
delta = store.get_events_by_id_since_seq_nr(key, base.seq_nr()+1)?
for row in delta: seq 連続 / manifest / payload.to_domain() / aggregate_id == id   // (3)
IntentExecution::replay(base, rows).with_version(snapshot.version())               // (4) panic 境界
```

## 3. 書込の原子性と競合（NFR3.3 / NFR4.7）

- **経路の分岐**（`:453-480`、分岐は `:465`、BR2.3）: `seq_nr == 1` は必ず `persist_event_and_snapshot`（期待版 0、本家が初期版を採番）。以後は
  `SnapshotStrategy::wants_snapshot(seq_nr)` が真なら同関数（payload と基底通番を更新）、偽なら `persist_event`（基底を維持し行の
  `version` だけ進む）。封筒は適用後集約の `id` / `seq_nr` / `last_updated_at` とイベント DTO から組み、manifest を付ける（BR2.6 —
  Repository は時計を持たない）。
- **Tx と CAS は本家が所有**: 我々は接続も Tx も PRAGMA も持たない（`interface-adapter/src/` に `PRAGMA` / `busy_timeout` 0 件）。単一 Tx
  であること・COMMIT 前のクラッシュが何も残さないこと・競合時にストアが変わらないことは、クラッシュ再構成テスト
  （`a_transaction_abandoned_by_a_crash_leaves_nothing_behind` ほか 5 件）と ITF 適合（`conflict_rejected` / `no_lost_update`）が実装に
  対して確認する。
- **競合の材料**: 本家の `OptimisticLockError` は整形済み文字列 1 本なので解析せず、競合時だけ `stored_version`（`get_latest_snapshot_by_id`
  の再読取、読めなければ 0）で `actual` を作る（`:313-321`）。再読取は材料のためだけで判定に関与しない。呼出側の集約は `&` なので動かず、
  続けて書くには `find_by_id` から取り直す（契約 `a_write_from_the_rehydrated_version_succeeds` / `a_write_from_a_stale_version_conflicts`）。
- **直列化の実体は CAS であって `&mut self` ではない**（NFR4.7、R-01）: `reopened()`（`:209-215`）は同じストアを指す 2 本目のハンドルを
  返し（本家の `Clone` が接続 / 表を共有）、各ハンドルは独立に `&mut` を取れる。同一プロセス複数ハンドルの同時書込を止めているのは本家の
  楽観 version CAS（契約 `concurrent_rehydration_conflicts` / `genesis_twice_conflicts`）。設計上の立場: (i) 合成ルート（U7）は 1 コマンド
  につき 1 ハンドルを開く（現行 `runtime.rs` の `open` 呼出 8 か所はいずれもコマンド単位）、(ii) `reopened()` はテストと再オープン相当の
  用途に限り、複数ハンドルの並行モデルは複数プロセスと併せて U7 の裁定に繰り延べる。
- **別プロセスの並行書込**: 本家接続に `busy_timeout` を設定できないため待たずに `SQLITE_BUSY` → `Io { kind: WouldBlock }`（黙って失敗
  しない）。RMU の別接続（`JournalReaderImpl::open_with_busy_timeout`、既定 5000ms）は U4 の所有で、本 Unit の保証ではない。

## 4. 障害ドメインと扱い（P8）

| 障害 | 検出 | 材料 | 扱い |
|---|---|---|---|
| ストア I/O（権限・ディスク・親 dir 欠落・DB でないファイル） | `store_failure::io_kind`（`store_failure.rs:20-34`）が rusqlite の code を写す: `DatabaseBusy` / `DatabaseLocked` → `WouldBlock`、`CannotOpen` / `NotFound` → `NotFound`、`PermissionDenied` / `ReadOnly` / `AuthorizationForStatementDenied` → `PermissionDenied`、`DatabaseCorrupt` / `NotADatabase` → `InvalidData`、`OperationInterrupted` → `Interrupted`、他 `Other`。SQLite 由来でない失敗は `Other` | `Io { kind, path }`（memory は `path: None`） | 呼出側へ返す。再試行なし。ストアの自動修復・親 dir の作成はしない（`opening_under_a_missing_parent_directory_is_a_not_found` / `a_broken_store_reports_the_file_it_was_reading` / `a_missing_journal_table_fails_the_not_found_check`） |
| 競合 | 本家 `OptimisticLockError` | `Conflict { expected, actual }` | ユースケースが再水和 + 1 回再試行（U5）。2 回続けば exit 1 + 逐語文言（C1、U7） |
| 破損（6 原因） | §2 (1)〜(3) | `Corrupt { id, seq_nr, source }` | 中断。部分集約を返さない。投影（U4）はジャーナルから冪等に再生成できるが、ジャーナル自体の破損は利用者の操作（バックアップからの復元）— 本 Unit は検出まで |
| クラッシュ境界 | §2 (4) | panic（`apply_event: corrupted history`） | プロセス終了。ストアは変わらない（読取経路で起きる）。書込時に検査済みの歴史だけが入る前提の破れであり、再現手順と DB を添えて調査する |
| Busy | `WouldBlock` | `Io { kind: WouldBlock, path }` | 待たずに中断し再実行を促す（文言は U7）。並行モデルは U7 の裁定 |

## 5. サプライチェーンと境界（NFR4.1 / NFR4.2 / NFR4.5 / NFR4.6）

- **依存を変えない**（NFR4.1、P10）: 中核 `event-store-adapter-rs = "=3.0.0"`（`sqlite` feature、完全固定 — 本家スキーマに結合）、
  `rusqlite 0.40.2`（`bundled`、本家と同じ crate を共有し `store_failure` の分類にだけ使う）、`serde` / `serde_json` / `chrono` はアダプタ所有。
  推移依存に `thiserror 2.0.20` が本家経由で入る（実測 `cargo tree`）が、我々は直接使わず手実装エラー enum の様式を維持する。
  `core-command-domain` の `[dependencies]` に serde / ESA は無く（`chrono` / `uuid v7` / `core-infrastructure` のみ）、`core-command-use-case`
  にも ESA は無い（実測）。本 Unit の Bolt で `Cargo.toml` / `Cargo.lock` の diff はゼロ。`cargo audit` は CI `audit` ジョブで走るが
  advisory（`ci-success` の `needs` 外）。
- **`unsafe_code = "forbid"`**（NFR4.2）: `[workspace.lints.rust]` で workspace 全体。SQLite の unsafe は `libsqlite3-sys` 内で依存として
  受け入れる。
- **境界を薄く保つ**（NFR4.5）: `interface-adapter/src/` に `std::env` / `println!` / `eprintln!` / `tracing::` 0 件（実測）。`print_stdout`
  deny と `cargo lint` `command-side-io`（`*_repository_impl.rs` 以外の fs / 乱数 / プロセス / ネットワーク I/O を所見）で機械強制。
  ペイロードの人間入力（`request` / `user_input` / `feedback` / `reason`）は逐語で保存し、加工・要約・ログ出力しない。
- **パスと権限**（NFR4.6）: 場所は `StorePath::for_space(aidlc_root, space)` = `<aidlc root>/spaces/<space>/intents/.aidlc-store.sqlite`
  （`store_path.rs:17-31`。生の `PathBuf` を受け取る口は無い）。先頭ドットで upstream の `.gitignore`（`aidlc/spaces/*/intents/.aidlc-*`）に
  掛かり git 管理外。umask 既定で作成し、親 `intents/` が無ければ作らず `Io { kind: NotFound }`（ディレクトリ構造の権威は upstream）。
  `open` は本家 `EventStoreForSqlite::new` の失敗を `Io` に写し、表と索引は本家が冪等に作る（`:147-166`）。

## 6. 退役の維持（NFR1.1 / NFR1.2 / NFR1.3 / NFR3.5）

- 初版の退役手順（P4）は B6 / B7 で完了した。本版で守るのは**再導入しない**こと: ロック機構（`WorkspaceLock` / `FsWorkspaceLock` /
  `LockProtocol` / `ProcessProbe` / `audit_lock.qnt`）・自前 3 表 DDL・接続 / Tx を露出する独自 `EventStore` ポート・
  `within_write_transaction`・ワイヤ型・domain の serde memento（BR2.4 / BR3.1 / BR3.2 / BR4.3）。合格基準は `modules/` / `formal/` の
  grep 0 件（`formal/orchestration/` = engine_loop / journal_protocol / stop_hook、実測）。
- **本家スキーマへの非干渉**（NFR1.3 / NFR3.5）: DDL・DELETE・UPDATE を発行しない。全集約横断の単調カーソル = 本家 `journal` の
  rowid（追記専用）を壊さない。チェックポイント表・`read_*` 表は U4 の所有で本 Unit は触らない。ピンを本 Unit で動かさない。
- **逸脱登録**（NFR1.1）: `deviations.md` #4 は v3.0.0 まで登録済み（達成済み）。観測可能な差を増やさない — ストアのパス・git 管理外・
  ロック dir 非生成・互換ファイル内容不変。

## 7. 決定性と協定の維持（NFR2.1〜NFR2.5 / NFR3.1）

- **再構成の決定性**（NFR3.1）: 最新スナップショットを基底に `base.seq_nr()+1` 以降の差分だけを通番順に `replay` し、
  `with_version(snapshot.version())` で版を保持する。時刻・乱数・環境を読まない。基底以前の journal は再検査しない。実装固有
  `the_rehydration_base_is_the_snapshot_not_the_journal_head`（インライン）/ `a_stale_snapshot_plus_delta_matches_the_freshest_state` /
  `a_replay_does_not_move_the_version_the_store_assigned` が固定する。
- **TDD と両バックエンド契約**（NFR2.1 / NFR2.2）: 契約 11 関数 × memory / SQLite = 22（`contract_tests!` マクロで同一関数群を課す、
  BR2.7）+ 実装固有 23 + 本家適合 5 関数 × 2（`upstream_event_store_conformance.rs` — DTO が本家契約を満たし往復でドメインへ戻る）。
  アダプタに PBT は無い（`proptest` は `core-command-domain` / `core-infrastructure` / `core-query/use-case` の dev-dependency、R-06）。
  決定性の証拠は契約・実装固有テストが再実行で同結果であること（相対ゲート差 0.00pp）。
- **カバレッジ**（NFR2.3）: 絶対 90% 床 + 相対 `TOLERANCE=0.01`、`PROPTEST_RNG_SEED=20260823` 固定。アダプタに除外を足さない。
- **規則の機械強制**（NFR2.4）: workspace lints deny、`cargo lint` 7 ルール（本 Unit に直接効くのは `port-naming` = ポート名 `XxxRepository`
  と `command-side-io`）、rustfmt。CI `check` ジョブ（fmt → clippy → lint → test、`tools/lint` の 3 ステップ含む）。
- **Quint / ITF の適用範囲**（NFR2.5、BR3.3 / BR3.5、P9）: `journal_protocol.qnt` は 1 集約・writer 2・投影 1・`every(1)` 限定
  （不変条件 8 / witness 4）。fixture 8 トレース（`trace-0xa1` 〜 `trace-0x202`）を `journal_protocol_conformance.rs` が
  `SnapshotStrategy::every(1)` 明示で `IntentExecutionRepositoryImpl` + `JournalReaderImpl` + 実 RMU 投影に再生する（`:129,:226-227`）。
  既定 10 / 任意 N の間欠更新と差分再生はモデルの証明範囲に含めず、実装固有 `the_first_store_always_writes_the_snapshot_base` /
  `the_strategy_refreshes_the_snapshot_every_n_events` が担う。「モデルは 1 文字も変えずに通った」は履歴であり現在の合格証拠にしない。
  モデル内の版の算術は抽象化で、Repository が版を通番から計算する規則ではない（`version_equals_journal`）。

## 8. 失敗の扱い（プロセス）

- 受入（FD BR5.2）のいずれかが落ちたら PR を戻す。設計に無い判断が要ったら推測で進めず developer-report の「設計質問」に書く。
- 本文の限定主張（「〜だけ」「〜のみ」「n 件」）は repo 全体の grep / テスト実行で裏を取ってから書く（nfr-requirements 再走の教訓 —
  R-01 / R-02 / R-04 / R-06 はいずれも限定主張の反証だった）。引用行番号は全ファイル `grep -n` の絶対行。
- 上流 `requirements.md:133-135`（NFR3 の合格基準 `audit_lock.qnt` / 集約名 `WorkflowExecution`）は失効している（R-03）。本 Unit は
  `journal_protocol.qnt` / `IntentExecution` へ読み替えて設計するが、上流の改訂要否はオーナー裁定に載せる（後方ジャンプか人間裁定）。

## 9. 要求への対応

| 要求 | 設計上の手当て |
|---|---|
| NFR1.1 | 逸脱 #4 の維持、観測可能な差を増やさない（§6） |
| NFR1.2 | ロック機構を再導入しない、grep 0 件（§6） |
| NFR1.3 | DDL 非発行・ピン不変・本家 2 表への非干渉（§6） |
| NFR2.1 | 契約テスト先行、両バックエンド同一関数群（§7、logical-components §4） |
| NFR2.2 | 決定性 — PBT なし、契約・実装固有テストの再実行同値（§7） |
| NFR2.3 | 90% 床 + TOLERANCE 0.01、アダプタに除外なし（§7） |
| NFR2.4 | workspace lints deny、`cargo lint` port-naming / command-side-io、CI check（§7） |
| NFR2.5 | Quint every(1) 限定 + ITF 8 トレース、間欠更新は実装テスト（§7） |
| NFR3.1 | 基底 + 差分の決定的再構成、版の保持（§7） |
| NFR3.2 | 検査点の四層と `CorruptDetail` 6 変種、クラッシュ境界（§2） |
| NFR3.3 | 本家 Tx + CAS、genesis / 間欠 snapshot の分岐、`Conflict` の材料（§3） |
| NFR3.4 | 書込前 ID 照合 = 層 (0)（§2） |
| NFR3.5 | チェックポイント・投影は U4、rowid カーソルを壊さない（§6） |
| NFR4.1 | 依存を変えない、domain / use-case に ESA なし、thiserror は推移のみ（§5） |
| NFR4.2 | `unsafe_code = forbid` workspace（§5） |
| NFR4.3 | panic の射程は層 (4) の 3 か所だけ、lints deny、エラー写像（§2 / §4） |
| NFR4.4 | 検査点を省略しない、検出範囲の限定を明記（§2） |
| NFR4.5 | env / ログなし、command-side-io、逐語保存（§5） |
| NFR4.6 | `StorePath` 導出、umask 既定、親 dir を作らない（§5） |
| NFR4.7 | 単一プロセス前提、WouldBlock 即時、直列化の実体は CAS、`reopened()` と複数プロセスは U7（§3 / §4） |

## 10. 前版からの変更（2026-09-07 再走）

- §1 方針を (a)〜(f) に再編（検査点は「1 段」ではなく四層 + 書込前、クラッシュ境界を明示、依存不変と中立の機械強制を追加）。
- §2 検査点を旧「ワイヤ / Domain Primitive / `from_state`」から現行「本家 serde / DTO `to_domain` + `IntentExecution::new` / 差分行の
  整合 / `replay` クラッシュ境界 + 書込前 ID 照合」へ全面差し替え。`Corrupt { cause }` → `Corrupt { id, seq_nr, source }` + `CorruptDetail`
  6 変種。テスト名で各層を固定。検出範囲の限定・末尾欠落の非検出・`# Panics` 3 か所を追記。
- §3 を `=2.0.0` / `persist_event_and_snapshot` 単独から `=3.0.0` / genesis + `SnapshotStrategy` 分岐へ。`reopened()` と CAS の関係を新設。
- §5 の「domain / use-case が ESA に依存、domain に serde」は**誤り**として撤回（現行は adapter のみ）。`md5` 除去・`thiserror` 注記は
  現行値へ。
- §6 を「退役の手順」から「退役の維持」へ。§7 に ITF の置き場（`modules/app/aidlc/tests/`）と every(1) 限定を反映。
- §9 を 18 ID → 20 ID（NFR1.3 / NFR3.4 / NFR4.7 新設、NFR3.5 の意味変更）。旧レビュー節は `review-history-20260823.md` へ退避。

## Review 履歴（2026-08-23、iteration 1、READY）

> 初版に対する advisory レビュー（全文は `review-history-20260823.md`）。当時のセンサー結果は履歴であり、本再走の承認根拠には使わない。

| # | Severity | 要旨 | 本再走での扱い |
|---|---|---|---|
| 1 | Minor | `indexing_slicing` / `panic` の deny が未設定で、3 段検査点の減算・添字が panic しうる | 解消 — 実測 `Cargo.toml:43-44` に deny あり。§2 で panic の射程を層 (4) に限定 |
| 2 | Minor | 上流 `entities.md` の `## Review` が NOT-READY のまま（本文は是正済み） | 失効 — 2026-09-05 是正で FD 成果物は再レビュー READY（`functional-design/review-history-20260905.md`） |
| 3 | Minor | C3 の `usize` → `u64` 改訂提案が contract-summary に未着地 | 失効 — ADR-010 で本家に従い `usize` に復帰（C3 2026-08-27 改訂） |

## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-09-07T06:33:57Z
**Iteration:** 1

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Major | `construction/u3-event-store-repository/nfr-design/security-design.md` > §3（`reopened()` と CAS、別プロセスの並行書込）／§4 の Busy 行 | 並行の面が 2 つ（`reopened()` の複数ハンドル／別プロセス）しか数えられていないが、出荷済みの合成ルートは **1 コマンドで同一ファイルに 2〜3 本の独立接続**を同時に開く。実測 `modules/app/aidlc/src/runtime.rs:306-308` は `IntentExecutionRepositoryImpl::open(&store)` / `IntentRepositoryImpl::open(&store)` / `WorkflowDefinitionRepositoryImpl::open(&store)` を同じ `StorePath` に対して同時に開き（同型の 3 本組は `:817-819` / `:1589-1591`、2 本組は `:402-403` / `:461-462`）、`intent_repository_impl.rs:126-136` のとおり兄弟 Repository も `IntentSqliteStore::new(path)` で**自前の本家ストア（= 別の rusqlite 接続）**を作る。SQLite の書込ロックはファイル単位でプロセス単位ではないため、`busy_timeout` を設定できないという §3 の前提は同一プロセス内の兄弟接続にもそのまま効く。それにもかかわらず §4 の Busy 行は「並行モデルは U7 の裁定」、§3 は `WouldBlock` を別プロセスの現象としてのみ記述しており、§3 (i) の「1 コマンドにつき 1 ハンドルを開く（`open` 呼出 8 か所）」も `IntentExecutionRepositoryImpl` に限った数え方である。現状は各ユースケースが await を逐次に並べるため実害は観測されない（`cargo test --locked -p core-command-interface-adapter` 55 件・`-p aidlc --test crash_reconstruction_test` 5 件すべて PASS）が、NFR4.7 が OK で通る根拠から兄弟接続の面が抜けている | §3 に「1 コマンドは同一ファイルに兄弟 Repository の接続を複数持つ」という事実を書き、(a) 逐次 await と短い Tx により競合しないという安全性の根拠を示すか、(b) `reopened()` と同じく U7 の並行モデル裁定へ明示的に含める、のいずれかを選ぶ。§4 の Busy 行の「別プロセス」限定も同様に是正する | New |
| R-02 | Minor | `construction/u3-event-store-repository/nfr-design/logical-components.md` > §1 `orchestration::kinds_codec` の行 | 「利用者は `WorkflowDefinitionDto`（実測 `workflow_definition_dto.rs:92`）」という限定主張が反証される。実測 `grep -rn kinds_codec modules/` では同じ command アダプタ内に 3 か所の利用者があり、`compiled_definition_repository_impl.rs:325` と `:753` が `#[serde(with = "crate::orchestration::kinds_codec")]` で使っている（`workflow_definition_dto.rs:92` は 3 番目）。結論（U3 の DTO は使わない）は成立するが、列挙が不完全で、security-design §8 が自ら課した「限定主張は repo 全体の grep で裏を取る」規律に反する | 利用者を 3 か所（`workflow_definition_dto.rs:92`、`compiled_definition_repository_impl.rs:325` / `:753`）へ訂正する。いずれも兄弟 Repository の所有であるという結論は維持してよい | New |
| R-03 | Info | `construction/u3-event-store-repository/nfr-design/security-design.md` > §2 層 (2) の記述 | `DtoDecodeError::Malformed { field, value }` と書かれているが、実測のフィールド名は `value` ではなく `found`（`modules/core/command/interface-adapter/src/orchestration/dto/dto_decode_error.rs:10-15`、`Malformed { field: &'static str, found: String }`）。変種の構成（`Malformed` / `InvariantViolation` の 2 つ）と役割の記述自体は正しい | フィールド名を `found` に訂正する | New |
| R-04 | Info | `construction/u3-event-store-repository/nfr-design/security-design.md` > §2 層 (2) の引用行 | `IntentExecution::new`（`intent_execution.rs:290-345`）の終端行が実測とずれる。実測では `pub fn new(` が `:290`、本体の終わり（`Ok(execution)` → `}`）が `:336-337` であり、`:345` は次の `replay` の doc コメント内（`# Panics` は `:347`）に落ちる。§8 が「引用行番号は全ファイル `grep -n` の絶対行」と宣言しているため、範囲末尾のずれは自らの規律に対する例外になる | 範囲を `:290-337` に訂正する（開始行と検査内容の記述は実測どおり） | New |
| R-05 | Info | `construction/u3-event-store-repository/nfr-design/logical-components.md` > §1 `intent_execution_repository_impl` の「依存（実測 `Cargo.toml`）」列 | 列挙が外部 crate に限られ、同 `Cargo.toml` の `[dependencies]` に実在する内部 3 本（`core-command-use-case` / `core-command-domain` / `core-infrastructure`）が落ちている。同表の domain 行は内部依存（`core-infrastructure`）を挙げているため、表内で数え方が揃っていない | adapter 行にも内部 3 本を加える（または列の定義を「外部 crate のみ」と明示する） | New |
| R-06 | Info | `construction/u3-event-store-repository/nfr-design/security-design.md` > §8 第 3 項／`traceability.json` の NFR3.x 行 | 上流 `inception/requirements-analysis/requirements.md:133-135`（NFR3 の合格基準 = 「改訂版 `audit_lock.qnt` の ITF 準拠」、集約名 `WorkflowExecution`）が失効している事実は実測で確認した（`ls formal/orchestration/` = engine_loop / journal_protocol / stop_hook、`audit_lock` の grep は `modules/` / `formal/` で 0 件）。設計は読み替えを明示しオーナー裁定へ送っており、上流 nfr-requirements の R-03 と同じ扱いで処理は妥当だが、`traceability.json` では NFR3.1〜3.5 が一律 OK として通るため、この未決が承認ゲートの表からは見えない | 追加の設計変更は不要。承認ゲートで「上流 requirements.md:133-135 の改訂要否」が未決である旨を人間に提示し、裁定結果に応じて後方ジャンプするか上流を改訂する | New |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| `aidlc-sensor-required-sections`（`security-design.md`） | PASS（`h2_count: 11`、`findings_count: 0`） | 必須節の欠落なし。テンプレート未供給のため見出し名の突合は行われない |
| `aidlc-sensor-required-sections`（`logical-components.md`） | PASS（`h2_count: 6`、`findings_count: 0`） | 同上 |
| `aidlc-sensor-traceability`（`traceability.json`） | PASS（`gaps` / `orphans` / `missing_from_table` / `missing_from_upstream_ids` / `invalid_entries` / `invalid_targets` すべて空） | 上流 NFR 20 ID と coverage 表が一致。上流 `security-requirements.md` の実測 ID も NFR1.1〜1.3 / 2.1〜2.5 / 3.1〜3.5 / 4.1〜4.7 の 20 件で過不足なし |
| `aidlc-sensor-linter` / `aidlc-sensor-type-check` | 非適用 | 対象成果物は Markdown / JSON で TS・JS の出力を持たない |
| `cargo test --locked -p core-command-interface-adapter --test intent_execution_repository_contract --test intent_execution_repository_impl_test --test upstream_event_store_conformance` | PASS（22 + 23 + 10 = 55 件） | §7 の「契約 11×2 = 22 / 実装固有 23 / 本家適合 5×2」を実測で確認 |
| `cargo test --locked -p aidlc --test crash_reconstruction_test` | PASS（5 件） | §3 のクラッシュ境界の証拠 5 件を実測で確認 |

### Summary

現行コードでの全数裏取りを行い、設計の主要主張はほぼすべて実測と一致した — 検査点 5 層の関数・行番号（`find_by_id` `:331-439` / `store` `:441-481` / `read_error` `:245-265` / `write_error` `:277-303` / `stored_version` `:313` / `reopened` `:209` / `CorruptDetail` `:101-113` / 書込前照合 `:447-452` / 分岐 `:465` / `store_failure::io_kind` `:20-34`）、`# Panics` 3 か所（`:347` / `:1506` / `:2364`、誕生変換の `fn from` は `:2373`）、テスト件数（契約 22・実装固有 23・本家適合 5×2・インライン 10/4/2/29・クラッシュ再構成 5・ITF fixture 8）、DTO 32 エントリとイベント変種 16、CI 7 ジョブと `ci-success` の `needs` から `audit` を除く構成、Quint の不変条件 8 / witness 4（`scripts/quint-gate.sh` が同名で実行）、依存（domain / use-case に ESA・serde なし、ピン `=3.0.0` / `rusqlite 0.40.2`、`thiserror` は推移のみ）、退役 6 パターンの grep 0 件、アダプタ `src/` の `PRAGMA` / `busy_timeout` / `std::env` / `println!` / `eprintln!` / `tracing::` 0 件、`Clock` 系の他クレート利用 0 件、`runtime.rs` の `open` 8 か所と `SnapshotStrategy` 未配線 0 件 — いずれも一致した。上流 nfr-requirements の R-01〜R-06 はすべて本版に反映されている（R-01 は §3 の「直列化の実体は CAS」、R-02 は `# Panics` 3 か所、R-04 は DTO 32 / 16、R-05 は検出範囲の限定、R-06 は proptest 3 クレート）。BR・NFR・C3 / C6 の相互参照はすべて解決し、センサー 3 本も PASS した。Critical は 0 件で、Major は R-01 の 1 件のみ（同一プロセス内の兄弟 Repository 接続という並行の面が §3 / §4 の数え上げから漏れている点。逐次 await のため実害は現時点で観測されず、是正は記述の追加または U7 繰延の明示で足りる）であり、advisory の READY 閾値（Critical 0、Major ≤2）を満たす。残る懸念は 2 つ — (1) R-01 の兄弟接続の面を U7 の並行モデル裁定に確実に載せること、(2) R-06 の上流 `requirements.md:133-135` の失効が承認ゲートで人間に見えるようにすること。いずれも本 Unit の設計を作り直す必要はない。
