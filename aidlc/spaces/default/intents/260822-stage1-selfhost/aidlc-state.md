# AI-DLC State Tracking

## Project Information
- **Project**: マイルストーン: stage-1（セルフホスト切替）への最短経路

目下の目標 = **stage-1（セルフホスト切替）に最短で到達する**。

切替条件の正本は `docs/specs/00-policy.md` §4（5 条件）。本 issue はその実行レベルのトラッキングで、各項目は PR で消化し、stage-1 到達（amadeus-ng 自身をホストにこのリポジトリの開発が回る）で close する。

D6 互換の配当により、upstream `dist/claude/` の資産（33 ステージ・エージェント・プロトコル・コンパイル済みグラフ）を**そのまま**使う — ステージ類は書かない。バイナリがそれを読んで動けばよい。

## クリティカルパス

- [ ] **0. stage-0 セットアップ＋ゴールデン採取** — 2026-08-22 に **0a/0b に分割**:
  - [x] **0a. ソース静的採取 — 完了（#19 で恒久化）** — ピン留め `3c3146cf` が公開リポジトリから取得可能と判明（dist 成果物込み）。EVENT_HEADINGS 86 / authority 残り 2 セット / 逐語文言 / FIELD_ORDER 実順序 / slugify / suffix writer / StateVersion 比較 / dist 実バイト（stage-graph.json / scope-grid.json = パリティ fixture）を 4 並列で採取中。**bun 不要**
  - [ ] **0b. 実行時採取＋自己開発ホスト（オーナー担当）** — bun ＋ upstream `dist/claude/` 導入。hash-canonical 受入表（ADR 0001 — 実入力に対する実ハッシュ出力）・CLI 実行出力ゴールデン・ドッグフード用 stage-0 ホストは実行環境が必要
- [x] 1. CI: fmt/clippy/test ＋ Quint ゲート＋カバレッジ（#6）【条件 5】— **完了**（#9。以後 `cargo lint` カスタムリンターも追加 #13/#15）
- [ ] 2. workspace 実装スライス【条件 2】— **一部完了**: 状態ファイル・ロック・`audit_lock.qnt` ITF 準拠は #10 で完了。**残件: 監査台帳（append + 位置付き読取）＋ audit-first 結合** — 契約マップ + 0a 逐語採取済み、スライス B-1 として着手予定（ロックの upstream 準拠は #18 で完了）
- [ ] 3. グラフリーダ＋ Next / Report ユースケース＋レビュアーレシート述語【条件 1・3】— **3 スライスに分割**:
  - [x] **A. グラフリーダ縦切り** — **完了**（#11 マージ済み。dist 実バイトのパリティ golden テストは #19）
  - [ ] **B. 監査台帳 Gateway（項目 2 残件）→ report_dispatch ＋ B10 述語最小 ＋ verification モジュール** — 契約マップ 3 本抽出済み・設計確定済み
  - [ ] **C. Next 21 分岐ラダー＋ load-steering / continue_token ＋ Continue** — 契約マップ抽出済み。着手前に next_decision の層配置裁定が 1 件必要
- [ ] 4. マルチコール CLI ＋文言カタログ配線（ディスパッチャ ROUTES 表）【条件 1】
- [ ] 5. 最小フック: Stop forwarding loop / HUMAN_TURN / state-transition guard / write-audit-log【条件 1・2】
- [ ] 6. doctor サブセット → **このリポジトリ自身でドッグフード** → stage-1 切替【条件 4】

## 最短のために明示的にやらないもの（スコープ外）

- swarm / Bolt 自律実行 — 切替後も **gated モード**で自己開発すれば不要（swarm は autonomous 限定発火。Construction は per-unit 反復＋ artifact 判定で回る）
- センサー・プラグイン・他 6 ハーネス・配布一般化（切替条件に含めないと 00-policy §4 で確定済み）
- OTel 配線（opt-in なので後回し可）・インストーラ（`target/release` 直接利用でよい）
- 12 / 13 号仕様の全文執筆（実装が突き当たった契約だけスライスで書く）
- **Project Type**: Brownfield
- **Scope**: classic
- **Start Date**: 2026-08-22T03:30:29Z
- **State Version**: 8
- **Active Agent**: aidlc-architect-agent
- **Worktree Path**:
- **Bolt Refs**:
- **Practices Affirmed Timestamp**: 2026-08-22T05:12:32Z

## Scope Configuration
- **Stages to Execute**: 0.1, 0.2, 0.3, 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 2.7, 2.8, 2.9, 3.1, 3.2, 3.3, 3.5, 3.6, 3.7
- **Stages to Skip**: 1.1 (intent-capture), 1.2 (market-research), 1.3 (feasibility), 1.4 (scope-definition), 1.5 (team-formation), 1.6 (rough-mockups), 1.7 (approval-handoff), 3.4 (infrastructure-design), 4.1 (deployment-pipeline), 4.2 (environment-provisioning), 4.3 (deployment-execution), 4.4 (observability-setup), 4.5 (incident-response), 4.6 (performance-validation), 4.7 (feedback-optimization)
- **Depth**: Standard
- **Test Strategy**: Standard
- **Review Override**: 

## Workspace State
- **Project Root**: /Users/j5ik2o/orca/workspaces/amadeus-ng/docs
- **Languages**: Unknown
- **Frameworks**: Unknown
- **Build System**: cargo (Cargo.toml)

## Execution Plan Summary
- **Total Stages**: 18
- **Completed**: 10
- **In Progress**: functional-design

## Runtime State
- **Revision Count**: 1

- **Construction Iteration**: unit-major
- **Skeleton Stance**: off
- **Active Unit**: u9-canon-docs
- **Unit State**: paused
- **Unit Pause Reason**: Claude/Codex harness maintenance was completed before Functional Design artifact work resumed
- **Unit Next Action**: Resume u9-canon-docs Functional Design and generate the missing functional-spec.md artifact
- **Parked**: 2026-09-05T01:38:17Z
- **Parked At Stage**: functional-design
## Phase Progress
<!-- Status values: Pending, Active, Verified, Skipped -->

- **Initialization**: Verified
- **Ideation**: Skipped
- **Inception**: Verified
- **Construction**: Active
- **Operation**: Skipped

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
- [x] reverse-engineering — EXECUTE
- [x] practices-discovery — EXECUTE
- [x] requirements-analysis — EXECUTE
- [S] user-stories — EXECUTE
- [S] refined-mockups — EXECUTE
- [x] domain-design — EXECUTE
- [x] units-generation — EXECUTE
- [x] contract-design — EXECUTE
- [x] delivery-planning — EXECUTE

### CONSTRUCTION PHASE
Per unit: [TBD]
- [-] functional-design — EXECUTE
- [ ] nfr-requirements — EXECUTE
- [ ] nfr-design — EXECUTE
- [ ] infrastructure-design — SKIP
- [ ] code-generation — EXECUTE
- [ ] build-and-test — EXECUTE
- [ ] ci-pipeline — EXECUTE

### OPERATION PHASE
- [ ] deployment-pipeline — SKIP
- [ ] environment-provisioning — SKIP
- [ ] deployment-execution — SKIP
- [ ] observability-setup — SKIP
- [ ] incident-response — SKIP
- [ ] performance-validation — SKIP
- [ ] feedback-optimization — SKIP

## Current Status
- **Lifecycle Phase**: CONSTRUCTION
- **Current Stage**: functional-design
- **Next Stage**: nfr-requirements
- **Status**: Running
- **Last Updated**: 2026-09-05T01:38:17Z

## Session Resume Point
- **Last Completed Stage**: delivery-planning
- **Next Action**: Execute Functional Design
- **Pending Artifacts**: none
