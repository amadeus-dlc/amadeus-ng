# u2_cov_rmu 検証報告（`core-read-model-updater` のカバレッジ向上）

担当: u2_cov_rmu / 所有 crate: `core-read-model-updater`（`modules/core/read-model-updater/`）。
共通 brief（[brief-common.md](brief-common.md)）・対象一覧（[targets-rmu.md](targets-rmu.md)）に従い、**プロダクトコードは変更せずテストだけを追加**した。ログは [u2_cov_rmu/](u2_cov_rmu/) にある。

## 計測条件

- 他担当と `target/` を共有すると試験バイナリが消される・他 crate の古い `.profraw` が混ざる事故が起きたため（`01-baseline-coverage.log` 冒頭）、`CARGO_TARGET_DIR` を隔離した一時ディレクトリに向けて計測した。閾値・除外・seed（`scripts/coverage.sh`）は変更していない。
- 「対象ファイルの未カバー行」は親の per-file JSON（`step9-logs/s9-coverage-head-per-file.json`、workspace 計測）から llvm の行判定規則（wrapped segment + region entry の最大 count）で導いた行集合を使い、担当 crate 単独の計測 JSON と突合した（`u2_cov_rmu/remaining.py` / `uncov_lib.py`）。この導出では担当対象の未カバー行は **746 行**（brief の 988 行は llvm summary の行数で、マクロ展開行などを含むため差がある）。

## 着手前後の crate 行カバレッジ（crate 単独計測、`cargo llvm-cov -p core-read-model-updater --tests`）

| 計測 | 行カバレッジ | ログ |
| --- | ---: | --- |
| 着手前 | 56.72%（34,478 行中 14,922 行未カバー） | [01-baseline-coverage.log](u2_cov_rmu/01-baseline-coverage.log) |
| 着地後 | 68.58%（34,786 行中 10,929 行未カバー） | [04-final-coverage.log](u2_cov_rmu/04-final-coverage.log) |

主要対象ファイル（crate 単独計測の行カバレッジ）:

| ファイル | 前 | 後 |
| --- | ---: | ---: |
| `orchestration/hook_health_reader.rs` | 52.24% | 86.86% |
| `orchestration/runtime_graph_read_model_updater.rs` | 30.28% | 97.23% |
| `orchestration/plan_approval_journal_reader_impl.rs` | 0.00% | 96.99% |
| `orchestration/plan_approval_files.rs` | 0.00% | 85.14% |
| `orchestration/plan_source.rs` | 0.00% | 100.00% |
| `orchestration/workflow_continuation_read_model_updater.rs` | 0.00% | 91.18% |
| `orchestration/intent_registry.rs` | 11.01% | 83.49% |
| `orchestration/dto/learnings_captured_dto.rs` | 0.00% | 100.00% |
| `orchestration/dto/memory_journals_observed_dto.rs` | 0.00% | 100.00% |
| `orchestration/dto/active_directive_dto.rs` | 0.00% | 98.72% |
| `read_tables/plan_approval_tables.rs` | 60.93% | 87.91% |
| `read_tables/plan_fingerprint_row.rs` | 0.00% | 87.65% |
| `workspace/projection.rs` | 90.91% | 94.73% |

## 対象未カバー行の削減（親 JSON との突合）

- 着手前 746 行 → 着地後 **126 行**（**83.1% 減**、`04-final-coverage.log` 末尾の `remaining.py` 出力）。完了条件 1 の 85% には約 14 行届いていない。残りの内訳と理由は後述のとおりで、大半は `?` の Err 分岐（I/O 失敗時のみ）と `#[cfg(test)]` 内のアサート失敗メッセージ行である。
- workspace 全体の相対ゲート判定は親が最後に `scripts/coverage.sh --base main` で行う。

## 品質ゲート

- `cargo test -p core-read-model-updater`: 594 件成功・0 失敗（[03-tests.log](u2_cov_rmu/03-tests.log)）。
- `cargo fmt -p core-read-model-updater -- --check` / `cargo clippy -p core-read-model-updater --all-targets -- -D warnings` / `cargo lint`: いずれも成功（[02-fmt-clippy-lint.log](u2_cov_rmu/02-fmt-clippy-lint.log)）。新規テストファイルには既存ファイルと同じ file-level `allow`（unwrap / expect / panic / indexing、ワイヤ形式を素の serde で書く `disallowed_methods`）を理由付きで置いた。

## 追加したテスト（ファイル・件数・検証した契約）

| ファイル | 件数 | 検証した契約 |
| --- | ---: | --- |
| `tests/hook_health_projection_contract.rs`（既存へ追加） | 7 | 初回 drop で始まる履歴の投影（heartbeat なし・drop 履歴の追記と再投影の冪等・利用者が切り詰めた分の補完）／heartbeat NOT NULL の古い読取表を作り直す／存在しない共有 DB を作らずに拒否／壊れた行 18 形（JSON 不正・ID 文法外・未知 kind・target/hook/reason 欠落や文法外・先頭が heartbeat・通番 1 始まりでない・負の通番）を `CorruptCause` 付きで拒否し読取表を作らない／aid 列と payload の集約 ID 不一致の拒否／列型不一致行の拒否／drop 履歴へ追記できない場合の拒否（読取表は確定済み） |
| `tests/runtime_graph_projection_contract.rs`（新規） | 9 | 監査台帳の対・センサー発火の対（passed / failed+Detail path / budget-override / 孤児の incomplete 判定と 60 秒閾値）・学びの件数（Source 別、窓外は数えない）・日誌観測・Agent 欄欠落時の計画担当・計画外/文法外 slug の除外を **固定本家形の JSON 逐語**で検証／状態ファイルの Scope が台帳より優先・読めない時刻を基準にしたとき孤児にしない／シャードの名前順連結と CRLF 正規化・`.md` 以外の無視／`WORKFLOW_STARTED` のない台帳・監査ディレクトリ不在では何も書かない／誕生記録のない実行は `PlanUnavailable`／UTF-8 でないシャードは `PublicationIo(InvalidData)`／出力先を置換できないときはパス付き `PublicationIo`／`journal` 表のないストアは開けない |
| `tests/execution_event_dto_contract.rs`（新規） | 4 | 読む側 DTO の `of` → 直列化 → `to_domain` の往復が 25 変種（Reported の全遷移・NoOp、Jumped の scope/observation、Directive 3 種、AnswerRecorded 3 内訳、PlanAnswerLogged、LearningsCaptured 3 内訳など）で不変／閉集合語彙の綴り固定／壊れた行 32 形（内訳・向き・操作列・選択の未知綴り、各 ID の文法外、根拠と警告の同居、証跡と要約ファイルの同居、遷移と操作列の不整合、指紋・unit の不正）を `DtoDecodeError` の欄名付きで拒否／intent 誕生記録の記録名・ソース基準の往復と greenfield 調整の不変条件 |
| `tests/plan_approval_projection_contract.rs`（新規） | 9 | 空ストア → 空の表・checkpoint 0・ディレクトリ不作成／`kind` 列のない旧読取表の移行／挑戦+応答+回答（承認・変更要求・中断）+世代（依頼・認証・失効）を操作行・回答行（`PLAN_APPROVAL_RECORDED` / `QUESTION_ANSWERED` / 中断文言逐語）・世代行・受領ファイルへ投影し、古いファイルを掃除し、再投影で同内容を書き直さない／公開物が無くなればディレクトリを除く／`.aidlc-sessions` がファイル・配下にサブディレクトリがあるときは `InvalidData` で拒否／壊れた行 14 形（通番・manifest・JSON・集約 ID・イベント ID・誕生でない先頭・挑戦 ID 不一致・別発行回の応答・応答要約の不正・回答状態と誤りの不整合・世代/受領状態の未知綴り）を拒否し checkpoint を進めない／checkpoint がジャーナルより先・アンカー不一致は `CheckpointAnchorMismatch`、同一アンカーは進む／`PlanApprovalTables::project` の通番検査 3 形 |
| `tests/plan_source_contract.rs`（新規） | 9 | unit 対象の文書ディレクトリ解決・欠落文書は空・質問票のプロジェクト相対名・状態ファイル SHA-256／状態ファイル不在は `None`／在るのに読めない文書・状態ファイル・memory 層はパス付き `SteeringRead`／記録がプロジェクト外なら `InvalidData`／規則ファイルのワークスペース相対名／閉じない HTML コメントの扱い／記録配置でない `ProjectionTargets` は project_dir を持たない |
| `tests/workflow_continuation_projection_contract.rs`（新規） | 3 | 停止要求の投影（読取表の行・`block-count.json` の逐語・checkpoint）／空ストリームは何も書かない／壊れた行 7 形（JSON 不正・別集約の混入・通番 1 始まりでない・要求 ID 文法外・guard の不変条件違反・回数不一致・wait の未知綴り）を `InvalidData` で拒否し行を作らない |
| `tests/read_model_updater_test.rs`（既存へ追加） | 9 | 人間応答ファイル・指示ファイル・登録簿が読めないときのパス付き `PublicationIo`／人間応答時刻の置換公開／記録ディレクトリがファイルのとき `StateFileRead`／pipeline 面を持たない読み手は `Unsupported`／公開名を持つ intent の登録と既存行の保持／dirName 不一致・別 intent が同じ dirName を持つときの `PublicationConflict`／登録簿が JSON でないときの `InvalidData` |
| `tests/read_tables_test.rs`（既存へ追加） | 3 | 計画指紋行が実行・intent の不在を名指す／空の計画は指紋でなく拒否理由を行に載せる／セッション監査が誕生から始まらなければ `MissingGenesis`、健全なら観測ごとの行 |
| `src/workspace/projection.rs`（`#[cfg(test)]` へ追加） | 8 | pipeline link 行の Repo / Workflow 欄／session 無しの在席応答行と無人応答の無視／決定行の Options / Rationale／Validation Warning を完了行へ／観測付き後方跳躍の 3 一覧欄／未知の checkbox 動作は計画へ戻す／欄を 1 行欠いた状態ファイルは欄名で拒否（9 欄 × イベント）／初期化以外が SKIP の計画は次の位置へ入らない |
| `src/read_tables/spelling.rs`（`#[cfg(test)]` へ追加） | 1 | レビュー・pipeline・計画応答の拒否理由 8 変種の綴り固定 |

合計 62 件。すべて「その行が実行される振る舞い」を、入力を与え・結果（行・ファイル・エラーの欄名/原因）を独立に書き下した期待値と比べる形で書いた。

## dead code 候補（到達不能と判断した行と根拠）

プロダクトコードは削除していない。以下は正常系・入力操作では到達できず、削除ではなく設計上の到達不能として報告する。

| ファイル:行 | 内容 | 根拠 |
| --- | --- | --- |
| `read_tables/execution_row.rs:59,61,63` | `ContinuationWait::Question / Conversation / Resume` の綴り分岐 | `IntentExecution::continuation_wait()` は `GateOrRevision` / `Decision` しか返さない（`intent_execution.rs:368-380`）。網羅 `match` の腕なので削除はできない |
| `orchestration/workflow_continuation_read_model_updater.rs:202-204` | 先頭事実からの `WorkflowContinuation::new` 失敗 | seq 1 では `before.after(request)` から選択値を導くため `new` の不変条件（`workflow_continuation.rs:35-46`）を破る入力を DTO から作れない（`count` 不一致は 205-206 で別途拒否、テスト済み） |
| `orchestration/hook_health_reader.rs:201-204,206-209` | `i64::try_from(usize)` 失敗 | 64bit 環境で `usize > i64::MAX` の通番は SQLite の INTEGER から生成不能 |
| `orchestration/plan_approval_files.rs:66` | 受領ファイル名がパス 1 要素でない | `PlanApprovalFile` の名前は受領キー（SHA-256 由来）から内部で組まれ、外部から与えられない |
| `read_tables/jump_result_row.rs:43-44,48-51,118-119` | 誕生のない実行・解決できない跳躍先 | `ReadTables::project` は先に `replay_executions` が同じ `MissingGenesis` で止めるため、この関数の分岐には届かない（既存テスト `a_stream_that_does_not_start_at_its_genesis_is_refused` が前段で拒否） |
| `workspace/projection.rs:1246` | 後方跳躍の観測が `None` の腕（`map_or_else` の既定） | 直前で `observation().is_some()` を検査済み |

## 残った未カバー行と理由（126 行）

| 分類 | 行数 | 該当 |
| --- | ---: | --- |
| `#[cfg(test)]` 内のアサート失敗メッセージ行（成功時は実行されない） | 22 | `workspace/projection.rs:2881〜4002` の `"{}"` 引数行、`orchestration/steering_source.rs:237-239,349` |
| `?` / `map_err` の Err 分岐（SQLite・ファイル I/O の失敗時のみ） | 約 55 | `read_tables/sql.rs`（13 行、全て `)?;`）、`orchestration/journal_reader_impl.rs`（9）、`orchestration/read_model_updater.rs:352-355,461,485,540`、`orchestration/runtime_graph_read_model_updater.rs:136-140,164,240,288,328,355`、`workspace/projection.rs:757,848-849,853,1554,1797,1824,1853-1857,2035`、`orchestration/hook_health_reader.rs:186-187,260-261`、`workspace/state_file_write_error.rs:47-50,57`、`orchestration/publication_file.rs:200,259`、`orchestration/plan_approval_files.rs:34,72` |
| 上記 dead code 候補 | 21 | 表のとおり |
| 他の担当対象との合わせ技が要るもの | 約 28 | `orchestration/read_model_updater.rs:434`（別 target の成果物監査。フェイク読み手が成果物行を持たない）、`orchestration/session_event_dto.rs:24,30,35`（セッション行の復号失敗。`JournalReaderImpl` のセッション表経由）、`read_tables/session_audit_row.rs:55-56`、`read_tables/artifact_audit_row.rs:48-49`、`orchestration/intent_registry.rs:37`、`orchestration/projection_targets.rs:56,204,208` ほか 1 行ずつの端 |

## 補足

- 本担当の作業中、`tests/journal_reader_impl_test.rs::publication_survives_process_termination` が共有 `target/` 上で 1 度失敗した（子プロセス起動時に試験バイナリが `NotFound`）。隔離 target では再現せず、他担当のビルドによる試験バイナリ差し替えが原因である。テスト自体は変更していない。
- 承認済み `code-generation-plan.md` / `unit-test-instructions.md` / `code-generation-questions.md`、`.claude/` 配下、`scripts/coverage.sh` は編集していない。
