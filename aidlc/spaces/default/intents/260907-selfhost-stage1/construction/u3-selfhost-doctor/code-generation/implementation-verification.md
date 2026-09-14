# U3 実装検証報告（Code Generation Part 2、担当 u3_doctor）

2026-09-12。承認済み [実装計画](code-generation-plan.md)（指紋 `sha256:5a235659…`、Testing Contract `sha256:d904f82d…`）の Step 1〜6 を TDD（Red → Green → Refactor）で着地した報告。Step 7（workspace 全通し・カバレッジ・Quint・レビュー・PR）は親が行う。計画本文のチェックボックスは触っていない。

## 1. 着地したもの（要約）

- `aidlc --doctor`（Orchestrate 面の先頭引数。`aidlc-orchestrate --doctor` も同じ）を `Request::Doctor` として受け、契約 C7 の D1.a〜D5.b を読取専用の Query で判定し、C7 の書式（ヘッダ、`─`×37、`✓  `/`✗  `、`<passed> passed, <failed> failed`、LF、stderr 空）で描画して `failed=0` なら 0、それ以外は 1 で終了する。追加引数は stderr + 終了 1 で拒む。`next --doctor` の既存 print directive（TS 委譲）は変えていない。
- 起動時に監査シャードがある記録だけ、U2 の `RecordHealthCheckUseCase` へ `HealthCheckResult(passed, failed)` を渡し、通常の RMU 反映で `HEALTH_CHECKED`（`Request: /aidlc --doctor`、`Details: <passed> passed, <failed> failed`）を投影する。記録失敗は診断 stdout を残したまま C2 の共通文言で stderr へ出し終了 1（DC10）。初回状態ではファイル・監査・ストアを何も作らない（DC1）。診断のための `restore_missing_files` は呼ばない（`catch_up_with(layout, false)`）。
- 配置は計画どおり: 観測 DAO `modules/core/query/interface-adapter/`（`DoctorObservationDaoImpl` + 私有モジュール 7 本）、判定 use case と View `modules/core/query/use-case/`（`DoctorReportUseCase` / `DoctorReport` / `DoctorCheck` / `DoctorCheckId` と 25 本の View、ポート `DoctorObservationDao`）、入口・描画・記録呼出し `modules/app/aidlc/`（`runtime/doctor.rs`）。ドメイン判断（状態版の分類）は U2 の `StateVersionClassification` を合成ルートが `StateVersionClassifier` として注入し、Query では再実装していない。配布資産 `.claude/` 配下は変更していない。

## 2. Step ごとの Red / Green の根拠ログ

すべて `tdd-logs/`（先頭にコマンドと UTC、末尾 `# exit=<code>`）。

| Step | Red | Green / Refactor |
| --- | --- | --- |
| 1 入力と実行基盤 | — | `01-runner-check.log`: `cargo test -p aidlc --test diagnostic_record_contract` 7 件成功（既存ランナーの確認） |
| 2 観測 DAO | `02-observation-dao-red.log`: 新 target `doctor_observation_contract` 8 件中 7 件が**アサーション**で失敗（骨格 API はコンパイル済み。1 件は値の素通しなので骨格でも成功） | `02-observation-dao-green.log`: 8 件成功。fmt / clippy `-D warnings` / `cargo lint` 成功 |
| 3 判定 use case | `03-report-use-case-red.log`: 新 target `doctor_report_contract` 8 件全てアサーション失敗（評価器は空のスタブ） | `03-report-use-case-green.log`: 8 件成功。Refactor: 行順と集計を `DoctorReport` 1 箇所へ、`while let`・`double_must_use`・引数 8/9 個のコンストラクタを `StageArtifactsView` / `RecordLocationView` へ分割 |
| 4 CLI 入口と描画 | `04-cli-entry-red.log`: `aidlc --doctor` が `Unknown subcommand: --doctor. Valid: next, continue, report, park`（終了 1）で 3 件失敗 | `04-cli-entry-green.log`: 4 件成功（cold の全文一致、追加引数の拒否、`next --doctor` の維持、失敗行の fix 表示と終了 1） |
| 5 実行記録の副作用 | `05-record-side-effects-red.log`: 初期化済み記録で `HEALTH_CHECKED` が 0 件、記録失敗でも終了 0 —— 4 件失敗（この時点で D4/D5 の 5 行は実ストアに対して既に成功） | `05-record-side-effects-green.log`: 8 件成功（DC2 / 失敗報告の実数記録 / DC10 / 状態欠落を復元しない） |
| 6 異常注入と本家対応 | `06-corpus-green.log`（初回実行はテスト側の行順の前提が誤りで失敗した。本家は shell → heartbeat、C7 は D2.f → D3.a の表順なので、集合比較 + C7 表順の検査へ直して成功。失敗ログはこの修正で上書きされたため残っていない） | `06-corpus-green.log`: corpus 20 観測のうち `audit-locked` を除く 19 が採用行のバイト一致（`<TS>` 正規化）と終了値で一致。`06-native-injection.log`: 独自診断の異常注入 5 形が失敗行になる |
| 完了条件 | — | `07-doctor-contract-debug.log`（10 件）、`07-doctor-contract-release.log`（`--release` 10 件）、`07-regression.log`（既存回帰 7 本 + 触った 3 crate の単体テスト） |

## 3. D1.a〜D5.b の採用行と corpus との対応

比較器は `modules/app/aidlc/tests/doctor_contract.rs::upstream_doctor_observations_agree_on_adopted_rows_and_exit_codes`。本家 50 行のうちラベル先頭で採用行を選び、`<TS>` だけを正規化して**集合として**バイト一致を見る（表示順は C7 表順で本家と異なる — §5）。独自行は存在と件数、対象外行は不在、終了値は一致を検査する。

| ID | 出力行（この build） | corpus での一致 |
| --- | --- | --- |
| D1.a | `bun installed (required for CLI tools and hooks)`／fix `install via \`curl -fsSL https://bun.sh/install \| bash\`` | `cold`〜全 19 観測で一致。`missing-bun` の失敗行・fix も一致 |
| D1.b | `Native engine entry points`（独自。原因 `current executable: unreadable (<原因>)` / `aidlc: missing entry points (<一覧>)`） | 本家行なし。入口欠落は `runtime/doctor.rs` の単体テストと `doctor_report_contract` の注入で失敗を固定 |
| D2.a | `<hook>.ts present`（名前順）、`Hook contract: settings.json unreadable — cannot verify wired hooks`、`Hook contract: settings.json wires no aidlc-*.ts hooks` と各 fix | `missing-hook` / `missing-settings` / `no-hooks` / `invalid-settings` で一致（17 本の並びも一致） |
| D2.b | `Hooks enabled (resolved disableAllHooks is not true)` / `Hooks DISABLED via "disableAllHooks": true in <層> — AI-DLC cannot run (…)` と層別 fix | `disabled-hooks` で一致。管理設定層の fix は `doctor_report_contract` で固定 |
| D2.c | `Claude managed hook policy: allowManagedHooksOnly=true` + fix（該当時のみ） | `managed-only` で一致 |
| D2.d | `settings.json present`／fix `copy from \`dist/claude/.claude/settings.json\`` | `missing-settings` で一致 |
| D2.e | `Native hook bindings`（独自。`.claude/settings.json: missing / unreadable (…) / invalid JSON: … / no hook bindings / binding mismatch (…)`） | 本家行なし。混在・不明呼出し・未知 native 名・パース不能を `doctor_report_contract` と CLI 注入で固定 |
| D2.f | `Hook heartbeats: not yet fired (first workflow stage will populate)` / `Hooks have never executed although this workflow has progressed N stage(s)` / `Hook heartbeat data` の 2 fix / `Hooks last fired: …` / `Hooks last fired X, but the workflow last advanced Y` と本家の復旧手順 | `cold`（初回）、`initialized`・`version-*`（進行後未発火 4 stages）、`heartbeat-unreadable`、`heartbeat-stale`（300001ms → 失敗）、`heartbeat-boundary`（300000ms → 成功、終了 0）で一致 |
| D3.a | `workspace shell ready (.claude/ + aidlc/spaces/default/memory/)` | 全観測で一致 |
| D3.b | `Scope validation: 2 scopes valid (6 advisories)`（bugfix 6 + feature 0。本家 `aidlc-graph.ts validateScope` を a277af21 の配布物に対して bun で実走して得た数と一致） | 本家は `11 scopes valid (47 advisories)`。C7 どおり集計差を許容し、比較器は接頭辞 + この build の固定文言で検査 |
| D3.c | `Cycle detection: 0 cycles` / `… N cycle(s) found` fix `cycles: …`（Tarjan の順は本家の実走で確認: `b → a; c; f → e → d`）、`Orphan stage files: 33 graph entries all have files` / `… have no file on disk` fix `missing files: …` | `cyclic-graph`、`missing-stage` で一致 |
| D3.d | `Schema validation: 33/33 stages validated` / `… N of M stage(s) failed` fix `slug: <最初のエラー>`、`Graph references: 122 artifacts + edges resolved` / `… N broken reference(s)` | `invalid-stage`（`mode must be one of inline \| subagent \| pipeline \| mob \| agent-team, got "invalid-mode"`）、`invalid-reference` で一致 |
| D4.a | `State Version: 8` / `state version readable` / `state version current` / `state version compatible` + U2 の wording（本家 `classifyStateVersion` の message 逐語） | `initialized`、`version-missing`、`version-past`、`version-future` で一致 |
| D4.b | `Native workflow state readable`（独自。`<record>/aidlc-state.md: missing / unreadable (…)`） | 本家行なし。状態欠落・不読を CLI 注入で固定。本家が catch で行を省く不読を明示 |
| D4.c | `Native workflow identity`（独自。曖昧・`active-intent` 未解決・カーソル欠落/不読・登録簿の欠落/不一致/不読・実行行の intent 不一致・ストア不読） | 本家行なし。`doctor_report_contract` で 8 形、CLI 注入で登録簿の不一致 |
| D5.a | `Native event store readable`（独自。`<store>: missing / unreadable (…) / incompatible schema (missing tables: …; read schema version 0)`） | 本家行なし。CLI 注入（ディレクトリ化）で失敗 |
| D5.b | `Native projection consistency`（独自。実行行未投影・チェックポイント欠落・ジャーナル位置がチェックポイントより先・未確定の公開・監査シャード無し） | 本家行なし。CLI 注入（`read_execution` の行削除）で `not projected` |

## 4. DC1〜DC10 の検証結果

| ケース | 検証 | 結果 |
| --- | --- | --- |
| DC1 | `a_cold_shipped_workspace_renders_the_c7_report_and_creates_nothing`（前後のファイル一覧が同一、`intents/` 不在）、corpus `cold` | 終了 0。D4–D5 の行なし。何も作らない |
| DC2 | `an_initialized_record_with_live_hooks_passes_and_records_one_health_check` | 終了 0、stderr 空、`34 passed, 0 failed`、監査に `HEALTH_CHECKED` 1 件（`Request` / `Details` 逐語）、状態ファイル不変。2 回目も投影整合が成功し 2 件目を記録 |
| DC3 | corpus `missing-bun` / `missing-hook` / `disabled-hooks` | 該当本家行が失敗、終了 1、stderr 空、他の行も評価 |
| DC4 | corpus `missing-settings` / `no-hooks` / `invalid-settings`、CLI 注入（native と .ts の混在） | D2 の本家行と独自 `Native hook bindings` が別行で失敗し、どちらが本家対応か区別できる |
| DC5 | corpus `cold`（初回成功行）、`initialized`（進行後未発火 → 失敗）、`heartbeat-unreadable`、`heartbeat-stale`（300001ms）、`heartbeat-boundary`（300000ms → 成功） | 境界値は遅延失敗にしない |
| DC6 | corpus `missing-stage` / `cyclic-graph` / `invalid-stage` / `invalid-reference`、advisory のみは `cold`（6 advisories、終了 0） | 本家対応行が失敗、終了 1。advisory だけでは終了 0 |
| DC7 | `cold`（非適用）、CLI 注入の状態不読（独自 D4.b、本家 D4.a は出ない）、corpus `version-missing/past/future`（本家 D4.a の失敗） | 一致 |
| DC8 | CLI 注入: 登録簿の不一致（D4.c）、ストアのディレクトリ化（D5.a / D5.b / D4.c）、実行行の削除（D5.b） | 該当の独自必須失敗、終了 1。元データは触らない（読取専用で開く） |
| DC9 | corpus `heartbeat-boundary`（本家は `Workflow diagnosis (advisory)` の `runtime-graph-missing` だけが出る入力） | 欄を出さず終了 0。`initialized` 等も欄を出さない |
| DC10 | `a_recording_failure_keeps_the_report_and_exits_one_without_a_forged_fact`（journal への INSERT を trigger で拒否） | 診断 stdout は残り、stderr に記録失敗、終了 1、監査とジャーナルは不変 |

## 5. C7 で決まらず記録に留めた差（実装は変えていない）

1. **表示順**: C7 は D1.a から表順（D2.f heartbeat → D3.a shell、D4.a は D3 の後）。本家は shell → heartbeat、`State Version` は heartbeat の直後。行の綴りは一致するが並びが違うので、corpus 比較は集合比較 + C7 表順の検査にした。
2. **D3.c / D3.d の対象集合**: 「選定した 2 スコープで利用するグラフ」を、両スコープが使う**コンパイル済み `stage-graph.json` 全体**（33 ステージ、無効ステージも含む本家 `loadStageGraphAll` の対象）と解釈した。これにより `Orphan stage files: 33 …` / `Schema validation: 33/33 …` / `Graph references: 122 …` が corpus とバイト一致する。2 スコープの EXECUTE 部分集合に絞る解釈だと数が変わる。裁定が要るなら比較器と実装の両方を変える。
3. **D3.b の集計**: `2 scopes valid (6 advisories)`（本家 11 / 47）。C7 どおり。
4. **`audit-locked`**: 本家の mkdir ロック（`acquireAuditLock`）は TS 固有で、この build に対応する競合面が無い（記録失敗は SQLite 経路で DC10 として固定）。比較対象から外した。
5. **評価不能の原因文**: 本家は Node の `errorMessage(e)`（`ENOENT: no such file or directory, open '…'` 等）。この build は Rust の `io::Error` 表示（`No such file or directory (os error 2)`）を同じ形（`Stage graph not readable at <path>: <原因>. Reinstall …`）に埋める。corpus に評価不能ケースは無い。
6. **本家が catch で省く行**: ステージ本体が存在するのに読めない場合、本家は `readFileSync` の例外で `Schema validation: check failed` になる。これを写した（1 ステージの失敗には数えない）。状態ファイルの不読は本家が D4.a を省くので、独自 D4.b が明示する。
7. **frontmatter パーサ**: 本家の正規表現を `regex` クレートで当てる。JS の `.` は `\r` を含まないので、本文を `\r\n → \n` に正規化してから当てる（LF の配布物には差が出ない）。
8. **D5.a の「対応 schema」**: クエリ側は RMU の `READ_SCHEMA_VERSION` を知らない（クレート境界）ので、必要表（`journal` / `amadeus_projection_checkpoint` / `read_execution`）の存在と `PRAGMA user_version != 0` で判定する。版の一致検査は行っていない。
9. **D2.e の接続定義**: U4 の接続定義がまだ無いため、`hooks` ブロックの各 command を `aidlc-<name>.ts`（配布）/ `aidlc hook <name>`（native、`NATIVE_HOOKS` 14 本）/ 不明に分類し、未接続・不明・混在・未知 native 名を失敗にする。`statusLine` は D2.e の対象外（D2.a の本家正規表現の対象には入る）。U4 が接続定義を確定したら判定基準を追従する。
10. **D1.b の実効性**: 実行中バイナリの実体（`current_exe`）と、`cli::parse` の配線表の自己照合（14 入口 + フック 14 本）。同じバイナリの中では配線が欠けることはなく、失敗は `current_exe` 不能か将来の配線退行だけである。
11. **状態ファイルを失った記録**: 本家 `activeIntent`（裁定 F-H1 = B）どおり `Layout` はその記録を選ばないため、`active-intent` が名指す実在ディレクトリを観測対象にして D4.b（`…/aidlc-state.md: missing`）と D4.c（`active-intent: unresolved (names <記録>, which has no aidlc-state.md)`）で報告する。監査は記録木の根が intents ディレクトリへ落ちるので `HEALTH_CHECKED` は記録しない（本家 `auditShards` と同じ挙動）。
12. **クエリ側ユースケースの判定**: `coding-rules/cqrs-boundaries.md` 規則 6 の 2026-09-02 追記（クエリ側ユースケースは `dao.find → View` だけで判断を持たない）に対し、承認済み計画は D1–D5 の判定を Query use case に置く。計画に従ったが、規則との整合はレビューで裁定を仰ぐ（観測 DTO と判定を分けたので、判定を別層へ移す変更は `DoctorReportUseCase` 配下に閉じる）。

## 6. 変更ファイル一覧

[source-manifest.json](source-manifest.json)（115 経路）。要点:

- 新規 `modules/core/query/use-case/`: `orchestration/port/doctor_observation_dao.rs`、`orchestration/port/doctor_view/`（25 本の View + `mod.rs`）、`orchestration/doctor_check.rs` / `doctor_check_id.rs` / `doctor_report.rs` / `doctor_report_use_case.rs` と配下 7 モジュール（環境・フック・heartbeat・配布資産・記録の評価器、frontmatter パーサ、schema 検証）、`tests/doctor_report_contract.rs`。`Cargo.toml` に `regex`（workspace 依存）を追加。
- 新規 `modules/core/query/interface-adapter/`: `doctor_observation_dao_impl.rs` と配下 7 モジュール（環境・settings・監査台帳・heartbeat・配布資産・記録・ストア）、`doctor_paths.rs`、`doctor_environment.rs`、`native_doctor_facts.rs`、`state_version_classifier.rs`、`tests/doctor_observation_contract.rs`。`Cargo.toml` の `chrono` を通常依存へ（heartbeat / 監査の ISO 8601 をミリ秒へ）。
- `modules/app/aidlc/`: `cli/request.rs`（`Request::Doctor`）、`runtime.rs`（`Completion::reported` / `reported_then_refused`、`NATIVE_HOOKS` の抽出、`catch_up_with(restore_missing)`）、`runtime/doctor.rs`（新規）、`wording.rs`（`doctor_takes_no_arguments`）、`tests/doctor_contract.rs`（新規、10 件）。
- `tests/golden/selfhost-stage1/doctor-shell/`（新規 fixture、50 ファイル + `provenance.json`）: a277af21 の `settings.json`・agents 14・stages 33・bugfix/feature の scope 定義。全ファイルが封印済み `source-manifest.sha256` と一致することを `doctor_contract` が実行時に確かめる。既存コーパスは変更していない。
- `Cargo.lock`: `core-query-use-case` の依存に `regex` が加わった 1 行。

## 7. 検査結果（担当範囲）

- `cargo test -p core-query-interface-adapter --test doctor_observation_contract`: 8 件成功
- `cargo test -p core-query-use-case --test doctor_report_contract`: 8 件成功
- `cargo test -p aidlc --test doctor_contract`: 10 件成功（debug）、`--release` でも 10 件成功
- 既存回帰 7 本と触った 3 crate の `--lib`: `07-regression.log`
- `cargo fmt --all --check`、`cargo clippy -p aidlc -p core-query-interface-adapter -p core-query-use-case --all-targets -- -D warnings`、`cargo lint`: 成功
- workspace 全通し・カバレッジ・Quint / ITF・CI・`cargo audit` は親（Step 7）

## 8. dead code 候補

- `HookBindingView::matcher()`（観測 DTO の読取。判定は event と command だけを使う。契約テストが読む）
- `StageFileView::phase()` は schema 検証の突合に使うが、`DoctorPaths::project_dir()` は現状 `relative()` だけが内部で使う
- `DoctorCheck::fix()` の成功行側の値（本家同様に描画しないが、`DoctorCheck::new` で任意に持てる）

いずれも DTO のアクセサで、削除は型の対称性を崩すため残置した。

## 9. 残課題

- U4: D2.e の接続定義（`aidlc hook <name>` の command 形）が確定したら判定基準を追従する。`settings.json` の `statusLine` を native 面へ寄せるかも U4。
- 切替工程: 本リポジトリの `.claude/settings.json` は `statusLine` が `statusline-combined.sh` を指すため D2.a の名指しは 16 本（a277af21 の 17 本と異なる）。本リポジトリでの `target/release/aidlc --doctor` 成功は切替条件として親が再確認する。
- §5 の 2（D3.c / D3.d の対象集合）と 12（クエリ側ユースケースの判定）は人間の裁定候補。

## Sources

- [code-generation-plan.md](code-generation-plan.md)、[unit-test-instructions.md](unit-test-instructions.md)、[contract-summary.md](../../../inception/contract-design/contract-summary.md) C5・C7、`tests/golden/upstream-a277af21/doctor/cases.json`、`scripts/goldens/capture-doctor.ts`、固定コミット `a277af21` の `aidlc-utility.ts` / `aidlc-lib.ts` / `aidlc-graph.ts` / `aidlc-stage-schema.ts`（`git show` で採取）。
