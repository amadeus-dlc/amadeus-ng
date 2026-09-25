# コード品質評価 — amadeus-ng

評価の範囲は `reverse-engineering-timestamp.md` の Scope of Analysis のとおりで、深く読んだのは Issue #134 の経路だけである。それ以外の所見はディレクトリ単位の棚卸しに基づく。

## テストとカバレッジ

- **テストの置き場**: 各クレートの `tests/`（`aidlc` 40 ファイル、`core-command-domain` 19、`core-command-interface-adapter` 23、`core-read-model-updater` 21、`core-query-interface-adapter` 10 ほか）、ソース内の `#[cfg(test)]` と `*_tests.rs`、リポジトリ直下の `tests/`（ゴールデン・conformance・支援）、`scripts/tests/`（Python）、`scripts/goldens/*.test.ts`（Bun）。
- **カバレッジ**: `scripts/coverage.sh`（cargo-llvm-cov）。ワークスペース全体の行カバレッジ **90% を絶対床**とし、除外は `modules/app/aidlc/src/main.rs` だけ。CI の `coverage` ジョブで実行する。
- **評価**: テストの厚みは十分。ただし並行性の検証は弱い。プロセスを跨ぐ `concurrent_duplicate_completions_persist_only_one_receipt`（`pipeline_link_contract.rs:409`）はタイミング依存で、失敗は偶発的にしか出ない。coverage 計測で処理が遅くなると窓が重なりやすい、という Issue の観測と合う。

## Lint と書式

- ワークスペース lints（`Cargo.toml` の `[workspace.lints]`）: `unsafe_code = "forbid"`、`missing_docs`・`unreachable_pub`・`unwrap_used`・`expect_used`・`panic`・`indexing_slicing`・`missing_errors_doc` ほかを deny。
- `clippy.toml`: テストでの unwrap / expect を許可し、契約 JSON の直列化関数を禁止する。
- `rustfmt.toml`: style_edition 2024、幅 100。
- `cargo lint`（`tools/lint`）: coding-rules の機械強制。
- `.markdownlint-cli2.jsonc`、`.coderabbit.yaml`。
- **評価**: 機械強制が非常に厚い。一方、**トランザクションの張り方（DEFERRED か IMMEDIATE か）を検出する規則は無い**ので、Issue #134 の原因候補は lint では捕まらない。

## CI/CD

`.github/workflows/ci.yml` のジョブは `aidlc-distribution`（upstream の取得とゴールデン検証）、`check`（fmt・clippy `-D warnings`・`cargo lint`・`cargo test --workspace`・tools/lint 一式）、`quint`、`coverage`、`audit`、`review-thread-resolution`、`ci-success`。merge queue で運用している（Issue #134 が queue に与えた影響は R-9）。

## ドキュメント

- `README.md` はほぼ空（12 バイト）。入口は `AGENTS.md` / `CLAUDE.md`。
- 設計判断は doc コメント（日本語、裁定日・ADR 番号つき）とコード規則 `aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/` に集まっている。doc コメントの密度は非常に高く、`replace_steering` には「読み取ってから書くので `BEGIN IMMEDIATE` で書込ロックを最初に取る (BR2.3)」と理由まで書かれている（`journal_reader_impl.rs:1161`）。

## 技術的負債

| # | 所見 | 場所 | 重さ |
| --- | --- | --- | --- |
| TD-1 | **（修正前の所見。#154 で修正済み — `replace_pipeline` は `TransactionBehavior::Immediate` で始まる）** **`replace_pipeline` だけが DEFERRED トランザクションで「読んでから書く」**。同じファイルのほかの書込（`rebuild_read_model` 266 行、`ensure_read_schema` 471 行、`advance_checkpoint` 1067 行、`replace_plan_fingerprint` 1080 行、`replace_code_generation_approval` 1093 行、`replace_testing` 1146 行、`replace_steering` 1167 行）と `publication_store.rs` の 245 / 397 行はすべて `TransactionBehavior::Immediate` | `modules/core/read-model-updater/src/orchestration/journal_reader_impl.rs:1105` | 高（Issue #134 の根本原因の最有力候補） |
| TD-2 | 同じ型の DEFERRED 読取後書込が RMU にあと 2 か所。どちらもロック競合を `Corrupt(InvariantViolation)` に写すので、再実行で解ける失敗が「壊れている」に誤分類される | `hook_health_reader.rs:181`、`workspace_doctor_read_model_updater.rs:183` | 中（link の経路には乗らない） |
| TD-3 | **更新動詞の成否が後段の投影の成否に結合している**。`catch_up` が失敗すると、記録済みでも `Completion::refused`。`runtime.rs`・`runtime/pipeline_link.rs`・`runtime/learnings.rs`・`runtime/reuse_artifact.rs` がこの境界を共有する。upstream（`.claude/tools/aidlc-log.ts:1120-1283`）は投影という後段を持たない | `modules/app/aidlc/src/runtime.rs:3226-3231` | 高（Issue #134 の症状の形） |
| TD-4 | 失敗文言から失敗箇所を特定できない。`CatchUpError::Read` の `Display` は `read: io: WouldBlock at <path>` だけで、どの段（`prepare_read_model` / `events_after` / `replace_pipeline` / `publish` など）で起きたかを運ばない | `catch_up_error.rs` の `Display` | 中（運用時の切り分け） |
| TD-5 | 巨大ファイル: `intent_execution.rs` 9,753 行、`runtime.rs` 5,038 行、`workspace/projection.rs` 4,496 行、`turn.rs` 3,154 行、`wording.rs` 3,137 行、`journal_reader_impl.rs` 2,722 行 | `modules/` 各所 | 低〜中 |
| TD-6 | 書式の崩れ: SQL と処理を 1 行に詰めた長大行（`journal_reader_impl.rs:1106`・`1123`）、`wording` の 1 変種 1 行（`pipeline_link.rs:213-229`）。rustfmt が長い文字列リテラルを折れないため残る | 同左 | 低 |

そのほかの信号: TODO/FIXME/HACK/XXX は `modules/` 全体で 6 件。テスト以外の `#[allow]` / `#[expect]` は 67 件、`amadeus-lint: allow` 抑制は 9 件で、いずれも理由つき。

## Risks / follow-up（後続ステージへの申し送り）

開発担当のハンドオフ（`inception/reverse-engineering/developer-scan.md`）から引き継ぐ。**確認済みの事実と仮説を分けて書く。**

### R-1. 根本原因の仮説 — DEFERRED 昇格の即時 BUSY（仮説）

- **確認済み**: `replace_pipeline` は DEFERRED で `SELECT` → `DELETE` / `INSERT` と進む（TD-1）。SQLite は読取トランザクションを持つ接続の書込昇格では busy handler を呼ばず、即座に `SQLITE_BUSY` を返す。したがって `open_with_busy_timeout` の 5000ms（`journal_reader_impl.rs:88`、`223`）が効かない。この性質は SQLite 単体の実測（sqlite 3.53.1）で確認済み — 他接続が `BEGIN IMMEDIATE` を持つ間、timeout 2 秒の接続で `BEGIN` → `SELECT` → `DELETE` は **0.000 秒**で `database is locked`、`BEGIN IMMEDIATE` なら 2.1 秒待ってから失敗。
- **仮説**: 競合相手は重複した側 B の `store` である。B は古い版で `UPDATE snapshot … WHERE version = ?` を DEFERRED トランザクションの先頭で行い、0 行で楽観ロック失敗に終わるまで RESERVED ロックを持つ（本家の挙動は `dependencies.md` の外部依存表を参照）。A の `replace_pipeline` がこの窓で昇格すると即 BUSY になる。
- **未確認**: **アプリ本体での再現はまだしていない**。Issue の症状と経路はコード上一致するが、ほかの段（`publish` など）で起きている可能性は TD-4 のため文言からは排除できない。

### R-2. 修正方針の候補（設計担当が比較する）

| 候補 | 中身 | 効く範囲 | 残る懸念 |
| --- | --- | --- | --- |
| (a) | `replace_pipeline` をほかの書込と同じ `TransactionBehavior::Immediate` に揃える | DEFERRED 昇格の即時 BUSY が消え、busy timeout 5 秒が効く。最小変更で、既存の作法に揃えるだけ | 5 秒を超えるロック保持には効かない |
| (b) | `after_projection` / `catch_up` で `WouldBlock` に限り投影を再試行する | 一時的な競合全般に効く | (a) なしでは即時 BUSY を繰り返す可能性が残る。再試行の上限と待ち方の設計が要る |
| (c) | (a) と (b) の両方 | 両方の利点 | 変更範囲が広がる |

TD-2 の 2 か所を今回の射程に含めるかは要判断。link の経路には乗らないので、bugfix の射程を広げないなら記録だけにとどめる。

### R-3. 既存の契約テスト 2 本と両立させること（制約）

`a_publication_failure_preserves_the_receipt_for_recovery`（`pipeline_link_contract.rs:435`）と `a_failed_pipeline_projection_preserves_the_prior_query_result_until_recovery`（同 532 行）は、**恒常的な**公開失敗・投影失敗では、記録済みでも終了コード 1 で拒否し、受領を保持し、以前のクエリ結果を消さないことを固定している。修正は「一時的なロック競合」だけを失敗扱いから外す形に限る。`after_projection` の拒否そのものを外すと、この 2 本と衝突する。

### R-4. 契約テストの 2 つ目のアサーション（制約）

監査の `PIPELINE_LINK_COMPLETED` が 1 件であることは、A の投影が `publish` まで完走して初めて満たされる。修正後に A が成功を返すなら、監査の公開も同じ起動の中で終わっている必要がある。

### R-5. 受入条件の再現テスト（方針の材料）

Issue は「ロック競合を確実に起こす再現テストを加え、修正前に失敗し、修正後に通ること」を求めている。

- 足場: 既存の `a_write_lock_held_by_another_connection_is_reported_as_would_block`（`journal_reader_impl.rs:1977`）が、`open_with_busy_timeout` と別接続の `BEGIN EXCLUSIVE` 保持でロック競合を決定的に起こしている。
- `replace_pipeline` 向けには「別接続が `BEGIN IMMEDIATE` を保持し、短時間で解放する」状況を作れば、修正前は即時 `WouldBlock`、修正後は待って成功、と判別できる。
- プロセスを跨ぐ契約テストはタイミング依存のままなので、決定的な再現は RMU 層の単体テストで取るのが確実。

### R-6. 本家ストアの並行性の前提とのずれ（事実、判断は保留）

本家 `EventStoreForSqlite` は同じ DB ファイルを複数のストアインスタンスやプロセスから開くことをサポート外としている（NFR3.9。詳細は `dependencies.md`）。native は 1 起動で同じファイルに本家接続 3 本（`IntentExecutionRepositoryImpl` / `IntentRepositoryImpl` / `WorkflowDefinitionRepositoryImpl`）と RMU 接続を開き、さらに複数プロセスが並行する。rusqlite の既定 busy timeout（5000ms）のおかげで即座には壊れないが、前提がずれているという事実は設計で把握しておく。

### R-7. ジャーナルモード（射程外として記録）

リポジトリ内に `journal_mode` / WAL の設定は無い。WAL へ切り替えると並行読取の性質が変わる（読み手が書き手を妨げない）が、影響がストア全体に及ぶので bugfix の射程を超える。

### R-8. 別事象との区別

Issue #134 のコメントに、macOS ローカルで `cargo test --workspace` が 1 件だけ（毎回違うテストで）落ちる事象が記録されている（PR #138）。署名（子プロセスがシグナルで終了）が異なり、同根とは言えない。

### R-9. 優先度の背景

Issue 本文はこの不具合を stage-1 切替条件の実地スモーク（`live_bugfix_smoke_pass`、#7）の題材にするとしている。コメントでは、CI の merge queue を実際に止めている（再実行に約 20 分）ことから優先度の再判断が提起されている。
