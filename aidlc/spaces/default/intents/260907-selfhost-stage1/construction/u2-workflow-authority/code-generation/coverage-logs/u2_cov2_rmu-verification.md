# u2_cov2_rmu 検証報告（`core-read-model-updater` のカバレッジ向上・2 回目）

担当: u2_cov2_rmu / 所有 crate: `core-read-model-updater`（`modules/core/read-model-updater/`）。
共通 brief（[brief-common.md](brief-common.md)）・1 回目の報告（[u2_cov_rmu-verification.md](u2_cov_rmu-verification.md)）・2 回目の対象（[targets2-rmu.md](targets2-rmu.md)）に従い、**プロダクトコード（`src/`）は変更せずテストだけを追加**した。ログと補助スクリプトは [u2_cov2_rmu/](u2_cov2_rmu/) にある。計測は `CARGO_TARGET_DIR=target/u2_cov2_rmu` に隔離した（`cargo llvm-cov clean` もこの target にだけ掛けた）。

## 1. 計測の前提 — `coverage.sh` が読む数値は「関数ごとの行統計の合算」である

`scripts/coverage.sh` は `cargo llvm-cov --summary-only` の `totals.lines.percent` を読む。llvm-cov の summary は**関数（実体化）ごとに行の被覆を数えて合算**し、同じ定義位置を持つ実体化グループでは **covered 行数が最大の 1 実体化だけ**を採る（`FunctionCoverageSummary::get(InstantiationGroup)` の `max`）。この規則により、targets2 の「未カバー 343 行」は次の 2 種類を含む。

1. **クロージャ**（`map_err(|_| …)` / `ok_or_else(|| …)` 等）は別関数として数えられ、一度も呼ばれなければ、その行が親関数側では被覆されていても未カバー 1 行に数えられる。
2. **同じ関数の別ビルド・別実体化**（lib ビルド = app と integration test が共有 / unit test ビルド = `#[cfg(test)]` / ジェネリック関数の呼出 crate ごとの実体化）は**和集合ではなく最大値**で評価される。例: `IntentExecutionEventDto::of` は unit test が誕生〜レビュー系の変種を、integration test が報告・学び系の変種を別々に踏んでいたため、どちらの実体化でも 24 行が未カバーとして数えられていた。`ReadModelUpdater::catch_up` は app crate の実体化（228/250 行）が最大で、`FakeReader` 実体化（126 行）が踏む 18 行は数えられなかった。

したがって本担当では (a) 未到達だったクロージャ・分岐を実際に通す、(b) **1 つの実体化が和集合を覆う**ようにテストを寄せる、の 2 つを行った。親の workspace JSON（`step9-logs/s9b-coverage-head-per-file.json`）と自 crate 計測の突合ツールは [u2_cov2_rmu/fsum2.py](u2_cov2_rmu/fsum2.py)（summary の再現。親 JSON で 345 行 vs 実測 343 行）と [u2_cov2_rmu/estimate2.py](u2_cov2_rmu/estimate2.py)（app 由来の実体化と自 crate の新しい実体化を合わせた次回 workspace 計測の見積り）である。

注記: 親 JSON は `u2_dead_code` が `workspace/projection.rs` から 2 行を削る前の計測なので、同ファイルの 1251 行目以降は 2 行ずらして突合した（`estimate2.py` の `shift`）。

## 2. 着手前後の crate 行カバレッジ（crate 単独計測、`cargo llvm-cov -p core-read-model-updater --tests`、summary 基準）

| 計測 | 総行 | 未カバー | 行カバレッジ | ログ |
| --- | ---: | ---: | ---: | --- |
| 着手前 | 17,239 | 1,259 | 92.70% | [01-baseline-coverage.log](u2_cov2_rmu/01-baseline-coverage.log) |
| 着地後 | 17,239 | 694 | **95.97%** | [04-final-coverage.log](u2_cov2_rmu/04-final-coverage.log) |

（1 回目の報告の「34,478 行」は llvm summary ではなく別の行数基準だったため、本報告とは比較しない。）

## 3. 対象未カバー行の見積り（workspace summary 基準、`estimate2.py`）

targets2 の 343 行（親 JSON 再現で 345 行）に対し、次回 workspace 計測での本 crate の未カバーは **約 182 行**（約 47% 減）と見積もる（[05-ws-estimate.txt](u2_cov2_rmu/05-ws-estimate.txt)）。完了条件の 120 行には届かない。残りの内訳は §6 のとおりで、大半は到達不能なクロージャ・`#[cfg(test)]` 内のアサート失敗メッセージ行・単一実体化では踏めない I/O 失敗である。

| ファイル | 前 | 後（見積り） | 主な変化 |
| --- | ---: | ---: | --- |
| `orchestration/journal_reader_impl.rs` | 38 | 7 | セッション/成果物監査行の復号・負の通番・定義/実行の照合を integration test で網羅 |
| `orchestration/read_model_updater.rs` | 27 | 8 | `FakeReader` 実体化が 250 行中 247 行を覆い最大実体化になった |
| `orchestration/dto/intent_execution_event_dto.rs` | 24 | 0 | 全 31 変種を lib ビルドで往復 |
| `orchestration/dto/*`（19 ファイル） | 43 | 2 | 識別子欄の `map_err` クロージャを全変種で通す |
| `read_tables/sql.rs` | 22 | 9 | 列欠落表への INSERT 失敗（15 表 + steering 2 表）・古い断面の無視 |
| `orchestration/hook_health_reader.rs` | 33 | 23 | 表喪失・読取専用 DB・ビュー・列欠落・公開先の作成/書込失敗 |
| `orchestration/plan_approval_files.rs` | 10 | 3 | 権限剥奪による stat/列挙/削除/作成/読取/書込の失敗 |
| `workspace/projection.rs` | 56 | 46 | 計画外ステージの拒否・任意欄・team 面の学びを lib ビルドで通す（残りは §6） |

## 4. 品質ゲート

- `cargo test -p core-read-model-updater`: **641 件成功・0 失敗**（[03-tests.log](u2_cov2_rmu/03-tests.log)。着手前 594 件）。
- `cargo fmt -p core-read-model-updater -- --check` / `cargo clippy -p core-read-model-updater --all-targets -- -D warnings` / `cargo lint`: いずれも成功（[02-fmt-clippy-lint.log](u2_cov2_rmu/02-fmt-clippy-lint.log)）。
- 承認済み `code-generation-plan.md` / `unit-test-instructions.md` / `code-generation-questions.md`、`.claude/` 配下、`scripts/coverage.sh` は編集していない。デバッグのため `src/orchestration/store_failure.rs` に一時的な `eprintln!` を入れて SQLite の文言を確認したが、直後に元の内容へ戻した（差分なし）。

## 5. 追加したテスト（ファイル・件数・検証した契約）

| ファイル | 件数 | 検証した契約 |
| --- | ---: | --- |
| `tests/execution_event_dto_contract.rs`（既存へ追加） | 4 | `events()` に 16 変種（Started / Gate 3 種 / StageRevised / StageSkipped / Parked / Unparked / Recomposed / SingleStageRunCommitted / ReviewRequested / ReviewCompleted / PracticesAffirmed / SkeletonStanceRecorded / AutonomyModeSet / Jumped forward）を足し、全 31 変種の往復を lib ビルドで固定／全変種で `id` / `aggregate_id` の文法外を欄名付き `Malformed` で拒否（`PlanAnswerLogged` は `event_id`、`SingleStageRunStarted` だけは欄名を添えず `InvariantViolation` — 現行の綴りとして固定）／ステージ参照を運ぶ 13 変種の `stage` / `target` / 報告の確定位置の文法外／`human_before`・`approval_observation_id`・要約 SHA-256・レビュー指紋（要求・判定）・ソース基準・誕生の `intent_id` の破損を欄名で拒否／intent 誕生記録 10 欄（`definition_id`・`definition_revision`・`record_name.directory`・stage の number/display/slug・scan など）の破損 |
| `tests/journal_corruption_contract.rs`（新規） | 11 | セッション監査・成果物監査の行が自分の列へ振り分けられ `project_audit_only` で行になる／セッション行の破損 7 形（JSON 不正・aid 不一致・未知種別・発行側の項目名・改行入り値・重複項目・対象文法外）／成果物行の破損 5 形／5 ストリーム全部で負の通番を拒否／定義行の空白 id・系譜不一致・ドメインへ写せない payload・誕生/改訂の通番違反／実行行の id 文法外・未知の型判別子・誕生/非誕生の通番違反／正のチェックポイントにアンカーが無ければ `CheckpointAnchorMismatch`／版違いの作り直しで誕生の欠けた歴史に当たれば版を上げずに止める／読取表 15 表の列欠落（`id, as_of` だけ）で該当表の INSERT が失敗しチェックポイントも他表の行も動かない／steering 2 表の列欠落／古い履歴位置のテスト契約断面は新しい断面を上書きしない |
| `tests/hook_health_projection_contract.rs`（既存へ追加） | 4 | `journal` 表の無い共有 DB は読取表を作らず拒否／読取専用 DB では CREATE・DELETE・DROP の各段で失敗しチェックポイント・公開ファイルを残さない／列欠落表・主キー無しチェックポイント表・読取表名のビューに当たった投影は同じ Tx ごと巻き戻る／対象ディレクトリの位置にファイル・heartbeat ファイルの位置にディレクトリがあれば読取表を確定したうえで公開の失敗だけをパス付きで報告 |
| `tests/projection_rows_contract.rs`（新規） | 10 | 計画回答の行は選択で `QUESTION_ANSWERED` / `PLAN_APPROVAL_RECORDED` を名乗り状態面に触らない／pipeline link 行の Repo・single-stage Workflow 欄の有無／在席応答の HUMAN_TURN 行とセッション欄、無人応答は行なし／隔離実行開始の担当欄と計画外ステージの拒否／同期の状態欄更新と計画外ステージの拒否／決定行の Options・Rationale・summary file の有無／却下の Feedback が 2 行に載り改訂回数が進む／報告の承認で Validation Warning / Basis が完了行へ載る／team 向けの学びが `team.md` の見出し下へ、project 向けが `project.md` へ（見出しは足りなければ作る）、メモリ層が無ければ `MemoryFilesMissing`／計画外ステージのゲート開始は骨格の行欠落で止まる |
| `tests/read_model_updater_test.rs`（既存へ追加） | 10 | `CatchUpError` の診断文言 6 変種／誕生からの初回キャッチアップ（骨格の合成・初期化ステージ完了・依頼原文 JSON・ソース基準一覧の公開・既存の依頼原文は書き直さない・ゲート付きステージの無い計画）／状態ファイル・依頼原文の在否を確かめられない経路は `StateFileRead`／保存済みの未確定公開要求を新しいイベントより先に確定し確定済み位置を描き直さない／別の出力先へ束ねた要求は `PublicationConflict`、履歴が届かない要求は `PlanUnavailable`／成果物・セッション監査の行が記録のシャードへ積まれ別記録の観測は積まない／監査だけのバッチは計画なしで投影しチェックポイントを進める／シャードが読めなければ公開しない／初回の人間応答時刻は作成として公開／指示ファイルの作成と置換／ソース基準を運ぶ報告は一覧を公開（`FakeReader` に成果物・セッションの行を持たせた） |
| `tests/plan_approval_projection_contract.rs`（既存へ追加） | 3 | 公開先を OS が拒む 7 形（根を辿れない・列挙不能・古いファイルを消せない・ディレクトリを作れない・空ディレクトリを消せない・受領ファイルを読めない・書けない）をパスと `PermissionDenied` で報告しチェックポイントを進めない／中身の違う同名受領ファイルは履歴から書き直す／応答準備・失効準備・応答観測・失効解決の識別子/space/セッション/選択の文法外と集約 id の綴り違いを `UndecodablePayload` で拒否し、健全な 4 変種は受理 |
| `tests/publication_file_contract.rs`（既存へ追加） | 3 | 親ディレクトリを同期できないときは親パス付き `PublicationIo`／状態ファイルの親を作れないときは `Io`／監査シャードの在否を確かめられない・開けないときは失敗し、正常なら見出し付きで追記 |
| `tests/runtime_graph_projection_contract.rs`（既存へ追加） | 1 | 同じ発火に終端が複数あれば時刻の遅いほうが結果になる（台帳の順ではない） |
| `tests/workflow_continuation_projection_contract.rs`（既存のケース表へ追加） | 1 ケース | 負の通番の行は `InvalidData` |

合計 46 テスト関数 + 2 ケース。すべて入力を与え、結果（行・ファイル・エラーの欄名/原因/パス）を独立に書き下した期待値と比べる形で書いた。TDD の red は「プロダクトコードを変えない」前提で踏めないため、各テストは契約を先に書き、実行して green を確認し、`cargo llvm-cov` の未カバー行の消滅で「その行を実際に検証した」ことを確かめた。

## 6. 残った未カバー行と理由（見積り約 182 行）

| 分類 | 行数（概算） | 該当 |
| --- | ---: | --- |
| `#[cfg(test)]` 内のアサート失敗メッセージ行（成功時は実行されない） | 24 | `workspace/projection.rs:2879〜4469` の `"{}"` 引数行 22 行、`orchestration/steering_source.rs:237-239,349`、`dto/definition_dto_tests.rs:287,335` |
| 到達不能なクロージャ・分岐（dead code 候補 §7） | 約 60 | 下表 |
| I/O 失敗時のみの `?` / `map_err`（単一実体化では注入できないもの） | 約 40 | `hook_health_reader.rs:178,182,217`（pragma 読取・BEGIN・COMMIT の失敗）、`:259-261`（open 後の write 失敗）、`journal_reader_impl.rs:355,1007`、`read_model_updater.rs:355`（状態ファイルは確かめられるのに依頼原文だけ確かめられない形）、`:351-355`、`publication_store.rs:104,123,360`（UTF-8 でないパス）、`state_file_write_error.rs:47-50`（`is_writable` の失敗）、`audit_shard.rs:64,67`、`publication_file.rs:259` 相当、`sql.rs:1040`（query_map の失敗）ほか |
| 実体化の最大値規則で残るもの | 約 25 | `workspace/projection.rs` の `project_one` / `started` / `append_started_rows` など app の実体化と unit の実体化が別々の分岐を踏む関数（lib ビルドは integration test の分だけ増え、app のテストが加わると和集合になるので workspace 計測では見積りより減る可能性がある）、`read_model_updater.rs:286`（跳躍のソース基準。`FakeReader` の骨格では跳躍を描けない）、`:421` |

## 7. dead code 候補（到達不能と判断した行と根拠）

プロダクトコードは削除していない。1 回目の報告の候補（`execution_row.rs:59,61,63`、`workflow_continuation_read_model_updater.rs:202-204`、`hook_health_reader.rs:201-204,206-209`、`plan_approval_files.rs:66`、`jump_result_row.rs:43-44,48-51,118-119`）に加え、今回の作業で到達不能と判断したもの:

| ファイル:行 | 内容 | 根拠 |
| --- | --- | --- |
| `hook_health_reader.rs:76` | `query_map` の失敗 | 1 引数のプレースホルダに 1 値を束縛するだけで、prepare が通れば失敗しない |
| `hook_health_reader.rs:102` | グループ先頭が無い | `groups.entry(aid).or_default().push` した直後で空になれない |
| `hook_health_reader.rs:132,142,144` | `HookHealthId::parse(&aid)` / `HookDropSummary::new(0,None)`・`(1,Some)` / `HookHealth::new` の失敗 | aid は 103 行で payload の集約 id と一致確認済みで、payload 側は 271 行で先に文法検査される。`HookDropSummary` の 2 形はどちらも不変条件を満たす |
| `hook_health_reader.rs:222` | `path.parent()` が `None` | 開けたファイルパスに親が無いのは `/` だけで、SQLite ファイルにならない |
| `runtime_graph_read_model_updater.rs:135-140` | `canon_json::to_value(&graph)` の失敗 | 直列化対象は `Serialize` 派生の DTO だけで失敗経路が無い |
| `runtime_graph_read_model_updater.rs:164` | 実行の `Started` より前に別イベントを見る腕 | `find_map` は最初の一致で止まり、復号は誕生を通番 1 に固定するので `_ => None` は評価されない |
| `runtime_graph_read_model_updater.rs:240,288,328,355` | `blocks.get(position)` が `None` | `position` は同じ `blocks` から採った索引 |
| `workflow_continuation_read_model_updater.rs:185,189` | `ContinuationGuard::new(None,0,false)` / `before.after(request)` の失敗 | 前者は既定の guard で不変条件を満たし、後者は 1 件目の要求から導く選択値で失敗形を作れない（回数不一致は 205-206 行で別途拒否済み） |
| `workflow_continuation_read_model_updater.rs:236,238` | `i64::try_from(usize)` の失敗 | 64bit 環境では `usize > i64::MAX` を SQLite の INTEGER から作れない |
| `sql.rs:551-552,1059,1083` | `usize`/`u64` → `i64` の失敗 | 同上 |
| `sql.rs:1027` | `canon_json::to_value` の失敗 | `Value` の列は常に直列化できる |
| `publication_store.rs:312` | 世代の `checked_add` 溢れ | u64 の世代番号が溢れる履歴は作れない |
| `plan_approval_journal_reader_impl.rs:81,86,102,135` | `checked_add` 溢れ・`usize` 変換・`i64` 変換・`path.parent()` | 上と同じ理由 |
| `plan_approval_files.rs:39,41` | `read_dir` の各 entry / その `symlink_metadata` の失敗 | 列挙が始まった後に個々の entry だけが失敗する状況を決定的に作れない |
| `projection.rs:1850-1855` | `append_under_heading` の `HeadingMissing` | 直前の `ensure_heading` が見出しを必ず作る |
| `projection.rs:1246,1312,1328-1329,2109,2170,2184` | `filter(|_| direction == Backward)` の閉包の残り腕、`then(|| PhaseBoundary::new)` 等の別実体化 | 同じ閉包が lib / unit の別ビルドに 2 実体化され、最大値規則で片方の未実行が残る（両ビルドの和集合では被覆済み） |
| `intent_registry.rs:88` | 既存行がオブジェクトでない | `uuid` 欄で一致した行はオブジェクトでしかありえない |
| `catch_up_error.rs:127` / `steering_source` の `SteeringPack` | `UnsplittableSection` の表示 | 構築経路が `pub(crate)` で、1 コードポイントが輸送目標を超えるセクションは作れない |

## 8. 気づき（人間の裁定を要するもの。コードは変えていない）

1. **同じ監査シャードへの追記計画が 1 バッチに 2 つ入ると 2 つ目が競合として拒否される。** `ReadModelUpdater::catch_up` は実行イベントの行・成果物監査の行・セッション監査の行をそれぞれ `PublicationFile::audit(shard, …)` として同じ `files` へ積む（`read_model_updater.rs:416-420,457-461,481-485`）。各計画は作成時点のシャード内容を `before` として固定するので、1 つ目の適用でシャードが伸びると 2 つ目の `after.strip_prefix(現在内容)` が失敗し `PublicationConflict` になる（`publication_file.rs:190-194`）。`FakeReader` で「実行イベント + 成果物行 + セッション行」を 1 窓に流すと実際に `PublicationConflict { path: <audit shard> }` で止まった（本報告の作業中に観測。テストは窓を分けて書いた）。実運用でフックの成果物/セッション行と実行イベントが同じキャッチアップ窓に入る場合の期待動作は上流仕様の裁定事項として提示する。
2. `SingleStageRunStartedDto::to_domain` と intent 面の `SourceBaselineDto` だけが欄名を添えず `InvariantViolation` で拒否する（他の変種は `Malformed{field}`）。契約テストは現行の綴りをそのまま固定した。揃えるかどうかは裁定事項。
3. `JournalReaderImpl::scan_range` は `manifest` を選ばず全行を読むので、同じ `journal` 表に `hook-health-event/1` や `plan-approval-event/1` の行が同居すると `decode_entry` の名乗り検査で `Corrupt` になる。現状の試験装置はストアファイルを分けているため問題は出ていないが、共有 DB 前提の投影（HookHealth / PlanApproval）と同居させる設計かどうかは確認事項。
