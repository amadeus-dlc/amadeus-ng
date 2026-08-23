# developer-brief-2 — 委任 2: 仕様 01 / 10 / 11 / 12 号（U9 / Bolt B4）

Conversation language: 日本語（仕様本文・注記・報告はすべて日本語。型名 / API 名 / ファイル名 / ID / YAML キー / 逐語文言は英語のまま）。

## 役割と範囲

あなたは aidlc-developer-agent。Unit **u9-canon-docs**（kind: spec、Bolt B4）の委任 2 を担当する。**コードは書かない** — 所有ファイルは次の 4 つだけ:

- `docs/specs/01-domain-model.md`
- `docs/specs/10-orchestration.md`
- `docs/specs/11-workspace.md`
- `docs/specs/12-workflow-definition.md`
- 報告: `aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u9-canon-docs/code-generation/developer-report-2.md`（**新規・あなただけが書く**）

それ以外（`coding-rules/*.md` / `deviations.md` / `components.md` — 委任 1 が並行して編集中、`modules/` / `tools/` / `scripts/` / `.github/` / `Cargo.*`、
`docs/specs/research/**`、`docs/specs/00-policy.md` 等の他号、計画 `code-generation-plan.md` / `unit-test-instructions.md` / `code-generation-questions.md`）は
**読むだけ**。`git commit` / `git add` はしない。`.claude/` 配下のツールは実行しない。

## 先に読むもの（順に）

1. `aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u9-canon-docs/code-generation/code-generation-plan.md`（§1〜§5、特に §1 の「語の区別」
   「U2 設計の出典鮮度」、§2 写像表、§5.2）
2. `aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u9-canon-docs/functional-design/rules.md`（BR2.1〜BR2.5、BR3.1〜BR3.3、BR3.6、BR5.1、BR5.2）と
   同ディレクトリの `pending-revision.md`（項目 1 = 12 号 5 箇所、3 = grep 範囲 / sentinel 7 語）
3. `aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u9-canon-docs/nfr-design/security-design.md`（§2 作法、§3 受入）
4. `aidlc/spaces/default/intents/260822-stage1-selfhost/inception/domain-design/decisions.md`（ADR-001〜008 — 出典注記の中核）
5. `aidlc/spaces/default/intents/260822-stage1-selfhost/inception/contract-design/contract-summary.md`（C3 `WorkflowExecutionRepository`、C4 `find_by_id`、
   C5 `Started` ペイロード、C6 SQLite スキーマ）
6. 実装の一次出典（読取のみ、`origin/main` = Bolt B3）: `modules/core/domain/src/orchestration/mod.rs`（公開面）、`workflow_execution.rs`（16 属性・
   12 コマンド・`next_decision`・`effective_plan`）、`workflow_execution_event.rs`（12 イベント）、`workflow_execution_snapshot.rs`、`stage_entry.rs`
   （`is_gated()` = phase ≠ initialization）、`start_request.rs`、`next_decision.rs`、`intent_id.rs`；`modules/core/domain/src/workflow_definition/`
   （`workflow_definition.rs` の `id()` / `revision()` と述語 6 つ + `grid().action()`、`workflow_definition_id.rs`、`definition_revision.rs`、`plan_action.rs`）；
   `modules/core/use-case/src/orchestration/workflow_definition_repository.rs`（`find_by_id`、`GraphReadError::{NotFound, HarnessIdentity}`）
7. U2 機能設計（参考 — pending-revision 未適用、名称は下記の規範名で）: `aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u2-domain-es-core/
   functional-design/{entities,rules,functional-spec}.md` と `pending-revision.md`（項目 9: `WorkflowExecutionSnapshot` → `WorkflowExecutionState` 改名承認済み、
   項目 8: `IntentId` = UUIDv7）
8. `aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/gateway-taxonomy.md` / `use-case-rules.md`（語彙の正本 — 委任 1 が同時改訂中だが §2b / §5 の
   語彙（`find_by_id` / `save` / `store`、`XxxRepositoryImpl` / `InMemoryXxxRepository`）は既定）

## 作業（計画 §5.2 の Step 4〜6）

**Step 4（Red）**: 次を実行し、件数と行を `developer-report-2.md` の「Red 基線」に記録する。

```bash
grep -rnE 'effective_plan_action|next_in_scope_stage|AuditLedgerRepository|AuditLedgerService|StateFileStore|report_forward|gate_start' docs/specs/*.md
grep -rnE 'WorkspaceLock|StageGraphQuery|StageNodeView|SensorBindingView|find\(' docs/specs/01-domain-model.md docs/specs/10-orchestration.md docs/specs/11-workspace.md docs/specs/12-workflow-definition.md
```

**Step 5（Green）**: 次の改訂を行う。改訂した文・表の行・箇条の末尾に出典を括弧書きで残す（`（ADR-008）` / `（C4 改訂 2026-08-23）` / `（Bolt B3 実装）` /
`（設計監査 C9）` / `（オーナー裁定 2026-08-23）`、複数は `/` 区切り）。**改訂するのは「構造の規範と所有の記述」だけ** — upstream 互換の逐語契約
（監査イベント名・CLI 語彙・`AIDLC_*`・逐語文言・ファイル形式・`research/` の抽出）には触れない。

- **01 号 `01-domain-model.md`**
  - §3.1（BR3.1 / BR2.2）: `WorkflowDefinition` を集約ルート（`WorkflowDefinitionId` = `harness.json` の `name`、内容が変わっても不変のエンティティ ID /
    `DefinitionRevision` = 3 入力の正準 JSON の `sha256:`、値属性）と明記し、Domain Primitive 候補に `WorkflowDefinitionId` / `DefinitionRevision` を追加。
    `PlanAction` は workflow_definition の所有（ADR-005、orchestration は利用のみ）。「状態機械: effectivePlanAction …」は残し、合成読みの所有者を
    集約 `WorkflowExecution`（`effective_plan`）と明記。
  - §3.2（BR2.2 / BR2.4 / BR3.3）: `WorkflowExecution` は ES 形の FSM（decide → イベント → apply_event、状態 16 属性、イベント 12 種、メメント
    `WorkflowExecutionState`（現行コード名 `WorkflowExecutionSnapshot`、B5 で改名）— ADR-002 / 004）。`PlanAction` / `CheckboxState` の定義はここに置かず
    所有元参照（`PlanAction` → workflow_definition、`CheckboxState` → workspace の値オブジェクト）。
  - §3.3（BR3.2）: 集約候補を `Intent`（intents.json の登録 — uuid / slug / dirName と生死、birth の単一チョークポイント）/ `Space` / `Worktree` に。
    `StateFile` / `AuditShard` は**リードモデル**（ReadModelUpdater の投影、真実源は SQLite ジャーナル — ADR-003 / 004）、`WorkspaceLock` は**退役**
    （ADR-007、SQLite Tx + 楽観 version）。Domain Primitive: `IntentId` = UUIDv7（維持）、`IntentDirName`（記録ディレクトリ名 kebab、投影のパス解決用 —
    新設）、`SpaceName` / `CloneId` / `ShardName` / `StateFieldValue` / `CheckboxState` / `StateVersion` は値オブジェクト。脚注: 「U2 実装（Bolt B3）の
    `IntentId::parse` は記録ディレクトリ名を受理している — Bolt B5（U3）で UUIDv7 + `IntentDirName` へ是正（オーナー裁定 2026-08-23）」。
    「状態機械: Audit lock lifecycle …」の Audit lock は退役注記（ADR-007）。
  - §7（BR3.6）: 「ドメインモデルの原則」小節を追加 — (1) 集約（エンティティ）と値オブジェクトが主役、(2) 純粋関数のドメインサービスは消極的
    （集約に置けない横断の判断のみ）、(3) ドメインモデル・ドメインサービスは永続化責務を持たない（永続化を呼ばない）、(4) 永続化の指揮は
    ユースケース層（Repository trait はユースケース層、実装はアダプタ層、Tx の所有は実装・呼出はユースケース）、(5) 集約間の依存は ID による間接参照
    （ADR-008）、(6) 集約は FSM として設計（状態・遷移（`&mut self` コマンド、ガード付き `Err`）・判断（クエリ）を同じ型に）。coding-rules
    （`use-case-rules.md` / `gateway-taxonomy.md` / `domain-equality.md` / `tell-dont-ask.md`）への相互参照を付ける（相対リンクは既存の書き方に倣う）。
  - §4 B1 行・§6 の `effectivePlanAction` は語として残し、所有者 = 集約 `WorkflowExecution` を明記（計画 §1「語の区別」）。
- **10 号 `10-orchestration.md`**
  - §2.1（BR3.3 / BR2.2）: ES 形に書き換え — 状態 16 属性（definition_id / definition_revision / stages: Vec<StageEntry> / plan / overlay / conditional /
    checkbox / cursor / status / parked_at / autonomy / approved / revision_count / seq_nr / version … 実コードの属性名で）、イベント 12 種（実コードの
    変種名）、`decide` 系コマンド → イベント → `apply_event`、`state()` / `from_state()`（メメント `WorkflowExecutionState`、現行名 `…Snapshot`、B5 で改名）、
    `next_decision` は `Result<NextDecision, CommandError>`（`DefinitionMismatch` — definition_id 不一致で拒否、revision 差は Ok）、
    `gated(stage) = phase ≠ initialization`（索引 0 の特別扱いなし。Quint slice-1 の stage 0 は ITF 用合成計画上の抽象）、`Started` は自己完結
    （definition_id / definition_revision / scope / request / depth? / test_strategy? / stages = StageEntry 列 — C5）、有効プランの畳み込み `effective_plan`
    （overlay）は集約の所有。出典: ADR-002 / 004 / 005 / 008、Bolt B3。旧 API（`report_forward` / `gate_start` 等）や stage 0 特別扱いを規範に残さない。
  - §2.2（BR2.4）: `PlanAction` 行を「所有元: workflow_definition（12 号 §2.2）— 参照のみ」に、`CheckboxState` 行は「所有元: workspace（11 号 §2.2）」参照に。
  - §3（BR2.3 / BR3.1）: ポート表を書き直す — 行は `WorkflowDefinitionRepository`（`find_by_id(&WorkflowDefinitionId)`、失敗 `NotFound { expected, actual }` /
    `HarnessIdentity { path, cause }`、実装 `WorkflowDefinitionRepositoryImpl` / `InMemoryWorkflowDefinitionRepository` — C4）、`WorkflowExecutionRepository`
    （ES 形 `store(event, aggregate)` / `find_by_id`、実装 `WorkflowExecutionRepositoryImpl`（SQLite EventStore: journal / snapshot / checkpoint — C6 を内包、
    状態ファイル・監査シャードはリードモデル（U4）） / `InMemoryWorkflowExecutionRepository` — C3 / ADR-006）、外部システムクライアント（Git）。
    `AuditLedgerRepository` 行と `WorkspaceLock` 行は**削除**（ADR-001 / 003 / 007）、『同上』は廃止。§8（実装順序）の「in-memory Gateway 一式
    （… `InMemoryAuditLedgerRepository` … / Lock）」も現行ポートへ。
- **11 号 `11-workspace.md`**
  - §2.1（BR3.2）: 集約は `Intent` / `Space` / `Worktree`。`StateFile` / `AuditShard` はリードモデル、`WorkspaceLock`（「集約ではなく並行性サービス」の文）は
    退役注記（ADR-007）に。
  - §2.2: `IntentId` UUIDv7 維持 + `IntentDirName` 追加、`CheckboxState` は本コンテキスト所有（BR2.4）。
  - §2.3（BR3.2）: 状態ファイル描画の純関数群は投影（ReadModelUpdater、U4）の責務へ移す旨を明記（値オブジェクトの Always Valid 検証はドメインに残す）。
  - §3（BR2.1）: ポート表・供給面表を gateway-taxonomy 語彙で再構成 — 列『ポート | 消費するユースケース | 契約 | 実装の所在』。ポートは
    `WorkflowExecutionRepository`（C3）と外部システムクライアント（Git — 例 `GitWorktreeClient`）だけ。`FileStore`（アトミック書込・追記 open）は
    Repository 実装の内部部品、`Clock` / `ProcessProbe` / `Tmpdir` はアダプタ層の機構（ポートではない）、`AuditLedgerService` は退役（監査シャードは
    ReadModelUpdater の投影 — ADR-003）。`GitPort` は外部システムクライアント名へ。
  - §4: 機構（Clock / ProcessProbe）と Repository 実装内部（FileStore / 正準 JSON / ハッシュ）の配置を明記。MD5 ロック dir 名の記述は退役注記。
- **12 号 `12-workflow-definition.md`**
  - §2.1（BR3.1）: `WorkflowDefinition` 行に `WorkflowDefinitionId`（`<harnessRoot>/tools/data/harness.json` の `name`）と `DefinitionRevision`
    （`{ stage_graph, scope_grid, scopes }` 正準 JSON の `sha256:`、値属性）を追記（ADR-008、Bolt B3 `workflow_definition_id.rs` / `definition_revision.rs`）。
  - §2.2（BR2.4）: `PlanAction` は本コンテキスト所有と明記（10 号・01 号は参照）。
  - §2.3（BR2.5）: `next_in_scope_stage` 行を削除（畳み込み・前進走査は集約 `WorkflowExecution` の `effective_plan` / `next_decision` — 設計監査 R2、
    Bolt B3 で削除済み）。残す述語 6 つ（`is_valid_scope` / `valid_scopes` / `scope_metadata` / `subgraph_for_scope` / `stages_in_scope` /
    `first_in_scope_stage_of_phase`）と `grid().action()` を列挙。「2 経路の順序使い分け」の段落は、文書順走査の担い手が集約側（`stages` =
    StageEntry 列の文書順）になった旨に書き換える。
  - §4 未知スコープ表の行 / §8 F2 行 / §9 ユビキタス言語例（BR2.5 拡張、pending-revision 項目 1）: `next_in_scope_stage` の言及を現行の述語名・
    集約側の責務に置き換える（履歴注記として残さない）。
  - §5（BR3.1 / BR2.5）: `find` → `find_by_id(&WorkflowDefinitionId)`、失敗態度 `NotFound` / `HarnessIdentity`。`StageGraphQuery` / `StageNodeView` /
    `SensorBindingView` の個別名を廃し「集約の述語面（§2.3 の 6 述語 + `grid().action()`）」の記述へ（設計監査 C9）。
  - §10 実装ノート: 集約昇格の第一理由を「3 入力は compile が lockstep で出す（一貫性単位）」へ（C10）。B1 の「畳み込みは呼び出し側」は
    「呼び出し側 = 集約 `WorkflowExecution`」と明記（BR3.3 (c)）。

**Step 6（Refactor）**: 出典注記の形式をそろえ、表は見出しと同じ列数（`\|` エスケープ）、同一文言の見出し重複なし。Step 4 の grep を再実行し、
sentinel 7 語 = 0（履歴注記の行だけ残ってよい — 残す場合は「旧」と明記した比較表の行に限る）、`WorkspaceLock` は退役注記と逸脱台帳参照のみ、
`StageGraphQuery` / `StageNodeView` / `SensorBindingView` 0、`find(` の Repository 呼出 0。`unit-test-instructions.md` §2 の表検査スクリプトを自分の
所有 4 ファイルで走らせ `tables ok` を確認。

## 作法（厳守）

- 最小変更。逐語契約と `docs/specs/research/**` には触れない。10 号 §1 の「逐語の完全列挙は抽出文書と upstream を正とする」の一文を維持。
- 日本語正本、固定トークンは英語。旧記述を残すのは「旧」と明記した比較表だけ。
- 設計（rules.md / 本ブリーフ / ADR）に無い判断が要ったら、推測で進めず `developer-report-2.md` の「設計質問」に書いて該当箇所は保留する。

## 報告（`developer-report-2.md`）

見出し: 「Red 基線」「改訂一覧（ファイル:節 → BR → 出典注記）」「Green / Refactor の検査結果（コマンドと出力）」「設計質問」「未了」。最終応答は
この報告の要約（日本語、10 行以内）。
