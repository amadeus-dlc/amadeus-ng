# コンポーネント一覧 — amadeus-ng

見出しはコンポーネント名そのもの（Cargo のパッケージ名、またはワークスペース外の単位名）。外部ライブラリとクレート間の依存図は `dependencies.md`、技術要素の版は `technology-stack.md` を参照。

健全度は Issue #134 の観点で付けた。**深く読んだのは `aidlc`・`core-command-use-case`・`core-command-interface-adapter`・`core-read-model-updater` の該当ファイルだけ**で、残りはディレクトリ単位の棚卸しにとどまる（範囲は `reverse-engineering-timestamp.md` の Scope of Analysis）。

| コンポーネント | 側 | 種別 | 健全度 | 分析の深さ |
| --- | --- | --- | --- | --- |
| core-command-domain | コマンド | ライブラリ | 健全（未検証） | 浅い |
| core-command-use-case | コマンド | ライブラリ | 健全 | 深い（1 ファイル） |
| core-command-interface-adapter | コマンド | ライブラリ | 要注意 | 深い（2 ファイル） |
| core-query-use-case | クエリ | ライブラリ | 健全（未検証） | 浅い |
| core-query-interface-adapter | クエリ | ライブラリ | 健全（未検証） | 浅い |
| core-read-model-updater | 中間（RMU） | ライブラリ | 要注意 | 深い（4 ファイル） |
| core-infrastructure | 共有層 | ライブラリ | 健全（未検証） | 浅い |
| aidlc | 合成ルート | バイナリ + ライブラリ | 要注意 | 深い（3 ファイル） |
| harness-claude | ハーネス | ライブラリ | 健全（未検証） | 浅い |
| harness-infrastructure | ハーネス | ライブラリ | 健全（未検証） | 浅い |
| tools/lint | 開発ツール | 独立クレート | 健全（未検証） | 浅い |
| .claude/tools | upstream 配布物 | TypeScript スクリプト群 | 対象外（比較の源） | 浅い |

## core-command-domain

- **パス**: `modules/core/command/domain/`
- **責務**: 書込モデルのドメイン。集約 `IntentExecution`（ワークフロー実行）、`Intent`、`WorkflowDefinition`、`CompiledDefinition`、それぞれのドメインイベント、pipeline 受領の判定（重複・順序違いを表す `PipelineLinkError`）。約 482 ファイル / 67k 行
- **内部依存**: `core-infrastructure`。serde や event-store-adapter-rs には依存しない（永続化中立）
- **Issue #134 との関係**: 重複した報告を `already completed` と判定する側。不具合の場所ではない

## core-command-use-case

- **パス**: `modules/core/command/use-case/`
- **責務**: 書込ユースケースと Repository trait。`RecordPipelineLinkUseCase` は受領を保存するだけで、表示用の値は返さない
- **内部依存**: `core-command-domain`
- **Issue #134 との関係**: 再試行は楽観ロックの `Conflict` を 1 回だけ（`record_pipeline_link_use_case.rs:37-42`）。投影の失敗は扱わない（呼び出し側の責務）

## core-command-interface-adapter

- **パス**: `modules/core/command/interface-adapter/`
- **責務**: Repository 実装（`IntentExecutionRepositoryImpl` / `IntentRepositoryImpl` / `WorkflowDefinitionRepositoryImpl` など）と永続化 DTO。本家 `EventStoreForSqlite` を内包し、`store` はジャーナル追記とスナップショット更新を行う。SQLite 失敗の写像 `store_failure.rs`
- **内部依存**: `core-command-use-case`、`core-command-domain`、`core-infrastructure`（外部では event-store-adapter-rs と rusqlite を使う）
- **Issue #134 との関係**: 重複した側 B の `store` が DEFERRED トランザクションの先頭で `UPDATE snapshot … WHERE version = ?` を行い、失敗するまでの短い間 RESERVED ロックを持つ（本家の挙動）。1 起動で同じファイルに本家接続を 3 本開く

## core-query-use-case

- **パス**: `modules/core/query/use-case/`
- **責務**: 読取ユースケース（`next` / `continue` の directive）と、`port/` に同居する DAO trait と View DTO。判断を持たず、DAO で引いた行を返すだけ
- **内部依存**: `core-infrastructure`（コマンド側・RMU・ドメインには依存しない）

## core-query-interface-adapter

- **パス**: `modules/core/query/interface-adapter/`
- **責務**: `read_*` 表を 1 表 1 引当で読む DAO 実装（`cargo lint` の `dao-single-table`）
- **内部依存**: `core-query-use-case`、`core-infrastructure`。dev-dependency でのみ RMU とドメインを引く

## core-read-model-updater

- **パス**: `modules/core/read-model-updater/`
- **責務**: RMU。ジャーナルを横断で読み（`JournalReaderImpl`）、純粋投影核でリードモデルを作り、`read_*` 表・状態ファイル・監査シャードへ公開する（`ReadModelUpdater`）。チェックポイントと公開計画も管理する。`JournalReader` ポートと SQLite 実装は RMU 自身が所有する
- **内部依存**: `core-command-domain`、`core-infrastructure`（外部では rusqlite を直接使う）
- **Issue #134 との関係**: **不具合の中心**。修正前は `replace_pipeline` だけが DEFERRED トランザクションで「読んでから書く」形だった（`journal_reader_impl.rs:1105`）。#154（`065ab78f`）で IMMEDIATE で始まるように直した。同じ型が `hook_health_reader.rs:181` と `workspace_doctor_read_model_updater.rs:183` にもある。所見の詳細は `code-quality-assessment.md`

## core-infrastructure

- **パス**: `modules/core/infrastructure/`
- **責務**: ドメインを持たない言語拡張（`canon_json`、codec、ハッシュ、排他ファイルロック、秘密鍵の鋳造、原子的なファイル書込）。DB アクセスや RPC は置かない
- **内部依存**: なし

## aidlc

- **パス**: `modules/app/aidlc/`
- **責務**: 合成ルート。マルチコール CLI の入口、`Request` の振り分け、動詞ごとの結線、`after_projection` による投影の共通失敗境界、利用者向け文言 `wording.rs`。コマンド側・クエリ側・RMU の全部を知ってよい唯一のプロダクトクレート
- **内部依存**: コア 7 クレートすべてと `harness-claude`
- **Issue #134 との関係**: 失敗文言を組み立て、記録済みでも終了コード 1 を返す場所（`runtime.rs:3226-3231`、`pipeline_link.rs:96`）

## harness-claude

- **パス**: `modules/harness/claude/`
- **責務**: Claude Code のフックとの結線（規則配送など）
- **内部依存**: `harness-infrastructure`、`core-infrastructure`、`core-command-domain`、`core-command-interface-adapter`

## harness-infrastructure

- **パス**: `modules/harness/infrastructure/`
- **責務**: ハーネス文脈の言語拡張
- **内部依存**: `core-infrastructure`

## tools/lint

- **パス**: `tools/lint/`
- **責務**: `cargo lint`。coding-rules の機械強制（`checkbox-vocabulary`、`no-public-fields`、`one-public-type`、`dao-single-table`、`port-naming`、`command-side-io` など）
- **内部依存**: なし（ワークスペース外の独立クレート）

## .claude/tools

- **パス**: `.claude/tools/`（70 ファイル）
- **責務**: upstream AI-DLC の TypeScript 配布物（Bun で実行）。本リポジトリが再実装している元の実装で、ゴールデン比較の源。pipeline link に相当する upstream の実装は `aidlc-log.ts` の link ハンドラ（1120〜1283 行）で、検査と監査追記を `withAuditLock` の中で済ませてから成功を返す。投影という後段は持たない
