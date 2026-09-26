# RMU の DAO 移行計画（Issue #153、2026-09-26）

リードモデル更新器を「ジャーナルを読む → 投影 → 表の DAO で更新する」形へ移す計画である。規則の正本は
[read-model-updater-structure.md](aidlc-shared/coding-rules/read-model-updater-structure.md)（オーナー裁定 2026-09-26）。
この文書は規則ではなく、順序と残作業の記録である。

## 現状（PR2 の後）

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
    接続を 1 本所有し（`open`）、表の DAO へトランザクションを渡す。履歴の読取と `prepare_read_model` は、まだ
    移していないので借りた `JournalReader` に残る。
  - 冪等は番号ではなく行の `source_digest` で取る（今までどおり）。「同じ照合子か、保存済みの行のほうが新しい
    履歴位置（`as_of`）なら書かない」判断は更新器が持つ。steering とテスト契約は、今までどおり書込ロックを取らずに
    比べてから、動いていれば IMMEDIATE で開いて比べ直す（計画指紋と開始可否は今までどおり IMMEDIATE の中で比べる）。
  - `JournalReader` から `steering_source_digest` / `replace_steering` / `testing_source_digest` / `replace_testing` /
    `replace_plan_fingerprint` / `replace_code_generation_approval` を消した（互換の口は残していない）。
  - **暫定**: 表の用意（DDL・版による DROP）をどこが持つかは PR4 で決める — それまで `JournalReaderImpl` が
    各 DAO の `create_table` を呼ぶ暫定。開く段で 5 表を作るのは `JournalReaderImpl::open` が各表の DAO の
    `create_table` を呼んで行い（クエリ側は面ごとの更新器がまだ走っていないストアでも表を引くため）、読み面の版
    （`PRAGMA user_version`）が動いたときに 5 表を落とす処理は `read_tables` の `DROP` に残してある。
  - **開く段の書込ロック**: 更新器の `open` と `JournalReaderImpl` の 5 表の用意は、各 DAO の `table_exists`
    （`sqlite_master` の読取）で表が揃っているかを書込ロックを取らずに見て、揃っていれば書込トランザクションを
    開かない。欠けているときだけ `BEGIN IMMEDIATE` で開いて `create_table` を呼ぶ（DEFERRED で開くとスキーマを
    読んでから書込へ昇格するので #134 の即時 `SQLITE_BUSY` になりうる）。これで、表が揃っていて参照入力も
    動いていない更新は、この 5 表について書込ロックを取らない。ただし `JournalReaderImpl::open` 全体としては、
    PR2 の範囲外の既存の書込（`shared_projection::initialize` の `INSERT OR IGNORE` など）が今も書込ロックを
    取る。これも PR4 の「表の用意をどこが持つか」と一緒に扱う。
- **未移行**: 下表の 3 以降。`JournalReader` はまだ `prepare_read_model` / `publish` / `advance_checkpoint` /
  `replace_pipeline` / `pending_publication` を抱えている。

## PR の順序

各 PR は 1 つの書込経路を移し、既存テストの意味を保ったまま、DAO の単体テスト（書いて読み戻す）と
更新器のテスト（番号の次から再開・冪等・途中失敗で何も動かない・IMMEDIATE で待つ）を足す。
先に「ファイルを含まない・単独の Tx で済む」経路を移し、ファイルと表が絡む経路を後ろに置く。

| PR | 対象 | 移す書込 | 表・ファイル（DAO 1 本ずつ） | 要点 |
| --- | --- | --- | --- | --- |
| 1 | 自己診断 | 済 | `read_doctor_report` / `read_doctor_check` / `workspace_doctor_projection_checkpoint` | 済（#161） |
| 2 | 参照入力由来の単独面（済） | `replace_steering` / `replace_testing` / `replace_plan_fingerprint` / `replace_code_generation_approval` と対の `*_source_digest` 読取 | `read_steering_plan` / `read_steering_part` / `read_testing_contract` / `read_plan_fingerprint` / `read_code_generation_approval` | もともとチェックポイントと別の Tx。ジャーナルではなく参照入力由来なので、番号ではなく `source_digest`（行の列）で冪等。steering は 2 表を 1 Tx で書く |
| 3 | Pipeline 面 | `replace_pipeline` | `read_pipeline_progress` | #134 の IMMEDIATE をそのまま DAO の呼び手（更新器）へ移す。「保存済みの位置より古ければ書かない」判定は更新器へ（DAO は 1 表の I/O だけ） |
| 4 | 構造化面 17 表 | `advance_checkpoint`（`replace_all` + チェックポイント前進 + アンカー照合）と `prepare_read_model` / `rebuild_read_model` | `read_*` 17 表を表ごとの DAO に、`amadeus_projection_checkpoint` を DAO に、`shared_projection` の表を DAO に | 最大の PR。17 表 + 番号を 1 IMMEDIATE Tx。アンカー照合（位置の行の `aid`/`seq_nr`）は番号の表 + ジャーナルをまたぐ検査なので、DAO ではなく更新器（ジャーナルの読み手で読んだ行と、番号の DAO で読んだ値を比べる）に置く。スキーマ版（`PRAGMA user_version`）の扱いも決める。**表の用意（DDL・版による DROP）をどこが持つかもここで決める** — それまで `JournalReaderImpl` が各 DAO の `create_table` を呼ぶ暫定（PR2 で入れた） |
| 5 | 公開（Markdown 面） | `publish` / `pending_publication` / `restore_missing_files` / `resolve_publication` | `aidlc-state.md`・監査シャード・規則ファイル（ファイル 1 本 = DAO 1 本）、公開計画の表（`amadeus_publication*` 6 表） | 下記「ファイルと表の原子性」。監査シャード（追記）はファイルごとの反映済み番号で冪等にする |
| 6 | 承認ランタイム | `PlanApprovalJournalReader::replace` | `read_plan_operation` / `read_plan_answer` / `read_plan_generation` / `amadeus_plan_projection_checkpoint`、承認ファイル群 | `PlanApprovalJournalReader` を読むだけに縮める。ファイルの公開を Tx の外へ出す（下記） |
| 7 | 心拍 | `HookHealthReadModelUpdater` の生 SQL とファイル書込 | `read_hook_health` / `hook_health_projection_checkpoint`、`<hook>.last`（置換）/ `<hook>.drops`（追記） | 番号が今は `MAX(seq_nr)`（集約内の通番）で、差分読取に使われていない。ジャーナル上の位置へ直す。`.drops` の冪等を「末尾一致の推測」からファイルごとの反映済み番号へ置き換える |
| 8 | 停止制御 | `WorkflowContinuationReadModelUpdater` の生 SQL とファイル書込 | `read_continuation_result` / `continuation_projection_checkpoint`、`.aidlc-stop-hook/block-count.json`（置換） | ファイル公開を Tx の外へ出し、「公開したか」を結果の列に持つ今の形と原則 6 の整合を取る |
| 9 | runtime-graph | ファイル書込 | `runtime-graph.json`（置換） | 表を持たない。ジャーナル読取を `JournalReader` の読むだけの口へ。監査シャードの読取は参照入力の読取 |
| 10 | 縮小と lint | `JournalReader` / `PlanApprovalJournalReader` をジャーナル専用へ縮める | — | 下記「最後の縮小」「最後に入れる lint」 |

## 最後の縮小

- `JournalReader` に残すのは `events_after` / `events_through`（位置より後／まで）だけにする。`checkpoint` は
  番号の表の DAO へ、書込はすべて表の DAO へ移り終えている。戻り値の `JournalBatch` はジャーナルの要素（事実と
  位置）だけを運ぶ。
- `PlanApprovalJournalReader` も `all_events`（または位置つきの読取）だけにする。`plan_approval_receipts` は
  読み手の上の関数のまま残せる。
- 面ごとの `…JournalReader`（`PlanApprovalJournalReader` / `WorkspaceDoctorJournalReader`）を 1 本の
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
