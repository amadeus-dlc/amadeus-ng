# コード構成 — amadeus-ng

## リポジトリの配置

| パス | 分類 | 中身 |
| --- | --- | --- |
| `modules/core/command/{domain,use-case,interface-adapter}/` | プロダクト（コマンド側） | 集約・ドメインイベント、書込ユースケースと Repository trait、Repository 実装と永続化 DTO |
| `modules/core/query/{use-case,interface-adapter}/` | プロダクト（クエリ側） | 読取ユースケースと DAO trait・View、DAO 実装 |
| `modules/core/read-model-updater/` | プロダクト（RMU） | ジャーナル読取・投影・公開 |
| `modules/core/infrastructure/` | プロダクト（共有層） | ドメインを持たない言語拡張 |
| `modules/app/aidlc/` | プロダクト（合成ルート） | CLI の入口、動詞ごとの結線、利用者向け文言 `wording.rs` |
| `modules/harness/{claude,infrastructure}/` | プロダクト（ハーネス） | Claude Code フックとの結線 |
| `tests/` | テスト | `golden/upstream-a277af21/`（upstream 観測のゴールデン）、`conformance/`、共通支援 `support/` |
| `tools/lint/` | 開発ツール | `cargo lint`（ワークスペース外の独立クレート） |
| `scripts/` | 開発ツール | `coverage.sh`、ゴールデン採取（Bun）、ガバナンス補助、Python テスト |
| `formal/` | 形式モデル | Quint モデル |
| `vendor/aidlc-workflows/` | 外部（submodule） | upstream 配布元 |
| `.claude/` / `.codex/` / `.agents/` / `.takt/` | ハーネス設定 | upstream の TS 配布物（`.claude/tools/*.ts` 70 ファイル）、フック、スキル |
| `aidlc/` | ワークフロー記録 | memory・knowledge・codekb・intents |

クレート単位の責務は `component-inventory.md` に、クレート間の依存は `dependencies.md` にまとめた。

## クレート内部の構成パターン

- **コンテキスト別モジュール**: 各クレートの `src/` 直下は `orchestration/`・`workspace/`・`workflow_definition/` などの境界づけられたコンテキストで分かれる。例: `modules/core/read-model-updater/src/orchestration/`。
- **1 ファイル 1 公開型**: `cargo lint` の `one-public-type` で強制。そのためファイル数が多い（`core-command-domain` だけで約 482 ファイル / 67k 行）。
- **mod は private、公開はファサードの `pub use`**: 利用側のパスは `<クレート>::<コンテキスト>::<型>` で平坦になる（`module-visibility.md`）。
- **ポートの置き場**: コマンド側は use-case 層に `XxxRepository`、クエリ側は use-case 層の `port/` に `XxxDao` と View DTO が同居する。実装は interface-adapter 層の `XxxRepositoryImpl` / `XxxDaoImpl`。
- **永続化 DTO**: interface-adapter の `dto/` に `<対象><面>Dto` を 1 型 1 ファイルで置く。RMU は自前の DTO を持つ（`modules/core/read-model-updater/src/orchestration/dto/`）。
- **エラー**: モジュールごとの手実装 enum。`Display` は材料だけを描き、利用者向けの逐語文言は出す側の `wording` が組む（`modules/app/aidlc/src/wording.rs`、RMU の `workspace/wording.rs`）。
- **テストの置き場**: 単体テストはソース内の `#[cfg(test)]` または `*_tests.rs`、プロセスを跨ぐ契約テストは各クレートの `tests/`（例: `modules/app/aidlc/tests/pipeline_link_contract.rs`）。

## Issue #134 の経路にあるファイル

| ファイル | 役割 |
| --- | --- |
| `modules/app/aidlc/src/runtime.rs` | 動詞の振り分け（213〜240 行付近、`Request::LogLink` は 237 行）と、投影の共通境界 `catch_up` / `catch_up_with` / `after_projection` / `catch_up_before_reading`（3159〜3240 行） |
| `modules/app/aidlc/src/runtime/pipeline_link.rs` | `log link` の構文検査・handoff 観測・リポジトリ 3 本の open・ユースケース呼出（87〜95 行）・`after_projection`（96 行） |
| `modules/core/command/use-case/src/orchestration/record_pipeline_link_use_case.rs` | 受領の保存。`Conflict` のときだけ 1 回再試行（37〜42 行） |
| `modules/core/command/interface-adapter/src/orchestration/intent_execution_repository_impl.rs` | 本家 `EventStoreForSqlite` を包む Repository 実装（`open` / `store`） |
| `modules/core/command/interface-adapter/src/orchestration/store_failure.rs` | コマンド側の SQLite 失敗 → `ErrorKind` 写像（本家の `Box<dyn Error>` を開く `io_kind_of_source` 付き） |
| `modules/core/read-model-updater/src/orchestration/read_model_updater.rs` | `catch_up` の手順（191 行〜）と `catch_up_pipeline`（57〜65 行） |
| `modules/core/read-model-updater/src/orchestration/journal_reader_impl.rs` | 接続の開き方（busy timeout 5000ms、88 行・223 行）、全トランザクション、`replace_pipeline`（1100〜1127 行） |
| `modules/core/read-model-updater/src/orchestration/store_failure.rs` | RMU 側の同じ写像（`SQLITE_BUSY` / `SQLITE_LOCKED` → `WouldBlock`、46 行）。コマンド側との複製は CQRS 境界のための意図的なもの |
| `modules/core/read-model-updater/src/orchestration/catch_up_error.rs` | `CatchUpError` とその `Display`（`Read` は `read: {inner}`） |

## 大きなファイル

5,000 行級のファイルが複数ある（`intent_execution.rs` 9,753 行、`runtime.rs` 5,038 行など）。一覧と評価は `code-quality-assessment.md` を参照。
