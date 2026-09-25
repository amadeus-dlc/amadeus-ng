# Reverse Engineering 実施記録 — amadeus-ng

| 項目 | 値 |
| --- | --- |
| 実施日時（UTC） | 2026-09-24T00:54:16Z |
| コミット（`git rev-parse HEAD`） | `5a2b51d31df6006fe7cc30ab5830a5aa4f0ad5a8`（`main` の先端 `5a2b51d3`、#149） |
| 事前スナップショットの source fingerprint | `git:b56861c5b1c7b90456a32e49bd8a10d9c79eb291` |
| 事前スナップショットのパス | `./` |
| ストアの状態 | NO_STORE（初回。9 成果物はすべてこの回の結果から新規に作成） |
| intent | `260924-pipeline-link-wouldblock`（Issue #134、scope: bugfix） |
| 深さ / テスト方針 | Minimal / Minimal |
| 作業ツリー | `modules/` に未コミットの差分なし（`git diff --stat HEAD -- modules` が空） |

## 範囲の判定

スキャンの目標は全体（`./`）だったが、実際に深く読んだのは Issue #134 の経路（`aidlc engine log link` → 記録 → 投影 → SQLite のロック）に関わるファイルと、ビルド・CI の設定だけである。リポジトリの残りはディレクトリ単位の棚卸しにとどまる。したがって `kind: partial` とし、`analyzed.paths` に `./` は入れない。

ファイル単位で記録したもののうち、次の 3 つは一部の行だけを読んでいる。

- `modules/app/aidlc/src/runtime.rs`: 190〜240 行（動詞の振り分け）と 3040〜3240 行（`prepare_definition_for_first_read` / `catch_up` / `catch_up_with` / `after_projection` / `catch_up_before_reading`）
- `modules/core/read-model-updater/src/orchestration/journal_reader_impl.rs`: 1〜580 行と 990〜1172 行（接続の開き方・busy timeout・全トランザクション・`JournalReader` 実装）、および 1977 行のテスト
- `modules/core/read-model-updater/src/orchestration/read_model_updater.rs`: 1〜260 行と 490〜515 行

`modules/core/read-model-updater/src/orchestration/store_failure.rs` は開発担当の Scan Coverage の「Analyzed deeply」には無いが、Handoff Summary が 46 行を根拠に挙げており、統合の段階で内容を確認したので含めた。

リポジトリ外で参照したもの（cargo レジストリ上の `event-store-adapter-rs-3.0.0/src/event_store_for_sqlite.rs`、`rusqlite-0.40.2/src/inner_connection.rs`）は分析範囲に数えない。

## Scope of Analysis

```yaml
scope_version: 1
kind: partial
intent: 260924-pipeline-link-wouldblock
fingerprint: dca948101a596b57eadf2633e46210af3b0282f0
analyzed:
  paths:
    - modules/app/aidlc/src/runtime/pipeline_link.rs
    - modules/app/aidlc/src/runtime.rs
    - modules/app/aidlc/tests/pipeline_link_contract.rs
    - modules/core/command/use-case/src/orchestration/record_pipeline_link_use_case.rs
    - modules/core/command/interface-adapter/src/orchestration/intent_execution_repository_impl.rs
    - modules/core/command/interface-adapter/src/orchestration/store_failure.rs
    - modules/core/read-model-updater/src/orchestration/journal_reader_impl.rs
    - modules/core/read-model-updater/src/orchestration/read_model_updater.rs
    - modules/core/read-model-updater/src/orchestration/catch_up_error.rs
    - modules/core/read-model-updater/src/orchestration/store_failure.rs
    - Cargo.toml
    - modules/core/command/domain/Cargo.toml
    - modules/core/command/use-case/Cargo.toml
    - modules/core/command/interface-adapter/Cargo.toml
    - modules/core/query/use-case/Cargo.toml
    - modules/core/query/interface-adapter/Cargo.toml
    - modules/core/read-model-updater/Cargo.toml
    - modules/core/infrastructure/Cargo.toml
    - modules/app/aidlc/Cargo.toml
    - modules/harness/claude/Cargo.toml
    - modules/harness/infrastructure/Cargo.toml
    - .github/workflows/ci.yml
  components:
    - aidlc
    - core-command-use-case
    - core-command-interface-adapter
    - core-read-model-updater
shallow:
  paths:
    - modules/core/command/domain/
    - modules/core/command/use-case/
    - modules/core/command/interface-adapter/
    - modules/core/query/use-case/
    - modules/core/query/interface-adapter/
    - modules/core/read-model-updater/
    - modules/core/infrastructure/
    - modules/app/aidlc/
    - modules/harness/claude/
    - modules/harness/infrastructure/
    - tests/
    - scripts/
    - tools/lint/
    - formal/
    - vendor/
    - .claude/
    - .codex/
    - .agents/
    - .takt/
    - aidlc/
```
