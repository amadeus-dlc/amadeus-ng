# AI-DLC State Tracking

## Project Information
- **Project**: stage-0 から stage-1（セルフホスト切替）へ最短で到達する。 ## 目的と到達条件 amadeus-ng 自身をホストにして、このリポジトリの開発が回る状態（stage-1）に到達する。切替条件は次の 5 つ（センサーとプラグインは含めない）。 1. エンジン `next` / `report` が Claude Code 上でゲート込みで動く 2. 状態・監査が upstream 互換で機能する（ワークスペース配置、監査イベント語彙、状態ファイルの形式、逐語文言） 3. 自プロジェクトの開発で使うスコープ（bugfix / feature 相当）のステージ一式が揃っている 4. `--doctor` 自己診断が green 5. smoke + unit 相当の CI が green Definition of Done は次の 3 つ。 1. 実地スモーク: このリポジトリで bugfix 相当の小さな intent を 1 本、amadeus-ng バイナリ（`target/release/aidlc`）をエンジンにして「開始 → 質問 → ゲート承認 → 完了」まで通す。 2. `aidlc --doctor` が本リポジトリで green。 3. CI 全ジョブ green を維持。 切替後は「ホスト = 直近の安定タグ、ターゲット = 開発版」の 2 版運用。 ## 基準と正本 - 基準はコード。`main` の現行コードを実測して事実を決め、推測で設計しない。過去の intent 記録・codekb・手書きの `docs/` は削除済みで、参照先は存在しない。既存コードの再設計・再文書化は行わない。 - upstream の正本は 2 つで、系譜は同じだが版が違う（2026-09-07 実測）。(1) 配布元 submodule `vendor/aidlc-workflows/` = fork `j5ik2o/aidlc-workflows` の `801c5700`（`AIDLC_VERSION = "2.7.1-j5ik2o.1"`。upstream `3c3146cf` を祖先に持ち、その先に upstream 101 コミット（v2.6.41〜2.7.x）+ fork 独自 4 コミット（Kimi Code 対応・Codex 認証継承・版名の j5ik2o 接尾辞・fork 版順序の文書）が乗る）。`.claude/` の配布資産はこの tip の `dist/claude/` と同一バイト（`stage-graph.json` md5 一致）で、`bun scripts/aidlc-sync.ts` が同期し `scripts/aidlc-sync/installed.json` に記録する。ステージ・エージェント・プロトコル・コンパイル済みグラフはこの配布資産をそのまま使い、ステージ類は書かない。(2) 観測契約のゴールデン `tests/golden/upstream-3c3146cf/` = upstream ピン `3c3146cf`（v2.6.40）の配布実バイト（`cli` 11 動詞、`hooks` 4 本、`hash-canonical`）。(1) と (2) の `stage-graph.json` は md5 が異なる。つまり「バイナリが読む配布資産（2.7.1 系）」と「バイナリの受入基準（2.6.40 系）」がずれているので、stage-1 の切替条件 2 を判定する前に、ゴールデンを fork tip へ再採取するか 2.6.40 に据え置くかを人間に裁定してもらう。 - 設計規則の正本: `aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/`（README の衝突優先順を含む）と `aidlc/spaces/default/memory/`（org / team / project は初期テンプレート。チーム規則は practices-discovery で再確認する）。 - 文書の置き場は AI-DLC ネイティブに限る: 規則は `memory/`、参照資料は `knowledge/documents/`（`/aidlc knowledge onboard` で目録化）、横断知識は `knowledge/aidlc-shared/`、ステージ成果物は intent 記録。リポジトリ直下に手書きの文書ツリーを作らない。 ## 現状（stage-0、2026-09-07 実測） | 面 | 状態 | |---|---| | CLI（`modules/app/aidlc`） | 配線済み: `aidlc next / continue / report / park`、`aidlc-utility intent-create`、`aidlc-log review`、`aidlc-state practices-promote`、`aidlc-bolt set-autonomy`。未配線: `aidlc-log decision / answer / link`、`aidlc-bolt` の他の動詞 | | フック 4 本（stop-forwarding-loop / record-human-turn / state-transition-guard / write-audit-log） | 未実装。ゴールデンは採取済み。置き場は `modules/harness/claude` と `modules/harness/infrastructure`（いずれも空の `lib.rs`） | | `--doctor` | 未実装 | | 永続化と投影 | SQLite イベントソーシング（`event-store-adapter-rs` 3.0.0）。RMU が状態ファイル・監査シャード・memory 面・`read_*` 17 表を投影。クラッシュ復旧の契約テストあり | | 既知の到達不能 | `cli/request.rs::parse_next` が `NextTurnInput` の 5 観測を立てないため、`next` の分岐 0 / 1 / 1b〜1d は CLI から到達不能（単体テストのみ） | | 品質ゲート | `cargo test --workspace` 2,354 件、カバレッジ床 90% + PR 相対ゲート、Quint 3 モデル（engine_loop / journal_protocol / stop_hook）+ ITF、`cargo lint` 7 ルール、CI 7 ジョブ | | 未解決の Issue | #71 WorkspaceScanner 実走査、#77 intent-create の集約間原子性、#82 カバレッジ相対ゲートのフレーク、#53 監査フックのログ品質 | ## 最短経路の決め方 bugfix スコープのステージ列（9 / 33）を upstream の `SKILL.md` と `stage-protocol.md` に沿って 1 周したときに、ハーネスが呼ぶ動詞・フック・受領記録を列挙し、バイナリに無いものだけを実装対象にする。列挙は実測（配布資産の該当箇所を引用）で行い、想像で増やさない。優先順の目安は次のとおりだが、列挙の結果で入れ替えてよい。 1. フック 4 本（切替条件 1・2） 2. スモークで踏むゲート・受領の動詞（`aidlc-log decision / answer` など列挙で判明したもの） 3. `--doctor` サブセット（条件 4） 4. 実地スモーク → 切替（ホスト = 安定タグ） ## スコープ外 swarm / Bolt 自律実行、センサー・プラグイン・他ハーネス・配布一般化、OTel 配線、インストーラ、12 / 13 号仕様の全文執筆、上表の未解決 Issue のうちスモークで踏まないもの。 ## 進め方の規律（memory の規則に加えて） - 実装は TDD（red → green → refactor）。Quint / ITF / ゴールデンは外側の受入ゲート。 - PR = Bolt、直列 1 本、squash-merge。マージ条件は CI green と収束ルール。 - 設計成果物は「実装に必要な契約差分」だけを書き、既存コードの説明を書き直さない。 - 上流仕様と現行コードが食い違ったら、読み替えず人間に裁定を求める。GitHub Issue は AI の判断で起票しない。 - 会話と成果物は日本語。術語には初出で注釈を添える。
- **Project Description Source**: project-description.json
- **Project Type**: Brownfield
- **Scope**: selfhost-stage1
- **Start Date**: 2026-09-07T13:11:34Z
- **State Version**: 8
- **Active Agent**: aidlc-developer-agent
- **Worktree Path**:
- **Bolt Refs**:
- **Practices Affirmed Timestamp**: 2026-09-07T13:29:51Z

## Scope Configuration
- **Stages to Execute**: 0.1, 0.2, 0.3, 2.2, 2.3, 2.7, 2.8, 2.9, 3.5, 3.6, 4.3
- **Stages to Skip**: 1.1 (intent-capture), 1.2 (market-research), 1.3 (feasibility), 1.4 (scope-definition), 1.5 (team-formation), 1.6 (rough-mockups), 1.7 (approval-handoff), 2.1 (reverse-engineering), 2.4 (user-stories), 2.5 (refined-mockups), 2.6 (domain-design), 3.1 (functional-design), 3.2 (nfr-requirements), 3.3 (nfr-design), 3.4 (infrastructure-design), 3.7 (ci-pipeline), 4.1 (deployment-pipeline), 4.2 (environment-provisioning), 4.4 (observability-setup), 4.5 (incident-response), 4.6 (performance-validation), 4.7 (feedback-optimization)
- **Depth**: Minimal
- **Test Strategy**: Standard
- **Review Override**: 

## Workspace State
- **Project Root**: .
- **Languages**: Unknown
- **Frameworks**: Unknown
- **Build System**: cargo (Cargo.toml)

## Execution Plan Summary
- **Total Stages**: 11
- **Completed**: 8
- **In Progress**: code-generation

## Runtime State
- **Revision Count**: 2

- **Construction Iteration**: unit-major
- **Unit Ownership**: solo
- **Skeleton Stance**: off
- **Parked**: 2026-09-11T15:04:08Z
- **Parked At Stage**: code-generation
## Phase Progress
<!-- Status values: Pending, Active, Verified, Skipped -->

- **Initialization**: Verified
- **Ideation**: Skipped
- **Inception**: Verified
- **Construction**: Active
- **Operation**: Pending

## Stage Progress
<!-- Checkbox states: [ ] not started, [-] in progress, [?] awaiting approval (gate open), [R] revising (user rejected gate), [x] completed, [S] skipped via --stage/--phase jump -->

### INITIALIZATION PHASE
- [x] workspace-scaffold — EXECUTE
- [x] workspace-detection — EXECUTE
- [x] state-init — EXECUTE

### IDEATION PHASE
- [ ] intent-capture — SKIP
- [ ] market-research — SKIP
- [ ] feasibility — SKIP
- [ ] scope-definition — SKIP
- [ ] team-formation — SKIP
- [ ] rough-mockups — SKIP
- [ ] approval-handoff — SKIP

### INCEPTION PHASE
- [ ] reverse-engineering — SKIP
- [x] practices-discovery — EXECUTE
- [x] requirements-analysis — EXECUTE
- [ ] user-stories — SKIP
- [ ] refined-mockups — SKIP
- [ ] domain-design — SKIP
- [x] units-generation — EXECUTE
- [x] contract-design — EXECUTE
- [x] delivery-planning — EXECUTE

### CONSTRUCTION PHASE
Per unit: [TBD]
- [ ] functional-design — SKIP
- [ ] nfr-requirements — SKIP
- [ ] nfr-design — SKIP
- [ ] infrastructure-design — SKIP
- [-] code-generation — EXECUTE
- [ ] build-and-test — EXECUTE
- [ ] ci-pipeline — SKIP

### OPERATION PHASE
- [ ] deployment-pipeline — SKIP
- [ ] environment-provisioning — SKIP
- [ ] deployment-execution — EXECUTE
- [ ] observability-setup — SKIP
- [ ] incident-response — SKIP
- [ ] performance-validation — SKIP
- [ ] feedback-optimization — SKIP

## Current Status
- **Lifecycle Phase**: CONSTRUCTION
- **Current Stage**: code-generation
- **Next Stage**: build-and-test
- **Status**: Running
- **Last Updated**: 2026-09-11T15:04:08Z

## Session Resume Point
- **Last Completed Stage**: delivery-planning
- **Next Action**: Execute Code Generation
- **Pending Artifacts**: none
