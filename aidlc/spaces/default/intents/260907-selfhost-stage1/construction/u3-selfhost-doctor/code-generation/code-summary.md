# U3 実装要約 — セルフホスト用の自己診断（`aidlc --doctor`）

対象は FR5・FR6（NFR1–NFR4）、契約 C5・C7（D1–D5、DC1–DC10）。比較基準は本家 2.7.1 固定コミット `a277af218f0df7f325d3b8be7b6d90fce2c5bd40` の doctor 観測（`tests/golden/upstream-a277af21/doctor/cases.json`）。

## 何を作ったか

`aidlc --doctor` を Orchestrate 面の新しい入口として受け、C7 の D1–D5（本家対応 11 行 + 独自の必須診断 6 行）を判定し、C7 の書式で描画して終了コードを返す。診断実施の記録だけを U2 の `RecordHealthCheckUseCase` 経由で `HEALTH_CHECKED` へ投影する。

流れは利用者裁定 2026-09-12（[doctor-placement-questions.md](doctor-placement-questions.md) Q2）のとおり、コマンド側集約が判定してイベントを吐き、RMU がリードモデルを作り、クエリ側はそれを表示する。

```text
観測  DoctorObservationDaoImpl (クエリ側 IA) → DoctorObservationView
写像  runtime/doctor/observation.rs          → workspace::DoctorObservation
書込  DiagnoseWorkspaceUseCase               → WorkspaceDoctor → WorkspaceDoctorEvent → store
投影  WorkspaceDoctorReadModelUpdater        → read_doctor_report / read_doctor_check
読取  DoctorReportUseCase (DAO 2 本)         → DoctorReport
描画  render                                 → C7 の書式
```

両側を知ってよいのは RMU と合成ルートだけなので、観測 View → ドメインの写像は合成ルート（`modules/app/aidlc/src/runtime/doctor/observation.rs`）に置いた（`coding-rules/cqrs-boundaries.md`）。観測（ファイル・環境・ストアの読取）は「読むだけ」なので規則 5 によりクエリ側に残した。

## 変更ファイル

[source-manifest.json](source-manifest.json)（175 経路）。要点は次のとおり。

- **新規 `modules/core/command/domain/src/workspace/`**: 集約 `WorkspaceDoctor`、イベント `WorkspaceDoctorEvent` と ID 2 種、エラー、第一級コレクション `DoctorChecks`（`evaluate` が判定の正本）、`doctor_checks/` 5 モジュール（環境・フック・heartbeat・配布資産・記録）、観測の値オブジェクト 24 本、`stage_frontmatter/`、契約テスト `tests/workspace_doctor_contract.rs`。
- **新規 `modules/core/command/use-case/`**: `DiagnoseWorkspaceUseCase`、ポート `WorkspaceDoctorRepository`、`WorkspaceDoctorCommandError`。`test_support.rs` に `InMemoryWorkspaceDoctorRepository` と観測フィクスチャ。
- **新規 `modules/core/command/interface-adapter/`**: `WorkspaceDoctorRepositoryImpl`、DTO 4 本、契約テスト `tests/workspace_doctor_repository_contract.rs`。
- **新規 `modules/core/read-model-updater/`**: `WorkspaceDoctorReadModelUpdater`（`read_doctor_report` / `read_doctor_check` への投影）、契約テスト。`read_tables/row_id.rs` に `doctor_check` 代理キーを追加し、クレート doc（`read_*` 表の正本）へ 2 表を追記。
- **新規 `modules/core/query/interface-adapter/`**: 観測 DAO `DoctorObservationDaoImpl` と配下 7 モジュール、`DoctorReportDaoImpl` / `DoctorCheckDaoImpl`、`DoctorPaths` / `DoctorEnvironment` / `NativeDoctorFacts` / `StateVersionClassifier`、契約テスト 2 本。`read_model_daos.rs` に DAO 2 本を追加。
- **新規 `modules/core/query/use-case/`**: ポート `DoctorObservationDao` / `DoctorReportDao` / `DoctorCheckDao`、`port/doctor_view/` の View 27 本、表示専用の `DoctorReportUseCase`、契約テスト。
- **`modules/app/aidlc/`**: `cli/request.rs`（`Request::Doctor`）、`runtime.rs`（`Completion::reported` / `reported_then_refused`、`NATIVE_HOOKS` の抽出、`catch_up_with(restore_missing)`）、`runtime/doctor.rs` と `runtime/doctor/observation.rs`、`wording.rs`（`doctor_takes_no_arguments`）、契約テスト `tests/doctor_contract.rs`。
- **`tests/golden/selfhost-stage1/doctor-shell/`**: 新規 fixture 50 ファイル + `provenance.json`（`a277af21` の `settings.json`・agents 14・stages 33・bugfix / feature の scope 定義）。全ファイルが封印済み `source-manifest.sha256` と一致することを `doctor_contract` が実行時に確かめる。既存コーパスは変更していない。
- `Cargo.lock` の差分は**ゼロ**。再配置の過程で `core-query-use-case` の `regex` 依存が未使用化したため外し、Part 1 が加えた 1 行が相殺された。
- 配布資産（`.claude/` 配下）と `tests/golden/upstream-a277af21/` の封印済み fixture は変更していない。

## 主要な実装判断

1. **判定はコマンド側集約に置く**（裁定 Q2）。`DoctorChecks::evaluate` が D1.a〜D5.b の成否・ラベル・修復案を決め、イベントがそれを運ぶ。RMU は集計（`passed` / `failed` / 終了コード）まで非正規化して焼き込み、クエリ側は行を写すだけで数えない。`cqrs-boundaries.md` 規則 6 の 2026-09-02 追記（行に無い事実を作った時点で CQRS 違反）に合わせた。
2. **診断イベントは毎回一時ストアへ書く**（裁定 Q4）。プロセス内の共有キャッシュ SQLite（`file:aidlc-doctor-<pid>-<n>?mode=memory&cache=shared`）で集約 → イベント → RMU → クエリを回し、永続する事実は `HEALTH_CHECKED` 1 件だけにした。理由は「承認済み計画からの逸脱」に記す。
3. **D3.c / D3.d の対象集合はコンパイル済みグラフ全体**（裁定 Q1）。33 ステージ（無効ステージを含む本家 `loadStageGraphAll` の対象）で、`Orphan stage files: 33 …` / `Schema validation: 33/33 …` / `Graph references: 122 …` が corpus とバイト一致する。
4. **状態版の分類は U2 の `StateVersionClassification` を使い、診断側で再実装しない**。合成ルートの `DomainStateVersionClassifier` が分類結果に本家文言（`wording`）を添えてクエリ側の写しへ変換する。
5. **観測 VO は分類の答えを持つ**。ドメインの `StateVersionObservation` は `StateVersionClassification` ではなく `StateVersionKind` + message を保持する。`classify(&str)` 以外の構築口が無く、観測 View から組み立てられないため。ドメインへ検査を迂回する構築口を足すより、観測が観測した事実を持つ形を採った。
6. **診断は正本を修復・再初期化しない**。記録後の投影も通常の RMU 反映だけで、`restore_missing_files` は呼ばない（C7 `automatic_repair: forbidden`）。

## テストとカバレッジ

TDD（red → green → refactor）で進め、各段の失敗出力と成功を `tdd-logs/01`〜`17` に残した。

| 境界 | テスト | 件数 |
| --- | --- | --- |
| ドメイン集約の判定 | `core-command-domain --test workspace_doctor_contract` | 10 |
| リポジトリ（保存・再構成・一時ストア） | `core-command-interface-adapter --test workspace_doctor_repository_contract` | 8 |
| コマンド側ユースケース | `core-command-use-case --lib`（診断分） | 6 |
| RMU 投影 | `core-read-model-updater --test workspace_doctor_projection_contract` | 8 |
| 観測 DAO | `core-query-interface-adapter --test doctor_observation_contract` | 8 |
| リードモデル DAO | `core-query-interface-adapter --test doctor_report_dao_contract` | 6 |
| 表示専用ユースケース | `core-query-use-case --test doctor_report_contract` | 7 |
| CLI 契約（入口・書式・終了・副作用・corpus） | `aidlc --test doctor_contract`（debug / release 各 10） | 10 |

本家対応は `upstream_doctor_observations_agree_on_adopted_rows_and_exit_codes` が corpus 19 観測の採用行バイト一致・非採用行の不在・終了値を固定する。副作用は DC1（監査なしで何も作らない）・DC2（`HEALTH_CHECKED` 1 件）・DC10（記録失敗でも診断出力が残り終了 1、`journal` 行数不変）で固定した。

工程末の検査（親が実測）:

| 検査 | 結果 |
| --- | --- |
| `cargo fmt --all --check` / `cargo clippy --workspace --all-targets -- -D warnings` / `cargo lint` | 成功 |
| `tools/lint` の fmt / clippy / test | 成功 |
| `cargo audit`（本体・`tools/lint`） | 成功 |
| `bun scripts/aidlc-sync.ts --check`、ハーネス bun テスト 7 ファイル | 成功 |
| ゴールデン bun テスト 5 ファイル、`verify-corpus.ts tests/golden/upstream-a277af21` | 成功 |
| `bash scripts/quint-gate.sh` | 成功 |
| `cargo test -p aidlc --release --test doctor_contract` | 成功 |
| `cargo test --workspace --no-fail-fast` | 3,632 成功 |
| カバレッジ（絶対床 90.0%、seed `20260823`、除外 `main.rs`） | **line 98.0666%、床 PASS**。113 テストバイナリ・**3,666 件成功・0 失敗** |

カバレッジは `scripts/coverage.sh` を書き換えず、同スクリプトと同じ設定（`ABSOLUTE_THRESHOLD=90.0`、`PROPTEST_RNG_SEED=20260823`、`--ignore-filename-regex '(^|/)modules/app/aidlc/src/main\.rs$'`）のまま `cargo llvm-cov` を直接呼び、`--no-fail-fast` だけ足して計測した。共有機の負荷依存の揺れ（下記）で計測が 2 回とも途中終了したためで、しきい値・シード・除外は変えていない。この計測実行自体は失敗 0 件で完走している。

## 承認済み計画からの逸脱

1. **判定の置き場所**。承認済み `code-generation-plan.md` §「責任と配置」2 は D1–D5 の判定を Query use case に置いた。これは `cqrs-boundaries.md` 規則 6 に反する計画側の誤りであり、裁定 Q2 でコマンド側集約へ移した。計画は凍結成果物なので本文は書き換えず、[relocation-brief.md](relocation-brief.md) に逸脱と目標の配線を記録した。計画の 1・3・4 は有効のまま。
2. **診断イベントの置き場所**。裁定 Q3 = A の括弧書き後半「記録があるときは既存ストアへ追記」は実装できなかった。DC10 は空間ストアの `journal` に `BEFORE INSERT … RAISE(ABORT)` を仕込んだうえで報告全文・終了 1・`journal` 行数不変を要求するので、診断イベントをそこへ追記すると報告が組み上がる前に倒れる。残る候補の runtime ストアは、不在時に `--doctor` が新しい永続ファイルを作るため `automatic_repair: forbidden` と DC1 に触れる。C7 の `effects` 自体が `initialized_record: HEALTH_CHECKED_via_U2_command_and_RMU` と定めているため、毎回一時ストアで回す形を裁定 Q4 = A で確定した。
3. **未使用の口の撤去**。上記 2 の帰結として呼び手の無くなった `WorkspaceDoctorRepositoryImpl::open(&StorePath)` を、`no-backward-compatibility.md` に従って撤去した。その契約テスト 6 件は削除せず `open_ephemeral()` へ移植し、件数は 8 件のまま。関連して外部に呼び手のいなくなった `WorkspaceDoctorSqliteStore` の公開も畳んだ（`module-visibility.md`）。
4. **性能**。診断 1 回に ES 往復（一時ストアへの書込 → 再投影 → 読取）が加わり、`doctor_contract` の所要が debug 17.22s → 19.82s（+15%）、release 4.49s → 5.43s（+21%）になった。外部の観測契約（行・書式・終了コード・副作用）は不変。

## 限界と申し送り

- **本リポジトリでの doctor 成功と切替は未達**。U3 の受入は「対象とした正常/異常構成を実際に識別すること」であり、本リポジトリでの `target/release/aidlc --doctor` 成功（切替条件 4）は U4・切替工程の達成条件として残る。本リポジトリの `.claude/settings.json` は `statusLine` が `statusline-combined.sh` を指すため D2.a の名指しは 16 本で、`a277af21` の 17 本と異なる。
- **D2.e の接続定義は U4 待ち**。`aidlc hook <name>` の command 形が確定したら判定基準を追従する。`statusLine` を native 面へ寄せるかも U4。
- **D5.a は版の一致を検査していない**。クエリ側は RMU の `READ_SCHEMA_VERSION` をクレート境界の向こうに持つため、必要表の存在と `PRAGMA user_version != 0` で判定する。
- **表示順は C7 の表順**。本家は shell → heartbeat、`State Version` は heartbeat の直後で並びが違う。行の綴りは一致するので、corpus 比較は集合比較 + C7 表順の検査にした。
- **`audit-locked` は比較対象外**。本家の mkdir ロック（`acquireAuditLock`）は TS 固有で、この build に対応する競合面が無い（記録失敗は SQLite 経路で DC10 として固定）。
- **DTO 4 本に外部の呼び手がいない**。`WorkspaceDoctorAggregateKeyDto` など 4 本は、`dto` mod が私有のため `pub use` を外すと `unreachable_pub`（deny）に当たり、構造体側を `pub(crate)` へ降格する必要がある。HookHealth / ArtifactAudit など既存 DTO 群と同じ形なので、doctor だけ崩さず横断の別件として申し送る。
- **試験装置の負荷依存の揺れ**。共有機（ログイン 28 人、load average 4.5–9.7）での全通し・計測実行では、子プロセスが `unix_wait_status(9)`（SIGKILL・出力空）で落ちる、SQLite が `WouldBlock` になる、といった失敗が `review_guards_contract` / `learnings_contract` / `pipeline_link_contract` で出る。いずれも単独再実行では成功し（`pipeline_link_contract` 12 件成功、当該 1 件は 3 連続成功）、失敗の 4 例はすべて U2 系の経路で doctor とは無関係である。U2 が F-X2 として記録した現象と同じ形。最終の計測実行（3,666 件）では 1 件も再現しなかったため、再現は確率的である。恒久対策（子プロセス起動と SQLite ロック待ちの設定）は U3 の範囲外とし、横断の別件として申し送る。

## Sources

- [要求書](../../../inception/requirements-analysis/requirements.md) FR5・FR6・NFR1–NFR4、[契約書](../../../inception/contract-design/contract-summary.md) C5・C7、[単位定義](../../../inception/units-generation/unit-of-work.md) U3
- [code-generation-plan.md](code-generation-plan.md)、[unit-test-instructions.md](unit-test-instructions.md)、[relocation-brief.md](relocation-brief.md)
- [doctor-placement-questions.md](doctor-placement-questions.md)（裁定 Q1–Q4）、[implementation-verification.md](implementation-verification.md)、[progress-status.md](progress-status.md)
- `tdd-logs/01`〜`17`、[source-manifest.json](source-manifest.json)、[traceability.json](traceability.json)
- `aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/`（`cqrs-boundaries.md`、`use-case-rules.md`、`gateway-taxonomy.md`、`no-backward-compatibility.md`、`domain-persistence-neutrality.md`、`module-visibility.md`）
- 本家固定コミット `a277af218f0df7f325d3b8be7b6d90fce2c5bd40` の `aidlc-utility.ts` / `aidlc-lib.ts` / `aidlc-graph.ts` / `aidlc-stage-schema.ts`、`tests/golden/upstream-a277af21/doctor/cases.json`
