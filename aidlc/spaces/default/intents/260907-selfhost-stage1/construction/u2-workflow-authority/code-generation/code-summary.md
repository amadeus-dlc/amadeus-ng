# U2（u2-workflow-authority）コード要約

2026-09-12。承認済み [実装計画](code-generation-plan.md)（指紋 `sha256:b83cf443…`、Testing Contract `sha256:d904f82d…`）の Step 1〜9 を着地した時点の要約。進捗の時系列は [progress-status.md](progress-status.md)、再開手順は [current-resume-note.md](current-resume-note.md)。

## 作成・変更したファイル

作業ツリーの変更は [source-manifest.json](source-manifest.json) に 1,330 経路（削除済み旧コーパス 2 本はディレクトリ claim）として申告した。U1 が申告した経路（`scripts/goldens/`、`.github/workflows/ci.yml`、`tests/golden/upstream-a277af21/`、記録用フック）を U1 完了後に U2 が変更しているため、U1 の終端レビュー受領の失効はエンジンの判定に従う。

| 領域 | 主な内容 |
| --- | --- |
| `modules/core/command/domain/` | `IntentExecution` の報告事実（Reported）、計画承認ランタイム、継続トークン、レビュアー越境、書込み先、学習観測などの集約・値オブジェクト。1 ファイル 1 公開型 |
| `modules/core/command/use-case/` | `CommitVerdictUseCase` の CQS 是正（成功戻り値 unit）、計画承認・継続・レビュー・学習・診断・単独実行の各ユースケースとエラー型 |
| `modules/core/command/interface-adapter/` | SQLite Repository 実装（実行・計画承認・継続・セッション監査・成果物監査・診断）と保存 DTO。配布 scope の frontmatter 読取（`keywords:` ブロック列） |
| `modules/core/read-model-updater/` | 報告結果・計画承認・継続・診断・runtime-graph の投影、状態/監査ファイルの描画（`<project-dir>` 伏せ字、状態欄のタブ）、読取 DTO |
| `modules/core/query/` | 報告結果・計画承認操作・診断・成果物監査の View / DAO（ドメイン非依存） |
| `modules/harness/claude/` / `modules/harness/infrastructure/` | Claude フックの JSON 封筒（停止制御・人間応答・状態遷移保護・保存監査・規則受渡し・review-freeze・reviewer-scope 等）とシェル文の解析 |
| `modules/app/aidlc/` | CLI 入口（`next` / `continue` / `report` / `intent-create` / `log` / `hook` 等）、print directive の本家逐語化、`next --stage` の execute 名指し、`Layout::shared` の lone-intent 後退、TaskUpdate 同期、learnings surface / persist、fold-usage |
| `tests/golden/` | 旧 `upstream-3c3146cf` / `supplemental-3c3146cf` を削除し、`upstream-a277af21`（`corpus-manifest.json` 再封印、`reviewer-scope/` 追加）と `selfhost-stage1/`（jump fixture）へ移行 |
| `tests/support/` | `coverage_profile_env.rs`（llvm-cov 計測下の `.profraw` 引き継ぎ）、`tool_link.rs`（契約テストのバイナリ配置をリンクへ） |
| `scripts/coverage.sh`、`.github/workflows/ci.yml`、`scripts/governance/verify-ci-governance.sh` | 相対ゲートの廃止（裁定 [coverage-gate-questions.md](coverage-gate-questions.md) Q1 = A）。絶対床 90% は維持 |
| `.claude/settings.json` | `env` / `model` / `effortLevel` 22 行の削除は利用者自身の設定変更（Bedrock 不使用）。U2 の実装変更ではない |

## 主要な実装判断

- **CQS**: 報告の成功戻り値を `Result<(), E>` にし、結果は report_id を自然キーとする読取りモデルを RMU が投影し Query で取得する。表示材料を返す別経路は残していない。
- **本家逐語**: print directive・状態/監査の描画・フック文言は固定コミット `a277af21` の採取コーパスとバイト一致を契約テストで固定した（classic 先頭 4 ケース、hook parity 165 件、fold-usage 40 観測、stage-rules 89 件中 80 件完全一致等）。
- **人間の裁定で決めた差**（すべて記録あり）: jump-redo（Q1 = A）、Step 8 D1〜D6（step8-decisions Q1〜Q4 = A）、フック差（hook-closeout Q1〜Q4 = A）、per-unit 受領証（step7-divergence Q1 = B）、lone-intent 後退（F-H1 = B: 本家に合わせ `aidlc-state.md` の有無で判定）、`keywords:` ブロック列（Q1 = A: 本家に合わせ読取追加）、`Display` 混入（Q2 = A）、dead code（Q3 = A: 型・不変条件で到達不能な 13 箇所を削除、24 箇所は残置）、カバレッジ相対ゲート（Q1 = A: 廃止）。
- **テスト装置**: 契約テストが 39 MB のバイナリを試験ごとにコピーして起動していたのを、ハードリンク優先の `tool_link` に置き換えた（macOS の初回起動検査で 1 起動十数秒かかっていた）。`upstream_271_contract.rs` はバイナリがワークスペース内の Source Baseline 走査対象になるため実体コピーのまま。

## テストとカバレッジ

| 検査 | 結果 |
| --- | --- |
| `cargo test --workspace`（`coverage.sh` の計測実行） | **3,599 件成功・0 失敗** |
| `cargo fmt --all --check` / `cargo clippy --workspace --all-targets -- -D warnings` / `cargo lint` / `tools/lint` | 成功 |
| `bash scripts/quint-gate.sh` | 全 29 ステップ成功 |
| `cargo build --release` + `cargo test -p aidlc --release --test upstream_271_contract` | 75 件成功 |
| `cargo audit`（workspace と `tools/lint/Cargo.lock`） | 成功 |
| CI の bun テスト群 12 ファイル、`verify-corpus.ts`、`aidlc-sync.ts --check` | 172 件成功、成功、同期済み |
| workspace 行カバレッジ | **98.55%**（着手時 94.77%。5 + 2 担当でテスト約 480 件を追加）。絶対床 90% PASS。相対ゲートは裁定で廃止 |

カバレッジ計測下でのみ落ちる契約テスト（`.profraw` の副産物）は `coverage_profile_env` で是正し、閾値・アサートは変更していない。負荷依存の揺れ 2 件（`pipeline_link_contract::concurrent_duplicate_completions_persist_only_one_receipt`、`review_guards_contract::only_the_dispatched_reviewer_is_enforced_against`）は単独再実行で成功し F-X1 / F-X2 として記録した。

## 計画からの逸脱と残課題

- **相対ゲートの廃止**は承認済み計画「90%床と相対ゲートを確認する」からの逸脱で、利用者の裁定（Q1 = A）による。`team.md` Testing Posture / `project.md` Mandated の「相対ゲートを維持」は直接編集せず、§13 学習記録で改訂を提案する。
- **ゼロに届かない未カバー行**（約 1,300 行）は、担当報告のとおり一度も呼ばれない 1 行クロージャ・I/O 失敗時のみの分岐・テストコード自身の `panic!` 腕が大半。dead code 候補のうち残置した 24 箇所と 2 回目の追加候補は各 `coverage-logs/*-verification.md` に根拠付きで残した。
- **切替条件 2（状態・監査の upstream 互換）の判定材料**として記録した観測差: F-C1（`next --compose` の print 文面の構造）、F-C2（gate-start の残り 3 ガード）、F-C3（Brownfield の `consumes` 解決先）、Q2〜Q4 の各 A（`tool_name` null の exit、stderr の OS エラー文言、完了監査の利用量欄）、per-unit 受領証で本 build のほうが厳しい 1 分岐、`keywords` の引用符剥がしとフロー列の字句解析の細部差、`source_baseline::read` が採取失敗を握り潰す点。一覧は [switchover-condition2-findings.md](switchover-condition2-findings.md) と各報告書。
- **追加の裁定候補**（U2 では実装せず記録）: `ResumeMenu` 変種の廃止、`PlanApprovalRuntimeId` 単一変種ゆえの `foreign approval` 検査、`AuditFieldKey` 無謬コンストラクタ、同一監査シャードへの複数追記計画の `PublicationConflict`、DTO 拒否形の不揃い、`scan_range` の manifest 非選別。
- **後続の達成条件**（U2 の完了で代用しない）: 実際の Claude Code と人間による実地スモーク、`target/release/aidlc` の自己診断成功、CI 全ジョブ成功、安定タグへの切替。

## Step 9（2026-09-13 改訂）: 未配線 8 サブコマンドの実装

bugfix 実地スモークが踏むのに Rust バイナリへ未配線だった 8 サブコマンドを TDD で配線した。権威資料 `scripts/aidlc-selfhost/required-surface.json` で 8 件すべてが `status: not-wired` かつ `bugfix_required: true` だったものを `wired` へ更新し、`bugfix_required` かつ未配線の動詞は 0 件になった（`distributed-ts` 枠は `aidlc-review-brief` の 3 動詞のみで不変）。

| 群 | 動詞 | 配置 |
| --- | --- | --- |
| A | `lookup` の phase-of / agent-for / validate-stage / next-stage、`scope-table`、`stage-table` | クエリ側（静的読取 DAO と View） |
| B | `project-description` | クエリ側（state とサイドカー） |
| C | `codekb-path`、`codekb-scope-diff` | クエリ側（読取専用、verdict 経路は常に exit 0） |
| D | `codekb-snapshot`、`codekb-publish` | コマンド側（ロック・CAS・原子的 rename） |

### 検証

- CLI ゴールデン: `workflow_authority_golden` 16、`codekb_authority_golden` 4、`codekb_write_golden` 3（採取 24 ケースの stdout バイトと終了コードを上流と突合）
- 層内: `tree_hash` 10（上流実測の `treeGeneration` 3 ベクタを含む）、ドメイン契約 23、ユースケース 10、Gateway 契約 9、`required_surface_contract` 7
- 静的: `cargo fmt --all --check`、`cargo clippy --workspace --all-targets -- -D warnings`、`cargo lint` いずれも exit 0
- 全体回帰 `cargo test --workspace` は Step 10 の担当であり本工程では未実行

### 承認済み計画からの逸脱

- `codekb-scope-diff` を計画は「どのモード・どの verdict でも exit 0」としたが、ピン `a277af21` の実測では usage 経路（`--mint` の `--paths` 欠落、`--compare` の対象欠落）が exit 1 だった。計画文言より上流実測を優先し upstream-exact で実装した。計画本文は凍結成果物のため書き換えていない。

### 是正した欠陥

- **群 A の来歴不整合**: 採取スクリプトが submodule 作業ツリー（fork `801c5700`）を実行しながら来歴へ `upstream_commit: a277af21` と記録し、ピンとの一致検証を持たなかった。ピンを `git archive` で一時ディレクトリへ実体化して実行し、実体化できなければ失敗する方式へ変更した。上書き前の実測でピン vs 既存ゴールデン 0/16 差分、ピン vs 作業ツリー 0/16 差分を確認し、再採取後もバイトは 1 件も変わらなかった。兄弟スクリプトは既に `git archive` 方式で、欠陥は群 A のみだった。
- **CQS 違反**: 群 D の初版が中断トランザクションの復旧（ディレクトリ rename）を `CodekbRepository::find_by_id` の内側に隠していた。利用者の裁定により、ポートへ `recover_interrupted_publications`（`&mut self` + `Result<(), E>`）を独立コマンドとして新設し、`find_by_id` を `observe_generation` だけの純粋な読取へ戻し、両ユースケースがロック区間の先頭で明示的に復旧を呼ぶ上流と同じ順序へ是正した。採取 24 ケースのバイトと `tree_hash` の上流実測 3 ベクタは不変。

### 既知の限界（利用者裁定により繰延）

- 失敗経路の stderr エンベロープ（上流 `{"error":..}`）と、状態ファイル存在時の `ERROR_LOGGED` 監査副作用は native に無い。標準出力空・終了コード 1 は一致し、bugfix スモークは有効入力で呼ぶため踏まない。切替条件 2 の判定前に要裁定として別 Bolt へ繰延した。差は広げていない。
- CLI ゴールデンは合成フィクスチャに対する採取であり、実グラフ 33 ノード・実 scope でのバイト一致は証明していない。
- `scope-table --check` / `stage-table --check` は実装対象外とした。本リポジトリの CI・スクリプト・フック・Rust テストのどこからも呼ばれず、スモークは plain 形のみ使う。
- doctor へのこの 8 動詞の配線検査追加は対象外（U4 以降へ引き継ぐ）。

### 未解決事項

- `find_by_id` に副作用が隠れている面が他の Repository にも無いか、横断点検が要る。`project.md` の規則により AI の判断だけで GitHub Issue を起票しないため、人間の裁定を待つ。

## Sources

- [code-generation-plan.md](code-generation-plan.md)、[unit-test-instructions.md](unit-test-instructions.md)、[traceability.json](traceability.json)、[source-manifest.json](source-manifest.json)
- 担当報告: `*-verification.md`（`parity-closeout`、`lone-intent-upstream`、`keywords-display`、`dead-code`、`coverage-logs/u2_cov*`）と各ログディレクトリ
- 検査ログ: `step9-logs/`

## Assumptions & Open Questions

None.
