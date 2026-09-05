# entities — U2 ドメイン ES コア（`u2-domain-es-core`）

> 2026-09-05 是正。以下の YAML が現行の論理モデルの正本であり、第 2 節と functional-spec.md 第 6 節はその導出ビュー。
> 根拠は [設計裁定](../../../inception/domain-design/decisions.md)、[共有契約](../../../inception/contract-design/contract-summary.md)、
> リポジトリルート基準の `aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/` にある
> `aggregate-commands.md`（2026-09-05 差分再生訂正）、`aggregate-references.md`、
> `domain-persistence-neutrality.md`、`cqrs-boundaries.md`、`gateway-taxonomy.md`。
> 旧 WorkflowExecution、memento、本家 trait 直実装、初期化を逐次完了する設計は後続裁定により失効した。
> 過去の質問回答・pending-revision・functional-spec.md の Review は履歴であり、現行 API の導入指示ではない。

## 1. エンティティ（正本）

```yaml
entities:
  - name: IntentId
    description: "orchestration が所有する Intent の識別子。UUIDv7 を検査する Domain Primitive。記録ディレクトリ名 dirName ではない"
    attributes:
      - { name: value, type: string, required: true, unique: true, constraints: "UUIDv7。IntentExecutionId と別型" }

  - name: IntentExecutionId
    description: "orchestration が所有する 1 回の実行の識別子"
    attributes:
      - { name: value, type: string, required: true, unique: true, constraints: "UUIDv7。IntentId の別名にしない" }

  - name: WorkflowDefinitionId
    description: "workflow_definition が所有する定義の系譜 ID。内容が変わっても同じ ID"
    attributes:
      - { name: value, type: string, required: true, unique: true, constraints: "trim 後に非空、制御文字なし。内容ハッシュではない" }

  - name: DefinitionRevision
    description: "workflow_definition が所有する内容版。ID ではない"
    attributes:
      - { name: value, type: string, required: true, constraints: "sha256:<hex64>。CompiledDefinition が内容の純粋な関数 DefinitionRevision::of_content で導出する（ADR-008改訂2026-09-02）。ファイル生バイトや未知フィールドを計算材料にしない" }

  - name: WorkflowDefinition
    description: "workflow_definition の集約ルート。通常の ES Repository で取得・保存する。読取専用・ES 対象外という旧分類は廃止"
    attributes:
      - { name: id, type: WorkflowDefinitionId, required: true, unique: true }
      - { name: revision, type: DefinitionRevision, required: true }
      - { name: graph, type: StageGraph, required: true }
      - { name: grid, type: ScopeGrid, required: true }
      - { name: scopes, type: "map<string, ScopeMetadata>", required: true }
    constraints:
      - "定義集約の内容と、CompiledDefinition が所有するコンパイル済み成果物の取得境界を混同しない"
      - "IntentExecution の overlay を所有せず、effective_plan_action / next_in_scope_stage を再導入しない"

  - name: Intent
    description: "orchestration の集約ルート。依頼・定義参照・解決済み静的計画・開始時スキャンを所有する"
    attributes:
      - { name: id, type: IntentId, required: true, unique: true }
      - { name: definition_id, type: WorkflowDefinitionId, required: true }
      - { name: definition_revision, type: DefinitionRevision, required: true }
      - { name: start_request, type: StartRequest, required: true, constraints: "scope / request / depth / test_strategy / review の所有者" }
      - { name: stages, type: "list<StageEntry>", required: true, constraints: "文書順の全ステージ、非空、slug 一意。生成後不変" }
      - { name: scan, type: WorkspaceScan, required: true }
      - { name: created_at, type: "DateTime<Utc>", required: true }
    constraints:
      - "create は定義と StartRequest から計画を解決・検査し、Intent と Created イベントを対で返す"
      - "静的 plan_action / conditional / display はこの集約が所有する"
    relationships:
      - { to: WorkflowDefinition, cardinality: "many-to-one", direction: "Intent → WorkflowDefinition", description: "definition_id による ID 参照。定義オブジェクトを埋め込まない" }
      - { to: StageEntry, cardinality: "one-to-many", direction: "Intent → StageEntry", description: "静的計画の所有" }

  - name: StageEntry
    description: "orchestration の値オブジェクト。Created と Started に履歴の材料として載る自己完結の計画要素"
    attributes:
      - { name: slug, type: StageSlug, required: true }
      - { name: phase, type: PhaseId, required: true }
      - { name: plan_action, type: PlanAction, required: true, constraints: "None → SKIP。initialization は EXECUTE" }
      - { name: conditional, type: boolean, required: true, constraints: "initialization は false" }
      - { name: display, type: StageDisplay, required: true, constraints: "番号・表題・担当の単一行表示材料" }

  - name: StageKey
    description: "orchestration の値オブジェクト。実行がイベントの slug を解決しフェーズを判定するための最小の添字帳"
    attributes:
      - { name: slug, type: StageSlug, required: true }
      - { name: phase, type: PhaseId, required: true }

  - name: StageIndex
    description: "orchestration の位置型。公開の位置解決は IntentExecution.stage_index(usize) → Option<StageIndex>"
    attributes:
      - { name: value, type: integer, required: true, min: 0, constraints: "所属実行の stage_count 未満。別実行から渡された位置はコマンドでも検査" }

  - name: IntentExecution
    description: "orchestration の集約ルート = 実行 FSM。decide / apply と状態に基づく判断を所有する。serde・ストア trait・memento 型を持たない"
    attributes:
      - { name: id, type: IntentExecutionId, required: true, unique: true }
      - { name: intent_id, type: IntentId, required: true }
      - { name: stage_keys, type: "list<StageKey>", required: true, constraints: "Started から導出する非空・slug 一意の添字帳。静的計画の複製ではない" }
      - { name: overlay, type: "list<PlanAction>", required: true, constraints: "実効計画。Started の plan_action を初期値とし Recomposed で変更" }
      - { name: checkbox, type: "list<CheckboxState>", required: true, constraints: "誕生時 initialization は Completed、最初の実効対象実ステージは InProgress、残り Pending" }
      - { name: cursor, type: StageIndex, required: true }
      - { name: status, type: Status, required: true, constraints: "Running / Completed。park は独立のマーカー" }
      - { name: parked_at, type: StageIndex, required: false, constraints: "存在時は cursor と一致し status は Running" }
      - { name: autonomy, type: AutonomyMode, required: true, defaults: "Gated" }
      - { name: skeleton_stance, type: SkeletonStance, required: false, constraints: "未記録は None。skeleton ゲートの判断は未解決" }
      - { name: review_attempts, type: "list<ReviewAttempt>", required: true, constraints: "現在の試行の依頼数・判定待ち・受領証。誕生時は全て空" }
      - { name: practices_affirmed, type: "list<boolean>", required: true, constraints: "現在の試行の昇格受領証。practices-discovery 以外は false" }
      - { name: approved, type: "list<boolean>", required: true, constraints: "ゲート承認履歴。initialization は false" }
      - { name: revision_count, type: "list<u32>", required: true, constraints: "GateRejected 適用時に対象を飽和加算で +1。イベントのフィールドではない" }
      - { name: last_gate_resolution_at, type: "DateTime<Utc>", required: false, constraints: "GateApproved / GateRejected / autonomous への切替で更新。人間の操作の確認材料" }
      - { name: seq_nr, type: usize, required: true, min: 1, constraints: "Started = 1。適用時は現在値 +1 のみ" }
      - { name: version, type: usize, required: true, min: 0, constraints: "ストア採番の不透明な楽観ロック用トークン。未保存 0、読み戻した版を with_version で保持。seq_nr から導かず、apply は変更しない" }
      - { name: last_updated_at, type: "DateTime<Utc>", required: true, constraints: "最後の apply に渡された発生時刻" }
    constraints:
      - "stage_keys / overlay / checkbox / review_attempts / practices_affirmed / approved / revision_count の長さが等しい"
      - "active = InProgress / AwaitingApproval / Revising は高々 1。in-flight はこれに Pending を加えた集合"
      - "Running の cursor は実効 EXECUTE。非 initialization の Completed は approved を伴う"
      - "実効対象の実ステージが無い誕生は Running、cursor 0、active 0。通常 next は Done を導出する"
      - "通常コマンドは拒否時に状態不変、成功時に単一イベントを適用して返す。再生ではイベントを新規生成しない"
    relationships:
      - { to: Intent, cardinality: "many-to-one", direction: "IntentExecution → Intent", description: "intent_id のみ保持。判断材料は &Intent で渡す。コマンドは IntentMismatch を返し、クエリ境界の未解決差異は correction-report.md に記録" }
      - { to: StageKey, cardinality: "one-to-many", direction: "IntentExecution → StageKey", description: "イベント適用の添字帳を所有" }
      - { to: StageIndex, cardinality: "one-to-many", direction: "IntentExecution → StageIndex", description: "cursor / parked_at の位置" }

  - name: IntentExecutionEventId
    description: "orchestration が所有するイベント固有の識別子。集約 ID と通番の組ではない"
    attributes:
      - { name: value, type: string, required: true, unique: true, constraints: "UUIDv7。通常コマンド内で生成する裁定済み例外。再生では採番しない" }

  - name: IntentExecutionEvent
    description: "イベント固有 ID と集約 ID を持つドメインイベント。16 変種。通番・時刻・schema・直列化はアダプタ封筒の責務"
    attributes:
      - { name: id, type: IntentExecutionEventId, required: true, unique: true }
      - { name: aggregate_id, type: IntentExecutionId, required: true }
      - { name: kind, type: enum, required: true, allowed_values: [Started, GateOpened, GateApproved, GateRejected, StageRevised, StageSkipped, Jumped, Parked, Unparked, Recomposed, AutonomyModeSet, SingleStageRunCommitted, SkeletonStanceRecorded, ReviewRequested, ReviewCompleted, PracticesAffirmed] }
    payloads:
      - { kind: Started, fields: "intent_id: IntentId, stages: list<StageEntry>" }
      - { kind: GateOpened, fields: "stage: StageSlug, artifacts: list<string>" }
      - { kind: GateApproved, fields: "stage: StageSlug, user_input: string?" }
      - { kind: GateRejected, fields: "stage: StageSlug, feedback: string?" }
      - { kind: StageRevised, fields: "stage: StageSlug" }
      - { kind: StageSkipped, fields: "stage: StageSlug, reason: string" }
      - { kind: Jumped, fields: "target: StageSlug" }
      - { kind: Parked, fields: "stage: StageSlug" }
      - { kind: Unparked, fields: "共通の id / aggregate_id のみ" }
      - { kind: Recomposed, fields: "skipped: list<StageSlug>, added: list<StageSlug>" }
      - { kind: AutonomyModeSet, fields: "mode: AutonomyMode" }
      - { kind: SingleStageRunCommitted, fields: "stage: StageSlug" }
      - { kind: SkeletonStanceRecorded, fields: "stance: SkeletonStance" }
      - { kind: ReviewRequested, fields: "stage: StageSlug, reviewer: string, iteration: u32, retry_pending: boolean" }
      - { kind: ReviewCompleted, fields: "stage: StageSlug, reviewer: string, iteration: u32, verdict: ReviewVerdict" }
      - { kind: PracticesAffirmed, fields: "stage: StageSlug, affirming_user: string, sections: list<PromotedSection>, mandated: list<string>, forbidden: list<string>" }
    constraints:
      - "次ステージ・jump 方向と差分集合・revision_count は apply が導出。phase_boundary は RMU が導出"
      - "Started の計画は歴史の材料であり、別集約の現在状態を埋め込むこととは異なる"
    relationships:
      - { to: IntentExecution, cardinality: "many-to-one", direction: "IntentExecutionEvent → IntentExecution", description: "aggregate_id が事実の対象を指す" }
      - { to: StageEntry, cardinality: "one-to-many", direction: "Started → StageEntry", description: "誕生時計画の自己完結した履歴" }

referenced_types:
  - { owner: "core-command-domain::workflow_definition", types: [StageSlug, PhaseId, PlanAction, StageGraph, ScopeGrid, ScopeMetadata, StageNumber, ReviewPolicy] }
  - { owner: "core-command-domain::workspace", types: [CheckboxState, HumanTurns, PracticesPromotion, PromotedSection] }
  - { owner: "core-command-domain::orchestration", types: [StartRequest, WorkspaceScan, StageDisplay, Status, AutonomyMode, SkeletonStance, ReviewAttempt, ReviewVerdict, NextRequest, NextDecision, GateDecision, JumpDirection, IntentEvent, Created] }
  - { owner: "chrono", types: ["DateTime<Utc>"] }
persistence_boundary:
  snapshot: "ある通番時点の IntentExecution 自身を基底にする。WorkflowExecutionState / WorkflowExecutionSnapshot のドメイン双子型は存在しない"
  transport: "command interface-adapter の IntentExecutionDto と封筒が直列化・復号を所有。RMU は専用 DTO を所有"
  replay: "最新 snapshot の seq_nr より大きい差分だけを昇順適用。差分空なら基底を返す"
```

## 2. 要約

| 所有者 | 所有する情報・責務 |
|---|---|
| WorkflowDefinition | 定義 ID・内容版・グラフ・グリッド・スコープ |
| Intent | 定義への ID 参照、依頼、静的 StageEntry 列、開始時スキャン |
| IntentExecution | Intent への ID 参照、StageKey 列、overlay、進捗・承認・受領証・通番・読取版 |
| IntentExecutionEvent | 独立 UUIDv7 のイベント ID、aggregate_id、起きた事実。Started は計画の履歴 |
| アダプタ | 封筒、永続化 DTO、最新スナップショットと差分の取得、版の受け渡し |
