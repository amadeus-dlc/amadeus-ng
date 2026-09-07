# U1 / U4 / U9 / U10 記録の裁定サーベイ

R = `aidlc/spaces/default/intents/260822-stage1-selfhost/construction`、K = `aidlc/spaces/default/knowledge/aidlc-shared/coding-rules`（パスはリポジトリ相対）。

## 裁定台帳

| # | 日付 | 裁定（1文） | 出典 path:line | 影響する仕様・共有契約の節 | コードで確認すべき点 |
|---|---|---|---|---|---|
| 1 | 2026-08-29 | ユースケース層とアダプタ層をコマンド側・クエリ側に完全分割する | `R/u4-read-model-updater/crate-structure-proposal.md:6`, `:11`, `R/u4-read-model-updater/brief-1.md:9` | 01号 §7、10号 §3、11号 §3、contract-summary C3 | `modules/core/command/{domain,use-case,interface-adapter}` と `modules/core/query/{use-case,interface-adapter}` の実在 |
| 2 | 2026-08-29 | ドメインはコマンド側の持ち物で `core-command-domain` とする | `R/u4-read-model-updater/crate-structure-proposal.md:16`, `:42`, `R/u4-read-model-updater/developer-report-2.md:28` | 01号 §7 原則1、11号 §2.2 | パッケージ名 `core-command-domain`、`core-domain` の不在 |
| 3 | 2026-08-29 | 旧記述「`core-domain` は両側が依存してよい共有層」→ 失効 | `R/u4-read-model-updater/developer-report-1.md:31` → 失効（`R/u4-read-model-updater/developer-report-2.md:28`, `crate-structure-proposal.md:42`） | 同上 | クエリ側 `Cargo.toml` にドメインが無いこと |
| 4 | 2026-08-29 | RMU はコマンド側でもクエリ側でもない「中間」で、側接頭辞を持たない | `crate-structure-proposal.md:17`, `:31`, `:59`, `developer-report-2.md:29`, `K/cqrs-boundaries.md:269` | 11号 §2.3、contract-summary C3/C6 | パッケージ `core-read-model-updater`、`modules/core/query/read-model-updater` の不在 |
| 5 | 2026-08-29 | 旧名 `core-query-read-model-updater` → 失効 | `developer-report-1.md:33` → 失効（`developer-report-2.md:29`） | 同上 | 同上 |
| 6 | 2026-08-29 | クエリ側クレートはドメインに絶対依存しない | `crate-structure-proposal.md:19`, `:63`, `K/cqrs-boundaries.md:270` | 10号 §3、11号 §3 | `core-query-*/Cargo.toml` にコマンド側が現れないこと |
| 7 | 2026-08-29 | `JournalReaderImpl` は RMU クレートが所有する（SQLite 依存は RMU の本質的結合） | `crate-structure-proposal.md:34`, `developer-report-1.md:70`, `K/cqrs-boundaries.md:262` | 10号 §3 ポート表、contract-summary C6 | `modules/core/read-model-updater/**/journal_reader_impl.rs` |
| 8 | 2026-08-29 | `infra-io` を `core-infrastructure` へ改名し、`harness-infrastructure` を新設する。infrastructure 層は言語拡張のみで RPC/DB を置かない | `crate-structure-proposal.md:45`, `developer-report-1.md:36`, `K/infrastructure-layer.md:3` | 00-policy D4/A8、01号 §7 | `modules/core/infrastructure` / `modules/harness/infrastructure` |
| 9 | 2026-08-29 | 後方互換ゼロ。旧クレート名の再輸出・shim・`#[deprecated]` を置かない | `R/u4-read-model-updater/brief-1.md:43`, `developer-report-1.md:39`, `R/u4-read-model-updater/brief-4.md:18` | 全号（改名の波及） | `core_use_case` / `core_interface_adapter` / `infra_io` の grep 0 件 |
| 10 | 2026-08-29 | コマンド側とクエリ側は互いの `Cargo.toml` に現れない。旧「アダプタ層は両側の契約を実装してよい」→ 失効 | `crate-structure-proposal.md:67` | 10号 §3、11号 §3 | 両側の `Cargo.toml` 相互不在 |
| 11 | 2026-08-29 | 共有部品の行き先: `StorePath` はドメインの workspace、`EVENT_MANIFEST` は orchestration、`CorruptCause` と `store_failure` は側ごとに専用化 | `crate-structure-proposal.md:73`, `developer-report-1.md:45` | 11号 §2.2、contract-summary C3 | `store_path.rs` / `event_manifest.rs` の所在、側ごとの enum |
| 12 | 2026-08-28 | RMU は二層構造（取得ループと純粋投影核 `project(entries, read_model)`）で、投影核はストレージ・接続・checkpoint を知らない | `R/u4-read-model-updater/brief-1.md:45`, `developer-report-1.md:69`, `R/u4-read-model-updater/functional-design/rules.md:18` | 11号 §2.3、contract-summary C5 | `updater.rs` の `catch_up` と `projection.rs` の `project` の署名 |
| 13 | 2026-08-29 | 投影出力は 0a 逐語契約に一致し、同じ checkpoint から何度流しても同一バイトになる（NFR3） | `R/u4-read-model-updater/brief-1.md:50`, `developer-report-1.md:71` | 11号 §3、NFR3 | 監査 86 語彙の見出し、状態ファイルの tmp+rename |
| 14 | 2026-08-29 | 監査行のタイムスタンプは追記時の壁時計ではなくイベントの発生時刻を使う | `developer-report-1.md:186` | 11号 §3、逸脱台帳 | 投影が `occurred_at` を秒精度 ISO 8601 で書くこと |
| 15 | 2026-08-29 | リードモデルを書いてから checkpoint を進める（at-least-once）。台帳の欠落は重複より重い | `developer-report-1.md:193`, `:310` | contract-summary C5、NFR3 | `catch_up` の書込と `advance_checkpoint` の順序 |
| 16 | 2026-08-29 | 監査行の偽造不能性を型で保証する（`AuditFieldKey` / `AuditFieldValue` / `AuditFields`）。`AuditFields` は挿入順を保つ第一級コレクション | `developer-report-1.md:171`, `:299` | 11号 §2.2 Domain Primitive | ドメイン workspace の 3 型と挿入順保持 |
| 17 | 2026-08-29 | `find_all_events` は順序付けの純関数（ドメイン）とシャード列挙・読取のI/O（投影側）に分割する | `developer-report-1.md:302`, `:339`, `R/u4-read-model-updater/doc-sync-report.md:24` | 11号 §2.3 | `audit_ordering.rs` と `audit_shard.rs::read_all` の分離 |
| 18 | 2026-08-29 | 状態ファイル・監査ブロックの描画（`render_audit_block` / `state_writers`）は投影（RMU）の責務でドメイン層ではない | `R/u9-canon-docs/code-generation/developer-report-2.md:81`, `doc-sync-report.md:24` | 11号 §2.3 | 描画関数が RMU 側にあること |
| 19 | 2026-08-29 | オーナー裁定 A。担当エージェント名・ステージ番号・表題（`StageDisplay`）と走査結果（`WorkspaceScan`）を `Started` イベントへ焼き込む。ADR-008 の限定的例外 | `developer-report-1.md:251`, `:274`, `:342` | 01号 §3.2、10号 §2.1、contract-summary C5、ADR-008 | `StageEntry.display` と `Started.scan` のフィールド名 |
| 20 | 2026-08-29 | イベントを太らせる案は不採。解決済み計画は投影核の引数 `ResolvedPlan` として渡す | `developer-report-1.md:266`, `:312` | contract-summary C5 | `resolved_plan.rs` が投影核の引数であること |
| 21 | 2026-09-01前後 | オーナー裁定 A。骨格生成は投影の責務外で、書くのは合成ルート（U7）。`ScaffoldTemplateUnavailable` は撤去せず `ScaffoldMissing` へ改名 | `R/u1-golden-recapture/developer-report-1.md:149`, `:156`, `:188`, `:336` | 11号 §2.3、NFR3 の適用範囲 | `ProjectionError::ScaffoldMissing` の存在と doc |
| 22 | 同上 | 骨格の正本は `cli/intent-create/classic-scope/state-full.md`（102行）。B8 §8-1 の「ゴールデンは差分しか持たない」→ 失効 | `R/u1-golden-recapture/developer-report-1.md:41`, `:36` → 失効元 `R/u4-read-model-updater/developer-report-1.md:330` | contract-summary C7 | ゴールデン配下の `state-full.md` |
| 23 | 同上 | ジャンプのフェーズ境界は `Jumped` の source/target と計画から導出し、イベントは 1 バイトも拡張しない | `R/u1-golden-recapture/developer-report-1.md:142` | contract-summary C5 | `Jumped` の payload 不変 |
| 24 | 同上 | `STAGE_COMPLETED` の Details は `Stage <表示名> completed` で、ゲート経由の `approved by gate` と文言が割れる（書き手が違うので寄せない） | `R/u1-golden-recapture/developer-report-1.md:72` | 11号 §3 逐語契約 | 2 経路の文言分離 |
| 25 | 同上 | `AUTONOMY_MODE_SET` の成功経路は upstream ピンで到達不能。フィールドキー `Mode` は暫定のまま `cases-missing.json` に理由を記録 | `R/u1-golden-recapture/developer-report-1.md:38`, `:102` | contract-summary C7 | 暫定である旨の doc コメント |
| 26 | 2026-09-06 | 公開計画を先に SQLite へ保存し、ファイル反映後に構造化リードモデル・チェックポイント・完了記録を同一 Tx で確定する（`prepare` と `publish_prepared` の分離） | `R/u4-read-model-updater/implementation-report.md:9`, `:49`, `R/u4-read-model-updater/functional-design/rules.md:66`, `:82` | 11号 §2.1、contract-summary C5/C6 | `publication` 系表と 2 Tx 構成 |
| 27 | 2026-09-06 | 1 回の `catch_up` は最大 2 計画（保存済み終点を先に確定し、追加分は別計画・別 Tx）。runtime に駆動ループを置かない | `implementation-report.md:14`, `R/u4-read-model-updater/functional-design/entities.md:99`, `rules.md:34` | 11号 §2.3 | `catch_up` の戻り値と U7 側にループが無いこと |
| 28 | 2026-09-06 | 共有リードモデル面 `amadeus_read_model_head` が位置・世代・規約版・内容ダイジェスト・検証状態を持つ。同位置は SAVEPOINT 内の行集合一致で維持し、不一致は破損として停止 | `R/u4-read-model-updater/functional-design/entities.md:112`, `rules.md:146`, `R/u4-read-model-updater/functional-design/pending-revision.md:17` | 11号 §2.1、contract-summary C6 | 同名テーブルと比較ロジック |
| 29 | 2026-09-06 | `catch_up_before_reading` は失敗を握りつぶさない。`next` / `resume` は error directive で exit 0、`report` / `practices_promote` / `set_autonomy` は refused で exit 1 | `implementation-report.md:21`, `pending-revision.md:28`, `rules.md:114` | 10号 §3、11号 §4 | CLI 各動詞の終了コード |
| 30 | 2026-09-06 | CLI の初回構造化投影とファイル投影のチェックポイントを分ける | `implementation-report.md:20`, `rules.md:98` | 11号 §2.1 | 個別チェックポイントの 2 系統 |
| 31 | 2026-09-07実測 | RMU は `read_*` 17 表と `amadeus_projection_checkpoint` / `amadeus_read_model_head` / `amadeus_publication*` 5 表を持つ | `R/u9-canon-docs/functional-design/gap-measurement-20260907.md:23` | 11号 §2.1、contract-summary C6 | 表名と 17 表の内訳 |
| 32 | 2026-09-06 | canon-json の所在は `core-infrastructure::canon_json`。旧 `shared/canon-json` へ戻す別名や互換ラッパーを作らない | `R/u1-canon-json-goldens/nfr-requirements/tech-stack-decisions.md:13`, `R/u1-canon-json-goldens/nfr-design/logical-components.md:11`, `R/u1-canon-json-goldens/code-generation/code-summary.md:14` | 01号 §7、contract-summary C7 | `modules/core/infrastructure/src/canon_json/` |
| 33 | 2026-08-22 | 契約 JSON の直列化と型付き値変換は canon_json を通す。`serde_json` の `to_string` / `to_vec` / `to_writer` 系と `to_value` の直接呼出は禁止 | `R/u1-canon-json-goldens/functional-design/rules.md:63`, `tech-stack-decisions.md:22` | contract-summary C7 | `clippy.toml` の disallowed-methods と局所 allow |
| 34 | 2026-08-22 | ダイジェストは 2 族。正準族は `sha256:` 接頭辞付き、非正準族は compact 出力の生 hex | `R/u1-canon-json-goldens/functional-design/rules.md:55`, `code-summary.md:38` | contract-summary C7 | `digest.rs` / `digest_family.rs` |
| 35 | 2026-08-22 | 3 プロファイル（contract-pretty / contract-compact / hash-canonical）で体裁とキー順を固定する。整数形式キーは全プロファイルで数値昇順の先頭 | `R/u1-canon-json-goldens/functional-design/rules.md:15`, `:23`, `:47` | contract-summary C7 | `canonical.rs::member_order`、`profile/` |
| 36 | 2026-08-22 | ゴールデンは upstream ピン `3c3146cf` の実出力・実ハッシュに限る。保存先は `tests/golden/upstream-3c3146cf/`、更新はピン更新 intent でのみ | `R/u1-canon-json-goldens/functional-design/rules.md:80`, `:112`, `code-summary.md:59` | contract-summary C7 | コーパスの配置と `provenance.json` |
| 37 | 2026-08-22 | 非決定値は `<TS>` / `<CLONE>` / `<ROOT>` / `<SESSION>` の 4 プレースホルダへ正規化し、期待値と実測値に同じ規則を当てる | `R/u1-canon-json-goldens/functional-design/rules.md:88` | contract-summary C7 | `normalization.json` と比較器 |
| 38 | 2026-09-06 | 配布定義の読み書きは `CompiledDefinitionRepositoryImpl` が担い、型付き DTO の読取と canon_json 経由の契約 JSON 書出しを区別する | `R/u1-canon-json-goldens/correction-report.md:26` | 12号 §2.1/§5 | 同名の実装と `serde_json::from_str` の位置 |
| 39 | 2026-08-23改訂 | 必須チェックは check / quint / coverage / CI Success の 4 コンテキストで strict 有効 | `R/u10-ci-governance/revision-baseline-20260906.md:13`, `R/u10-ci-governance/code-generation/superseding-decisions.md:26`, `R/u10-ci-governance/nfr-design/security-design.md:29` | 仕様外（CI 規範） | `.github/workflows/ci.yml` の集約ジョブ |
| 40 | 2026-08-22 | `unsafe_code = "forbid"` は `[workspace.lints.rust]` で宣言し全メンバーが継承。detached の `tools/lint` は個別宣言する | `R/u10-ci-governance/nfr-design/security-design.md:91`, `R/u10-ci-governance/code-generation/code-summary.md:127` | 00-policy | ルート `Cargo.toml` と 10 メンバーの `[lints] workspace = true` |
| 41 | 2026-08-22 | ツールチェーンの正本は `rust-toolchain.toml`（1.95.0、rustfmt/clippy/llvm-tools、minimal）。CI は `toolchain-inputs.sh` で導出する | `security-design.md:89`, `code-summary.md:117` | 仕様外 | ワークフローに版を書き写していないこと |
| 42 | 2026-08-23 | カバレッジは絶対 90% 床、相対許容差 0.01pt、シード 20260823、除外は `main.rs` 1 ファイルのみ | `security-design.md:97`, `code-summary.md:113`, `revision-baseline-20260906.md:18` | 仕様外 | `scripts/coverage.sh` の 3 定数 |
| 43 | 2026-08-23 | 暫定 `TOLERANCE=0.05` と単独アンカー除外 regex `^modules/...` → 失効。現行は 0.01 と `(^\|/)...` | `superseding-decisions.md:10`, `:11` → 失効（`revision-baseline-20260906.md:18`, `code-summary.md:258`） | 仕様外 | 現行値が 0.01 であること |
| 44 | 2026-08-23 | ワークフロー既定 permissions は `contents: read`。昇格は `review-thread-resolution` ジョブのみ | `security-design.md:41`, `revision-baseline-20260906.md:16` | 仕様外 | `ci.yml` の permissions ブロック |
| 45 | 2026-09-06 | `cargo audit` は workspace と `tools/lint` の 2 つの `Cargo.lock` を検査し、必須チェックにも CI Success 集約にも含めない | `security-design.md:35`, `revision-baseline-20260906.md:15` | 仕様外 | audit ジョブの needs と対象 |
| 46 | 2026-08-23 | FR9.6（エラー様式規則の正本化）は U9 の責務で U10 には含めない | `superseding-decisions.md:15`, `code-summary.md:23` | 仕様外 | `coding-rules/error-handling.md` の帰属 |
| 47 | 2026-08-23 | 01号 §7.1 に「ドメインモデルの原則」5 項を新設する（集約と値オブジェクトが主役、純関数ドメインサービスは消極的、ドメインは永続化責務を持たない、永続化の指揮はユースケース層、集約間は ID 参照） | `R/u9-canon-docs/code-generation/developer-report-2.md:61`, `R/u9-canon-docs/functional-design/rules.md:135` | 01号 §7.1 | 該当節の存在（`docs/specs/01-domain-model.md:272-278` に実在） |
| 48 | 2026-08-23 | workspace の集約は `Intent` / `Space` / `Worktree`。`StateFile` と `AuditShard` はリードモデル、`WorkspaceLock` は退役 | `R/u9-canon-docs/functional-design/rules.md:103`, `developer-report-2.md:56`, `:79` | 01号 §3.3、11号 §2.1 | 未実装（gap-measurement で「予定」と明記が要ると判定） |
| 49 | 2026-08-23 | `PlanAction` の所有は workflow-definition、`CheckboxState` の所有は workspace に一意化する | `R/u9-canon-docs/functional-design/rules.md:77`, `developer-report-2.md:70`, `:80` | 10号 §2.2、11号 §2.2、12号 §2.2 | 再輸出が無いこと |
| 50 | 2026-08-23 | 12号から `next_in_scope_stage` を削除し、畳み込みは集約の `effective_plan` / `next_decision` が所有する | `R/u9-canon-docs/functional-design/rules.md:85`, `developer-report-2.md:93` | 12号 §2.3/§4/§8/§9 | sentinel 8 語の 0 件 |
| 51 | 2026-08-23 | ADR-008 を仕様へ。`WorkflowDefinitionId`（`harness.json` の name）と `DefinitionRevision`（3入力の正準JSONの sha256）を明記 | `R/u9-canon-docs/functional-design/rules.md:95`, `developer-report-2.md:51` | 01号 §3.1、12号 §2.1/§5 | 2 型の実在と付与位置 |
| 52 | 2026-08-23 | ゲート判定は `gated(stage) = phase ≠ initialization`（索引 0 の特別扱いなし）。`Started` は自己完結、`effective_plan` は集約所有 | `R/u9-canon-docs/functional-design/rules.md:111` | 10号 §2.1/§2.3、12号 §4 | ゲート判定の実装 |
| 53 | 2026-08-23 | `IntentId` は UUIDv7 で、記録ディレクトリ名は別値 `IntentDirName` とする | `R/u9-canon-docs/functional-design/functional-design-questions.md:117`, `developer-report-2.md:57` | 01号 §3.3、11号 §2.2 | `intent_id.rs` の UUIDv7 検証、`intent_dir_name.rs` |
| 54 | 2026-08-23 | エラー様式は手実装 enum。`Display` は材料のみ、`thiserror` / `anyhow` 不使用、`# Errors` 必須 | `R/u9-canon-docs/functional-design/rules.md:145`, `R/u9-canon-docs/code-generation/code-summary.md:21` | 01号/10号/11号/12号の相互参照 | `K/error-handling.md` と依存の不在 |
| 55 | 2026-09-05 | オーナー裁定。集約の再構成は「最新スナップショット + それより後の差分イベント」。旧「ジャーナル全再生」の規則は訂正する | `R/u9-canon-docs/functional-design/verification/verification.md:7`, `:46`, `R/u9-canon-docs/functional-design/functional-spec.md:113` | 01号 §3.2、10号 §2.1、components.md 冒頭注記、contract-summary C3 | `IntentExecution::replay(snapshot, events)` |
| 56 | 2026-09-07 | オーナー指示。あいまいな提案をせず、現状のコードを実測してから提案する | `R/u9-canon-docs/functional-design/functional-design-questions.md:92`, `R/u9-canon-docs/functional-design/gap-measurement-20260907.md:3` | 作法（BR5.2/BR5.3） | 該当なし |
| 57 | 2026-09-07 | オーナー回答 A。U9 再走は P1〜P4（設計記録・coding-rules・仕様4号・共有契約2本）を 1 Bolt で行う。検証規律を BR5.3 に規則化する | `R/u9-canon-docs/functional-design/functional-design-questions.md:105`, `:111`, `gap-measurement-20260907.md:115` | 全対象文書 | BR5.3 は `rules.md` に未記載（下表参照） |
| 58 | 2026-09-07実測 | 現行のクレートは 10、集約は 4（`Intent` 7属性 / `IntentExecution` 12属性・コマンド15・イベント16 / `WorkflowDefinition` / `CompiledDefinition`）、FCC は 16 型 | `gap-measurement-20260907.md:11`, `:12`, `:19` | 01号 §3、10号 §2.1、12号 §2.1、components.md | 各数の実測一致 |
| 59 | 2026-09-07実測 | コマンド側ポートは 4 で `expected_version` 引数は無く、楽観 version は集約の内側にある。`Rehydrated*` 型はコードに 0 件 | `gap-measurement-20260907.md:17`, `:20` | 10号 §3、11号 §3、contract-summary C3 | `IntentExecutionRepository` の署名 |
| 60 | 2026-09-07実測 | テストダブル型 `InMemoryXxxRepository` は存在せず、コマンド側は `XxxRepositoryImpl::in_memory()`、クエリ側は `InMemoryXxxDao` | `gap-measurement-20260907.md:21`, `:47`, `:105` | 10号 §3/§8-3、11号 §7-3、`K/use-case-rules.md:36` | 型名の実在 |
| 61 | 2026-09-02 | クエリ側ユースケースは DAO で View を読んで返すだけで、判断・導出・選択・文言をしない。計算結果は RMU が非正規化リードモデルとして投影する | `K/cqrs-boundaries.md:20`（見出し部の裁定履歴） | 10号 §3、11号 §2.3 | クエリ側に判断が無いこと |
| 62 | 2026-09-03 | DAO は 1 表 1 引当（JOIN も非正規化の焼き込みもしない） | `K/cqrs-boundaries.md:25`, README 表 | 11号 §2.1 | `cargo lint` の `dao-single-table` |

補足。#3、#5、#22、#43 は「後の裁定が前を上書き」した項目で、出典の後段に失効先を併記している。#7 に関連して、B8 開発者報告は `K/cqrs-boundaries.md` の判定表と機械強制欄が矛盾すると主張している（`R/u4-read-model-updater/developer-report-1.md:336`）。現行本文では該当箇所が `K/cqrs-boundaries.md:269-271` に併記されており、是正済みか否かの判定は依頼者側で行うこと。

## coding-rules 一覧

| ファイル名 | 規則の一言 | 裁定日 | 機械強制 | 仕様の相互参照 |
|---|---|---|---|---|
| `abstract-data-type.md` | 土台。AVDM/DP は抽象データ型で、操作で定義され表現では定義されない。1ファイル1公開型 | 2026-08-24（改訂 2026-09-01） | 部分的（`no-public-fields` / `one-public-type`） | なし |
| `aggregate-commands.md` | 集約のコマンドは単一のドメインイベントを返す。decide/apply 分離、拒否はガード付き Err | 2026-08-29 | レビュー基準（lint候補） | 01(1) 10(1) 11(2) 12(2) |
| `aggregate-references.md` | 集約は他集約を ID で参照する。オブジェクト埋め込み禁止 | 2026-08-29 | レビュー基準（lint候補） | 10(2) |
| `command-query-separation.md` | Query は `&self` + 戻り値、Command は `&mut self` | 2026-08-23 | レビュー基準 | なし |
| `cqrs-boundaries.md` | 両側は相互非依存で RMU だけが橋。DAO は 1 表 1 引当 | 2026-08-24（改訂 08-28 / 08-29 / 08-30 / 08-31 / 09-02 / 09-03） | クレート分離 + `dao-single-table` | 01(1) 10(3) 11(2) 12(2) |
| `domain-equality.md` | 同値関係は `Eq`/`PartialEq`。名前付き比較メソッド禁止 | 2026-08-22 | レビュー基準 | 01(1) |
| `domain-object-kinds.md` | 基本は 4 種（エンティティ・値オブジェクト・FCC・ドメインイベント）。ドメインサービス新設は人間の裁定必須 | 2026-09-02 | レビュー基準 | なし |
| `domain-persistence-neutrality.md` | ドメインは永続化知識から中立。DTO はアダプタが所有 | 2026-08-30 | クレート依存 | なし |
| `domain-services.md` | ドメインサービスは最後の手段 | 2026-08-29 | レビュー基準 | なし |
| `error-handling.md` | 手実装エラー enum、`Display` は材料のみ、thiserror/anyhow 不使用 | 2026-08-23（FD Q1 = A） | `missing_errors_doc` ほか workspace lints | 01(1) 10(1) 11(1) 12(2) |
| `factory-naming.md` | 基本コンストラクタ 1 本に構築経路を集約。setter 不使用 | 2026-08-24 | レビュー基準 | 01(1) 10(1) 11(1) 12(1) |
| `field-visibility.md` | フィールドはデフォルト private。例外を認めない | 2026-08-22（改訂 2026-08-24） | `cargo lint`（`no-public-fields`） | なし |
| `first-class-collections.md` | コレクション操作を優先し、イテレータ公開は最後の手段 | 2026-09-06 | 設計・レビュー基準 | なし |
| `gateway-taxonomy.md` | Gateway は Repository と外部システムクライアントの 2 つ。クエリ側は `XxxDao` | 2026-08-22（改訂 08-31 / 是正 09-01 / 昇格 09-02） | `cargo lint`（`port-naming` / `command-side-io`） | 01(1) 10(5) 11(5) 12(4) |
| `infrastructure-layer.md` | infrastructure は言語拡張のみ。RPC/DB を置かない | 2026-08-29 | クレート分離 + レビュー基準 | なし |
| `interior-mutability.md` | 内部可変性は既定で禁止 | 2026-08-23 | レビュー基準 | なし |
| `module-visibility.md` | mod はデフォルト private。利便性の再エクスポート禁止 | 2026-08-22（同日追補） | `unreachable_pub` | なし |
| `no-backward-compatibility.md` | 後方互換のコードを残さない | 2026-08-23 | レビュー基準 | 10(1) |
| `tell-dont-ask.md` | ユースケースからドメインの getter を呼ばない | 2026-08-22（追記 2026-09-05） | `cargo lint`（`checkbox-vocabulary` / `use-case-domain-getter`） | 01(1) |
| `ubiquitous-language.md` | ドメインの名前はユビキタス言語。例外は doc に理由必須 | 2026-08-24 | レビュー基準 | なし |
| `upstream-contracts.md` | 借り物の契約を曲げず、境界で変換する | 2026-08-26 | レビュー基準 | なし |
| `use-case-rules.md` | DIP、スタティックバインディング既定、ユースケース間呼出禁止 | 2026-08-22 | Cargo クレート分離 | 01(1) |
| `good-examples.md` | 規則ではなく実在ファイルの索引 | 作成 2026-08-24 | 該当なし | なし |
| `CONSISTENCY-AUDIT-2026-08-24.md` | 規則間の衝突監査記録（Critical 3 は是正済み） | 2026-08-24 | 該当なし | なし |
| `README.md` | 索引と衝突時の優先順（観測互換 > 例外なし規則 > 土台の目的 > 本文の裁定日 > 表の既定） | 継続更新 | 該当なし | 01(1) |

相互参照の欄は `docs/specs/{01,10,11,12}` 各号の出現行数。`README.md:41-63` の表に載る 22 規則のうち、仕様から一度も参照されていないのは `abstract-data-type` / `command-query-separation` / `domain-object-kinds` / `domain-persistence-neutrality` / `domain-services` / `field-visibility` / `first-class-collections` / `infrastructure-layer` / `interior-mutability` / `module-visibility` / `ubiquitous-language` / `upstream-contracts` の 12 本。

## 仕様への反映が未了と明記された箇所

| 出典 path:line | 何が未了か | 対象の文書 |
|---|---|---|
| `R/u9-canon-docs/functional-design/gap-measurement-20260907.md:35`, `:56`, `:69`, `:80` | 冒頭の B12 読み替え注記と B13 優先順位注記が「全文追従は後続 Bolt」のまま残っている | 01号 / 10号 / 11号 / 12号の各冒頭 |
| `gap-measurement-20260907.md:36-52` | 10号 §2.1 が属性数・コマンド数・イベント数・version の置き場・`next_decision` の署名・memento の 6 点でコードと食い違う（約 20 行） | `docs/specs/10-orchestration.md` |
| `gap-measurement-20260907.md:57-63` | 12号 §2.3/§4/§8/§5 の `WorkflowExecution` 名称と `find_for_intent` 動詞（約 9 行） | `docs/specs/12-workflow-definition.md` |
| `gap-measurement-20260907.md:68-76` | 11号 §2.1 の workspace 集約 3 つが未実装、§3 ポート表の `expected_version`、§3 供給面 4 つが未実装（約 8 行）。「予定」の明記が要る | `docs/specs/11-workspace.md` |
| `gap-measurement-20260907.md:81-89` | 01号 §3.2/§3.3/§7.1 の集約名・属性数・脚注（約 9 行） | `docs/specs/01-domain-model.md` |
| `gap-measurement-20260907.md:95-104` | coding-rules 6 ファイル 10 箇所（旧クレート名・message-catalog・存在しないエラー型・`InMemoryXxxRepository`） | `K/README.md`, `error-handling.md`, `factory-naming.md`, `gateway-taxonomy.md`, `module-visibility.md`, `use-case-rules.md` |
| `gap-measurement-20260907.md:112` | `components.md` にクエリ側・RMU の `read_*` 表・`CompiledDefinition` / `Intent` 集約が存在せず、冒頭注記が「全再生」のまま。全面改訂級 | `inception/domain-design/components.md` |
| `gap-measurement-20260907.md:113` | `contract-summary.md` の C1 / C3 / C4 / C5 / C6 / §4 が v2 世代のまま。C6 に `read_*` / `amadeus_read_model_head` / publication 表が無い | `inception/contract-design/contract-summary.md` |
| `R/u9-canon-docs/functional-design/pending-revision.md:7-15` | BR2.5 の適用範囲、BR1.5 新設、BR5.1 の grep 範囲と diff スコープ、entities と rules の 1:1 対応の 5 項目が未適用 | U9 の `rules.md` / `entities.md` |
| `R/u9-canon-docs/functional-design/functional-spec.md:118-121` | 上記 5 項目と、後続裁定で古くなった BR3.3・BR4.1 が未同期 | 同上 |
| `R/u9-canon-docs/functional-design/functional-design-questions.md:111` | 検証規律を BR5.3 として規則化する約束。`rules.md` に BR5.3 は未記載（grep で 0 件） | U9 `rules.md` |
| `R/u9-canon-docs/code-generation/code-summary.md:71-75` | B5 送り（memento 改名、`audit_lock.qnt` 協定モデル化に伴う 10号 §6 I14 / 11号 §6 W1-W5 / 01号 §3.3 の差し替え、deviations #4 の SQLite パス、`intents.json` 直列化機構） | 01号 / 10号 / 11号 |
| `code-summary.md:75` | 11号 §3 の audit 5 動詞と投影の関係が U4 / U5 送りのまま | `docs/specs/11-workspace.md` |
| `R/u9-canon-docs/code-generation/developer-report-2.md:180-181` | 11号 §2.1 の `Intent` / `Space` トランザクション境界と、§9 の stage-0/1 併用期のロック物理形式互換がオーナー裁定待ちで §10 の未決事項へ移動 | `docs/specs/11-workspace.md` §10 |
| `R/u9-canon-docs/code-generation/pending-revision.md:6-19` | code-summary §2 の行数と unit-test-instructions §1 の検査コマンド書式が未修正 | U9 `code-summary.md` / `unit-test-instructions.md` |
| `R/u4-read-model-updater/developer-report-1.md:336` | `K/cqrs-boundaries.md` の機械強制欄と判定表の矛盾を「委任者が完了後の窓で是正」と記載 | `K/cqrs-boundaries.md` |
| `developer-report-1.md:341` | contract-summary C5 の「同一シャード内で直接行と投影行の順序」が未定義のまま | `contract-summary.md` C5 |
| `developer-report-1.md:343` | 層の一覧を持つ文書に `infra-io` → `core-infrastructure` と `harness-infrastructure` 新設が反映済みか要確認 | `docs/specs/01-domain-model.md` ほか |
| `R/u4-read-model-updater/doc-sync-report.md:60-65`, `doc-sync-report-2.md:80-83` | `unit-of-work.md` U3 の EventStore 独自スキーマ記述（journal/snapshot/checkpoint 3表・`InMemoryWorkflowExecutionRepository`）が ADR-010 由来のドリフトのまま未着手 | `inception/units-generation/unit-of-work.md` |
| `doc-sync-report.md:77`, `doc-sync-report-2.md:83` | contract-summary §4 の `GateApproved` の phase 境界（PHASE_VERIFIED の要否）が未解決 | `contract-summary.md` §4 |
| `R/u1-golden-recapture/developer-report-1.md:345-349` | recompose の `- **Completed**:` の実装要否が実バイトで判別できず未実装。`cli/jump/execute-forward-to-conditional` が投影検収に未接続 | ゴールデン / RMU |
| `R/u10-ci-governance/code-generation/superseding-decisions.md:19-20` | 凍結文書の本文修正はステージゲート前の回復レビューでまとめて行う予定 | U10 の凍結成果物 |
| `R/u10-ci-governance/revision-baseline-20260906.md:29-31` | 要件・設計・実装記録の 3 文書を現行設定へ整合させる作業が後続 | U10 各 `traceability.json` ほか |
| `R/u1-canon-json-goldens/code-generation/code-summary.md:107` | 機能設計 R-08（重複キーの記載不足、Minor）を変更せず引き継ぐ | U1 `functional-spec.md` |

## 読めなかった・判断がつかなかったもの

- `R/u10-ci-governance/code-generation/developer-brief-3.md`（872行）・`developer-brief-4.md`（1248行）・`reviewer-brief-1.md`（595行）、`R/u1-canon-json-goldens/code-generation/developer-brief-1.md`（972行）・`developer-brief-2.md`（967行）は、memory 層の規則を逐語で埋め込む派遣ブリーフ。構造の規範としては本文の裁定と重複するため、裁定の抽出対象から外した。固定裁定の一覧は `R/u4-read-model-updater/brief-1.md:41` と `brief-4.md:16` から採っている。
- `R/u10-ci-governance/code-generation/ruleset/*.json` と `ruleset-observed-20260906.json` は GitHub の ruleset スナップショットで、コードの規範ではないため台帳に含めていない。
- `R/u1-canon-json-goldens/approval-control-repair.md` と `R/u1-canon-json-goldens/summary-confirmation-recovery-20260906.md`、`R/u9-canon-docs/functional-design/verification/save-guard.md` は承認ガード・要約確認の復旧記録で、システムの構造ではなくハーネス運用の話のため除外した。
- `K/cqrs-boundaries.md:269-271` の「RMU はどちらが現れてもよい」が B8 報告の指摘する矛盾として残存しているのか是正済みなのかは、本文だけでは判断がつかなかった。両方を並記している。
