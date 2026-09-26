# RMU の DAO 移行計画（Issue #153、2026-09-26）

リードモデル更新器を「ジャーナルを読む → 投影 → 表の DAO で更新する」形へ移す計画である。規則の正本は
[read-model-updater-structure.md](aidlc-shared/coding-rules/read-model-updater-structure.md)（オーナー裁定 2026-09-26）。
この文書は規則ではなく、順序と残作業の記録である。

## 現状（PR4 の後）

- **移行済み（PR1）**: 自己診断（`WorkspaceDoctorReadModelUpdater`）。表の DAO 3 本（`DoctorReportDao` /
  `DoctorCheckDao` / `WorkspaceDoctorProjectionCheckpointDao`）と、ジャーナルの読み手 `WorkspaceDoctorJournalReader`。
  処理したシーケンス番号（ジャーナル上の位置）より後だけを読み、2 表と番号を 1 つの IMMEDIATE トランザクションで確定する。
- **移行済み（PR2）**: 参照入力由来の単独面。表の DAO 5 本（`SteeringPlanDao` / `SteeringPartDao` /
  `TestingContractDao` / `PlanFingerprintDao` / `CodeGenerationApprovalDao`、各表の DDL は各 DAO が持つ）と、
  DAO が運ぶ行（`SteeringPlanRow` / `SteeringPartRow` / `TestingContractRow` / `PlanFingerprintRow` /
  `CodeGenerationApprovalRow`）と読む出所 `SourceStamp`（`source_digest` 列 + `as_of` 列）を `orchestration/port/` に
  置いた。行は完全コンストラクタ `new` だけを持つ値であり、材料から行を組む投影は `read_tables` 側の投影単位の
  結果型が持つ（`SteeringTables::pack` / `TestingTables::project` / `PlanFingerprintTables::project` /
  `CodeGenerationApprovalTables::project`）。
  - steering は新しい更新器 `SteeringReadModelUpdater` が描く。取得ループ（`OrchestrationReadModelUpdater`）は
    steering の更新器を型引数で持ち、ジャーナル差分の探りより前に起動するだけになった。2 表は 1 つの IMMEDIATE
    トランザクションで差し替える。
  - `TestingReadModelUpdater` / `PlanFingerprintReadModelUpdater` / `CodeGenerationApprovalReadModelUpdater` は
    接続を 1 本所有し（`open`）、表の DAO へトランザクションを渡す。履歴の読取は借りた `JournalReader`
    （`events_after`）に残る。`prepare_read_model` は PR4 で取り除いた（下記）。
  - 冪等は番号ではなく行の `source_digest` で取る（今までどおり）。「同じ照合子か、保存済みの行のほうが新しい
    履歴位置（`as_of`）なら書かない」判断は更新器が持つ。steering とテスト契約は、今までどおり書込ロックを取らずに
    比べてから、動いていれば IMMEDIATE で開いて比べ直す（計画指紋と開始可否は今までどおり IMMEDIATE の中で比べる）。
  - `JournalReader` から `steering_source_digest` / `replace_steering` / `testing_source_digest` / `replace_testing` /
    `replace_plan_fingerprint` / `replace_code_generation_approval` を消した（互換の口は残していない）。
  - **暫定（PR4 で解消）**: 表の用意（DDL・版による DROP）をどこが持つかは PR4 で決める — それまで
    `JournalReaderImpl` が各 DAO の `create_table` を呼ぶ暫定。開く段で 5 表を作るのは `JournalReaderImpl::open` が各表の DAO の
    `create_table` を呼んで行い（クエリ側は面ごとの更新器がまだ走っていないストアでも表を引くため）、読み面の版
    （`PRAGMA user_version`）が動いたときに 5 表を落とす処理は `read_tables` の `DROP` に残してある。
  - **開く段の書込ロック**: 更新器の `open` と `JournalReaderImpl` の 5 表の用意は、各 DAO の `table_exists`
    （`sqlite_master` の読取）で表が揃っているかを書込ロックを取らずに見て、揃っていれば書込トランザクションを
    開かない。欠けているときだけ `BEGIN IMMEDIATE` で開いて `create_table` を呼ぶ（DEFERRED で開くとスキーマを
    読んでから書込へ昇格するので #134 の即時 `SQLITE_BUSY` になりうる）。これで、表が揃っていて参照入力も
    動いていない更新は、この 5 表について書込ロックを取らない。ただし `JournalReaderImpl::open` 全体としては、
    PR2 の範囲外の既存の書込（`shared_projection::initialize` の `INSERT OR IGNORE` など）が今も書込ロックを
    取る。これも PR4 の「表の用意をどこが持つか」と一緒に扱う（PR4 で解消 — 下記）。
- **移行済み（PR3）**: Pipeline 面。表の DAO `PipelineProgressDao`（`read_pipeline_progress` の DDL・`table_exists`・
  実行ごとの出所の読取 `find_stamp`・実行ごとの差し替え `replace_for_execution`）と、DAO が運ぶ行
  `PipelineProgressRow`（完全コンストラクタ `new` だけの値。代理主キーの導出を含め、行を組む投影は
  `PipelineTables::project`）を `orchestration/port/` に置いた。出所は PR2 の `SourceStamp` を使う
  （`source_digest` 列 + `event_position` 列）。
  - 新しい更新器 `PipelineProgressReadModelUpdater` が描く。取得ループ（`OrchestrationReadModelUpdater`）の読み手を
    借りて全履歴を読み、handoff の観測と合わせて投影し、接続を 1 本所有して `BEGIN IMMEDIATE` の中で
    「保存済みの出所を読む → 同じ照合子か、保存済みの位置のほうが新しければ書かない → 実行の行を差し替える」を行う
    （#134 の IMMEDIATE はそのまま更新器へ移した。判定は更新器が持ち、DAO は 1 表の I/O だけ）。
  - 取得ループは、描く先（共有ストアのパス — `with_pipeline_handoff(store, handoff)`）と現在の handoff を持ち、
    steering の更新の後・ジャーナル差分の探りの前に Pipeline の更新器を組んで起動するだけになった（起動の時点は
    今までと同じ）。取得ループの読み手を借りるので、steering と違って型引数では持たない。
  - `JournalReader` から `replace_pipeline`（既定実装 `Err(Unsupported)` ごと）と `JournalReaderImpl` の実装を消した
    （互換の口は残していない）。#134 の回帰試験は `tests/reference_surface_updater_contract.rs` の
    `the_pipeline_update_waits_for_a_write_lock_held_by_another_connection` へ移し、更新器の書込トランザクションを
    DEFERRED に戻すと `WouldBlock` で落ちることを確かめた。
  - 表の用意は PR2 と同じ暫定に載せた — `read_tables` の共通 DDL から `read_pipeline_progress` を外し、
    `JournalReaderImpl::ensure_reference_tables`（開く段、6 表）と更新器の `open` が DAO の `table_exists` /
    `create_table` を呼ぶ。版による `DROP` は `read_tables` に残した（PR4 で決める）。
  - **小さな挙動の差**: 保存済みの `event_position` が負（手で壊した行）のとき、以前は「新しくない」と見なして
    上書きしていたが、今は PR2 の DAO と同じく `Corrupt` を返す。書く側は負の値を書かない（`u64` の位置を
    `i64` に収まらなければ `Corrupt` で止める）ので、壊れた行でしか起きない。
- **移行済み（PR4）**: 構造化面（ジャーナル由来の `read_*` **20 表** — 計画の「17 表」は古い数え方）。
  - **表 → DAO**（trait は `orchestration/port/`、実装は `orchestration/*_dao_impl.rs`、各表の DDL と索引は
    各 DAO が持つ）: `read_session_audit` → `SessionAuditDao` / `read_artifact_audit` → `ArtifactAuditDao` /
    `read_answer_result` → `AnswerResultDao` / `read_report_result` → `ReportResultDao` /
    `read_jump_result` → `JumpResultDao` / `read_definition` → `DefinitionDao` /
    `read_definition_stage` → `DefinitionStageDao` / `read_definition_scope` → `DefinitionScopeDao` /
    `read_definition_scope_keyword` → `DefinitionScopeKeywordDao` /
    `read_definition_scope_stage` → `DefinitionScopeStageDao` /
    `read_definition_scope_phase_entry` → `DefinitionScopePhaseEntryDao` / `read_intent` → `IntentDao` /
    `read_intent_stage` → `IntentStageDao` / `read_execution` → `ExecutionDao` /
    `read_execution_stage` → `ExecutionStageDao` / `read_next_answer` → `NextAnswerDao` /
    `read_next_jump` → `NextJumpDao` / `read_next_jump_phase` → `NextJumpPhaseDao` /
    `read_run_stage` → `RunStageDao` / `read_scope_change` → `ScopeChangeDao`。管理表は
    `amadeus_projection_checkpoint` → `ProjectionCheckpointDao`（行 `ProjectionCheckpointRow`、アンカー
    `JournalAnchor`）、`amadeus_read_model_head`（旧 `shared_projection`）→ `ReadModelHeadDao`（行
    `ReadModelHeadRow`）、`PRAGMA user_version` → `ReadSchemaVersionDao`（値 1 つの表とみなす）。名前から
    `amadeus_` を除くのは `read_` と同じ理由（本家の表と衝突しないための接頭辞）。
  - **行の値**: 20 表の行（`DefinitionRow` ほか）を `read_tables` から `orchestration/port/` へ移し、完全
    コンストラクタ `new` だけの値にした。材料から行を組む投影は `read_tables/<表>_projection.rs` の自由関数
    （`row` / `rows`）として `ReadTables::project` が呼ぶ（PR2 と同じ分け方）。
  - **更新器** `StructuredReadModelUpdater`（型引数はジャーナルの読み手・番号の DAO・記録の DAO・20 表の DAO。
    既定の型引数が実物の組）。接続を 1 本所有し、`update_read_models` で「共有面の点検（旧
    `prepare_read_model` — 旧い変換・記録なし・未照合なら全履歴から描き直す、旧 `rebuild_read_model`）→
    投影名を束ねていれば、番号より後に事実があるときだけ全履歴を投影し、20 表・共有面の記録・番号を 1 つの
    IMMEDIATE トランザクションで確定する（旧 `advance_checkpoint`）」を行う。書く手順と表をまたぐ検査
    （内容のダイジェスト・同じ位置の一致・共有面より古い断面を確定しない判断・アンカー照合）は
    `structured_surface`（接続を持たない手順。trait を持たないのでポートではない）と
    `StructuredSurfaceContent`（20 表の内容の値）が持ち、DAO は 1 表の I/O だけ。ダイジェストの形式は
    分ける前と同一（実測で一致を確かめ、空の面の値と実データでの一致を試験で釘留めした）。
  - **ジャーナルの読み手**: 更新器のトランザクションの上で読む暫定の面ごとの読み手 `StructuredJournalReader`
    （`events_after` / `events_through` / `anchor_at`）。旧 `JournalReader` はまだ公開を抱えているため
    （`WorkspaceDoctorJournalReader` と同じ理由）。最後の縮小で `JournalReader` 1 本へまとめる。
  - **表の用意をどこが持つか（PR2/PR3 からの持ち越しを決めた）**: 各表の DDL はその表の DAO。表を書く更新器は
    開く段で自分の表を揃える（`table_exists` を書込ロック無しで見て、欠けているときだけ `BEGIN IMMEDIATE` で
    `create_table`）。それとは別に、**ストアの読み面の表をすべて揃え、読み面の版を守る 1 か所**を
    `read_model_schema::prepare` に置き、構造化面の更新器の開く段（`StructuredReadModelUpdater::open`）だけが
    呼ぶ。理由は 2 つ — (1) クエリ側は面ごとの更新器がまだ走っていないストアでも表を引く（表が無いと
    「行が無い」ではなく読取の失敗になる）、(2) 読み面の版は読み面全体の性質で、版が動いたらすべての
    `read_*` 表を落として作り直す必要がある（落とした表は空で作り直す）。構造化面の更新器は、本番のどの経路
    （取得ループ・初回の定義の用意・テスト契約／計画指紋／開始可否の前の共有面の点検）でも最初に開かれる。
    **`JournalReaderImpl::open` はリードモデル側の表を 1 つも作らなくなった**（まだ抱えている公開計画の表
    `amadeus_publication*` だけ — 揃っていれば書込ロックを取らない）。20 表の `table_exists` は表に加えて
    その表の索引も数える — 以前は開く段が毎回 `CREATE INDEX IF NOT EXISTS` を打っていたので、索引の DDL が
    崩れた表の形（列の欠け）を開く段で見つけていた。その検出を保つため。
  - **版（`PRAGMA user_version`、現行 7）の扱い**: 版が現行で表も記録も揃っていれば何もしない（書込ロック
    無し）。動いていれば 1 つの IMMEDIATE トランザクションで「投影が 1 度でも進んだストアなら全履歴から行を
    組む（描けない歴史はここで `Corrupt` にして止める）→ `read_*` 26 表を落として作り直す → 組んだ行を
    構造化面へ書く → 版を記録 → 共有面の記録を未照合へ戻す」。チェックポイントは戻さない。以前は DROP を
    トランザクションの外で先に打っていたので、描き直しに失敗すると空の新しい形の表が残った。今は失敗すれば
    何も変わらない（版も上がらない — 次に開いたときにやり直す）。
  - **開く段の書込ロック**: 共有面の記録の `INSERT OR IGNORE`（旧 `shared_projection::initialize`）を
    「記録の行が無いときだけ `ReadModelHeadDao::save`」に変え、存在の確認は書込ロック無しで行う。
    これで、表が揃ったストアではジャーナルの読み手も構造化面の更新器も開く段で書込ロックを取らない。
  - `JournalReader` から `prepare_read_model` と `advance_checkpoint` を消し、`JournalReaderImpl` から
    `rebuild_read_model` と、開く段の表の用意（`ensure_read_schema` / `ensure_reference_tables` /
    `shared_projection` の初期化と無効化）を消した。`read_tables/sql.rs`（20 表の DDL・`replace_all`・
    `matches_rows`・`content_digest`・版）と `shared_projection.rs` は削除した（互換の口は残していない）。
  - 共有面の点検を、借りた読み手の `prepare_read_model` で行っていた更新器の扱い: 取得ループ
    （`OrchestrationReadModelUpdater`）は点検の更新器を型引数 `P` で持ち、更新の先頭で起動する（steering と
    同じ形）。テスト契約・計画指紋・開始可否の更新器は点検をやめ、合成ルート（`runtime/testing_posture.rs`）が
    先に構造化面の更新器を起動する（CLI から見た順序は同じ）。これらの更新器は読み手を `&mut` ではなく `&` で
    借りるようになった。
  - 公開（PR5）は、同じトランザクションの中で構造化面の手順（`structured_surface` — 表の DAO）を呼ぶ。
    ファイルの公開と 20 表・番号の確定を 1 つの IMMEDIATE トランザクションに閉じる今の形はそのまま。
  - **小さな挙動の差**（どれも手で壊した行か、到達しない値域でしか起きない）: 列に収まらない数・走査位置は、
    SQLite の変換失敗（`Io`）ではなく `Corrupt(InvariantViolation)` で止まる（PR2 の DAO と同じ）。
    負の `as_of` / 負の位置の番号は、共有面の位置の推定の段で `Corrupt` になる（以前は `MAX` の計算に紛れた）。
    ジャーナル側の行の通番が負のときのアンカー照合は `Corrupt(InvariantViolation)`（以前は
    `CheckpointAnchorMismatch`）。表の DAO の `Io` の所在は、更新器が開いたストアの綴りに揃える
    （`store_failure::InStore`）。
- **未移行**: 下表の 5 以降。`JournalReader` に残るのは `events_after` / `events_through`（最後まで残す読取）と、
  公開の口 `pending_publication` / `publish` / `checkpoint`（`publish` が進める番号の読み手なので、`publish` と
  一緒に PR5 で動かす。実装はすでに番号の DAO とアンカー照合を通る）。`JournalReaderImpl` の固有メソッドは
  `open` / `open_with_busy_timeout` / `path` / `restore_missing_files` / `resolve_publication`（PR5）。

## PR の順序

各 PR は 1 つの書込経路を移し、既存テストの意味を保ったまま、DAO の単体テスト（書いて読み戻す）と
更新器のテスト（番号の次から再開・冪等・途中失敗で何も動かない・IMMEDIATE で待つ）を足す。
先に「ファイルを含まない・単独の Tx で済む」経路を移し、ファイルと表が絡む経路を後ろに置く。

| PR | 対象 | 移す書込 | 表・ファイル（DAO 1 本ずつ） | 要点 |
| --- | --- | --- | --- | --- |
| 1 | 自己診断 | 済 | `read_doctor_report` / `read_doctor_check` / `workspace_doctor_projection_checkpoint` | 済（#161） |
| 2 | 参照入力由来の単独面（済） | `replace_steering` / `replace_testing` / `replace_plan_fingerprint` / `replace_code_generation_approval` と対の `*_source_digest` 読取 | `read_steering_plan` / `read_steering_part` / `read_testing_contract` / `read_plan_fingerprint` / `read_code_generation_approval` | もともとチェックポイントと別の Tx。ジャーナルではなく参照入力由来なので、番号ではなく `source_digest`（行の列）で冪等。steering は 2 表を 1 Tx で書く |
| 3 | Pipeline 面（済） | `replace_pipeline` | `read_pipeline_progress` | #134 の IMMEDIATE をそのまま DAO の呼び手（更新器 `PipelineProgressReadModelUpdater`）へ移した。「同じ照合子か、保存済みの位置のほうが新しければ書かない」判定は更新器へ（DAO は 1 表の I/O だけ） |
| 4 | 構造化面 20 表（済） | `advance_checkpoint`（`replace_all` + チェックポイント前進 + アンカー照合）と `prepare_read_model` / `rebuild_read_model` | `read_*` 20 表（表ごとの DAO）、`amadeus_projection_checkpoint`、`amadeus_read_model_head`、`PRAGMA user_version` | 済。20 表 + 共有面の記録 + 番号を 1 IMMEDIATE Tx（`StructuredReadModelUpdater`）。アンカー照合・内容の照合は更新器側の手順（`structured_surface`）。表の用意（DDL・版による DROP）は各 DAO の DDL と `read_model_schema::prepare` 1 か所（構造化面の更新器の開く段）に決めた |
| 5 | 公開（Markdown 面） | `publish` / `pending_publication` / `restore_missing_files` / `resolve_publication` | `aidlc-state.md`・監査シャード・規則ファイル（ファイル 1 本 = DAO 1 本）、公開計画の表（`amadeus_publication*` 6 表） | 下記「ファイルと表の原子性」。監査シャード（追記）はファイルごとの反映済み番号で冪等にする |
| 6 | 承認ランタイム | `PlanApprovalJournalReader::replace` | `read_plan_operation` / `read_plan_answer` / `read_plan_generation` / `amadeus_plan_projection_checkpoint`、承認ファイル群 | `PlanApprovalJournalReader` を読むだけに縮める。ファイルの公開を Tx の外へ出す（下記） |
| 7 | 心拍 | `HookHealthReadModelUpdater` の生 SQL とファイル書込 | `read_hook_health` / `hook_health_projection_checkpoint`、`<hook>.last`（置換）/ `<hook>.drops`（追記） | 番号が今は `MAX(seq_nr)`（集約内の通番）で、差分読取に使われていない。ジャーナル上の位置へ直す。`.drops` の冪等を「末尾一致の推測」からファイルごとの反映済み番号へ置き換える |
| 8 | 停止制御 | `WorkflowContinuationReadModelUpdater` の生 SQL とファイル書込 | `read_continuation_result` / `continuation_projection_checkpoint`、`.aidlc-stop-hook/block-count.json`（置換） | ファイル公開を Tx の外へ出し、「公開したか」を結果の列に持つ今の形と原則 6 の整合を取る |
| 9 | runtime-graph | ファイル書込 | `runtime-graph.json`（置換） | 表を持たない。ジャーナル読取を `JournalReader` の読むだけの口へ。監査シャードの読取は参照入力の読取 |
| 10 | 縮小と lint | `JournalReader` / `PlanApprovalJournalReader` をジャーナル専用へ縮める | — | 下記「最後の縮小」「最後に入れる lint」 |

## 最後の縮小

- `JournalReader` に残すのは `events_after` / `events_through`（位置より後／まで）だけにする。`checkpoint` は
  番号の表の DAO へ（PR5 で公開と一緒に）、書込はすべて表の DAO へ移り終えている。戻り値の `JournalBatch` はジャーナルの要素（事実と
  位置）だけを運ぶ。
- `PlanApprovalJournalReader` も `all_events`（または位置つきの読取）だけにする。`plan_approval_receipts` は
  読み手の上の関数のまま残せる。
- 面ごとの `…JournalReader`（`PlanApprovalJournalReader` / `WorkspaceDoctorJournalReader` /
  `StructuredJournalReader`）を 1 本の
  `JournalReader`（manifest か事実の型で引数を取る形）へまとめて消す（下記「決定事項」1）。

## 最後に入れる lint（移行完了の PR で、既存違反 0 件の状態で入れる）

1. **`JournalReader` の純度**: `…JournalReader` を名乗る trait が、`&mut self` のメソッド・`Transaction` を
   受け取るメソッド・`read_tables` の型（行・表）を引数や戻り値に持つメソッドを持ったら所見。
2. **RMU の DAO の形と置き場**: RMU クレートで `…Dao` を名乗る trait は `orchestration/port/` にだけ置く。
   実装 `…DaoImpl` は `…_dao_impl.rs` に置き、その中の SQL リテラルが触れる表は 1 つだけ
   （クエリ側の `dao-single-table` を RMU の `*_dao_impl.rs` へ広げる）。
3. **更新器に生の SQL・ファイル I/O を置かない**: RMU クレートの `*_dao_impl.rs` / `*journal_reader_impl.rs` 以外に
   SQL リテラル（`SELECT` / `INSERT` / `UPDATE` / `DELETE` / `CREATE`）と `std::fs` / `core_infrastructure::atomic` /
   `append_only` が現れたら所見（`command-side-io` の RMU 版）。トランザクションを開く・確定する呼出は更新器に許す。
4. **クエリ側の `…Dao` は `find` だけ**: クエリ側 use-case の `port/` の `…Dao` trait が `find` 以外のメソッドを
   持ったら所見。**既存の違反が 1 件ある** — `ContinuationResultDao::unsettled`
   （`modules/core/query/use-case/src/orchestration/port/continuation_result_dao.rs`）。lint を入れる PR で
   `find_…` への改名か、`find` の引数（キー）で表す形へ直す。

## ファイルと表の原子性 — 今の実装が前提にしているが、成り立たない箇所

原則 6 のとおり、ファイルはロールバックされない。次の箇所は「トランザクションの中でファイルを書いてから
コミットする」形になっており、コミットに失敗するとファイルだけが進む。

1. **公開の確定（`publication_store::publish_prepared`）**: IMMEDIATE Tx の中で `saved.apply()`（状態ファイルの置換と
   監査シャードの追記）をしてから、チェックポイントの前進・構造化面の差し替え・計画の確定をコミットする。
   コミットの前に落ちてもファイルは戻らない。**救済はある** — 計画（前後のバイト）を先に別 Tx で保存しているので、
   次の実行が同じ計画を再適用し、`apply` は「すでに後の内容なら何もしない」「追記は未反映の残りだけ」で二重適用を
   避ける。つまり今の安全性は Tx の原子性ではなく**書き込み前の計画（redo の記録）**に依っている。PR5 では、この
   計画による冪等を、原則 6 の「追記ファイルはファイルごとの反映済み番号」に置き換え、計画の仕組みはなくす（下記「決定事項」2）。
2. **承認ランタイム（`PlanApprovalJournalReaderImpl::replace`）**: IMMEDIATE Tx の中で `plan_approval_files::publish`
   （ファイルの置換・削除・ディレクトリの削除）をしてからコミットする。コミットに失敗すると、ファイルは新しく表と
   番号は古い。全履歴からの再計算なので次の実行で揃うが、**それまでの間、ファイルと表が食い違って見える**。
   削除（`remove_file` / `remove_dir`）も戻らない。
3. **停止制御（`WorkflowContinuationReadModelUpdater::project`）**: IMMEDIATE Tx の中で `publish_counter`
   （`block-count.json` の置換）をしてから、`counter_published` を行に書いてコミットする。コミットに失敗すると、
   ファイルは書かれたのに「公開した」の記録が残らない。置換なので再実行で同じ内容になるが、「公開したか」の列は
   ファイルの実際と食い違いうる。
4. **心拍（`HookHealthReadModelUpdater::project`）**: こちらは表をコミットしてからファイルを書く（逆順）。
   ファイルの前に落ちると表だけが進むが、毎回全履歴から書き直すので次の実行で揃う。`.drops`（追記）の冪等は
   「ファイル末尾が履歴のどこまでと一致するか」の推測で、同じ行が繰り返す履歴や、利用者が末尾を編集した場合に
   二重追記・取りこぼしを起こしうる。

## 決定事項（オーナー裁定 2026-09-26）

1. **ジャーナルを読む口は `JournalReader` 1 本**。面ごとの `…JournalReader`（`PlanApprovalJournalReader`・
   `WorkspaceDoctorJournalReader`）は移行中の暫定で、最後の PR で `JournalReader`（読む事実の種類は引数で絞る）へ
   まとめて消す。`gateway-taxonomy.md` §3 の例外は増やさない。
2. **追記するファイルの二重追記は「ファイルごとの反映済み番号」で防ぐ**（原則 6）。公開計画（書込前に前後の
   バイトを保存して適用し直す仕組み、`amadeus_publication*` の表と適用し直す処理）は、公開の移行（行 5）でなくす。
3. **トランザクションの受け渡しは PR1 の形にそろえる**（更新器が IMMEDIATE で開き、表の DAO へ
   `&mut Transaction` で渡す）。DAO の責務はリードデータの読み書きだけ。
