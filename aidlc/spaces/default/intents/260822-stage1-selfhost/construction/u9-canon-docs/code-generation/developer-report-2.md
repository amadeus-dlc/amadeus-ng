# developer-report-2 — 委任 2: 仕様 01 / 10 / 11 / 12 号（U9 / Bolt B4）

> `code-generation-plan.md` §5.2（Step 4〜6）の実施報告。所有ファイルは `docs/specs/{01-domain-model,10-orchestration,11-workspace,12-workflow-definition}.md`
> の 4 本のみ。コードは書いていない（`git diff` で確認）。`git add` / `git commit` は実施していない。

## Red 基線

Step 4 のコマンドと、改訂前（作業開始時点）の結果。件数は**行数**（`grep -c` 相当）。

### 1. sentinel 7 語 — `grep -rnE '…' docs/specs/*.md`

```
docs/specs/11-workspace.md:78:   | `AuditLedgerService` | orchestration / verification / knowledge / フック | audit-first 追記、shard 列挙、位置付き読取（B9 の述語材料） |
docs/specs/11-workspace.md:95:   …ハッシュ（SHA-256 prefix-hash、MD5 ロック dir 名）は純粋部品。…チョークポイントは `AuditLedgerService` に置く。
docs/specs/10-orchestration.md:77:   | `WorkflowExecutionRepository` | …`WorkflowExecutionRepositoryImpl`（B-2）が crate 内部品 `state_file_io`・`AuditLedgerRepository`・`WorkspaceLock` を**合成**する — 削除済みの公開ファイルポート（旧 `StateFileStore` 相当）は再導入しない |
docs/specs/10-orchestration.md:78:   | `AuditLedgerRepository` | 集約 `AuditLedger`（11-workspace.md §2.1）の追記と射影読取（**B-1 で設計**）… | 同上 |
docs/specs/10-orchestration.md:175:  3. **in-memory Gateway 一式**（… / `InMemoryAuditLedgerRepository` / … / Lock）でユースケーステストを回す。
docs/specs/12-workflow-definition.md:68:   | `next_in_scope_stage` | (graph, after: `StageSlug`, scope, 状態の checkbox 射影) → `Option<&StageNode>` | … |
docs/specs/12-workflow-definition.md:73:   **2 経路の順序使い分け（本仕様の中核）**: … `next_in_scope_stage` は**文書順**を走査する。…
docs/specs/12-workflow-definition.md:143:  | 4 | 未知スコープ | **非対称**: … `next_in_scope_stage` / `first_in_scope_stage_of_phase` / `stages_in_scope` は `None` / 空を返す |
docs/specs/12-workflow-definition.md:193:  | F2 | 配列の**文書順を保持**し、`subgraph_for_scope`（数値順ソート）と `next_in_scope_stage`（文書順走査）の 2 経路を潰さない。… |
docs/specs/12-workflow-definition.md:209:  1. **ドメイン例をユビキタス言語のテストとして書く**: …「`subgraph_for_scope` は拒否し `next_in_scope_stage` は `None` を返す」…
```

| sentinel | 基線（行数） | 所在 |
|---|---|---|
| `effective_plan_action` | 0 | — |
| `next_in_scope_stage` | 5 | 12 号 §2.3（本文の 2 経路段落を含め 2 行）/ §4 表 #4 / §8 表 F2 / §9-1（FD pending-revision 項目 1 の 5 箇所と一致） |
| `AuditLedgerRepository` | 3 | 10 号 §3 表 ×2 行、§8-3 |
| `AuditLedgerService` | 2 | 11 号 §3 供給面表、§5 |
| `StateFileStore` | 1 | 10 号 §3 表（`WorkflowExecutionRepository` 行の実装欄） |
| `report_forward` | 0 | — |
| `gate_start` | 0 | — |

### 2. 退役機構・個別サービス名・旧動詞 — 所有 4 ファイルに限定した grep

| 語 | 基線（行数） | 所在 |
|---|---|---|
| `WorkspaceLock` | 4 | 01 §3.3 集約候補 / 10 §3 表（`WorkflowExecutionRepository` 行・`WorkspaceLock` 行）/ 11 §2.1 末尾 |
| `StageGraphQuery` | 1 | 12 §5 供給面表 |
| `StageNodeView` | 1 | 12 §5 供給面表 |
| `SensorBindingView` | 1 | 12 §5 供給面表 |
| `find(` | 0 | — （12 §5 / 10 §3 の旧動詞は `find` と地の文で書かれていて `find(` にはヒットしない） |

## 改訂一覧（ファイル:節 → BR → 出典注記）

### `docs/specs/01-domain-model.md`

| 節 | 改訂 | BR | 出典注記 |
|---|---|---|---|
| §3.1 集約 | `WorkflowDefinition` に識別子 `WorkflowDefinitionId`（`harness.json` の `name`、不変の系譜 ID）と内容版 `DefinitionRevision`（3 入力の正準 JSON の `sha256:`、値属性）を追記。付与は Repository 実装 | BR3.1 | （ADR-008 / Bolt B3 実装） |
| §3.1 Domain Primitive | `WorkflowDefinitionId` / `DefinitionRevision` を追加。`PlanAction` に「所有は本コンテキスト、orchestration は利用のみ・再輸出なし」を追記 | BR3.1 / BR2.4 | （ADR-005 / ADR-008 / Bolt B3 実装） |
| §3.1 状態機械 | `effectivePlanAction` は語として維持し、合成読みの**所有者は集約 `WorkflowExecution`**（`effective_plan`）、本コンテキストの供給はグリッド 3 値照会だけ、と明記 | BR3.3 (c) | （ADR-002 / 設計監査 R2、Bolt B3 実装） |
| §3.2 集約 | `WorkflowExecution` を ES 形 FSM として記述（decide 12 コマンド → 単一イベント → `apply_event`、状態 16 属性、イベント 12 種、メメント `WorkflowExecutionState`（現行コード名 `WorkflowExecutionSnapshot`、Bolt B5 で改名））。「状態遷移動詞 11 個の唯一の所有者」の語は 11 号 §3 が引用しているため維持 | BR2.2 / BR3.3 | （ADR-001 / ADR-002 / ADR-004、Bolt B3 実装） |
| §3.2 Domain Primitive | `PlanAction` の定義は §3.1、`CheckboxState` の定義は §3.3 の所有で、本コンテキストは参照のみ・再定義しない旨を追記 | BR2.4 | （ADR-005 / 設計監査 R1・C12） |
| §3.3 集約 | 集約を `Intent`（intents.json の登録 — uuid / slug / dirName と生死）/ `Space` / `Worktree` に。`StateFile` / `AuditShard` は**リードモデル**（ReadModelUpdater（U4）の投影、真実源は SQLite ジャーナル）、`WorkspaceLock` は**退役**（逸脱台帳参照） | BR3.2 | （ADR-003 / ADR-004 / ADR-007） |
| §3.3 Domain Primitive | `IntentDirName`（記録ディレクトリ名 kebab、投影のパス解決用）を新設。`IntentId` は UUIDv7 のまま維持。`CheckboxState` に「本コンテキストの所有」を追記。値オブジェクト 7 種を列挙。脚注で「U2 実装（Bolt B3）の `IntentId::parse` は記録ディレクトリ名を受理 → Bolt B5（U3）で UUIDv7 + `IntentDirName` へ是正」 | BR3.2 / BR2.4 | （オーナー裁定 2026-08-23 / ADR-003 / ADR-004） |
| §3.3 状態機械 | Audit lock lifecycle を退役注記へ（`audit_lock.qnt` は「ジャーナル / スナップショット / version / チェックポイント協定」へ改訂して存続） | BR3.2 | （ADR-007） |
| §4 表 B1 | `effectivePlanAction` の合成の所有者 = 集約 `WorkflowExecution` の `effective_plan` を追記（語は残す） | BR3.3 (c) | （ADR-002 / 設計監査 R2、Bolt B3 実装） |
| §6 第一陣 | `effectivePlanAction` 行に同上の所有者注記。`Audit lock lifecycle` 行を取消線＋協定モデルへの改訂に | BR3.2 / BR3.3 | （ADR-002 / ADR-007 / 設計監査 R2） |
| §7.1（新設） | 「ドメインモデルの原則」小節を追加 — (1) 主役は集約と値オブジェクト、(2) 純関数ドメインサービスは消極的、(3) ドメインは永続化責務を持たない、(4) 永続化の指揮はユースケース層（trait = ユースケース層 / 実装 = アダプタ層 / Tx 所有 = 実装・呼出 = ユースケース）、(5) 集約間依存は ID の間接参照、(6) 集約は FSM。`use-case-rules.md` / `gateway-taxonomy.md` / `domain-equality.md` / `tell-dont-ask.md` へ相対リンク | BR3.6 | （オーナー確認 2026-08-23 / ADR-002 / ADR-006 / ADR-008） |

### `docs/specs/10-orchestration.md`

| 節 | 改訂 | BR | 出典注記 |
|---|---|---|---|
| §1 B1 | 合成の所有者 = 集約の `effective_plan`、永続化の委譲先を Repository 実装に | BR3.3 (c) | （ADR-002 / ADR-003 / 設計監査 R2、Bolt B3 実装） |
| §2.1 | **全面 ES 形へ書き換え** — decide → 1 イベント → `apply_event`／状態 16 属性（実コードの属性名）／park マーカーの直交性（既存記述を維持）／コマンド 12（集約面）と upstream の CLI 動詞 11（§9 S3・§6 I9）の書き分け／ドメインイベント 12 種と封筒／`Started` の自己完結（`definition_id` / `definition_revision` / `scope` / `request` / `depth?` / `test_strategy?` / `stages`）／upstream 監査行は U4 の投影であって集約の発行物ではない／メメント `state()` / `from_state()` と `WorkflowExecutionState`（現行名 `…Snapshot`、B5 で改名）／`gated(stage) = phase ≠ initialization`（索引 0 の特別扱いなし、Quint slice-1 の stage 0 は ITF 用合成計画の抽象）／`effective_plan` は集約の所有／`next_decision` は `Result` で `DefinitionMismatch`（revision 差は Ok）／Tx 境界は SQLite 1 Tx + 楽観 version | BR3.3 / BR2.2 | （ADR-001 / ADR-002 / ADR-004 / ADR-005 / ADR-008、C5 / C6、Bolt B3 実装） |
| §2.2 表 | `PlanAction` 行を「所有元: workflow-definition（12 §2.2）— 参照のみ」に、`CheckboxState` 行を「所有元: workspace（11 §2.2）」に、`EffectivePlan` 行を「合成は集約の `effective_plan` が所有」に | BR2.4 / BR3.3 (c) | （ADR-005 / 設計監査 C12 / R2、Bolt B3 実装） |
| §2.3 表の直後 | `next_decision` / `jump_resolve` は集約のクエリメソッドであり独立ドメインサービスではない旨を注記（表は入出力の規範として維持）。`human_acted_since_gate` は横断判断なのでドメインサービスのまま | BR3.3 (d) / BR3.6 | （ADR-002 ④、Bolt B3 実装） |
| §3 ポート表 | **書き直し**。「1 trait 1 Impl」の前置きを追加し『同上』を廃止。行は `WorkflowExecutionRepository`（ES 形 `store(event, aggregate)` / `find_by_id(&IntentId)`、実装 = SQLite EventStore（C6 を内包）＋ `InMemoryWorkflowExecutionRepository`、状態ファイル・監査シャードはリードモデル（U4））、`WorkflowDefinitionRepository`（`find_by_id(&WorkflowDefinitionId)`、失敗 `NotFound { expected, actual }` / `HarnessIdentity { path, cause }`、実装 `…Impl` ＋ `InMemory…`）、外部システムクライアント（Git）、マーカー永続化 Gateway（既存）。`AuditLedgerRepository` 行と `WorkspaceLock` 行は**削除**し、削除理由を表下に 1 段落 | BR2.3 / BR3.1 | （ADR-001 / ADR-003 / ADR-006 / ADR-007、C3 / C4 改訂 2026-08-23 / C6、設計監査 C11） |
| §8-2 | 集約は ES 形で書く旨と、proptest の対象に「コマンド適用後の状態 = 旧状態 + そのイベント」「イベント列の再生が実行済み集約を再現する」を追記。`EffectivePlan` → `effective_plan` | BR3.3 | （Bolt B3 実装） |
| §8-3 | in-memory Gateway 一式を現行ポート 2 本（`InMemoryWorkflowExecutionRepository` / `InMemoryWorkflowDefinitionRepository`）へ | BR2.3 | （ADR-003 / ADR-007） |

### `docs/specs/11-workspace.md`

| 節 | 改訂 | BR | 出典注記 |
|---|---|---|---|
| §2.1 | 集約表を `Intent` / `Space` / `Worktree` の 3 行に（`AuditLedger` 行を削除、`Intent` の内包から `StateFile` を外す）。表下に**リードモデル**段落（`StateFile` / `AuditShard`、真実源は SQLite ジャーナル、U4 の投影でバイト互換、追記専用・opaque・他クローン読取専用は投影の規範として維持）と**退役**段落（`WorkspaceLock`、逸脱台帳参照）を追加 | BR3.2 | （ADR-001 / ADR-003 / ADR-004 / ADR-007） |
| §2.2 | `IntentDirName` 行に「`IntentId`（UUIDv7）とは別の値、投影先パス解決に使う」を追記。`CheckboxState` 行を**新設**（本コンテキストの所有、orchestration は参照のみ）。`LockIdentity` 行を「（退役予定 — ADR-007）」とし、残る規範は keying だけで md5 dir 名の互換維持は §9 / §10 の論点である旨をセル末尾に追記 | BR2.4 / BR3.2 | （01 §3.3 / 設計監査 C12 / ADR-007、オーナー裁定 2026-08-23） |
| §2.3 | 表の前に、状態ファイル・監査ブロックの**描画**（`render_audit_block` / `state_writers`）は投影（ReadModelUpdater、U4）の責務へ移る旨を明記。ドメインに残るのは値オブジェクトの Always Valid 検証と横断判断（`find_all_events` / `classify_state_version` は本コンテキストに残す） | BR3.2 | （ADR-003 / ADR-004、01 §7.1 原則 2） |
| §3 | **ポート表・供給面表を再構成**。ポート表は『ポート / 消費するユースケース / 契約 / 実装の所在』の 4 列で `WorkflowExecutionRepository`（C3）と外部システムクライアント（Git、例 `GitWorktreeClient`）の 2 行だけ。「ポートではないもの」段落に `FileStore`（Repository 実装・投影ライタの内部部品）、`Clock` / `ProcessProbe` / `Tmpdir`（アダプタ層の機構）、監査追記サービスとロックサービスの退役を明記。供給面表は `WorktreeService` / `OpaqueFlagStore` / `ScopedStorage` / `SessionStampStore` の 4 行に。状態ファイル・監査シャードの読取は供給、書込は投影の責務と明記。§3 本文の `StateFileService` への言及もリードモデル読取へ差し替え | BR2.1 | （設計監査 R3 / C3 / C4 / C11、ADR-003 / ADR-004 / ADR-007、gateway-taxonomy §1・§2・§3・§5） |
| §4 | 「配置の規範」箇条を新設（`Clock` / `ProcessProbe` は機構モジュール＋composition root 配線、`FileStore` / 正準 JSON / ハッシュは Repository 実装・投影ライタの内部部品、use-case 層に trait を置かない）。Gateways 箇条の `GitPort` を外部システムクライアント（Git）へ、ロック dir 実装を退役注記へ | BR2.1 / BR3.2 | （gateway-taxonomy §1、設計監査 C4、ADR-007） |
| §5 | `AuditLedgerService` の言及を削除し、派生イベント発行のチョークポイントを「ジャーナル追記の Tx コミット成功後（`…Impl` の `store`）」へ。MD5 ロック dir 名を退役注記へ | BR2.1 / BR3.2 | （ADR-003 / ADR-007） |
| §7-3 | in-memory の一覧を現行の面（`InMemoryWorkflowExecutionRepository` / Git クライアントの fake / 機構の fake）へ。並行テストの対象を楽観 version の競合と再試行へ | BR2.1 | （ADR-007、gateway-taxonomy §5・§6） |
| §9 | ロックの物理形式を互換維持するという前提を取消線＋失効注記に（ADR-007 でロック dir を生成しないため）。`AIDLC_WORKSPACE_LOCK_OWNER_PID` の env 互換維持は残す | BR3.2 | （ADR-007、逸脱台帳参照） |
| §10 | 未決事項を 2 件追加（stage-0/1 併用期の相互排他 / `intents.json` の直列化機構）、`audit_lock.qnt` slice 2 の項目を協定モデルへの改訂に置換 | BR3.2 | （ADR-007） |

### `docs/specs/12-workflow-definition.md`

| 節 | 改訂 | BR | 出典注記 |
|---|---|---|---|
| §1 B1 | 合成の所有者を「集約 `WorkflowExecution` の `effective_plan`」と特定 | BR3.3 (c) | （設計監査 R2） |
| §2.1 | `WorkflowDefinition` 行に `WorkflowDefinitionId`（`<harnessRoot>/tools/data/harness.json` の `name`）と `DefinitionRevision`（`{ stage_graph, scope_grid, scopes }` 正準 JSON の `sha256:`、値属性）を追記。**集約昇格の第一理由**を「3 入力は compile が lockstep で出す（一貫性の単位）」に、命名規則を第二理由に | BR3.1 / BR2.5 | （ADR-008、Bolt B3 実装 `workflow_definition_id.rs` / `definition_revision.rs`、設計監査 C10） |
| §2.2 | `PlanAction` 行に「所有は本コンテキスト、10 号・01 号は参照のみ・再輸出なし」を追記 | BR2.4 | （ADR-005、Bolt B3 実装） |
| §2.3 | 見出しを「集約の述語面（純関数）」へ。`next_in_scope_stage` 行を**削除**し、残す 6 述語（`is_valid_scope` / `valid_scopes` / `scope_metadata` / `subgraph_for_scope` / `stages_in_scope` / `first_in_scope_stage_of_phase`）＋ `grid().action()` を列挙。「2 経路の順序使い分け」段落を、文書順走査の担い手は集約側（`stages` = `StageEntry` 列の文書順）である旨に書き換え | BR2.5 / BR3.3 (c) / BR3.6 | （設計監査 R2 / C8 / C9、Bolt B3 で定義側から削除済み、01 §7.1 原則 2） |
| §4 表 #4 / #7 | #4 の `next_in_scope_stage` を現行述語名（`first_in_scope_stage_of_phase` / `stages_in_scope` / `scope_metadata`）へ。#7 の「畳み込みは呼び出し側」を「呼び出し側 = 集約 `WorkflowExecution` の `effective_plan`」に | BR2.5 / BR3.3 (c) | （設計監査 R2 / C8） |
| §5 | ポート段落を `find_by_id(&WorkflowDefinitionId)`（引数を取らない旧動詞 `find` は廃止・併存なし）＋ 失敗態度 `NotFound { expected, actual }` / `HarnessIdentity { path, cause }` ＋ `id` / `revision` 付与は実装の責務、に改訂。供給面表から `StageGraphQuery` / `ScopeCatalog` / `SkeletonAnchor` / `StageNodeView` / `SensorBindingView` の個別名を廃し、**顧客 / 使う面 / 契約の要点**の 4 行表（集約の述語面と内包 `StageNode` の読取）へ | BR3.1 / BR2.5 | （C4 改訂 2026-08-23 / ADR-008、設計監査 C9、gateway-taxonomy §3） |
| §8 表 F2 / F8 | F2 の `next_in_scope_stage`（文書順走査）を `stages_in_scope`（文書順）＋「前進走査は集約の `stages` 上」に。F8 の「呼び出し側」を集約の `effective_plan` に特定 | BR2.5 / BR3.3 (c) | （設計監査 R2） |
| §9-1 / §9-2 / §9-3 / §9-4 | ユビキタス言語例の `next_in_scope_stage` を `stages_in_scope` へ。TDD 対象を集約 `WorkflowDefinition` に（`StageGraph` は内包の成果物値）。`plan_action_in_grid` → `grid().action()`。述語 5 種 → 述語面（6 述語 ＋ `grid().action()`）。ゴールデンで `id` / `revision` の付与規則も固定する旨を追記 | BR2.5 / BR3.1 | （設計監査 C9 / C10、ADR-008、Bolt B3 実装） |

## Green / Refactor の検査結果（コマンドと出力）

### 検査 1 — sentinel 7 語（`docs/specs/*.md`、`research/` を含まない）

```
$ grep -rnE 'effective_plan_action|next_in_scope_stage|AuditLedgerRepository|AuditLedgerService|StateFileStore|report_forward|gate_start' docs/specs/*.md
0 hits
```

**7 語すべて 0 件**。履歴注記として残した行も無い（比較表を作らずに現行語彙へ置換できたため）。

### 検査 2 — 退役機構・個別サービス名・旧動詞（所有 4 ファイル）

```
$ grep -rncE 'WorkspaceLock|StageGraphQuery|StageNodeView|SensorBindingView|find\(' \
    docs/specs/01-domain-model.md docs/specs/10-orchestration.md docs/specs/11-workspace.md docs/specs/12-workflow-definition.md
docs/specs/01-domain-model.md:1
docs/specs/10-orchestration.md:0
docs/specs/11-workspace.md:1
docs/specs/12-workflow-definition.md:0
```

残る 2 件はいずれも**退役注記＋逸脱台帳参照**であり、規範としての `WorkspaceLock` ではない。

- `01-domain-model.md:101` — §3.3「**退役**: `WorkspaceLock` — 並行制御は SQLite Tx + 楽観 version に置換され、ロック dir は生成しない（ADR-007。逸脱台帳 `deviations.md` 参照）。」
- `11-workspace.md:34` — §2.1「**退役**: `WorkspaceLock`（旧: …並行性サービス）。…（ADR-007。逸脱台帳 `deviations.md` 参照）。」

`StageGraphQuery` / `StageNodeView` / `SensorBindingView` は **0 件**。`find(` も **0 件**（旧動詞への言及は「引数を取らない旧動詞 `find` は廃止」と地の文で書き、Repository 呼出の形にしていない）。

### 検査 3 — 表の列数（`unit-test-instructions.md` §2 のスクリプト、所有 4 ファイルに限定）

```
tables ok
```

### 検査 4 — 見出しの重複

```
$ for f in docs/specs/{01-domain-model,10-orchestration,11-workspace,12-workflow-definition}.md; do
    grep -n '^#' "$f" | sed 's/^[0-9]*://' | sort | uniq -d; done
（出力なし）
```

### 検査 5 — コード変更ゼロ / 抽出文書の不変

```
$ git diff --stat origin/main..HEAD -- modules tools scripts .github Cargo.toml Cargo.lock docs/specs/research
$ git diff --stat --            modules tools scripts .github Cargo.toml Cargo.lock docs/specs/research
（いずれも出力なし）
```

### 検査 6 — 相対リンクの解決（新規に張ったリンクを含む全リンク）

所有 4 ファイルの相対リンク（`http` を除く）をすべて `os.path.exists` で検査 → `links ok`。
新規リンクは `01 §7.1` からの `coding-rules/{README,tell-dont-ask,use-case-rules,gateway-taxonomy,domain-equality}.md` と、
各号からの `deviations.md`（同一ディレクトリ）。

### 検査 7 — 作業範囲

```
$ git status --porcelain -- docs/specs
 M docs/specs/01-domain-model.md
 M docs/specs/10-orchestration.md
 M docs/specs/11-workspace.md
 M docs/specs/12-workflow-definition.md
 M docs/specs/deviations.md     ← 委任 1 の変更（本委任は触れていない）
```

行数の増減（本委任の 4 ファイル）: 01 号 `+27/-11`、10 号 `+25/-15`、11 号 `+31/-17`、12 号 `+28/-29`。

## 設計質問

推測で進めず保留・報告に回した論点。**(1) は本委任の成果物の記述そのものに関わるため、裁定次第で 10 号 §2.1 と 01 §3.2 の 1 語を直す必要がある。**

1. **メメントのアクセサ名 `state()` / `from_state()` が上流成果物と食い違う**（要裁定）。委任ブリーフ Step 5（10 号 §2.1）は「`state()` / `from_state()`（メメント `WorkflowExecutionState`、現行名 `…Snapshot`、B5 で改名）」と指示しており、これに従って書いた。一方 U2 の `functional-design/pending-revision.md` 項目 9 は「`WorkflowExecutionSnapshot` を `WorkflowExecutionState`（memento）へ改名する（**責務は変えない**: serde なし、`snapshot()` / `from_snapshot()` = 状態の写しと不変条件つき復元）」と、**メソッド名は据え置く**と明記している。ブリーフを委任の正として `state()` / `from_state()` を採ったが、B5（U3）の実装時に U2 側の記述と衝突する。**型名だけ改名（メソッドは `snapshot()` / `from_snapshot()` 据え置き）か、メソッドも改名か**をコンダクタで裁定し、片方に寄せてほしい。該当箇所は 10 号 §2.1 のメメント箇条 1 行のみ（01 §3.2 は型名しか書いていないためどちらでも成立する）。
2. **10 号 §10 表 S2 が退役済み機構を「維持必須の仕様」として書いている**（本委任の改訂範囲外のため未着手）。S2 は「各遷移は原子的（**withAuditLock 区間で audit-first**。…）| 集約トランザクションが**ロック**と audit-first を所有。…`audit_lock.qnt` が受け皿」。ADR-007（ロック退役）・ADR-001/003（真実源はジャーナル）と整合させるなら「各遷移は原子的（クラッシュしても state と監査が食い違わない）| Repository 実装の SQLite Tx ＋ 投影のチェックポイントが受け皿」へ寄せることになる。§2.1 では既に Tx 境界を SQLite 1 Tx と書いたので、**同一文書内に旧機構の規範が 1 行残っている**。ブリーフの改訂対象（§2.1 / §2.2 / §3 / §8）に含まれないため保留した。
3. **10 号 §6 I14 と 11 号 §6 W1〜W5 の不変条件表が mkdir ロック前提のまま**（同上、範囲外）。I14「監査 emit が状態書込に先行し…`audit_lock::audit_first`」、W1〜W5 の `audit_lock::*` 定義名群。ADR-007 は「`audit_lock.qnt` を『ジャーナル / スナップショット / version / チェックポイント協定』の検証モデルへ**改訂**する」と決めているので、E4 定義名の差し替えはモデル改訂（U3 相当）と同期させるのが筋に見える。01 号 §6・11 号 §8 では「改訂して存続」と書ける範囲だけ注記し、不変条件表そのものは触っていない。
4. **11 号 §3 の「audit 5 動詞（append / append-batch / append-raw / audit-fork / audit-merge）」の去就**（範囲外）。監査シャードがリードモデルになると、`append` 系は「投影が書く」ものになり、CLI 動詞としての位置づけ（upstream 互換の CLI 語彙 = 逐語契約 D6）と投影の責務の関係を決める必要がある。逐語契約に触れない方針で保留した。
5. **11 号 §2.1 `Intent` / `Space` のトランザクション境界**。`intents.json` は SQLite ジャーナルの外にあり、ADR-007 でロックが退役した後の直列化機構が上流成果物に無い。表のセルは「1 トランザクション（直列化の機構は §10 の未決事項）」と保留形で書き、§10 に未決事項として立てた（W13 の keying 規範自体は維持）。
6. **11 号 §9 の stage-0/1 併用期のロック物理形式互換**。「初期フェーズはロックの物理形式も互換維持する」は ADR-007（ロック dir を生成しない）と真っ向から矛盾するため、取消線＋失効注記にし、§10 の未決事項へ移した（併用を許すか、許すならどの機構かはオーナー裁定待ち）。踏み込んだ判断はしていない。

## 未了

- `git add` / `git commit` は行っていない（計画 §7 のとおりコンダクタの担当）。
- 受入チェック 3（README の無矛盾）・5（deviations の登録）は委任 1 の所有ファイルに対する検査のため未実施。
- 受入チェック 6（CodeRabbit スレッド）は PR 作成後。
- 設計質問 1 の裁定が「メソッド名据え置き」なら、10 号 §2.1 のメメント箇条を `snapshot()` / `from_snapshot()` に戻す 1 行の修正が要る。
- 設計質問 2〜4 は本委任の改訂範囲外として保留（コンダクタ裁定待ち。B4 内で直すか後続 Bolt へ回すかの判断が要る）。
