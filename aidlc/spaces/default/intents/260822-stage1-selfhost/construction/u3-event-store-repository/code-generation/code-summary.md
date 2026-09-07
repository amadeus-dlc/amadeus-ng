# code-summary — U3 イベントストアと IntentExecutionRepository（`u3-event-store-repository`）

> Unit: U3（kind: library）。**2026-09-07 再走（Modify）版**。本版は「現行の実装が何であり、どう検証されたか」を書く。
> Bolt B5（PR #29）当時の TDD 証跡・裁定表・コミット列・件数（674 / 98.42%）は歴史であり、
> `code-summary-history-2026-08-23.md` に全文保存した。ここには現在の状態として再掲しない。
> 実測は 2026-09-07 16:18〜16:41 JST、ワークツリー `stage1-selfhost`（HEAD `7ad3d7b9`。コードは `origin/main` = `f2b6b6a9` と同一 —
> `git diff --stat origin/main HEAD -- modules tests formal scripts Cargo.toml Cargo.lock .github tools` が空）。
> 実行記録の逐語は `developer-report-11.md`。

## 1. 結果（実測）

### 1.1 環境

| 項目 | 版 |
|---|---|
| `rustc -V` | 1.95.0 (59807616e 2026-04-14)（`rust-toolchain.toml` の固定と一致） |
| `cargo llvm-cov --version` | 0.8.5 |
| `cargo audit --version` | cargo-audit-audit 0.22.2 |
| `quint --version` | 0.32.0 |

### 1.2 Unit 限定コマンド 11 本（`unit-test-instructions.md` §2）

すべて終了コード 0・`failed` 0・`ignored` 0（11 本を `&&` で連結した実行が完走）。

| # | コマンド（`cargo test --locked` に続く部分） | passed | 計画準備時の期待 | 判定 |
|---|---|---|---|---|
| 1 | `-p core-command-interface-adapter --test intent_execution_repository_contract` | 22 | 22 | 一致 |
| 2 | `-p core-command-interface-adapter --test intent_execution_repository_impl_test` | 23 | 23 | 一致 |
| 3 | `-p core-command-interface-adapter --test upstream_event_store_conformance` | 10 | 10 | 一致 |
| 4 | `-p core-command-interface-adapter --lib orchestration::intent_execution_repository_impl` | 10 | 10 | 一致 |
| 5 | `-p core-command-interface-adapter --lib orchestration::store_failure` | 4 | 4 | 一致 |
| 6 | `-p core-command-interface-adapter --lib orchestration::snapshot_strategy` | 2 | 2 | 一致 |
| 7 | `-p core-command-interface-adapter --lib orchestration::dto` | 47 | 29 | **不一致（テスト手順の集計漏れ。§3 M-4）** |
| 8 | `-p core-command-domain --lib workspace::store_path` | 4 | 4 | 一致 |
| 9 | `-p core-command-domain --lib workspace::intent_dir_name` | 9 | 9 | 一致 |
| 10 | `-p aidlc --test crash_reconstruction_test` | 5 | 5 | 一致 |
| 11 | `-p aidlc --test journal_protocol_conformance` | 5 | 5 | 一致 |

Step 3（Quint 凡例の追従）の**後**に再実行した 2 本も同値で緑である。

| コマンド | passed | 完了時刻 |
|---|---|---|
| `-p aidlc --test journal_protocol_conformance`（凡例追従後） | 5 | 16:25:16 JST |
| `-p aidlc --test crash_reconstruction_test`（凡例追従後） | 5 | 16:25:19 JST |

### 1.3 受入（計画 Step 4）

| 受入 | コマンド | 結果 |
|---|---|---|
| (a) NFR2.4 | `cargo fmt --all --check` | 終了コード 0 |
| (a) NFR2.4 | `cargo clippy --workspace --all-targets -- -D warnings` | 終了コード 0（警告なし） |
| (a) NFR2.4 | `cargo lint` | 終了コード 0（所見 0 件） |
| (a) NFR2.4 | `cargo test --manifest-path tools/lint/Cargo.toml` | 93 passed / 0 failed / 0 ignored、終了コード 0 |
| (b) NFR2.5 | `bash scripts/quint-gate.sh`（凡例追従の**後**） | `[PASS] quint gate: all steps green` — typecheck 3 / 不変条件 run 3 / witness 18 / 決定的シナリオ 1 の全 25 ステップ PASS |
| (c) NFR2.3 | `bash scripts/coverage.sh` 1 回目 | head line coverage **99.15169660678644%**、`[PASS] absolute gate ... >= 90.0%` |
| (c) NFR2.3 | `bash scripts/coverage.sh` 2 回目 | head line coverage **99.15169660678644%**、`[PASS]`。**差 0.00 ポイント**（受入目標を満たす） |
| (d) NFR4.1 | `cargo audit` | 終了コード 0。advisory DB 取得成功（`https://github.com/RustSec/advisory-db.git`）、1239 advisories 読込、`Cargo.lock` の **125 crate** を走査、脆弱性報告なし |
| (d) NFR4.1 | `cargo audit --file tools/lint/Cargo.lock` | 終了コード 0。同 DB、`tools/lint/Cargo.lock` の **5 crate** を走査、脆弱性報告なし |
| (e) NFR1.2 / BR3.1 | 退役語彙 grep（13 語を `modules tools scripts formal .github Cargo.toml`） | **0 件**。`ls formal/orchestration/` = `engine_loop.qnt` / `journal_protocol.qnt` / `stop_hook.qnt` の 3 つだけ |
| (f) 全体ゲート | `PROPTEST_RNG_SEED=20260823 cargo test --workspace` | **2354 passed / 0 failed / 0 ignored**（`test result:` 行 54 本の合計）、終了コード 0、実時間 26.42 秒（ビルド温存時） |

(f) は Unit 限定コマンドではなく、CI `check` ジョブ相当の全体ゲートである。

コンダクタの独立再実測（2026-09-07 07:45〜07:58 UTC、同一ワークツリー・凡例追従後）: Unit 限定コマンドのうち契約 22 / 実装固有 23 /
本家適合 10 / `orchestration::dto` 47 / クラッシュ再構成 5 / ITF 適合 5 が同値で緑、`cargo fmt --all --check` / `cargo clippy --workspace
--all-targets -- -D warnings` / `cargo lint` が終了コード 0、`tools/lint` 自己テスト 93、`bash scripts/quint-gate.sh` が `[PASS] quint gate: all
steps green`、`bash scripts/coverage.sh` 3 回目も **99.15169660678644%**（差 0.00）、`cargo audit` 2 件が 125 / 5 crate で報告なし、退役 grep と
旧名 grep が 0 件、`git diff` が凡例 5 行のみ — 開発担当の報告と全項目一致した。

quint-gate の内訳（`scripts/quint-gate.sh` 実測）: `journal_protocol.qnt` は typecheck 1 + 不変条件 run 1（`conflict_rejected` /
`snapshot_tracks_journal` / `version_equals_journal` / `checkpoint_monotone` / `checkpoint_bounded` / `projection_idempotent` /
`truth_is_journal` / `no_lost_update` の 8 本を 1 回の `quint run` に載せる）+ witness 4（`w_conflict` / `w_crash_then_catchup` /
`w_interleaved_writers` / `w_idempotent_catchup`、反転判定）。残りは `engine_loop` / `stop_hook`（U2 / 他 Unit の所有）である。

## 2. 変更したファイルと現行の実装ファイル

### 2.1 本 Bolt で変更したもの（1 ファイル・5 行）

`formal/orchestration/journal_protocol.qnt` の**凡例コメントだけ**。状態機械本体（`var` / `action` / `val` / `run` / witness）は
1 文字も変えていない。`git diff --stat` = `1 file changed, 5 insertions(+), 5 deletions(-)`。

```diff
-//   snapVersion   ↔ snapshot 表の version 列 = WorkflowExecution::version() (楽観 version)
-//   snapSeq       ↔ snapshot 表の seq_nr 列 = WorkflowExecution::seq_nr() (適用済みイベント数)
+//   snapVersion   ↔ snapshot 表の version 列 = IntentExecution::version() (楽観 version)
+//   snapSeq       ↔ snapshot 表の seq_nr 列 = IntentExecution::seq_nr() (適用済みイベント数)
-//                   = WorkflowExecutionRepository::find_by_id が with_version で載せた値
+//                   = IntentExecutionRepository::find_by_id が with_version で載せた値
-//   load(w)           ↔ WorkflowExecutionRepository::find_by_id (未書込なら genesis を持つ)
-//   store_ok(w)       ↔ WorkflowExecutionRepository::store が Ok (Tx 内で journal + snapshot)
+//   load(w)           ↔ IntentExecutionRepository::find_by_id (未書込なら genesis を持つ)
+//   store_ok(w)       ↔ IntentExecutionRepository::store が Ok (Tx 内で journal + snapshot)
```

同じ凡例の U4 側の名前（`JournalReader::checkpoint(ProjectionName)`、`events_after` / `advance_checkpoint`）は現行
`read-model-updater/src/orchestration/journal_reader.rs` と一致するので触っていない。

旧名 `WorkflowExecution` の grep 結果: 追従**前**は `journal_protocol.qnt` の `:10` / `:11` / `:15` / `:22` / `:23` の 5 行だけ
（`modules tests scripts .github Cargo.toml tools formal` の他は 0 件）、追従**後**は同じ範囲で **0 件**。

### 2.2 現行の実装ファイル（本 Bolt では変更していない — 来歴を 1 行ずつ）

| ファイル | 行数 | 来歴 |
|---|---|---|
| `modules/core/command/interface-adapter/src/orchestration/intent_execution_repository_impl.rs` | 822 | B5（PR #29）で新設 → B7 で本家 v3 `EventEnvelope` へ → B12 / B13 で改名・版の内側化 → 2026-09-05 書込前 ID 照合（PR #109）→ b51（PR #118） |
| 同 `snapshot_strategy.rs` | 58 | B5 で新設（既定 10、`every(NonZeroUsize)`）。以後変更なし |
| 同 `store_failure.rs` | 124 | B5 で新設（rusqlite code → `std::io::ErrorKind`）。`io_kind` は `:20-34` |
| 同 `dto/`（32 エントリ） | — | B12 / B13 で `wire` から DTO へ全面移行。イベント変種 DTO 16 種（`IntentExecutionEventDto` の腕数と一致） |
| 同 `orchestration/mod.rs` ファサード | — | 実装 mod は private、公開は `pub use`（`:38-41` / `:46` / `:52` / `:62` / `:65-66` 起点。最後の `pub use` は `:68` で閉じる） |
| `modules/core/command/use-case/src/orchestration/port/intent_execution_repository.rs` | 201 | ポート trait（AFIT、`find_by_id(&self)` / `store(&mut self)`）。B13 で現行名 |
| 同 `port/repository_error.rs` | 168 | `RepositoryError<Id>` — `NotFound` / `Conflict` / `Io` / `Corrupt { id, seq_nr, source }` |
| `modules/core/command/domain/src/orchestration/intent_execution.rs` | — | `new` `:290-337`、`replay` `:352`、`apply_event` `:1514`、`with_version` `:254`、誕生変換 `From<(Started, DateTime<Utc>)>` の `fn from` `:2373` |
| `modules/core/command/domain/src/workspace/store_path.rs` | 91 | `StorePath::for_space`。生の `PathBuf` を受けない |
| 同 `workspace/intent_dir_name.rs` | 208 | `IntentDirName` の文法（6 桁日付 + 区切り + 小文字 slug、連続ハイフン / 空区間 / 大文字 / 64 字超を拒否） |
| `formal/orchestration/journal_protocol.qnt` + `tests/conformance/fixtures/journal_protocol/`（8 トレース） | — | 本 Bolt の変更対象（凡例 5 行のみ）。fixture は 8 本で不変 |
| `modules/app/aidlc/tests/journal_protocol_conformance.rs` | — | ITF 再生（`SnapshotStrategy::every(1)` 明示、`:226-228`） |
| `modules/app/aidlc/tests/crash_reconstruction_test.rs` | — | クラッシュ再構成 5 件 |

## 3. 設計との照合（計画 Step 2）

不一致は **6 件**で、いずれも**設計文書側の行番号・件数の引用精度**の問題である。現行コードの振る舞いと設計の主張が食い違う箇所は
**0 件**であり、したがって「現行コードで落ちる Red テスト」を構成できる不一致は無い（詳細と根拠は `developer-report-11.md` §2）。
本 Bolt では計画どおり凡例 5 行以外のコード・設計文書を変更していない。

| ID | 対象 | 設計の主張 | 実測 | 判定 |
|---|---|---|---|---|
| M-1 | `security-design.md` §2 (2) | `IntentExecution::new` は `intent_execution.rs:290-345` | `new` は `:290-337`（`:338-345` は `replay` の doc コメント） | 不一致（終端 +8 行） |
| M-2 | `security-design.md` §2 (2) | `IntentExecutionDto::to_domain` は `dto/intent_execution_dto.rs:208-293` | 最終文は `:293`、関数の閉じ括弧は `:294` | 不一致（終端 −1 行） |
| M-3 | `security-design.md` §3 | `stored_version` は `:313-321` | `:313-320`（`:321` は `impl` ブロックの閉じ括弧） | 不一致（終端 +1 行） |
| M-4 | `unit-test-instructions.md` §2 | `dto` の属性合計は 29 | `--lib orchestration::dto` の実行は 47。`dto/tests.rs` が 29、残り 18 は同ディレクトリ 6 ファイル（`dto_vocabulary` 6 / `workflow_definition_dto` 6 / `dto_decode_error` 2 / `intent_execution_aggregate_key_dto` 2 / `intent_aggregate_key_dto` 1 / `workflow_definition_aggregate_key_dto` 1） | 不一致（フィルタ範囲の集計漏れ。`security-design.md` §2 の「`dto/tests.rs` 29 件」は正しい） |
| M-5 | 計画 §3 Step 2 (d) | ファサードは `orchestration/mod.rs:38-66` | 起点 5 か所は完全一致。最後の `pub use` は `:66` に始まり `:68` で閉じる | 不一致（範囲終端のみ） |
| M-6 | `developer-brief-9.md` | 「旧 `WorkflowExecutionRepository` の名前を現行公開 API に残さない」は functional-spec **§5** `:137` | 行番号 `:137` は正しいが、その行は **§6 退役した設計**（§5 は `:121`、§6 は `:133`） | 不一致（節番号のみ） |

一致した主張（抜粋。すべて `grep -n` と実行結果で確認した）:

| 項目 | 設計の主張 | 実測 |
|---|---|---|
| (a) 層 (0) 書込前 ID 照合 | `store` 冒頭 `:447-452` | `if event.aggregate_id() != aggregate.id()` `:447`、`});` `:452` — 一致 |
| (a) 層 (0') 書込契約 | `write_error` `:277-303` | 一致（`SerializationError` / `ContractViolation` → `Corrupt(WriteContract)`） |
| (a) 層 (1) ストア復号 | `read_error` `:245-265` | 一致（`DeserializationError` → `Corrupt { seq_nr: None, StoreDeserialization }`） |
| (a) 層 (3) 差分ループ | `find_by_id` `:331-439`、差分 `:378-438` | `find_by_id` `:331-439`、`let delta` `:378`、`replay` 行 `:438` — 一致 |
| (a) 層 (4) クラッシュ境界 | `replay:352` / `apply_event:1514` / 誕生変換 `:2373`、`# Panics` は 3 か所（正本 `:40-41`） | 一致（`# Panics` は `:347` / `:1506` / `:2364` の 3 か所のみ） |
| (a) `CorruptDetail` | `:101-113`、6 変種 | 一致 |
| (a) 固定するテスト名 | 契約 2 本・実装固有 9 本 | すべて実在（`intent_execution_repository_impl_test.rs` の関数 23 本を列挙して確認） |
| (b) 保存経路の分岐 | 分岐は `:465`、`stored_version` `:313`、`reopened` `:209-215` | 一致（`let outcome = if aggregate.seq_nr() == 1 \|\| self.strategy.wants_snapshot(...)` が `:465`） |
| (c) 障害写像表 | `store_failure.rs:20-34` | `const fn io_kind` `:20`、閉じ括弧 `:34` — 完全一致 |
| (d) DTO | `dto/` 32 エントリ、イベント変種 16 | `ls` で 32、`IntentExecutionEventDto` の腕 16 — 一致 |
| (d) 依存 | domain に serde / ESA なし（`serde_json` は dev のみ）、use-case に ESA / adapter なし、`thiserror` は推移のみ | `Cargo.toml` 実測で一致。`cargo tree -i thiserror` = `event-store-adapter-rs v3.0.0` 経由のみ |
| (d) 境界 | `interface-adapter/src/` に `CREATE TABLE` / `PRAGMA` / `busy_timeout` が 0 件 | grep 0 件 — 一致 |
| (e) `store` / `find_by_id` の手順 | functional-spec §3.1 1〜5 / §3.2 1〜5 | 逐条で一致（genesis `seq_nr == 1` → `persist_event_and_snapshot`、基底欠落 + journal あり → `Corrupt(MissingSnapshot)`、`version = snapshot.version()` を `:364` で束ね `:438` で `with_version(version)`） |
| (f) `rules.md` BR1.1〜BR5.2 | 23 本 | 23 本すべて現行コード・テストと整合。BR4.1（小文字 36 字・version 7・variant RFC4122）は `intent_execution_id.rs:40-41` とテスト `:72` / `:156`、BR4.2 の受理 / 拒否例は `intent_dir_name.rs:91-92` / `:131` / `:168` / `:173` で確認 |
| (g) 旧名 grep | 凡例 5 行のみ、他 0 件 | 一致（追従後は全範囲 0 件） |
| §3 合成ルート | `runtime.rs` の `open` 呼出は 8 か所、いずれもコマンド単位 | 8 か所（`report` / `single_report` / `skeleton_stance_report` / `log_review` / `practices_promote` / `set_autonomy` / `park` / `mint_intent`）— 一致 |
| §5 / NFR2.4 | `cargo lint` 7 ルール | 7 本すべて実在（`port-naming` / `command-side-io` / `no-public-fields` / `one-public-type` / `dao-single-table` / `checkbox-vocabulary` / `use-case-domain-getter`） |
| NFR2.4 | 検査点 lint は `Cargo.toml:39-44` | `unwrap_used` `:39` / `expect_used` `:40` / `indexing_slicing` `:43` / `panic` `:44` — 完全一致 |
| NFR2.5 | ITF は `every(1)` 明示（`:226-227`）、fixture 8 | 一致（`SnapshotStrategy::every(NonZeroUsize::new(1))` が `:226-228`、fixture 8 本） |
| NFR4.1 | `ci.yml:186-190` に `cargo audit` 2 件 | 一致 |
| CI | 7 ジョブ | `aidlc-distribution` / `check` / `quint` / `coverage` / `audit` / `review-thread-resolution` / `ci-success` — 一致 |

## 4. テスト配置の件数（`logical-components.md` §4 との照合）

| 種別 | 設計の件数 | 実測 | 判定 |
|---|---|---|---|
| 契約テスト（両バックエンド） | 11 関数 × 2 | 22 実行 | 一致 |
| 実装固有 | 23 | 23 | 一致 |
| 本家適合 | 5 × 2 | 10 実行 | 一致 |
| インライン `impl` | 10 | 10 | 一致 |
| インライン `store_failure` | 4 | 4 | 一致 |
| インライン `snapshot_strategy` | 2 | 2 | 一致 |
| インライン `dto/tests.rs` | 29 | 29 属性（ただし `orchestration::dto` フィルタの実行は 47。§3 M-4） | 一致（フィルタ範囲は不一致） |
| クラッシュ再構成 | 5 | 5 | 一致 |
| ITF 適合 | 8 トレース | fixture 8 本 | 一致 |
| ゲート | CI 7 ジョブ | 7 ジョブ | 一致 |

配置はテストピラミッド（ユニット層が厚く、結合は境界ごと、E2E は最小）を保っている。

## 5. 依存（実測）

| クレート | 依存 | dev 依存 |
|---|---|---|
| `core-command-domain` | `chrono`、`uuid`（v7）、`core-infrastructure` | `proptest`、`serde_json` |
| `core-command-use-case` | `core-command-domain`、`chrono` | `tokio` |
| `core-command-interface-adapter` | `core-command-use-case`、`core-command-domain`、`core-infrastructure`、`rusqlite`、`serde`、`serde_json`、`chrono`、`event-store-adapter-rs`（`sqlite` feature） | `tempfile`、`tokio` |

- 本家のピンは `Cargo.toml:119` の `event-store-adapter-rs = "=3.0.0"` で不変（NFR4.1）。
- `thiserror` はワークスペースの直接依存ではなく `event-store-adapter-rs v3.0.0` 経由の推移依存のみ（`cargo tree -p core-command-interface-adapter -i thiserror` 実測）。error-handling 規則（thiserror / anyhow 不使用）は保たれている。
- ドメインの永続化中立は `Cargo.toml` の不在（serde / ESA なし）で機械強制されている。
- ワークスペース lints は **50 ルール**（`[workspace.lints.clippy]` 44 deny + `[workspace.lints.rust]` 4 deny + `unsafe_code = "forbid"` 1 + `[workspace.lints.rustdoc]` 1 deny）。`team.md` と本 Bolt のブリーフ前文が記す「47 本 deny」は現行と合わない（`developer-report-11.md` §2 の注記を参照。本 Bolt では memory を変更しない）。

## 6. 未検証範囲

本 Bolt の実測が示していないもの。設計（`security-design.md` §2 / §3、`functional-spec.md` §7）が未検証と明記しているものと同じである。

- **CI の全ジョブ実行**: ローカルで走らせたのは `check` 相当（fmt / clippy / lint / test）・`quint` 相当・`coverage` 相当・`audit` 相当である。`aidlc-distribution` / `review-thread-resolution` / `ci-success` と、マージキューの完走は未実行。
- **複数プロセスの並行書込**: 本家接続に `busy_timeout` を設定できないため待たずに `SQLITE_BUSY` → `Io { kind: WouldBlock }` になる、という設計上の記述は、単一プロセス内のテストでは実証していない。
- **`reopened()` 複数ハンドルと兄弟接続の並行**: 直列化の実体が本家 CAS であることは契約テスト（`concurrent_rehydration_conflicts` / `genesis_twice_conflicts`）が単一プロセスで示すのみ。並行モデルは U7 の裁定に繰り延べ。
- **末尾欠落の検出**: 差分の途中欠落は層 (3) が捕まえるが、末尾行の消失は別の終端記録が無いため検出できない（BR1.2、意図された範囲）。
- **改竄の検出範囲**: 不変条件を満たす形への書き換え（別の妥当なステージ・判定への差し替え）は検出せずそのまま再生する。暗号学的完全性は要求外（NFR4.4 / R-05）。
- **`SnapshotStrategy` の合成ルート配線**: 既定 10 のまま `runtime.rs` から設定されていない（`grep SnapshotStrategy modules/app/aidlc/src/` = 0 件）。値の確定は U7。

## 7. 申し送り

- **U7 の裁定 3 件**（`tech-stack-decisions` §3 / `contract-summary` §4）: (1) 複数プロセスの並行モデルと `reopened()` 複数ハンドルの扱い、(2) 登録簿 `intents.json` の直列化（旧 `within_write_transaction` は退役済み）、(3) `SnapshotStrategy` 既定値の確定。本 Unit はいずれも先取りしていない。
- **上流 `requirements.md` の失効**: `:9` / `:44-47` / `:63` / `:133-135` / `:168-170` / `:186` が旧名 `WorkflowExecution` / `WorkflowExecutionRepository`・退役した `audit_lock.qnt` のまま残る（NFR 設計レビュー R-06 は `:133-135` のみを挙げたが、実測では 6 か所）。本 Bolt では直していない（オーナー裁定待ち — 後方ジャンプで上流を改訂するか、失効注記だけを付すか）。
- **`docs/specs/` の 4 ファイル**に残る `WorkflowExecution` は取り消し線付きの履歴記述であり **U9 の所有**。本 Bolt の追従対象外（今回の grep 範囲にも入れていない）。
- **凍結中の設計文書の折り戻し先**: 機能設計 / NFR 要求 / NFR 設計の各 `pending-revision.md` に確定文面を置き、ステージゲートの Request Changes 経路で折り戻す。§3 の M-1〜M-3（`security-design.md` の行番号 3 件）と M-4（`unit-test-instructions.md` の `dto` 件数）はこの経路に載せる候補である。`traceability.json` の BR5.2 は当初 `code-summary.md`（記録側）を target にしていたが、OK target は実装・テスト側の実在ファイルに限るというステージ規約に合わせ、受入を機械実行する `.github/workflows/ci.yml` へ差し替えた（コンダクタ判断）。
- **`coding-rules/README.md` の機械化ロードマップ**が「実装済みは 6 本」（2026-09-04 更新）と書くが、実測は 7 本（`use-case-domain-getter` が 2026-09-05 に加わっている）。同 README の一覧表側は既に 7 本目を載せており、ロードマップ節の本文だけが古い。coding-rules は本 Unit の所有ではないため直していない。
- **カバレッジの現況**: 実測 99.15%（絶対床 90% に対し十分な余裕）。`team.md` が記す 94.87〜95.29% は B5 当時の値であり、現行と乖離している。

## 8. B5 版（2026-08-23）からの変更

本版は B5 の記録を上書きするのではなく、**役割を分けた**。

- B5 の TDD 証跡（Red / Green の逐語）、当時の件数（674 テスト / 98.42%）、裁定表、コミット列は歴史であり `code-summary-history-2026-08-23.md` に全文保存した。本版はそれらを「現在の実施」として再掲しない。
- 本版は 2026-09-07 時点の実測だけを書く。B7（本家 v3）・B12 / B13（集約分割と版の内側化）・b40（イベント ID）・2026-09-05 是正（書込前 ID 照合）・b51 を経た**現行の実装**が対象である。
- 本再走で書き直した理由は、旧版がクレート名・型名（`core-interface-adapter` / `WorkflowExecutionRepository` / `wire` / `CorruptCause`）ごと現行と食い違っており、記録として読めば誤誘導になるためである。
- 本 Bolt が実際に変更したワークスペース側のファイルは `formal/orchestration/journal_protocol.qnt` の凡例 5 行のみで、`source-manifest.json` の `writes` もこの 1 件だけである。
