# 開発担当コードスキャン — Issue #134（pipeline link の並行完了報告が WouldBlock で失敗する）

- 対象リポジトリ: `amadeus-ng`（単一リポジトリ、リポジトリ修飾なし）
- スキャン範囲: 全体（`./`）を浅く棚卸しし、Issue #134 に関わる経路（`aidlc engine log link` → 記録 → 投影 → SQLite のロック）だけを深く読んだ
- 深さ: Minimal / Test Strategy: Minimal / scope: bugfix
- 基準: `main` の先端 `5a2b51d3`。`modules/` に未コミットの差分は無い（`git diff --stat HEAD -- modules` は空）

## Developer Code Scan Results

### Scan Coverage
- **Analyzed deeply**:
  - `modules/app/aidlc/src/runtime/pipeline_link.rs`（全体。`log link` の構文検査・handoff 観測・保存・応答）
  - `modules/app/aidlc/src/runtime.rs`（190〜240 行の動詞振り分けと、3040〜3240 行の `prepare_definition_for_first_read` / `catch_up` / `catch_up_with` / `after_projection` / `catch_up_before_reading`）
  - `modules/core/command/use-case/src/orchestration/record_pipeline_link_use_case.rs`（全体）
  - `modules/core/command/interface-adapter/src/orchestration/intent_execution_repository_impl.rs`（`open` / `read_error` / `write_error` / `store`）
  - `modules/core/command/interface-adapter/src/orchestration/store_failure.rs`（全体）
  - `modules/core/read-model-updater/src/orchestration/journal_reader_impl.rs`（1〜580 行と 990〜1172 行。接続の開き方・busy timeout・全トランザクション・`JournalReader` 実装）
  - `modules/core/read-model-updater/src/orchestration/read_model_updater.rs`（1〜260 行と 490〜515 行。`catch_up` の手順と `catch_up_pipeline`）
  - `modules/core/read-model-updater/src/orchestration/catch_up_error.rs`（型と `Display`）
  - `modules/app/aidlc/tests/pipeline_link_contract.rs`（ハーネス、`concurrent_duplicate_completions_persist_only_one_receipt`、`a_publication_failure_preserves_the_receipt_for_recovery`、`a_failed_pipeline_projection_preserves_the_prior_query_result_until_recovery`）
  - `Cargo.toml` と、ワークスペース 10 クレートの各 `Cargo.toml`
  - `.github/workflows/ci.yml`（ジョブ構成とコマンド）
- **Skimmed only**:
  - `modules/core/command/domain/`
  - `modules/core/command/use-case/`（上記 1 ファイル以外）
  - `modules/core/command/interface-adapter/`（上記 2 ファイル以外）
  - `modules/core/query/use-case/`
  - `modules/core/query/interface-adapter/`
  - `modules/core/read-model-updater/`（上記以外。`publication_store.rs` / `shared_projection.rs` / `hook_health_reader.rs` / `workspace_doctor_read_model_updater.rs` はトランザクションの張り方だけ確認）
  - `modules/core/infrastructure/`
  - `modules/app/aidlc/`（上記以外）
  - `modules/harness/claude/`
  - `modules/harness/infrastructure/`
  - `tests/`（`conformance/`・`golden/`・`support/`）
  - `scripts/`（`coverage.sh` の冒頭のみ読んだ）
  - `tools/lint/`
  - `formal/`
  - `vendor/`
  - `.claude/`（`tools/aidlc-log.ts` の link ハンドラのロック区間だけ確認）
  - `.codex/`、`.agents/`、`.takt/`
  - `aidlc/`
- **リポジトリ外で参照したもの（分析範囲には数えない）**: cargo レジストリ上の `event-store-adapter-rs-3.0.0/src/event_store_for_sqlite.rs`（接続の開き方と書込トランザクション）、`rusqlite-0.40.2/src/inner_connection.rs`（接続時の既定 busy timeout）

### Packages Found
- `core-command-domain` — ライブラリ — Rust — コマンド側ドメイン（集約 `IntentExecution` / `Intent` / `WorkflowDefinition`、ドメインイベント、pipeline 受領の判定 `PipelineLinkError`）。約 482 ファイル / 67k 行
- `core-command-use-case` — ライブラリ — Rust — コマンド側ユースケースとリポジトリポート（`RecordPipelineLinkUseCase` など）
- `core-command-interface-adapter` — ライブラリ — Rust — コマンド側リポジトリ実装（`IntentExecutionRepositoryImpl` など。event-store-adapter-rs の SQLite ストアを包む）と永続化 DTO
- `core-query-use-case` — ライブラリ — Rust — クエリ側ユースケースとビュー型（`next` / `continue` の読取、directive）
- `core-query-interface-adapter` — ライブラリ — Rust — リードモデル（`read_*` 表）を引く DAO 実装
- `core-read-model-updater` — ライブラリ — Rust — RMU。ジャーナルを横断で読み、構造化リードモデル・状態ファイル・監査シャードへ投影する（`JournalReaderImpl` / `ReadModelUpdater`）
- `core-infrastructure` — ライブラリ — Rust — ドメインを持たない部品（canon_json、codec、ハッシュ、排他ファイルロック、秘密鍵鋳造）
- `aidlc` — バイナリ + ライブラリ — Rust — 合成ルート。マルチコールバイナリ（`aidlc-orchestrate` / `aidlc-utility` / `aidlc-log` など）と `aidlc engine <noun> <verb>` 形の起動
- `harness-claude` — ライブラリ — Rust — Claude Code フックとの結線（規則配送など）
- `harness-infrastructure` — ライブラリ — Rust — ハーネス側の部品
- `tools/lint`（`cargo lint`）— ワークスペース外の独立クレート — Rust — coding-rules の機械強制（`checkbox-vocabulary` / `no-public-fields` / `one-public-type` など）
- `.claude/tools/*.ts`（71 ファイル）— upstream AI-DLC の TypeScript 配布物 — TypeScript（Bun）— 本リポジトリが再実装している元の実装。ゴールデン比較の源

### Build System
- **Type**: Cargo ワークスペース（resolver 3、edition 2024、toolchain `1.95.0` 固定）。補助に Bun（ゴールデン採取の TS スクリプト）と Quint 0.32.0（形式モデル）
- **Config Files**: `Cargo.toml`、`Cargo.lock`、各クレートの `Cargo.toml`、`rust-toolchain.toml`、`rustfmt.toml`、`clippy.toml`、`.cargo/config.toml`（`cargo lint` の別名）、`tools/lint/Cargo.toml`、`.gitmodules`（`vendor/aidlc-workflows`）
- **Build Dependencies**:
  - `aidlc` → 全コア 7 クレート + `harness-claude`（合成ルートだけが両側を知る）
  - `core-command-interface-adapter` → `core-command-use-case` → `core-command-domain` → `core-infrastructure`
  - `core-read-model-updater` → `core-command-domain`、`core-infrastructure`（コマンド側とクエリ側の中間）
  - `core-query-interface-adapter` → `core-query-use-case` → `core-infrastructure`（コマンド側・RMU には依存しない。dev-dependency のみ RMU とドメインを引く）
  - `harness-claude` → `harness-infrastructure`、`core-infrastructure`、`core-command-domain`、`core-command-interface-adapter`
  - CQRS の側分割はクレート分離で物理的に強制されている（`coding-rules/cqrs-boundaries.md`）

### APIs Discovered
- CLI（マルチコールバイナリ）— `modules/app/aidlc/src/cli/request.rs` — `Request` 列挙で 39 変種へ振り分け（`runtime.rs:215-240` 付近）。Issue #134 の入口は `Request::LogLink` → `pipeline_link::run`（`runtime.rs:237`）
- CLI 出力契約 — `Completion`（stdout 1 行 JSON、stderr 診断、終了コード 0/1/3）。link 成功時は `{"emitted":"PIPELINE_LINK_COMPLETED",...}` を返す（`pipeline_link.rs:96-115`）
- 内部ポート（Rust trait）— コマンド側 use-case・クエリ側 use-case・RMU で `pub trait` が 55 個。Issue に関わるのは `IntentExecutionRepository` / `IntentRepository` / `WorkflowDefinitionRepository`（コマンド側）と `JournalReader`（RMU。`replace_pipeline` を含む）
- 永続化面 — 1 スペース 1 本の SQLite ファイル `aidlc/spaces/<space>/intents/.aidlc-store.sqlite`。本家の `journal` / `snapshot` 表と、自前の `amadeus_projection_checkpoint`・`amadeus_publication*`・`amadeus_read_model_head`・`read_*` 表が同居する
- HTTP / gRPC の外部 API は無い

### Frameworks & Libraries
- `event-store-adapter-rs` — `=3.0.0`（完全固定、`sqlite` feature）— 集約のイベントソーシング永続化
- `rusqlite` — `0.40.2`（`bundled`）— SQLite 接続。接続時に既定で busy timeout 5000ms を設定する
- `serde` — `1`（derive）/ `serde_json` — `1`（`preserve_order`・`float_roundtrip`）— DTO と契約 JSON（契約 JSON の直列化は `canon_json` 経由に限定、`clippy.toml` で強制）
- `chrono` — `0.4` — 発生時刻（`DateTime<Utc>`）
- `uuid` — `1.26`（合成ルートとドメインで `v7`）— 識別子の解析と採番
- `tokio` — `1`（`rt`・`macros`、合成ルートは `process`・`time` も）— current_thread ランタイム
- `hmac` `0.12` / `base64` `0.22` / `sha2` `0.10` / `getrandom` `0.3` — 封緘・ダイジェスト・乱数
- `hostname` `0.4` — 監査シャード名のホスト部
- `regex` `1` — Testing Posture の語句照合、テストの正規化
- `libc` `0.2` / `nix` `0.30` — `O_NOFOLLOW` などの OS 呼出、シグナル・ファイル操作
- `proptest` `1`、`tempfile` `3` — テスト専用

### Test Coverage
- **Test Directories**: 各クレートの `tests/`（`aidlc` 40 ファイル、`core-command-domain` 19、`core-command-interface-adapter` 23、`core-read-model-updater` 21、`core-query-interface-adapter` 10、ほか）、ソース内の `#[cfg(test)]` と `*_tests.rs`、リポジトリ直下の `tests/`（`golden/upstream-a277af21` など upstream 観測のゴールデン、`conformance/`、共通支援 `support/`）、`scripts/tests/`（Python）、`scripts/goldens/*.test.ts`（Bun）
- **Test Frameworks**: Rust 標準テスト + `tokio::test`、`proptest`、Quint の ITF トレース準拠テスト、Bun test、Python unittest
- **Coverage Config**: あり。`scripts/coverage.sh`（cargo-llvm-cov、ワークスペース全体の行カバレッジ 90% を絶対床、除外は `modules/app/aidlc/src/main.rs` のみ、`PROPTEST_RNG_SEED=20260823` 固定）。CI の `coverage` ジョブで実行

### Code Quality Indicators
- **Linting**: ワークスペース lints（`Cargo.toml` の `[workspace.lints]`。`unsafe_code = "forbid"`、`missing_docs`・`unwrap_used`・`expect_used`・`panic`・`indexing_slicing` などを deny）、`clippy.toml`（テストでの unwrap/expect 許可、契約 JSON の直列化関数を禁止）、`rustfmt.toml`（style_edition 2024、幅 100）、独立リンター `tools/lint`（`cargo lint`）、`.markdownlint-cli2.jsonc`、`.coderabbit.yaml`
- **CI/CD**: `.github/workflows/ci.yml` — `aidlc-distribution`（upstream 取得とゴールデン検証）、`check`（fmt・clippy `-D warnings`・`cargo lint`・`cargo test --workspace`・tools/lint 一式）、`quint`、`coverage`、`audit`。`.github/workflows/review-thread-resolution.yml`。merge queue で運用
- **Documentation**: `README.md` はほぼ空（12 バイト）。`AGENTS.md` / `CLAUDE.md` が案内。設計判断は doc コメント（日本語、裁定日・ADR 番号つき）とコード規則 `aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/` に集約されている。doc コメントの密度は非常に高い

### Technical Debt Signals
- **`replace_pipeline` だけが DEFERRED トランザクションで「読んでから書く」** — `modules/core/read-model-updater/src/orchestration/journal_reader_impl.rs:1105`（`self.connection.transaction()`）。同じファイルのほかの書込（`rebuild_read_model` 266 行、`ensure_read_schema` 471 行、`advance_checkpoint` 1067 行、`replace_plan_fingerprint` 1080 行、`replace_code_generation_approval` 1093 行、`replace_testing` 1146 行、`replace_steering` 1167 行）と `publication_store.rs` の 245 / 397 行はすべて `TransactionBehavior::Immediate`。`replace_steering` には「読み取ってから書くので `BEGIN IMMEDIATE` で書込ロックを最初に取る (BR2.3)」と理由まで書かれている（1161 行）。Issue #134 の根本原因候補（下記 Handoff Summary）
- **同型の DEFERRED 読取後書込が RMU にあと 2 か所** — `hook_health_reader.rs:181` と `workspace_doctor_read_model_updater.rs:183`。どちらもロック競合の失敗を `Corrupt(InvariantViolation)` に写しており、「再実行で解ける」失敗が「壊れている」に誤分類される。link の経路には乗らないが同じ種類の負債
- **更新動詞の成否が後段の投影の成否に結合している** — `runtime.rs` の `after_projection`（`catch_up` が失敗すると、記録済みでも `Completion::refused`）。更新動詞を持つ `runtime.rs`・`runtime/pipeline_link.rs`・`runtime/learnings.rs`・`runtime/reuse_artifact.rs` がこの境界を共有する。upstream（`.claude/tools/aidlc-log.ts:1120-1283`）は検査と監査追記を `withAuditLock` の中で済ませ、その後に成功を返すだけで、投影という後段を持たない
- **巨大ファイル** — `core-command-domain/src/orchestration/intent_execution.rs`（9,753 行）、`app/aidlc/src/runtime.rs`（5,038 行）、`read-model-updater/src/workspace/projection.rs`（4,496 行）、`app/aidlc/src/turn.rs`（3,154 行）、`app/aidlc/src/wording.rs`（3,137 行）、`journal_reader_impl.rs`（2,722 行）
- **書式の崩れ** — `journal_reader_impl.rs:1106` と `1123` は 1 行に SQL と処理を詰め込んだ長大行、`pipeline_link.rs:213-229` の `wording` も 1 変種 1 行の長大行。rustfmt が長い文字列リテラルを折れないため残っている
- **失敗文言から失敗箇所を特定できない** — `CatchUpError::Read` の `Display` は `read: io: WouldBlock at <path>` だけで、どの段（`prepare_read_model` / `events_after` / `replace_pipeline` / `publish` など）で起きたかを運ばない（`catch_up_error.rs` の `Display`）
- TODO/FIXME/HACK/XXX は `modules/` 全体で 6 件と少ない。`#[allow]` / `#[expect]` はテスト以外で 67 件、`amadeus-lint: allow` 抑制は 9 件（いずれも理由つきの作法）
- 既知の別事象: Issue #134 のコメントに、macOS ローカルで `cargo test --workspace` が 1 件だけ（毎回違うテストで）落ちる事象が記録されている（PR #138）。署名（子プロセスがシグナルで終了）が異なり、同根とは言えない

## Handoff Summary
- **Intent-relevant finding**:
  1. **失敗文言の出どころ**: `aidlc-orchestrate: projection: read: io: WouldBlock at …/.aidlc-store.sqlite` は、`after_projection`（`modules/app/aidlc/src/runtime.rs` 3225〜3230 行付近）→ `catch_up_with` の末尾 `updater.catch_up()` の `map_err(|error| format!("projection: {error}"))`（3222 行）→ `CatchUpError::Read`（`read: …`）→ `JournalReadError::Io { kind: WouldBlock }`（`io: WouldBlock at <path>`）の連なりで組み立てられる。`WouldBlock` は RMU の `store_failure.rs:46` が `SQLITE_BUSY` / `SQLITE_LOCKED` を写したもの。`require_unpublished`（3184 行）も同じ `projection:` 接頭辞を付けるが、読取のみなので競合源になりにくい
  2. **記録と投影の順序**: `pipeline_link::run` は `RecordPipelineLinkUseCase::execute` でイベントをストアへ保存し（`pipeline_link.rs:87-95`）、その後で `after_projection` を呼ぶ（96 行）。投影が失敗すると、保存は済んでいるのに終了コード 1 で拒否される。これは Issue の読み（「書き込みに成功した側が、その後の投影の読み取りでロック競合に当たり失敗として返した」）とコード上一致する。ユースケースの再試行は楽観ロックの `Conflict` を 1 回だけで（`record_pipeline_link_use_case.rs:37-42`）、投影の失敗は再試行しない
  3. **根本原因の最有力候補**: 投影の `catch_up` → `catch_up_pipeline`（`read_model_updater.rs:57-65`、`catch_up` の 215 行から呼ばれる）→ `JournalReaderImpl::replace_pipeline`（`journal_reader_impl.rs:1100-1127`）が **DEFERRED トランザクション**（1105 行）で `SELECT … FROM read_pipeline_progress` を先に実行し、その後 `DELETE` / `INSERT` へ書込昇格する。SQLite は、読取トランザクションを既に持つ接続の書込昇格では busy handler を呼ばず（デッドロック回避）、即座に `SQLITE_BUSY` を返す。したがって `open_with_busy_timeout` の 5000ms（88 行、223 行）が効かない。この性質は本スキャンで SQLite 単体の実測により確かめた — 他接続が `BEGIN IMMEDIATE` を保持している状態で、timeout 2 秒の接続が `BEGIN` → `SELECT` → `DELETE` を行うと **0.000 秒**で `database is locked`、`BEGIN IMMEDIATE` なら 2.1 秒待ってから失敗（sqlite 3.53.1）。アプリ本体での再現はまだしていない
  4. **競合相手になりうる相手**: 重複した側（B）は `already completed` で拒否される前に、古い版で `store` を試みうる。本家の `persist_event` は DEFERRED トランザクションの先頭で `UPDATE snapshot … WHERE version = ?`（`event_store_for_sqlite.rs` 503〜533 行）を行うため、0 行で楽観ロック失敗に終わるまでの短い間 RESERVED ロックを握る。その後 `Conflict` の 1 回再試行で読み直して `Duplicate` になる。A の `replace_pipeline` がちょうどこの窓で書込昇格すると即 BUSY になる。coverage 計測で処理が遅くなると窓が重なりやすいという Issue の観測と整合する。B は `run` の早い段階で拒否されるため、`after_projection` には到達しない（`pipeline_link.rs:91-94`）
  5. **失敗時の監査**: A の投影は `catch_up_pipeline` の段で落ちると、その後の `publish`（監査シャードへの `PIPELINE_LINK_COMPLETED` の書き出しを含む）まで進まない。受領はジャーナルに残り、次にいずれかの動詞が `catch_up` した時点で公開される（`a_publication_failure_preserves_the_receipt_for_recovery` が固定している「失敗しても受領は残り、回復で公開される」振る舞い）
- **Risks / follow-up**:
  - **既存の契約を壊さないこと**: `a_publication_failure_preserves_the_receipt_for_recovery`（`pipeline_link_contract.rs:435`）と `a_failed_pipeline_projection_preserves_the_prior_query_result_until_recovery`（同 532 行）は、**恒常的な**公開失敗・投影失敗では記録済みでも終了コード 1 で拒否し、受領を保持し、以前のクエリ結果を消さないことを固定している。修正は「一時的なロック競合」だけを失敗扱いから外す形に限る必要がある。`after_projection` の拒否そのものを外すと、この 2 本と衝突する
  - **修正方針の候補（設計担当が比較すること）**: (a) `replace_pipeline` を他の書込と同じ `TransactionBehavior::Immediate` に揃え、busy timeout を効かせる（最小変更。ほかの書込と作法を揃えるだけ）、(b) `after_projection` / `catch_up` で `WouldBlock` に限って投影を再試行する、(c) 両方。(a) だけでは busy timeout（5 秒）を超える保持には効かない。(b) だけでは DEFERRED 昇格の即時 BUSY を繰り返す可能性が残る。同じ種類の `hook_health_reader.rs:181` と `workspace_doctor_read_model_updater.rs:183` を今回の射程に含めるかは要判断（link の経路には乗らない。bugfix の射程を広げないなら記録だけにとどめる）
  - **受入条件の再現テスト**: Issue は「ロック競合を確実に起こす再現テストを加え、修正前に失敗し、修正後に通ること」を求めている。既存の `a_write_lock_held_by_another_connection_is_reported_as_would_block`（`journal_reader_impl.rs:1977`）が、`open_with_busy_timeout` と別接続での `BEGIN EXCLUSIVE` 保持でロック競合を決定的に起こす足場になる。`replace_pipeline` 向けには「別接続が `BEGIN IMMEDIATE` を保持 → 短時間で解放、という状況で `replace_pipeline` が待って成功する」形が、修正前に即時 `WouldBlock`・修正後に成功、と判別できる。プロセスを跨ぐ契約テスト（`concurrent_duplicate_completions_persist_only_one_receipt`）はタイミング依存のままなので、決定的な再現は RMU 層の単体テストで取るのが確実
  - **契約テストの 2 つ目のアサーション**: 監査の `PIPELINE_LINK_COMPLETED` が 1 件であることは、A の投影が `publish` まで完走して初めて満たされる。修正後に A が成功を返すなら、監査の公開も同じ起動の中で終わっている必要がある
  - **コマンド側の並行性の前提**: 本家 `EventStoreForSqlite` の doc は「同じ DB ファイルを複数のストアインスタンスやプロセスから開くことはサポート外（NFR3.9）」と明言している（`event_store_for_sqlite.rs` 70〜75 行付近）。native は 1 起動で同じファイルに本家接続 3 本（`IntentExecutionRepositoryImpl` / `IntentRepositoryImpl` / `WorkflowDefinitionRepositoryImpl`）と RMU 接続を開き、さらに複数プロセスが並行する。rusqlite の既定 busy timeout（5000ms、`inner_connection.rs:118`）が掛かっているので即座には壊れないが、この前提のずれは設計担当が把握しておくべき事実である
  - **ジャーナルモード**: リポジトリ内に `journal_mode` / WAL の設定は無く、SQLite 既定のロールバックジャーナルで動いている。WAL へ切り替えると並行読取の性質が変わる（読み手が書き手を妨げない）が、影響範囲がストア全体に及ぶため bugfix の射程を超える
  - **stage-1 との関係**: Issue 本文は、この不具合を stage-1 切替条件の実地スモーク（`live_bugfix_smoke_pass`、#7）の題材にするとしている。コメントでは、CI の merge queue を実際に止めている（再実行に約 20 分）ことから優先度の再判断が提起されている
