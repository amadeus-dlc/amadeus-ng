# security-requirements — U3 イベントストアと IntentExecutionRepository（`u3-event-store-repository`）

> NFR Requirements（Construction 3.2）成果物（Unit: U3、kind: library）。**改訂履歴**: 初版 2026-08-23（Bolt B5 着手前、レビュー READY Major 2 / Minor 1）→
> **再走 2026-09-07（本版、Modify）**: unit-major 再走で現行コード `origin/main` = `f2b6b6a9` を実測し、旧名（`WorkflowExecutionRepository` /
> `core-use-case` / `core-interface-adapter` / `core-domain`）・退役した機構（自前 SQLite ストア・ローカル `EventStore` trait・`within_write_transaction`・
> InMemory ダブル・Clock 注入・`from_state`・`updated_at`・`user_version`）・失効したエラー分類を現行へ同期した。構造（§1〜§5）は維持し、
> §6（前版からの変更）と Review 履歴を追加した。
>
> 出典: `../functional-design/functional-spec.md`（§1 責務・配置、§2 公開契約と版の持ち回り、§3 保存と再構成、§4 永続化表現、§5 検証モデルの適用範囲、
> §7 検証と確認できた範囲 — 2026-09-05 是正版・2026-09-07 再レビュー READY）、`../functional-design/rules.md`（BR1.1〜BR5.2）、
> `../../../inception/requirements-analysis/requirements.md`（NFR1〜NFR5、FR1.2 / FR1.3）、`../../../inception/contract-design/contract-summary.md`
> （C3 コマンド側ポート 4 本と `RepositoryError<Id>`、C6 本家 2 表 + 我々の表、§4 未解決 — 2026-09-07 現行化）、`Cargo.toml`（`[workspace.dependencies]` /
> `[workspace.lints]`）、`Cargo.lock`、`rust-toolchain.toml`、`.github/workflows/ci.yml`、`scripts/coverage.sh`（実測）、`aidlc/spaces/default/memory/team.md`
> （Testing Posture / CI ゲート / サプライチェーン裁定）、確認事項 `nfr-requirements-questions.md`（P1〜P6 = 2026-08-23 履歴、**P7〜P12 = 2026-09-07**、Looks correct）。
>
> 各要求は Inception の NFR ID を継承し枝番を付ける（NFR1.x〜NFR4.x）。NFR5 は非目標として §5 に置く。実測の所在は `path:line` または型・関数名で添える。

## 1. 範囲と信頼境界

- U3 の現行の所有（functional-spec §1 の表と同じ）: `core-command-use-case` の Repository ポート 4 本のうち **`IntentExecutionRepository`**
  （`orchestration/port/intent_execution_repository.rs` — `find_by_id(&IntentExecutionId)` / `store(&mut self, &IntentExecutionEvent, &IntentExecution)`）と
  共通エラー `RepositoryError<Id>`（`port/repository_error.rs`）、`core-command-interface-adapter` の **`IntentExecutionRepositoryImpl<S>`**
  （`orchestration/intent_execution_repository_impl.rs` — `open` = `EventStoreForSqlite` / `in_memory` = `EventStoreForMemory`）、`SnapshotStrategy`
  （`snapshot_strategy.rs`、既定 `every(10)`）、永続化 DTO 3 型（`dto/` — `IntentExecutionDto` / `IntentExecutionEventDto` / `IntentExecutionAggregateKeyDto`）、
  `core-command-domain` の `StorePath`（`workspace/store_path.rs`）と `IntentDirName`、形式モデル `formal/orchestration/journal_protocol.qnt` と ITF 適合
  `modules/app/aidlc/tests/journal_protocol_conformance.rs`。~~自前 SQLite ストア・ローカル `EventStore` trait・ロック機構~~ は退役済み（ADR-007 / ADR-010、BR3.1）。
  `JournalReader` と投影チェックポイント・`read_*` 表は U4（`core-read-model-updater`）の所有（BR1.4）。
- 入力: (a) ユースケース（U5 `CommitVerdictUseCase` 等、静的束縛）からの `store` / `find_by_id`、(b) ディスク上の SQLite ファイル
  `<aidlc root>/spaces/<space>/intents/.aidlc-store.sqlite`（`StorePath::for_space`、実測 `store_path.rs:17-31`。`.gitignore` の
  `aidlc/spaces/*/intents/.aidlc-*` で git 管理外 — 実測 `.gitignore:32`）。接続・DDL・Tx・CAS は本家 event-store-adapter-rs が所有し、我々は PRAGMA も
  接続も持たない（実測 `interface-adapter/src/` に `PRAGMA` / `busy_timeout` 0 件）、(c) 合成ルート（U7）から注入される `StorePath` と `SnapshotStrategy`
  （Clock は注入しない — 封筒の `occurred_at` は適用後集約から組む、BR2.6）。
- 出力: (a) 再構成した集約 `IntentExecution`（ドメイン型、`with_version` で版を保持）、(b) 本家が書く `journal` / `snapshot` 行（manifest
  `intent-execution-event/1`）、(c) `RepositoryError<IntentExecutionId>`（`NotFound { id }` / `Conflict { expected, actual }` / `Io { kind, path }` /
  `Corrupt { id, seq_nr, source }` — 材料のみ、文言なし）。
- 信頼境界: ストアファイルは**信頼しない入力**（DTO の復号 → `to_domain()` → 検査付き `IntentExecution::new`、差分の通番連続性・manifest・`aggregate_id`
  検査 — BR1.2）。DTO として成立した壊れた遷移（未知ステージ等）は `IntentExecution::replay` の**クラッシュ境界**であり、`Corrupt` に畳むとは約束しない
  （aggregate-commands 裁定 2026-08-30、BR1.5）。ユースケースから受けるドメイン型は境界の内側（Always Valid）だが、`event` と `aggregate` の**対の対応**は
  型では保証されないため書込前に ID 照合する（BR1.3、2026-09-05 R-07 是正）。本家ライブラリは上流の境界づけられたコンテキスト（Conformist — 契約を
  1 文字も変えない、upstream-contracts.md）。
- 秘密情報・資格情報・ネットワークを扱わない。環境変数を読まない（パスは注入）。ログ出力を持たない（実測 `interface-adapter/src/` に `std::env` /
  `println!` / `eprintln!` / `tracing::` 0 件 — P12）。

## 2. 要求

| ID | 要求 | 合格基準 | 出典 |
|---|---|---|---|
| NFR1.1 | **逸脱登録の維持** — ES 化の観測可能な差（`aidlc/spaces/<space>/intents/.aidlc-store.sqlite` の追加・git 管理外・ロック dir 非生成・互換ファイル内容不変）は `docs/specs/deviations.md` #4 に ADR-010 / v3.0.0 まで登録済み（**達成済み**、実測 `deviations.md:10`）。本 Unit の変更で観測可能な差を増やさない。upstream 互換ファイル（`aidlc-state.md` / 監査シャード）の形式には触れない（投影は U4） | `deviations.md` と `.gitignore` に U3 由来の diff が無い。`StorePath::for_space` のパスが `deviations.md` #4 と一致（実測 `store_path.rs:17,19`）。U1 ゴールデンパリティ・U4 投影ゴールデンが赤にならない | NFR1, BR2.1 / BR5.1, deviations #4 |
| NFR1.2 | **ロック dir を生成しない** — 旧 `WorkspaceLock` / `FsWorkspaceLock` / `LockProtocol` / `ProcessProbe` / `audit_lock.qnt` は退役済みで、mkdir ロック・`.aidlc-lock` 生成経路を再導入しない | `modules/` / `formal/` の grep 0 件を維持（実測 2026-09-07: `ls formal/orchestration/` = engine_loop / journal_protocol / stop_hook） | NFR1, ADR-007, BR3.1 / BR3.2 |
| NFR1.3 | **本家スキーマへの非干渉** — `journal` / `snapshot` の DDL は本家所有。我々は DDL を発行せず、旧自前 3 表を作らない。ピン `event-store-adapter-rs = "=3.0.0"` を本 Unit で動かさない | `Cargo.toml` / `Cargo.lock` のピンが `=3.0.0` のまま（実測）。RMU のスキーマガード（`the_upstream_journal_schema_is_the_pinned_one`、U4）が緑のまま | NFR1, BR2.2, C6 |
| NFR2.1 | **TDD** — 契約テスト（`intent_execution_repository_contract.rs`、memory / SQLite の両バックエンドに同一関数群）を先に赤で書き、実装で緑にする。ITF・Quint は外側のゲート | 新規テストは PR のコミット列で tests → src の順。`cargo test --workspace` 全緑（実測 2026-09-07: 契約 22 + 実装固有 23 = 45 件 PASS） | NFR2, team.md Testing Posture, BR2.7 |
| NFR2.2 | **再構成テストの決定性** — アダプタに PBT は無い（`proptest` は `core-command-domain` の値オブジェクトのみ — 実測）。決定性の証拠は契約テスト（保存 → 再取得の同値・Conflict・参照不変）と実装固有テスト（初回必須・間隔 2 での基底更新・古い基底からの差分再生・版維持）で、時刻・乱数・環境に依存しない | 45 件が再実行で同結果。`scripts/coverage.sh` の相対ゲートで差 0.00pp（実測 `:76`「PBT シード固定 — 再計測で差 0.00pp」） | NFR2, BR1.2 / BR2.3 / BR2.7 |
| NFR2.3 | **カバレッジ** — 絶対 90% 床 + PR 相対ゲート `TOLERANCE=0.01`（`PROPTEST_RNG_SEED=20260823` 固定 — 実測 `scripts/coverage.sh:40-43`。旧 pending-revision 1「0.05 → 0.01 の引き締め」は**達成済み**）。アダプタクレートに除外を足さない（除外は合成ルートのみ、`IGNORE_FILENAME_REGEX`） | CI `coverage` ジョブ緑（pull_request は絶対 + 相対、merge_group は絶対のみ — 実測 `ci.yml:150-156`） | NFR2, team.md |
| NFR2.4 | **規則の機械強制** — workspace lints deny（`unwrap_used` / `expect_used` / `indexing_slicing` / `panic` / `print_stdout` / `dbg_macro` ほか — 実測 `Cargo.toml:39-69`）、`cargo lint` 7 ルール（`port-naming` / `command-side-io` / `no-public-fields` / `one-public-type` / `dao-single-table` / `checkbox-vocabulary` / `use-case-domain-getter` — 実測 `tools/lint/src/`。本 Unit の Repository 実装には `port-naming`（ポート名は `XxxRepository`）と `command-side-io`（I/O は `*_repository_impl.rs` のみ）が直接効く）、rustfmt | CI `check` ジョブ緑（fmt → clippy → lint → test、`tools/lint` の 3 ステップ含む — 実測 `ci.yml:74-95`） | NFR2, team.md Code Style, gateway-taxonomy §1d |
| NFR2.5 | **Quint ゲート** — `journal_protocol.qnt` の typecheck / 不変条件 8 本（conflict_rejected / snapshot_tracks_journal / version_equals_journal / checkpoint_monotone / checkpoint_bounded / projection_idempotent / truth_is_journal / no_lost_update）/ witness 4 本（w_conflict / w_crash_then_catchup / w_idempotent_catchup / w_interleaved_writers）が `scripts/quint-gate.sh` で緑。モデルは 1 集約・writer 2・投影 1・`every(1)` の構成限定で、既定 10 の間欠更新の証明とは主張しない | CI `quint` ジョブ緑。ITF fixture 8 トレース（`tests/conformance/fixtures/journal_protocol/`、実測）を `journal_protocol_conformance.rs` が `every(1)` 明示で再生（実測 `:226-227`） | NFR2, BR3.3 / BR3.4 / BR3.5, ADR 0003 |
| NFR3.1 | **再構成の決定性** — `find_by_id` は最新スナップショット（`get_latest_snapshot_by_id`）を基底に `base.seq_nr()+1` 以降の差分だけを通番順に `replay` し、`with_version(snapshot.version())` で版を保持する（2026-09-05 裁定）。同じ DB から何度読んでも同値。時刻・乱数・環境を読まない。基底以前の journal は再検査しない | ラウンドトリップ契約テスト（両バックエンド）+ 実装固有（基底 + 差分、版維持、snapshot 単独復元）。実測 `intent_execution_repository_impl.rs:331-438` | NFR3, BR1.2, FR1.3 |
| NFR3.2 | **健全性検査** — 基底欠落（journal あり）・ストア復号失敗・DTO 検査失敗・差分通番の飛び・foreign manifest・別実行 payload・書込契約違反は `Corrupt { id, seq_nr, source }` で拒否し `NotFound`（基底も journal も無い）と区別する。原因は**ポート契約に載せず**アダプタ私有 `CorruptDetail` 6 変種（MissingSnapshot / ForeignManifest / SequenceGap / Undecodable / StoreDeserialization / WriteContract — 実測 `intent_execution_repository_impl.rs`）を `Error::source` で運ぶ。DTO として成立した壊れた遷移は `replay` のクラッシュ境界（`# Panics` 明記） | 実装固有テスト（基底欠落・DTO 破損・通番飛び・foreign manifest・別実行 payload・未知ステージでのクラッシュ — functional-spec §7）。`RepositoryError` は `PartialEq` を持たず、テストは `matches!` + `source` 文字列で判定 | NFR3, BR1.2 / BR1.5, error-handling.md |
| NFR3.3 | **原子性と楽観 version** — Tx と CAS は本家が所有（`persist_event` / `persist_event_and_snapshot`）。期待版は `aggregate.version()` そのまま（`seq_nr` から導かない）。genesis は `seq_nr == 1`・版 0 で `persist_event_and_snapshot`、以後は `SnapshotStrategy::wants_snapshot(seq_nr)` が真のときだけ基底も更新。競合は `Conflict { expected, actual }`（actual は診断のための再読取、読めなければ 0）でストア状態は変わらない。`store` 成功後も呼出側の集約は動かない（続けて書くには `find_by_id` から取り直す） | 契約テスト Conflict（両バックエンド）+ ITF（conflict_rejected / snapshot_tracks_journal / version_equals_journal / no_lost_update）+ クラッシュ再構成 `modules/app/aidlc/tests/crash_reconstruction_test.rs`（実測 5 件 PASS）。実測 `intent_execution_repository_impl.rs:453-476` | NFR3, BR1.3 / BR2.3, FR1.2, ADR-010 |
| NFR3.4 | **書込前 ID 照合（新設）** — `event.aggregate_id() != aggregate.id()` は本家ストアを呼ぶ**前**に `Corrupt(WriteContract)` で拒否する。旧「型により構成不能」の主張は撤回済み。照合は ID の一致だけを保証し、同 ID の任意イベントと状態の意味的対応は証明しない | 契約テスト `an_event_from_another_execution_is_rejected_before_writing`（両バックエンド、genesis 双方 NotFound 維持・更新拒否後の元状態維持）。実測 `intent_execution_repository_impl.rs:447-452` | NFR3, BR1.3, functional-spec §3.1 / §7（2026-09-05 R-07 是正） |
| NFR3.5 | **チェックポイント・投影の所有は U4** — 旧 NFR3.4（`advance_checkpoint` 単調性）/ 旧 NFR3.5（`within_write_transaction`）は所有移転・退役で U3 の要求から外す。U3 が守るのは「全集約横断の単調カーソル = 本家 journal の rowid（追記専用）」を壊さないこと（DDL・削除を発行しない） | NFR1.3 と同じ（ピン不変 + RMU スキーマガード緑）。`JournalReader` 契約の検証は U4 の成果物に従う | NFR3, BR1.4 / BR2.2 / BR2.4, C6 制約 (5) |
| NFR4.1 | **依存を変えない** — 中核 `event-store-adapter-rs = "=3.0.0"`（`sqlite` feature）、`rusqlite = 0.40.2`（`bundled` — 本家と同じ crate を共有、`store_failure` のエラー分類にだけ使う）、`serde` / `serde_json`（`preserve_order` / `float_roundtrip`）/ `chrono` はアダプタ所有。`core-command-domain` の `[dependencies]` に serde / ESA が無い（domain-persistence-neutrality の機械強制、実測）。`core-command-use-case` は domain + chrono、dev に tokio のみ。`cargo audit` は CI `audit` ジョブ（workspace + `tools/lint` の独立 `Cargo.lock`）で走るが **advisory**（`ci-success` の `needs` 外 — 実測 `ci.yml`。旧 pending-revision 3 の注記） | 本 Unit の Bolt で `Cargo.toml` / `Cargo.lock` の diff がゼロ。`cargo audit` 緑（advisory） | NFR4, team.md サプライチェーン, P7, ADR-010 |
| NFR4.2 | **`unsafe_code = "forbid"` 維持** — `[workspace.lints.rust]` で workspace 全体に適用済み（実測 `Cargo.toml`）。`tools/lint` は detached のため同宣言を自前で持つ。自クレートに unsafe を書かない（SQLite の unsafe は `libsqlite3-sys` 内で、依存として受け入れる） | clippy / rustc 緑 | NFR4, FR9.2 |
| NFR4.3 | **プロダクト経路で panic しない** — `unwrap_used` / `expect_used` / `indexing_slicing` / `panic` を deny（実測 `Cargo.toml:39-44`。旧 pending-revision 2「`indexing_slicing` / `panic` の昇格」は**達成済み**）。例外は 2 つだけ: (a) `IntentExecution::replay` / `apply_event` のクラッシュ境界（壊れた歴史はクラッシュが正 — `# Panics` 明記、allow に理由）、(b) `SnapshotStrategy::default` の `unwrap_or(NonZeroUsize::MIN)`（panic しない形）。本家が `Box<dyn Error>` に包む SQLite の失敗は `store_failure` で `Io { kind, path }`（`DatabaseBusy` → `WouldBlock`、その他 `Other`）/ `Corrupt` / `Conflict` へ写す | clippy deny + エラー写像のテスト（実測 `intent_execution_repository_impl.rs:568,688,763`） | NFR4, error-handling.md, aggregate-commands.md |
| NFR4.4 | **改竄の検出（完全性）** — ストアは信頼しない入力。復号後の検査（NFR3.2）を省略しない。暗号学的完全性（署名 / HMAC）は要求しない（ローカル単一ユーザ、git 管理外 — 必要なら後続 intent） | NFR3.2 のテスト | NFR4, P3 / P9 |
| NFR4.5 | **秘密情報・ログ・環境変数を扱わない** — `std::env` / `println!` / `eprintln!` / `tracing::` が `interface-adapter/src/` に 0 件（実測）。`print_stdout` deny と `command-side-io`（`*_repository_impl.rs` 以外の fs / 乱数 / プロセス / ネットワーク I/O を所見）で機械強制。ペイロードの人間入力は逐語で保存（加工・要約しない） | grep 0 件の維持 + `cargo lint` 緑 | NFR4, P12, audit-format「Human decisions recorded verbatim」 |
| NFR4.6 | **ファイル権限とパス** — ストアは umask 既定で作成（upstream のワークスペースファイルと同じ）。パスは `StorePath::for_space(aidlc_root, space)` = `<aidlc root>/spaces/<space>/intents/.aidlc-store.sqlite`（実測 `store_path.rs:17-31`）。親ディレクトリ `intents/` が無ければ作らず `Io { kind: NotFound }`（ディレクトリ構造の権威は upstream レイアウト） | 実装固有テスト `opening_under_a_missing_parent_directory_is_a_not_found`（実測 `intent_execution_repository_impl_test.rs:403`） | NFR4, BR2.1 |
| NFR4.7 | **並行アクセスは単一プロセス前提（新設）** — 本家のストア接続に `busy_timeout` を設定できないため、別プロセスの並行書込は `SQLITE_BUSY` → `Io { kind: WouldBlock }` で呼出側へ返す（黙って失敗しない。再試行政策はユースケース）。同一プロセス内は `&mut self` の排他で直列化（内部可変性なし）。複数プロセスの並行モデルと登録簿 `intents.json` の直列化は U7 の裁定事項 | エラー写像テスト（`DatabaseBusy` → `WouldBlock`、実測 `intent_execution_repository_impl.rs:568,688`）。U7 裁定まで並行テストは追加しない（contract-summary §4 の未決を引き継ぐ） | NFR4, C6 §4, BR2.4, interior-mutability.md |

## 3. 脅威の検討（STRIDE、ライブラリ規模）

| 区分 | 該当 | 扱い |
|---|---|---|
| Spoofing / Elevation of Privilege | 該当なし（認証・認可を持たないローカル CLI。誰がコマンドを発行したかは U7 / フック側） | — |
| Tampering | ストアファイルの改竄・欠損・部分破損、基底と差分の不整合、別実行のイベントを対にした保存、並行書込による上書き | NFR3.2（`Corrupt` 6 原因 + クラッシュ境界）、NFR3.3（本家 Tx + 楽観 version、`Conflict`）、NFR3.4（書込前 ID 照合）、NFR4.7（Busy は `Io`）。暗号学的完全性は非要求（NFR4.4） |
| Repudiation | 該当なし（来歴は封筒の `aid` / `seq_nr` / `occurred_at`、全集約横断は rowid。イベントは自前 `IntentExecutionEventId` と `aggregate_id` を持つ。人間の決定は逐語保存） | — |
| Information Disclosure | ペイロードの人間入力が平文で SQLite に入る | upstream 同等（監査シャードも平文）。ストアは git 管理外・ローカル。ログ出力なし（NFR4.5） |
| Denial of Service | 巨大なジャーナルの replay、`SQLITE_BUSY` の待ち | 差分再生は既定 `every(10)` で最大 9 イベント（基底は seq_nr 1 と 10 の倍数）。Tx は本家がコマンド 1 回に閉じる。Busy は待たずに `Io` で返す（NFR4.7）。性能は非目標（§5） |

## 4. データ分類

| データ | 分類 | 扱い |
|---|---|---|
| `journal` / `snapshot`（ドメインイベントの payload・集約状態の DTO） | Internal（ローカル、git 管理外） | 平文（本家 serde_json）。ワークスペースの他ファイルと同じ権限 |
| 我々の表（`amadeus_projection_checkpoint` / `read_*` / publication 系） | Internal — **U4 の所有** | 本 Unit は触らない（C6 現行化） |
| ペイロード内の人間入力（`request` / `user_input` / `feedback` / `reason`） | upstream の監査行と同等（平文で逐語） | 加工しない。秘密情報を載せる経路は設けない |

## 5. 非目標（NFR5）

- 数値の性能目標は立てない。`bundled` SQLite のビルド時間増は CI キャッシュで吸収。実測で明確な劣化があれば課題化。`SnapshotStrategy` の既定 10 は本家 example と同じで、実際の値は合成ルート（U7）が確定する（実測 `snapshot_strategy.rs:31`）。

## 6. 前版からの変更（2026-09-07 再走）

- §1 範囲を現行の所有（ポート 4 本のうち 1 本 + `RepositoryError<Id>` / `IntentExecutionRepositoryImpl<S>` / DTO 3 型 / `StorePath` / Quint + ITF）へ差し替え。InMemory ダブル・Clock 注入・`from_state`・`updated_at` の記述を撤去。
- NFR1.3（本家スキーマへの非干渉）/ NFR3.4（書込前 ID 照合）/ NFR4.7（単一プロセス前提）を新設。旧 NFR3.4（チェックポイント）/ 旧 NFR3.5（`within_write_transaction`）は所有移転・退役として NFR3.5 に畳んだ。
- NFR2.2 を PBT から契約・実装固有テストの決定性へ、NFR3.2 のエラー分類を `Corrupt { id, seq_nr, source }` + `CorruptDetail` 6 変種へ、NFR3.3 の Tx 所有を本家へ、NFR4.1 の依存差分を「変えない」へ、NFR4.3 の例外（`replay` クラッシュ境界）を明記。
- 旧 pending-revision 3 件を閉じた（1 = TOLERANCE 0.01 達成、2 = `indexing_slicing` / `panic` deny 達成、3 = `audit` advisory を NFR4.1 に注記）。

## Review 履歴（2026-08-23、iteration 1、READY）

> 初版に対する advisory レビュー（全文は `review-history-20260823.md`）。当時のセンサー結果は履歴であり、本再走の承認根拠には使わない。

| # | Severity | 要旨 | 本再走での扱い |
|---|---|---|---|
| 1 | Major | `scripts/coverage.sh` の TOLERANCE 0.05 → 0.01 の引き締めが合格基準に無い | 解消 — 実測 `TOLERANCE=0.01`（B5 で実施済み）。NFR2.3 に達成済みとして記載 |
| 2 | Major | NFR4.3 が主張する `indexing_slicing` / `panic` の deny が workspace lints に無い | 解消 — 実測 `Cargo.toml:43-44` に deny あり。NFR4.3 に達成済みとして記載 |
| 3 | Minor | `cargo audit` の CI ジョブが advisory であることの注記が無い | 反映 — NFR4.1 に advisory（`ci-success` の `needs` 外）を注記 |

## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-09-07T06:10:59Z
**Iteration:** 1

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Major | `aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u3-event-store-repository/nfr-requirements/security-requirements.md` > §2 NFR4.7（並行アクセスは単一プロセス前提） | 「同一プロセス内は `&mut self` の排他で直列化（内部可変性なし）」という合格の根拠が、同じ Unit の公開 API で反証される。`intent_execution_repository_impl.rs:203-215` の `pub fn reopened(&self) -> IntentExecutionRepositoryImpl<S>` は同じストアを指す 2 本目のハンドルを返し（doc コメント「本家のストアはどのバックエンドでも `Clone` が基底状態（SQLite なら接続、memory なら表）を共有する」）、既存テスト `the_volatile_store_is_shared_by_the_reopened_handle`（`:804`、PASS 実測）がその共有を実証している。2 本のハンドルはそれぞれ独立に `&mut` を取れるため、借用検査は同一プロセス内の同時書込を止めない。実際に直列化しているのは本家の楽観 version CAS（`Conflict`）であって `&mut self` ではない。結果として「同一プロセス・複数ハンドル」という経路が、NFR4.7 の当該条項にも「複数プロセスは U7 の裁定事項」という繰延にも掛からず無記載のまま残る | NFR4.7 の根拠を実態に合わせて書き直す。同一プロセス内の直列化保証は `&mut self` ではなく本家の楽観 version CAS であることを明記し、`reopened()` が生む複数ハンドル経路を (a) NFR4.7 の射程に明示的に含める、(b) U7 の並行モデル裁定へ明示的に繰り延べる、のいずれかで扱う | New |
| R-02 | Major | `aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u3-event-store-repository/nfr-requirements/security-requirements.md` > §2 NFR4.3（プロダクト経路で panic しない） | 「例外は 2 つだけ」と網羅を宣言しているが、プロダクト経路の `# Panics` 公開 API を 1 つ取りこぼしている。`intent_execution.rs` のモジュール doc（`:40-41`）自身が「`# Panics` を持つ公開 API はこの 3 か所だけ」と書いており、実測でも `:347`（`replay`）・`:1506`（`apply_event`）に加えて `:2364` の誕生変換 `From<(Started, DateTime<Utc>)> for IntentExecution`（`#[allow(clippy::expect_used, reason = "壊れた歴史は回復不能 …")]`、`:2371-2374`）が 3 つ目として存在する。NFR4.3 の (a) は `replay` / `apply_event` しか挙げていないため、この成果物だけを読んで panic 面を監査すると誕生変換が漏れる | NFR4.3 (a) の列挙に誕生変換 `From<(Started, DateTime<Utc>)>` を加えて 3 か所にするか、「例外は 2 つだけ」という網羅の断定を外して正本（`intent_execution.rs` モジュール doc）を参照する形にする | New |
| R-03 | Minor | `aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u3-event-store-repository/nfr-requirements/security-requirements.md` > §2 NFR3.x および `traceability.json` の NFR3 行 | 上流 `requirements.md:133-135` の NFR3 は合格基準を「改訂版 `audit_lock.qnt` の ITF 準拠 + クラッシュ再構成テスト」と定め、集約名も旧称 `WorkflowExecution` のままである。`audit_lock.qnt` は退役済み（実測 `ls formal/orchestration/` = engine_loop / journal_protocol / stop_hook。本成果物の NFR1.2 自身が退役を記載）であり、本成果物は合格基準を `journal_protocol.qnt` へ黙って読み替えている。読み替え自体は妥当だが、上流の合格基準が失効している事実が traceability の NFR3「OK」の裏に隠れる。project.md の規律「上流成果物の間に矛盾を見つけたら、読み替えて進まず人間へ裁定を求める」に照らすと、少なくとも差分の明記が要る | NFR3.x か §6 に「上流 NFR3 の合格基準 `audit_lock.qnt` は退役済みで `journal_protocol.qnt`（不変条件 8 / witness 4）が後継、集約名は `IntentExecution` へ改名（B12）」と差分を明記する。上流 `requirements.md` の改訂要否はオーナー裁定に載せる | New |
| R-04 | Minor | `aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u3-event-store-repository/nfr-requirements/security-requirements.md` > §1 範囲と信頼境界（永続化 DTO 3 型）／`tech-stack-decisions.md` > §1 永続化表現 | 「永続化 DTO 3 型（`dto/` — …）」＋「`dto/` に 1 型 1 ファイル」と読むと `dto/` は 3 ファイルに見えるが、実測では `dto/` に 30 ファイル（型ファイル約 28）ある。うち `started_dto.rs` / `jumped_dto.rs` / `gate_approved_dto.rs` ほかイベント変種の DTO 群と `dto_decode_error.rs`（`CorruptDetail::Undecodable` が運ぶ `DtoDecodeError` の正本）は、payload のバイト形（BR2.5・NFR1.1 の観測面）を実際に決めている U3 所有物である。名指しの 3 型はストアの型引数であって、所有面の全体ではない | §1 の表現を「本家ストアの型引数となる 3 型」と限定したうえで、イベント変種 DTO 群と `DtoDecodeError` が同じく U3 所有であることを 1 行加える | New |
| R-05 | Minor | `aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u3-event-store-repository/nfr-requirements/security-requirements.md` > §2 NFR4.4（改竄の検出） | 見出しは「改竄の検出（完全性）」だが、合格基準は NFR3.2 の検査（DTO 復号 + 集約不変条件）だけである。実測テスト `a_tampered_snapshot_state_is_corrupt` が示すのも不変条件を破る改竄に限られ、不変条件を満たす形に書き換えた改竄（別の妥当なステージ・別の妥当な判定への差し替え）は検出できず、そのまま真実として replay される。暗号学的完全性を非要求とする裁定自体は妥当だが、検出できる改竄の範囲が見出しより狭い | NFR4.4 に「検出できるのは不変条件・通番・manifest を破る改竄に限る。不変条件を満たす改竄は検出せず replay する（ローカル単一ユーザ・git 管理外という前提で受容）」と検出範囲を明記する | New |
| R-06 | Minor | `aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u3-event-store-repository/nfr-requirements/security-requirements.md` > §2 NFR2.2／`tech-stack-decisions.md` > §1 テスト | 「`proptest` は `core-command-domain` の値オブジェクトのみ — 実測」とあるが、実測では `proptest` は 3 クレート（`core-command-domain` / `core-infrastructure` / `core-query/use-case`）の dev-dependency である。合格基準が依存する主張（アダプタに PBT が無い）は実測どおり成立しているが、カバレッジ相対ゲートの安定（差 0.00pp）は workspace 全体の PBT に掛かるため、範囲の書き方が実態より狭い | 「アダプタに PBT は無い（`proptest` を持つのは `core-command-domain` / `core-infrastructure` / `core-query/use-case` の dev-dependency。シードは `scripts/coverage.sh` で固定）」へ書き換える | New |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| `aidlc-sensor-required-sections`（security-requirements.md） | PASS（h2_count 7、findings 0） | 見出し構成に欠落なし |
| `aidlc-sensor-required-sections`（tech-stack-decisions.md） | PASS（h2_count 4、findings 0） | 同上 |
| `aidlc-sensor-traceability`（traceability.json） | PASS（gaps / orphans / invalid_targets いずれも空） | NFR1〜NFR5 と本文 ID（NFR1.1〜NFR4.7）の対応に欠落なし |
| `aidlc-sensor-upstream-coverage`（consumes: functional-spec, rules, requirements, contract-summary） | PASS（unreferenced 空） | 上流 4 成果物すべてを参照済み |
| `cargo test --locked -p core-command-interface-adapter --test intent_execution_repository_contract --test intent_execution_repository_impl_test` | PASS（契約 22 + 実装固有 23 = 45） | NFR2.1 / NFR2.2 の件数主張と一致 |
| `cargo test --locked -p aidlc --test crash_reconstruction_test` | PASS（5 件） | NFR3.3 のクラッシュ再構成 5 件と一致 |
| linter / type-check センサー | 対象外 | TS/JS 出力を持たないステージのため非適用 |

### Summary

NFR 要求 20 行の合格基準と件数・行番号・grep の主張は、現行ワーキングツリーでほぼ全数一致した（契約 22 + 実装固有 23 + クラッシュ 5、ITF fixture 8、Quint 不変条件 8 / witness 4、`cargo lint` 7 ルール、CI 7 ジョブと `ci-success` の `needs` から `audit` 除外、`CorruptDetail` 6 変種、`RepositoryError` 4 変種で `PartialEq` 無し、PRAGMA / `busy_timeout` / `std::env` / `println!` / `eprintln!` / `tracing::` / `proptest` がアダプタに 0 件、`.gitignore:32` と `deviations.md` #4 の v3.0.0 追記、`TOLERANCE=0.01` と `PROPTEST_RNG_SEED` 固定、`unsafe_code = "forbid"` の workspace 昇格）。残る主要な懸念は 2 点で、いずれも記述の正確性であり設計の破綻ではない: 公開 API `reopened()` が同一ストアを共有するため NFR4.7 の「`&mut self` の排他で直列化」という根拠が成立しないこと（R-01）と、NFR4.3 の panic 例外の網羅宣言が誕生変換を取りこぼしていること（R-02）。
