# API — amadeus-ng

HTTP / gRPC の外部 API は無い。外から見える面は **CLI とその出力契約**、**永続化面（SQLite ファイルと公開ファイル）**の 2 つで、内部には Rust trait のポートがある。

## 1. CLI（マルチコールバイナリ）

- 入口: `modules/app/aidlc/src/runtime.rs` の `run(argv0, args, cwd)`（192 行）。`argv0` のファイル名で面（`Face`、`modules/app/aidlc/src/cli/face.rs`）を決め、`modules/app/aidlc/src/cli/request.rs` の `parse` が `Request` 列挙（39 変種）へ振り分ける。
- 面の例: `aidlc-orchestrate`（素の `aidlc` も同じ）、`aidlc-utility`、`aidlc-log`、`aidlc-state`、`aidlc-bolt`、`aidlc-jump`、`aidlc-learnings`、`aidlc-review-brief`。`aidlc engine <noun> <verb>` の形でも起動できる。
- 動詞は 2 種類に分かれる。
  - **読取動詞**（`next` / `continue` など）: 読む前に `catch_up_before_reading` で投影を終え、クエリ側で `read_*` 表を読む。
  - **更新動詞**（`report` / `park` / `log review|decision|answer|link` / `jump` / `intent-create` など）: コマンド側で記録した後、`after_projection` を通ってから結果を返す。

### 出力契約 `Completion`

`runtime.rs` の `Completion`（124〜190 行付近）が stdout・stderr・終了コードの組を 1 つにまとめる。

| 形 | stdout | stderr | 終了コード |
| --- | --- | --- | --- |
| `emitted` | 1 行 JSON | — | 0 |
| ビジネス拒否（`error` directive） | directive | — | 0 |
| `refused`（自己防衛の拒否、失敗全般） | — | 診断 1 行 | 1 |
| `reported` | 本文 | — | 指定値（0 / 1 / 3 など） |

### `aidlc-log link`（Issue #134 の入口）

- 振り分け: `Request::LogLink(args)` → `pipeline_link::run`（`runtime.rs:237`）
- 引数: 必須の `--stage` / `--link`、任意の `--repo` / `--single` / `--artifact`（handoff ファイルのパス）。`--intent` / `--space` は拒否する。handoff の期待位置は `<record>/inception/reverse-engineering/developer-scan.md`（`--repo` 指定時は `developer-scan-<repo>.md`）で、実在と内容は `observe` で観測して集約の判定材料にする
- 成功: stdout に `{"emitted":"PIPELINE_LINK_COMPLETED","stage":…,"link":…}`（`repo` と `single` は指定時だけ付く）、終了コード 0。監査シャードに `PIPELINE_LINK_COMPLETED` が 1 件追記される。
- 業務上の拒否（順序違い・重複 `already completed` など）: `PipelineLinkCommandError::Rejected` を `pipeline_link.rs` の `wording` で文言にし、`refused`（終了コード 1）で返す。
- 投影の失敗: 記録が済んでいても `after_projection` が `Completion::refused(wording::orchestrate_failure(cause))` を返す。stderr の例は `aidlc-orchestrate: projection: read: io: WouldBlock at …/.aidlc-store.sqlite`。
- この契約を固定しているテストは `modules/app/aidlc/tests/pipeline_link_contract.rs`。どのテストが何を固定しているかは `code-quality-assessment.md` を参照。

## 2. 永続化面

| 面 | 場所 | 書き手 | 読み手 |
| --- | --- | --- | --- |
| `journal` / `snapshot` 表 | `aidlc/spaces/<space>/intents/.aidlc-store.sqlite` | 本家 `EventStoreForSqlite`（コマンド側 Repository 実装が包む） | Repository の `find_by_id`、RMU の `JournalReaderImpl` |
| `amadeus_projection_checkpoint` / `amadeus_publication*` / `amadeus_read_model_head` | 同上 | RMU | RMU |
| `read_*` 表（例: `read_pipeline_progress`） | 同上 | RMU | クエリ側 DAO |
| `aidlc-state.md` | intent 記録ディレクトリ | RMU の公開 | 人間・upstream ツール・フック |
| 監査シャード `audit/<host>-<clone>.md` | intent 記録ディレクトリ | RMU の公開 | 人間・upstream ツール |

ジャーナルモードの設定はリポジトリ内に無く、SQLite 既定のロールバックジャーナルで動く。

## 3. 内部ポート（Rust trait）

コマンド側 use-case・クエリ側 use-case・RMU に `pub trait` が 55 個ある。命名は `cargo lint` の `port-naming` で、use-case 層はコマンド側 `XxxRepository` とクエリ側 `XxxDao` に限られる。例外は ES 基盤ポート `EventStore` / `JournalReader` の 2 本だけ。Issue #134 に関わるものは次のとおり。

| ポート | 所有クレート | 主なメソッド | 契約上の失敗 |
| --- | --- | --- | --- |
| `IntentExecutionRepository` | `core-command-use-case` | `find_by_id`、`store(event, aggregate)` | `RepositoryError<Id>`（`Conflict` は楽観 version の不一致、`Io { kind, path }`、`Corrupt`） |
| `IntentRepository` / `WorkflowDefinitionRepository` | `core-command-use-case` | `find_by_id`、`store` | 同上 |
| `JournalReader` | `core-read-model-updater` | `prepare_read_model`、`pending_publication`、`events_through`、`events_after`、`publish`、`checkpoint`、`advance_checkpoint`（`replace_steering` / `replace_testing` / `replace_plan_fingerprint` / `replace_code_generation_approval` は Issue #153 の PR2、`replace_pipeline` は PR3 で表の DAO へ移り、この trait から消えた） | `JournalReadError`（`Io { kind, path }` ほか）。`catch_up` では `CatchUpError::Read` に包まれる |

`ReadModelUpdater::catch_up(&mut self) -> Result<GlobalSeqNr, CatchUpError>` が RMU の公開面である。`CatchUpError` の変種は `Read` / `Projection` / `StateFileRead` / `StateFileWrite` / `PublicationIo` / `PublicationConflict` / `ReadTables` / `SteeringRead` / `SteeringPack` ほか。
