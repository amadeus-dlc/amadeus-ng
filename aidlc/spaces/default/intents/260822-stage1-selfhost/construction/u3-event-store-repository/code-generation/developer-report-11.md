# developer-report-11 — 委任 9: U3 再走（Quint 凡例の追従 + 受入の再実測 + 記録の現行化）

> Unit: u3-event-store-repository（kind: library）。ブリーフ `developer-brief-9.md`、計画 `code-generation-plan.md`（2026-09-07 再走）、
> テスト手順 `unit-test-instructions.md`。Testing Contract 指紋 `sha256:303d9bb7b5d777d54a6761be9ed154d85d5bb3f2d6b9cce02f71f4ed1b3a4ff3`
> はブリーフ先頭のマーカーおよび計画末尾の JSON と一致することを確認したうえで着手した。
> 作業ディレクトリ: git worktree `/Users/j5ik2o/orca/workspaces/amadeus-ng/stage1-selfhost`（ブランチ `stage1-selfhost`、HEAD `7ad3d7b9`）。
> このディレクトリの外へ `cd` していない。`git add` / `git commit` / `git push` / `git stash` / `gh` を実行していない。
> `bun .claude/tools/aidlc-*.ts` は計画 Step 6 の traceability センサー 1 本のみ実行した。`AIDLC_*` 環境変数を設定していない。

## 0. 環境と所要時間

| 項目 | 値 |
|---|---|
| `rustc -V` | `rustc 1.95.0 (59807616e 2026-04-14)`（`rust-toolchain.toml` の固定と一致） |
| `cargo llvm-cov --version` | `cargo-llvm-cov 0.8.5` |
| `cargo audit --version` | `cargo-audit-audit 0.22.2` |
| `quint --version` | `0.32.0` |
| 開始 | 2026-09-07 16:18:14 JST |
| 終了 | 2026-09-07 16:41 JST |
| 所要 | 約 23 分 |

着手時の前提確認: `git diff --stat origin/main HEAD -- modules tests formal scripts Cargo.toml Cargo.lock .github tools` は**空**で、
ワークスペースのコードは `origin/main` = `f2b6b6a9` と同一だった（計画 §1 の主張と一致）。

## 1. Unit 限定コマンド 11 本（計画 Step 1）

`unit-test-instructions.md` §2 の 11 本をワークスペースルートで順に実行した。すべて `failed` 0 / `ignored` 0。
終了コードは 11 本を `&&` で連結した 1 回の実行が `ALL_ELEVEN_EXIT_0` を出力したことで確認した（1 本でも非 0 なら連結は途中で止まる）。

| # | コマンド（`cargo test --locked` に続く部分） | passed | failed | ignored | filtered out | 完了時刻 (JST) |
|---|---|---|---|---|---|---|
| 1 | `-p core-command-interface-adapter --test intent_execution_repository_contract` | 22 | 0 | 0 | 0 | 16:18:36 |
| 2 | `-p core-command-interface-adapter --test intent_execution_repository_impl_test` | 23 | 0 | 0 | 0 | 16:18:42 |
| 3 | `-p core-command-interface-adapter --test upstream_event_store_conformance` | 10 | 0 | 0 | 0 | 16:18:46 |
| 4 | `-p core-command-interface-adapter --lib orchestration::intent_execution_repository_impl` | 10 | 0 | 0 | 91 | 16:18:51 |
| 5 | `-p core-command-interface-adapter --lib orchestration::store_failure` | 4 | 0 | 0 | 97 | 16:18:54 |
| 6 | `-p core-command-interface-adapter --lib orchestration::snapshot_strategy` | 2 | 0 | 0 | 99 | 16:18:57 |
| 7 | `-p core-command-interface-adapter --lib orchestration::dto` | **47** | 0 | 0 | 54 | 16:19:01 |
| 8 | `-p core-command-domain --lib workspace::store_path` | 4 | 0 | 0 | 695 | 16:19:09 |
| 9 | `-p core-command-domain --lib workspace::intent_dir_name` | 9 | 0 | 0 | 690 | 16:19:13 |
| 10 | `-p aidlc --test crash_reconstruction_test` | 5 | 0 | 0 | 0 | 16:19:16 |
| 11 | `-p aidlc --test journal_protocol_conformance` | 5 | 0 | 0 | 0 | 16:19:23 |

**期待件数との差は #7 の 1 件だけ**（計画準備時の期待 29 に対し実測 47）。原因はテスト追加でも削除でもなく、
`unit-test-instructions.md` §2 の属性集計が `dto/tests.rs` の 1 ファイルしか数えていないことである（§2 の M-4 参照）。
コードは `origin/main` と同一で `git log` 上の追加はない。ほかの 10 本は期待どおり。

### 1.1 Step 3（凡例追従）後の再実行

| コマンド | passed | failed | ignored | 完了時刻 (JST) |
|---|---|---|---|---|
| `-p aidlc --test journal_protocol_conformance` | 5 | 0 | 0 | 16:25:16 |
| `-p aidlc --test crash_reconstruction_test` | 5 | 0 | 0 | 16:25:19 |

凡例（コメント）の書き換えで ITF 適合・クラッシュ再構成に影響が出ていないことを確認した。

## 2. 設計との照合（計画 Step 2）

行番号はすべて `grep -n` / `sed -n` の絶対行で確かめた。**不一致 6 件**、いずれも設計文書側の引用精度の問題であり、
**現行コードの振る舞いと設計の主張が食い違う箇所は 0 件**である。

### 2.1 不一致

| ID | 対象 | 設計の主張 | 実測 | Red テスト案 |
|---|---|---|---|---|
| M-1 | `nfr-design/security-design.md` §2 行 (2) | `IntentExecution::new` は `intent_execution.rs:290-345` | `pub fn new(` が `:290`、`Ok(execution)` が `:336`、閉じ括弧が `:337`。`:338-345` は次の `replay` の doc コメント | **構成不能**。行番号の引用先がずれているだけで、`new` の検査（cursor / parked_at 範囲・`seq_nr >= 1`・`check_invariants`）は設計どおり実在し動作する。落とせるテストが無い。是正は文書側で `:290-337` へ |
| M-2 | 同 §2 行 (2) | `IntentExecutionDto::to_domain` は `dto/intent_execution_dto.rs:208-293` | `pub fn to_domain(` が `:208`、最終文 `.map_err(...)` が `:293`、関数の閉じ括弧が `:294` | **構成不能**（同上）。是正は `:208-294` へ |
| M-3 | 同 §3 | `stored_version` は `:313-321` | `async fn stored_version(` が `:313`、閉じ括弧が `:320`。`:321` は `impl` ブロックの閉じ括弧 | **構成不能**（同上）。是正は `:313-320` へ |
| M-4 | `unit-test-instructions.md` §2 | `dto` の `#[test]` + `#[tokio::test]` 属性合計は 29 | `--lib orchestration::dto` フィルタの実行は 47。属性の実測も 47（`grep -rn '#\[test\]\|#\[tokio::test\]' .../dto/ \| wc -l`）。内訳: `tests.rs` 29 / `dto_vocabulary.rs` 6 / `workflow_definition_dto.rs` 6 / `dto_decode_error.rs` 2 / `intent_execution_aggregate_key_dto.rs` 2 / `intent_aggregate_key_dto.rs` 1 / `workflow_definition_aggregate_key_dto.rs` 1 | **構成不能**（件数の集計漏れであり、production コードの振る舞いではない）。是正はテスト手順の期待値を 47 へ。なお `security-design.md` §2 の「`dto/tests.rs` 29 件」は**正しい** |
| M-5 | 計画 §3 Step 2 (d) | ファサードは `orchestration/mod.rs:38-66` | 起点 5 か所（`:38-41` / `:46` / `:52` / `:62` / `:65-66`）は**完全一致**。ただし `:66` に始まる最後の `pub use workflow_definition_repository_impl::{ ... };` は `:68` で閉じるので、ブロックの終端は `:68` | **構成不能**。是正は範囲を `:38-68` へ |
| M-6 | `developer-brief-9.md` 冒頭の説明 | 「旧 `WorkflowExecutionRepository` の名前を現行公開 API に残さない」は `functional-spec.md` **§5** の `:137` | 行番号 `:137` は正しいが、その行が属する節は **§6 退役した設計**（`## 5.` は `:121`、`## 6.` は `:133`） | **構成不能**。是正はブリーフの節番号のみ |

計画の指示（「照合で不一致が新たに見つかったら、コードを直さず Red テスト案を報告に書いて返す」）に従い、**コードは 1 行も直していない**。
上の 6 件はいずれも「現行コードで落ちるテスト」を構成できない種類の不一致である。理由は共通で、**doc コメント・行番号の引用・件数の集計**は
production の振る舞いを持たないためである。人工的に Red を作るためにコードを壊すことはしていない（テスト手順 §5）。

### 2.2 一致（(a)〜(g) の逐条）

**(a) `security-design.md` §2 検査点表**

| 層 | 設計の主張 | 実測 | 判定 |
|---|---|---|---|
| (0) 書込前 | `store` 冒頭 `:447-452` | `if event.aggregate_id() != aggregate.id() {` が `:447`、`});` が `:452`（`}` は `:453`） | 一致 |
| (0') 本家書込契約 | `write_error` `:277-303` | `async fn write_error(` が `:277`、閉じ括弧が `:303` | 完全一致 |
| (1) ストア復号 | `read_error` `:245-265` | `fn read_error(` が `:245`、閉じ括弧が `:265` | 完全一致 |
| (2) DTO → ドメイン | `to_domain` / `IntentExecution::new` | 関数は実在し検査内容も設計どおり。行番号のみ M-1 / M-2 | 一部不一致（行番号） |
| (3) 差分行の整合 | `find_by_id` `:331-439`、差分ループ `:378-438` | `async fn find_by_id(` `:331`、閉じ括弧 `:439`、`let delta = self` `:378`、`Ok(IntentExecution::replay(base, events).with_version(version))` `:438` | 完全一致 |
| (4) クラッシュ境界 | `replay:352` / `apply_event:1514` / 誕生変換 `:2373`、`# Panics` は 3 か所（正本 `:40-41`） | `pub fn replay(` `:352`、`pub fn apply_event(` `:1514`、`fn from((started, occurred_at): ...)` `:2373`（`impl From<...>` は `:2343`）。`# Panics` は `:347` / `:1506` / `:2364` の 3 か所のみ。正本の記述は `:40-41` | 完全一致 |
| `CorruptDetail` | `:101-113`、6 変種 | `enum CorruptDetail {` `:101`、最後の変種 `WriteContract,` `:113`。変種は `MissingSnapshot` / `ForeignManifest` / `SequenceGap` / `Undecodable(DtoDecodeError)` / `StoreDeserialization` / `WriteContract` の 6 つ | 完全一致 |
| panic の射程 | `Cargo.toml:39-44` で `unwrap_used` / `expect_used` / `indexing_slicing` / `panic` を deny | `:39` `unwrap_used` / `:40` `expect_used` / `:43` `indexing_slicing` / `:44` `panic`（`:41-42` は裁定コメント） | 完全一致 |

設計が「固定するテスト」欄に挙げた名前はすべて実在した。契約側 2 本（`an_event_from_another_execution_is_rejected_before_writing` /
`a_genesis_with_a_non_zero_version_is_a_contract_violation`）、実装固有 9 本（`a_tampered_snapshot_payload_is_corrupt` /
`a_journal_row_with_an_unknown_event_type_is_corrupt` / `a_tampered_snapshot_state_is_corrupt` /
`a_snapshot_that_breaks_the_aggregate_invariants_is_corrupt` / `a_replayed_row_whose_spelling_is_outside_the_closed_set_is_refused` /
`a_journal_without_a_snapshot_is_corrupt_not_missing` / `a_foreign_manifest_before_the_snapshot_base_is_not_read` /
`a_snapshot_alone_is_a_sufficient_rehydration_base` / `a_replayed_event_naming_a_stage_outside_the_plan_crashes_reconstruction`）。
`intent_execution_repository_impl_test.rs` のテスト関数は `:125` から `:700` まで **23 本**で、`--test` 実行の 23 件と一致する。

**(b) `security-design.md` §3 の分岐・再読取・再オープン**

| 主張 | 実測 | 判定 |
|---|---|---|
| 保存経路の分岐は `:465` | `let outcome = if aggregate.seq_nr() == 1 \|\| self.strategy.wants_snapshot(aggregate.seq_nr())` が `:465` | 完全一致 |
| `stored_version` は競合時だけの再読取 | `:313`、`get_latest_snapshot_by_id` を `.ok().flatten().map_or(0, ...)` で読み、`write_error` の `OptimisticLockError` 腕からのみ呼ばれる | 一致（範囲の終端のみ M-3） |
| `reopened()` は `:209-215` | `pub fn reopened(&self)` が `:209`、閉じ括弧が `:215`。`store.clone()` / `location.clone()` / `strategy` を複製 | 完全一致 |
| 合成ルートは 1 コマンド 1 ハンドル（`open` 8 か所） | `runtime.rs` の `IntentExecutionRepositoryImpl::open` は `:306` / `:402` / `:461` / `:817` / `:1056` / `:1272` / `:1381` / `:1590` の **8 か所**。囲む関数は `report` / `single_report` / `skeleton_stance_report` / `log_review` / `practices_promote` / `set_autonomy` / `park` / `mint_intent` で、いずれもコマンド単位 | 完全一致 |
| `interface-adapter/src/` に `PRAGMA` / `busy_timeout` / `CREATE TABLE` が無い | grep **0 件** | 一致 |

**(c) `security-design.md` §4 の障害写像表**: `const fn io_kind(` が `store_failure.rs:20`、閉じ括弧が `:34`。
`DatabaseBusy` / `DatabaseLocked` → `WouldBlock`、`CannotOpen` / `NotFound` → `NotFound`、`PermissionDenied` / `ReadOnly` /
`AuthorizationForStatementDenied` → `PermissionDenied`、`DatabaseCorrupt` / `NotADatabase` → `InvalidData`、
`OperationInterrupted` → `Interrupted`、その他 → `Other`。**完全一致**。

**(d) `logical-components.md` §1 のコンポーネントと依存**

| 主張 | 実測 | 判定 |
|---|---|---|
| ファサード `mod.rs` の `pub use` 起点 5 か所 | `:38-41`（4 本）/ `:46` / `:52` / `:62` / `:65` / `:66` — すべて一致 | 一致（範囲終端のみ M-5） |
| `dto/` 32 エントリ | `ls` で 32 | 完全一致 |
| イベント変種 DTO 16 | `IntentExecutionEventDto` の腕は `:46`〜`:76` の 16（`Started` / `GateOpened` / `GateApproved` / `GateRejected` / `StageRevised` / `StageSkipped` / `Jumped` / `Parked` / `Unparked` / `Recomposed` / `AutonomyModeSet` / `SingleStageRunCommitted` / `SkeletonStanceRecorded` / `ReviewRequested` / `ReviewCompleted` / `PracticesAffirmed`） | 完全一致 |
| domain に serde / ESA なし、`serde_json` は dev のみ | `core-command-domain/Cargo.toml`: `[dependencies]` = `chrono` / `uuid`(v7) / `core-infrastructure`、`[dev-dependencies]` = `proptest` / `serde_json`。serde derive・event-store-adapter-rs は不在 | 完全一致 |
| use-case に ESA / adapter なし | `core-command-use-case/Cargo.toml`: deps = `core-command-domain` / `chrono`、dev = `tokio` のみ | 完全一致 |
| `thiserror` は推移のみ | `cargo tree -p core-command-interface-adapter -i thiserror` → `thiserror v2.0.20` ← `event-store-adapter-rs v3.0.0` の 1 経路のみ | 完全一致 |
| 本家ピン `=3.0.0` | `Cargo.toml:119` `event-store-adapter-rs = "=3.0.0"` | 一致 |
| `SnapshotStrategy` は合成ルートに配線されていない | `grep SnapshotStrategy modules/app/aidlc/src/` = 0 件 | 一致 |

**(e) `functional-spec.md` §3.1 / §3.2 の手順**: 逐条で一致した。

- §3.1-2 書込前 ID 照合 → `:447-452`。
- §3.1-3 封筒は集約から ID / 通番 / 発生時刻、payload はイベント DTO、manifest は `intent-execution-event/1`（`EVENT_MANIFEST` は `:61`、照合は `:406`）、期待版は `aggregate.version()`（`:455`）。
- §3.1-4 `seq_nr == 1` は必ず `persist_event_and_snapshot`、以後は `wants_snapshot` の真偽で分岐（`:465-471`）。
- §3.2-2 基底が無いとき通番 1 から journal を読み、空なら `NotFound`、あれば `Corrupt(MissingSnapshot)`（`:343-363`）。
- §3.2-5 `IntentExecution::replay(base, events).with_version(...)`。設計は `with_version(snapshot.version())` と書き、コードは `let version = snapshot.version();`（`:364`）を `:438` で `.with_version(version)` として渡す。**同値**であり不一致としない。

**(f) `rules.md` BR1.1〜BR5.2**: `id: BR` の行は **23 本**（設計の主張どおり）。logic 欄はすべて現行コード・テストと整合した。機械的に確かめた代表:

- BR1.2（基底なし・journal なし → `NotFound` / journal あり → `Corrupt`）: `:343-363`。
- BR1.3（genesis は `seq_nr=1` かつ版 0、書込前 ID 照合）: `:447-452` / `:465`。
- BR2.1（独自 `PRAGMA user_version` を版契約にしない）: `PRAGMA` grep 0 件。
- BR3.3（不変条件 8・witness 4 の保持）: `scripts/quint-gate.sh:91-94`（8 不変条件を 1 回の `quint run`）と `:133`（witness 4 本のループ）。
- BR4.1（小文字 36 字・version 7・variant `10xx`）: `intent_execution_id.rs:40-41` の `get_version_num() != 7 || get_variant() != uuid::Variant::RFC4122`、テスト `:72` `parse_accepts_a_lowercase_uuidv7` / `:156` `the_variant_nibble_must_encode_the_rfc_variant`。
- BR4.2（`260822-a--b` / `260822-a-` / `260822-` を拒否）: `intent_dir_name.rs:168` / `:173` / `:131`、受理例は `:91-92`。

**(g) 旧名 grep（Step 3 の前）**: `WorkflowExecution` を `modules tests scripts .github Cargo.toml tools formal` で grep した結果は
**`formal/orchestration/journal_protocol.qnt` の 5 行のみ**（`:10` / `:11` / `:15` / `:22` / `:23`）で、他は 0 件。計画準備時の実測と一致。

### 2.3 照合中に気づいたワークスペース側の事実（設計文書の主張ではない）

- **ワークスペース lints の本数**: ブリーフ前文と `team.md` は「47 本 deny」と書くが、現行 `Cargo.toml` は **50 ルール**である
  （`[workspace.lints.clippy]` 44 deny + `[workspace.lints.rust]` 4 deny + `unsafe_code = "forbid"` 1 + `[workspace.lints.rustdoc]` 1 deny）。
  `= "deny"` の総数は 49、`= "forbid"` は `:29` の `unsafe_code` 1 本。U3 の NFR2.4 は総数を主張せず `Cargo.toml:39-69` の範囲と例示のみなので
  設計不一致ではないが、前提の数値は現行と合っていない。memory は本 Bolt の所有外なので変更していない。
- **`cargo lint` の実装本数**: NFR2.4 が言う 7 本はすべて実在した（`port-naming` / `command-side-io` / `no-public-fields` /
  `one-public-type` / `dao-single-table` / `checkbox-vocabulary` は `tools/lint/src/check.rs:27-76`、`use-case-domain-getter` は
  `tools/lint/src/domain_getter/usage.rs:12`）。一方 `coding-rules/README.md` の機械化ロードマップ節は「実装済みは 6 本」（2026-09-04 更新）
  のままで、同 README の一覧表（`use-case-domain-getter` を含む）と食い違う。coding-rules は本 Unit の所有外なので直していない。
- **カバレッジの現況**: `team.md` の 94.87〜95.29% に対し実測 99.15%。B5 当時からの上昇であり、床 90% には十分な余裕がある。

## 3. 凡例の追従（計画 Step 3）

`formal/orchestration/journal_protocol.qnt` の凡例コメント 5 行のみを書き換えた。状態機械本体（`var` / `action` / `val` / `run` / witness）は
触っていない。`git diff --stat` は `formal/orchestration/journal_protocol.qnt | 10 +++++-----` / `1 file changed, 5 insertions(+), 5 deletions(-)`
で、**1 ファイル・5 行**である。

```diff
diff --git a/formal/orchestration/journal_protocol.qnt b/formal/orchestration/journal_protocol.qnt
index c7a1f975..45364ba7 100644
--- a/formal/orchestration/journal_protocol.qnt
+++ b/formal/orchestration/journal_protocol.qnt
@@ -7,20 +7,20 @@
 //
 // モデル型 ↔ Rust Domain Primitive 対応表 (ADR 0003 決定 6):
 //   journalLen    ↔ journal 表の行数 (単一集約なので global 通番 GlobalSeqNr の最大値と一致)
-//   snapVersion   ↔ snapshot 表の version 列 = WorkflowExecution::version() (楽観 version)
-//   snapSeq       ↔ snapshot 表の seq_nr 列 = WorkflowExecution::seq_nr() (適用済みイベント数)
+//   snapVersion   ↔ snapshot 表の version 列 = IntentExecution::version() (楽観 version)
+//   snapSeq       ↔ snapshot 表の seq_nr 列 = IntentExecution::seq_nr() (適用済みイベント数)
 //   checkpoint    ↔ checkpoint 表の global 通番 = JournalReader::checkpoint(ProjectionName)
 //   readModelSeq  ↔ 投影 (U4) が描き終えた最後の global 通番。本モデルは値だけを持つ
 //   loadedVersion ↔ writer ごとの「再水和した集約が載せている版」
-//                   = WorkflowExecutionRepository::find_by_id が with_version で載せた値
+//                   = IntentExecutionRepository::find_by_id が with_version で載せた値
 //   lastAction    ↔ ITF リプレイ用アクション記録 (ADR 0003 決定 5 の lastAction 規約)
 //   lastActor     ↔ 同上。writer を一意に選ぶための添字 (lastAction × lastActor で駆動する)
 //   prev*         ↔ (モデル専用) 状態遷移レベル不変条件のための前状態スナップショット。
 //                   Rust 対応物なし
 //
 // アクション ↔ Rust ポート操作:
-//   load(w)           ↔ WorkflowExecutionRepository::find_by_id (未書込なら genesis を持つ)
-//   store_ok(w)       ↔ WorkflowExecutionRepository::store が Ok (Tx 内で journal + snapshot)
+//   load(w)           ↔ IntentExecutionRepository::find_by_id (未書込なら genesis を持つ)
+//   store_ok(w)       ↔ IntentExecutionRepository::store が Ok (Tx 内で journal + snapshot)
 //   store_conflict(w) ↔ 同 store が Err(RepositoryError::Conflict) — 状態を変えない
 //   catchup           ↔ JournalReader::events_after + advance_checkpoint
 //   crash             ↔ Tx 済み・投影未反映のままプロセスが落ちる (状態不変のマーカー)
```

計画どおり `:15` の「`with_version` で載せた値」という表現は現行 `intent_execution.rs:254` と一致するため保持した。
U4 側の名前（`JournalReader::checkpoint(ProjectionName)`、`events_after` / `advance_checkpoint`）も触っていない。

旧名 grep の前後:

| 時点 | 結果 |
|---|---|
| 追従**前** | `formal/orchestration/journal_protocol.qnt` の 5 件のみ（`:10` / `:11` / `:15` / `:22` / `:23`）。`modules` / `tests` / `scripts` / `.github` / `Cargo.toml` / `tools` は 0 件 |
| 追従**後** | 同じ 7 範囲（`formal` を含む）で **0 件** |

コメント行にはテストが無いため TDD の Red は作っていない（計画 §4 の明示どおり）。検証は Step 4 (b) の quint-gate（typecheck を含む）と、
§1.1 の ITF 適合 / クラッシュ再構成の再実行で行った。

## 4. 受入（計画 Step 4）

### (a) 静的ゲートと `tools/lint`（NFR2.4）

| コマンド | 終了コード | 出力の要点 |
|---|---|---|
| `cargo fmt --all --check` | 0 | 差分なし（`FMT_EXIT_0`） |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 | `Finished dev profile ...` のみ。警告 0 |
| `cargo lint` | 0 | 標準出力なし = 所見 0 件 |
| `cargo test --manifest-path tools/lint/Cargo.toml` | 0 | `test result: ok. 93 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out`（`test result:` 行は 1 本のみ） |

### (b) Quint ゲート（NFR2.5、凡例追従の**後**に実行）

`bash scripts/quint-gate.sh` → 最終行 **`[PASS] quint gate: all steps green`**。summary は 25 ステップすべて `[PASS]`。

| 区分 | 件数 | 内訳 |
|---|---|---|
| typecheck | 3 | `engine_loop.qnt` / `stop_hook.qnt` / **`journal_protocol.qnt`** |
| invariants run | 3 | `engine_loop` / `stop_hook` / **`journal_protocol`**（後者は `conflict_rejected` / `snapshot_tracks_journal` / `version_equals_journal` / `checkpoint_monotone` / `checkpoint_bounded` / `projection_idempotent` / `truth_is_journal` / `no_lost_update` の **8 不変条件**を 1 回の `quint run` に載せる — `scripts/quint-gate.sh:91-94`） |
| witness（反転判定） | 18 | `engine_loop` 9 + `stop_hook` 5 + **`journal_protocol` 4**（`w_conflict` / `w_crash_then_catchup` / `w_interleaved_writers` / `w_idempotent_catchup`） |
| 決定的シナリオ | 1 | `quint test --match 'r_.*'`（`stop_hook.qnt`） |

コメント変更で状態機械が壊れていないことの機械確認として、typecheck が緑であることが直接の証拠である。

### (c) カバレッジ（NFR2.3）

同一リビジョン（凡例追従後）・同一ツールチェーン（1.95.0）・同一シード（`PROPTEST_RNG_SEED=20260823`、`scripts/coverage.sh:43` で固定）で 2 回実行した。

| 回 | 生の head 値 | 絶対ゲート |
|---|---|---|
| 1 回目 | `99.15169660678644%` | `[PASS] absolute gate: head (99.15169660678644%) >= threshold (90.0%)` |
| 2 回目 | `99.15169660678644%` | `[PASS] absolute gate: head (99.15169660678644%) >= threshold (90.0%)` |
| 差 | **0.00000000000000 ポイント**（生の値が完全一致） | 受入目標（差 0.00pp）を満たす |

`TOLERANCE`（0.01）・除外設定（`IGNORE_FILENAME_REGEX`）・シードは一切変更していない。相対ゲート（`--base`）は
計画に無いため実行していない（CI の `coverage` ジョブが PR で実行する）。

### (d) 依存の脆弱性走査（NFR4.1）

| コマンド | 終了コード | advisory DB | 走査対象 |
|---|---|---|---|
| `cargo audit` | 0 | `https://github.com/RustSec/advisory-db.git` から取得**成功**、1239 advisories 読込 | `Cargo.lock` の **125 crate dependencies**。脆弱性の報告行なし |
| `cargo audit --file tools/lint/Cargo.lock` | 0 | 同上（1239 advisories） | `tools/lint/Cargo.lock` の **5 crate dependencies**。脆弱性の報告行なし |

DB の取得はネットワーク経由で成功しており、「未導入」「取得失敗を成功と書く」といった扱いはしていない。

### (e) 退役の維持（NFR1.2 / BR3.1）

`WorkspaceLock|FsWorkspaceLock|LockProtocol|LockIdentity|ProcessProbe|audit_lock|within_write_transaction|reap_eligible|OwnerStamp|AcquireBudget|LockGuard|process_alive|reap-decision-locality`
を `modules tools scripts formal .github Cargo.toml` で `grep -rnE` した結果は **0 件**（`wc -l` = 0）。

`ls formal/orchestration/` = `engine_loop.qnt` / `journal_protocol.qnt` / `stop_hook.qnt` の **3 つだけ**（退役した `audit_lock.qnt` は存在しない）。

### (f) 全体ゲート（Unit 限定コマンドではない）

`PROPTEST_RNG_SEED=20260823 cargo test --workspace`:

| 指標 | 値 |
|---|---|
| passed | **2354** |
| failed | 0 |
| ignored | 0 |
| `test result:` 行 | 54 本 |
| 終了コード | 0（`WORKSPACE_TEST_EXIT_0`） |
| 実時間 | 26.42 秒（`/usr/bin/time -p`、ビルド温存時。user 15.02 / sys 48.60） |

## 5. 記録の現行化（計画 Step 5・Step 6）

### 5.1 `code-summary.md`（書換）

現行の事実で全面的に書き直した。構成は計画 Step 5 のとおり §1 結果（環境・11 コマンド・受入）、§2 変更ファイル（凡例 5 行の逐語 diff）と
現行実装ファイルの一覧（来歴 1 行ずつ）、§3 設計との照合表（不一致 6 件を含む）、§4 テスト配置の件数、§5 依存、§6 未検証範囲、
§7 申し送り、§8 B5 版からの変更。B5 の TDD 証跡・当時の件数（674 / 98.42%）・裁定表・コミット列は「歴史であり
`code-summary-history-2026-08-23.md` に保存済み」と明記し、現在の実施としては再掲していない。

### 5.2 `traceability.json`（書換）

**46 ID**（`FR1.2` / `FR1.3` / `NFR3` の 3 + `BR1.1`〜`BR5.2` の 23 + `NFR1.1`〜`NFR4.7` の 20）を、それぞれ**実在するワークスペース相対パス
1 本**へ対応付けた。旧版は 44 ID で `NFR1.3` と `NFR4.7` が欠けており、target も旧クレート構成（`modules/core/interface-adapter/...`、
`event_store_impl.rs`、`wire/`、`workflow_execution_state.rs` など既に存在しないパス）を指していたため全面差し替えである。

traceability センサーの出力（`bun .claude/tools/aidlc-sensor-traceability.ts --stage code-generation --output-path <本ファイル>`）:

```json
{"pass":false,"gaps":[],"orphans":[],"missing_from_table":[],"missing_from_upstream_ids":["FR1","FR1.1","FR2",...,"NFR5"],"invalid_entries":[],"invalid_targets":[],"findings_count":40}
```

- **`invalid_targets` は空**（46 件すべて実在ファイルを指す）。
- `invalid_entries` / `gaps` / `orphans` / `missing_from_table` も空。
- `pass:false` の原因は `missing_from_upstream_ids` の 40 件のみで、その中身は `FR1`〜`FR9.6` / `NFR1` / `NFR2` / `NFR4` / `NFR5` という
  **他 Unit が担う上流 ID** である（計画 Step 6 が「既知のノイズ」と明記したもの）。U3 の ID は 1 件も欠けていない。

### 5.3 `source-manifest.json`（新規）

```json
{
  "stage": "code-generation",
  "unit": "u3-event-store-repository",
  "version": 1,
  "writes": [
    "formal/orchestration/journal_protocol.qnt"
  ]
}
```

実際に変更したアプリケーション側パスはこの 1 件だけである。

## 6. `git status --short`（計画 Step 7）

```
 M aidlc/spaces/default/intents/260822-stage1-selfhost/aidlc-state.md
 M aidlc/spaces/default/intents/260822-stage1-selfhost/audit/j5ik2o-mac-studio-lan-c4a9057ffc1c.md
 M aidlc/spaces/default/intents/260822-stage1-selfhost/construction/code-generation/memory.md
 M aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u3-event-store-repository/code-generation/code-generation-plan.md
 M aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u3-event-store-repository/code-generation/code-generation-questions.md
 M aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u3-event-store-repository/code-generation/code-summary.md
 M aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u3-event-store-repository/code-generation/traceability.json
 M aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u3-event-store-repository/code-generation/unit-test-instructions.md
 M formal/orchestration/journal_protocol.qnt
?? aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u3-event-store-repository/code-generation/code-generation-plan-history-2026-08-23.md
?? aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u3-event-store-repository/code-generation/code-generation-questions-history-2026-08-23.md
?? aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u3-event-store-repository/code-generation/code-summary-history-2026-08-23.md
?? aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u3-event-store-repository/code-generation/developer-brief-9.md
?? aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u3-event-store-repository/code-generation/developer-report-11.md
?? aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u3-event-store-repository/code-generation/source-manifest.json
?? aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u3-event-store-repository/code-generation/traceability-history-2026-08-23.json
?? aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u3-event-store-repository/code-generation/unit-test-instructions-history-2026-08-23.md
```

- **ワークスペース側の差分は `formal/orchestration/journal_protocol.qnt` の 1 ファイルだけ**である。
- 本委任が変更した記録は `.../u3-event-store-repository/code-generation/` 配下の 4 件（`code-summary.md` 書換、`traceability.json` 書換、
  `source-manifest.json` 新規、`developer-report-11.md` 新規）に限られる。
- それ以外の差分（`aidlc-state.md`、監査シャード、ステージ日誌 `construction/code-generation/memory.md`、`code-generation-plan.md` /
  `code-generation-questions.md` / `unit-test-instructions.md` の変更、`*-history-2026-08-23.*` と `developer-brief-9.md` の新規）は
  **親セッションが本委任の前に作ったもの**で、本委任は 1 文字も触っていない。ステージ日誌の追記 3 行はタイムスタンプが
  `2026-09-07T06:53:37Z` / `07:14:34Z`（= JST 15:53 / 16:14）で、本委任の開始時刻 16:18:14 JST より前である。

## 7. 未検証範囲と申し送り

### 7.1 未検証範囲（`code-summary.md` §6 と同じ）

- **CI の全ジョブ実行**: ローカルで確認したのは `check` 相当・`quint` 相当・`coverage` 相当・`audit` 相当。`aidlc-distribution` /
  `review-thread-resolution` / `ci-success` とマージキューの完走は未実行。
- **複数プロセスの並行書込**: 本家接続に `busy_timeout` を設定できないため `SQLITE_BUSY` → `Io { kind: WouldBlock }` になる、という
  設計上の記述は単一プロセスのテストでは実証していない。
- **`reopened()` 複数ハンドルと兄弟接続の並行**: 直列化の実体が本家 CAS であることは契約テストが単一プロセスで示すのみ。
- **末尾欠落の検出**: 差分の途中欠落は層 (3) が捕まえるが、末尾行の消失は検出できない（BR1.2、意図された範囲）。
- **改竄の検出範囲**: 不変条件を満たす形への書き換えは検出せずそのまま再生する（NFR4.4 / R-05）。
- **`SnapshotStrategy` の合成ルート配線**: 既定 10 のまま `runtime.rs` から設定されていない。値の確定は U7。

### 7.2 申し送り

1. **U7 の裁定 3 件**（複数プロセス並行モデルと `reopened()`、登録簿 `intents.json` の直列化、`SnapshotStrategy` 既定値）は先取りしていない。
2. **上流 `requirements.md` の失効**: ブリーフが挙げた `:44-47` / `:63` / `:133-135` / `:168-170` / `:186` はいずれも実在を確認した
   （旧名 `WorkflowExecution` / `WorkflowExecutionRepository`、退役した `audit_lock.qnt` への参照）。**加えて `:9`**（冒頭の改訂注記
   「WorkflowExecution 集約ルート・ロック機構退役」）も同じ旧名を含む。指示どおり直していない（オーナー裁定待ち）。
3. **`docs/specs/` の旧名**: `01-domain-model.md` / `10-orchestration.md` / `11-workspace.md` / `12-workflow-definition.md` の **4 ファイル**
   に残る（取り消し線付きの履歴記述、**U9 の所有**）。今回の grep 範囲に含めておらず、追従もしていない。
4. **凍結中の設計文書への折り戻し候補**: §2.1 の M-1 / M-2 / M-3（`security-design.md` の行番号 3 件）と M-4
   （`unit-test-instructions.md` の `dto` 期待件数 29 → 47）。M-5 は計画本文、M-6 はブリーフ本文の話であり、いずれも
   ステージゲートの Request Changes 経路に載せるのが筋である。本委任では計画・テスト手順・質問票・設計文書を変更していない。
5. **`coding-rules/README.md` の機械化ロードマップ**が「実装済みは 6 本」のままで、実測 7 本（`use-case-domain-getter` を含む）と
   食い違う。同 README の一覧表側は既に 7 本目を載せている。coding-rules は本 Unit の所有外なので直していない。
6. **前提の数値のずれ**: ブリーフ前文・`team.md` の「ワークスペース lints 47 本 deny」は実測 50 ルール（49 deny + 1 forbid）と合わない。
   同じく `team.md` のカバレッジ 94.87〜95.29% は実測 99.15% と乖離している。どちらも memory 層の記述であり、本委任の書込対象外。
7. **`cargo test --workspace` の総数**: `team.md` は 234 テストと記すが実測 2354。intent の進行に伴う増加であり、異常ではない。
