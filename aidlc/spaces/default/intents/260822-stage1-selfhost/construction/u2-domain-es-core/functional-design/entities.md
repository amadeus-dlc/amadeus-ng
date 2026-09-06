# entities — U2 ドメイン ES コア（`u2-domain-es-core`）

> 2026-09-05 是正。以下の YAML が現行の論理モデルの正本であり、第 2 節と functional-spec.md 第 6 節はその導出ビュー。
> 根拠は [設計裁定](../../../inception/domain-design/decisions.md)、[共有契約](../../../inception/contract-design/contract-summary.md)、
> リポジトリルート基準の `aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/` にある
> `aggregate-commands.md`（2026-09-05 差分再生訂正）、`aggregate-references.md`、
> `domain-persistence-neutrality.md`、`cqrs-boundaries.md`、`gateway-taxonomy.md`。
> 旧 WorkflowExecution、memento、本家 trait 直実装、初期化を逐次完了する設計は後続裁定により失効した。
> 過去の質問回答・pending-revision・functional-spec.md の旧 Review（`functional-spec-review-history-2026-09-05.md`）は履歴であり、現行 API の導入指示ではない。
>
> 2026-09-07 再走（Modify、質問票 Q4 / Q4a / Q5・P7〜P10）: オーナー裁定 2026-09-06 `coding-rules/first-class-collections.md` と
> 2026-09-07 回答「リードモデルでは使わず、コマンド側ドメインモデルの配列部分は FCC」を反映し、コマンド側の配列を
> ファーストクラスコレクション（FCC）として定義した（BR5.5）。`next_decision` の ID 照合（Q5 = A）も反映した。
> ここに書く FCC 型は設計であり、実装は U2 の code-generation 再走で行う（現行コードは `Vec` と `&[..]` のまま）。

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
      - { name: stages, type: StageEntries, required: true, constraints: "文書順の全ステージ、非空、slug 一意。生成後不変。FCC（BR5.5）" }
      - { name: scan, type: WorkspaceScan, required: true }
      - { name: created_at, type: "DateTime<Utc>", required: true }
    constraints:
      - "create は定義と StartRequest から計画を解決・検査し、Intent と Created イベントを対で返す"
      - "静的 plan_action / conditional / display はこの集約が所有する"
    relationships:
      - { to: WorkflowDefinition, cardinality: "many-to-one", direction: "Intent → WorkflowDefinition", description: "definition_id による ID 参照。定義オブジェクトを埋め込まない" }
      - { to: StageEntries, cardinality: "one-to-one", direction: "Intent → StageEntries", description: "静的計画（FCC）の所有。要素は StageEntry" }

  - name: StageEntry
    description: "orchestration の値オブジェクト。Created と Started に履歴の材料として載る自己完結の計画要素"
    attributes:
      - { name: slug, type: StageSlug, required: true }
      - { name: phase, type: PhaseId, required: true }
      - { name: plan_action, type: PlanAction, required: true, constraints: "None → SKIP。initialization は EXECUTE" }
      - { name: conditional, type: boolean, required: true, constraints: "initialization は false" }
      - { name: display, type: StageDisplay, required: true, constraints: "番号・表題・担当の単一行表示材料" }

  - name: StageEntries
    description: "orchestration のファーストクラスコレクション。静的計画の列（Intent.stages / Created.stages / Started.stages が共有する型）"
    attributes:
      - { name: items, type: "list<StageEntry>", required: true, constraints: "非空、slug 一意、文書順。内部表現は private" }
    constraints:
      - "操作: at(StageIndex) / position_of(slug) / filter / fold_left / map（slug 衝突は Result で拒否）/ combine（連結、slug 衝突は Result で拒否）/ divide（他方に含まれる slug を除いた列、空可の型へ戻る）、業務操作: first_of(phase, plan_action)（skeleton 対象の特定）、check_plan（先頭 EXECUTE、initialization は EXECUTE かつ非 conditional、slug 一意）"
      - "要素列挙は DTO の符号化とリードモデルへの写しだけの理由付き例外（fold_left を優先）"

  - name: StageKey
    description: "orchestration の値オブジェクト。実行がイベントの slug を解決しフェーズを判定するための最小の添字（StageSlot の鍵部分）"
    attributes:
      - { name: slug, type: StageSlug, required: true }
      - { name: phase, type: PhaseId, required: true }

  - name: StageSlot
    description: "orchestration の値オブジェクト。1 つの位置の添字と進捗記録（旧 7 並列列の 1 行）"
    attributes:
      - { name: key, type: StageKey, required: true }
      - { name: plan_action, type: PlanAction, required: true, constraints: "実効計画（旧 overlay）。Started の plan_action を初期値とし Recomposed で変更" }
      - { name: checkbox, type: CheckboxState, required: true }
      - { name: approved, type: boolean, required: true, constraints: "ゲート承認履歴。initialization は false" }
      - { name: revision_count, type: u32, required: true, constraints: "GateRejected 適用時に飽和加算で +1" }
      - { name: review_attempt, type: ReviewAttempt, required: true, constraints: "現在の試行の依頼数・判定待ち・受領証。誕生時は空" }
      - { name: practices_affirmed, type: boolean, required: true, constraints: "現在の試行の昇格受領証。practices-discovery 以外は false" }

  - name: StageSlots
    description: "orchestration のファーストクラスコレクション。実行の位置ごとの記録の列（旧 stage_keys / overlay / checkbox / approved / revision_count / review_attempts / practices_affirmed の 7 並列列を 1 要素 1 位置に統合）"
    attributes:
      - { name: items, type: "list<StageSlot>", required: true, constraints: "非空、slug 一意、文書順（Started の StageEntries と同じ長さ・順序）。内部表現は private" }
    constraints:
      - "操作: at(StageIndex) / position_of(slug) / filter / fold_left / map（要素型は StageSlot のまま、slug 衝突は Result で拒否）/ combine（連結、slug 衝突は Result で拒否）/ divide（他方に含まれる slug を除いた列、空可の型へ戻る）"
      - "業務操作: active_count（InProgress / AwaitingApproval / Revising の数）、positions(述語) → StageIndexSet、next_effective_execute_after(StageIndex)、first_effective_execute、stage_key(StageIndex)、with_slot(StageIndex, 更新)、with_slots(StageIndexSet, 更新)（apply_event の更新入口。jump の一括 Skipped / Pending 戻しは位置集合の演算で書く）、clear_receipts（全位置のレビュー試行・昇格受領証を消す）"
      - "旧『7 列の長さが等しい』不変条件はこの型で消える。旧 IntentExecution.stage_keys() は stage_key(StageIndex) と fold_left による導出ビューになる"

  - name: StageIndexSet
    description: "orchestration のファーストクラスコレクション。位置（StageIndex）の集合。jump の介在位置・巻き戻し対象・受領証リセット対象の合成に使う"
    attributes:
      - { name: items, type: "list<StageIndex>", required: true, constraints: "重複なし、昇順。所属実行の stage_count 未満" }
    constraints:
      - "操作: at / filter / fold_left / combine（和集合）/ divide（差集合）/ range(from, to)（区間の生成）。空集合を単位元とする Monoid として試験する"
      - "例: forward の Skipped 対象 = range(cursor+1, target) ∩ in_scope ∩ in_flight、backward の Pending 戻し対象 = range(target+1, end) ∩ in_scope ∖ Pending"

  - name: ArtifactPaths
    description: "orchestration のファーストクラスコレクション。GateOpened が運ぶ成果物パスの列"
    attributes:
      - { name: items, type: "list<string>", required: true, constraints: "順序保持、空を許す" }
    constraints:
      - "操作: at / filter / fold_left / combine（連結、集合ではないので重複を消さない）/ divide（他方と等しい要素を除く）。map は不要（要素の変換先が無い）"

  - name: StageSlugSet
    description: "orchestration のファーストクラスコレクション。Recomposed の skipped / added が運ぶ反転対象の集合"
    attributes:
      - { name: items, type: "list<StageSlug>", required: true, constraints: "重複なし、文書順。recompose 入力の重複はここで集合化" }
    constraints:
      - "操作: at / filter / fold_left / combine（和集合）/ divide（差集合）。空集合を単位元とする Monoid として試験する。recompose 入力の位置集合（StageIndexSet）から添字帳で slug へ写して作る"

  - name: StageIndex
    description: "orchestration の位置型。公開の位置解決は IntentExecution.stage_index(usize) → Option<StageIndex>"
    attributes:
      - { name: value, type: integer, required: true, min: 0, constraints: "所属実行の stage_count 未満。別実行から渡された位置はコマンドでも検査" }

  - name: IntentExecution
    description: "orchestration の集約ルート = 実行 FSM。decide / apply と状態に基づく判断を所有する。serde・ストア trait・memento 型を持たない"
    attributes:
      - { name: id, type: IntentExecutionId, required: true, unique: true }
      - { name: intent_id, type: IntentId, required: true }
      - { name: slots, type: StageSlots, required: true, constraints: "Started から導出する位置ごとの記録（添字・実効計画・checkbox・承認・差し戻し回数・レビュー会計・昇格受領証）。静的計画の複製ではない。誕生時 initialization は Completed、最初の実効対象実ステージは InProgress、残り Pending。FCC（BR5.5）" }
      - { name: cursor, type: StageIndex, required: true }
      - { name: status, type: Status, required: true, constraints: "Running / Completed。park は独立のマーカー" }
      - { name: parked_at, type: StageIndex, required: false, constraints: "存在時は cursor と一致し status は Running" }
      - { name: autonomy, type: AutonomyMode, required: true, defaults: "Gated" }
      - { name: skeleton_stance, type: SkeletonStance, required: false, constraints: "未記録は None。skeleton ゲートの判断は未解決" }
      - { name: last_gate_resolution_at, type: "DateTime<Utc>", required: false, constraints: "GateApproved / GateRejected / autonomous への切替で更新。人間の操作の確認材料" }
      - { name: seq_nr, type: usize, required: true, min: 1, constraints: "Started = 1。適用時は現在値 +1 のみ" }
      - { name: version, type: usize, required: true, min: 0, constraints: "ストア採番の不透明な楽観ロック用トークン。未保存 0、読み戻した版を with_version で保持。seq_nr から導かず、apply は変更しない" }
      - { name: last_updated_at, type: "DateTime<Utc>", required: true, constraints: "最後の apply に渡された発生時刻" }
    constraints:
      - "位置ごとの記録は slots の 1 要素であり、旧 7 列の長さ一致は型で保証される。revision_count は apply が導出しイベントのフィールドではない"
      - "active = InProgress / AwaitingApproval / Revising は高々 1。in-flight はこれに Pending を加えた集合"
      - "Running の cursor は実効 EXECUTE。非 initialization の Completed は approved を伴う"
      - "実効対象の実ステージが無い誕生は Running、cursor 0、active 0。通常 next は Done を導出する"
      - "通常コマンドは拒否時に状態不変、成功時に単一イベントを適用して返す。再生ではイベントを新規生成しない"
    relationships:
      - { to: Intent, cardinality: "many-to-one", direction: "IntentExecution → Intent", description: "intent_id のみ保持。判断材料は &Intent で渡す。コマンド・書込前ガード・next_decision はすべて ID 不一致を IntentMismatch の Err で拒否する（Q5 = A、2026-09-07）" }
      - { to: StageSlots, cardinality: "one-to-one", direction: "IntentExecution → StageSlots", description: "位置ごとの記録の列を所有。添字帳はその鍵部分" }
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
      - { kind: Started, fields: "intent_id: IntentId, stages: StageEntries" }
      - { kind: GateOpened, fields: "stage: StageSlug, artifacts: ArtifactPaths" }
      - { kind: GateApproved, fields: "stage: StageSlug, user_input: string?" }
      - { kind: GateRejected, fields: "stage: StageSlug, feedback: string?" }
      - { kind: StageRevised, fields: "stage: StageSlug" }
      - { kind: StageSkipped, fields: "stage: StageSlug, reason: string" }
      - { kind: Jumped, fields: "target: StageSlug" }
      - { kind: Parked, fields: "stage: StageSlug" }
      - { kind: Unparked, fields: "共通の id / aggregate_id のみ" }
      - { kind: Recomposed, fields: "skipped: StageSlugSet, added: StageSlugSet" }
      - { kind: AutonomyModeSet, fields: "mode: AutonomyMode" }
      - { kind: SingleStageRunCommitted, fields: "stage: StageSlug" }
      - { kind: SkeletonStanceRecorded, fields: "stance: SkeletonStance" }
      - { kind: ReviewRequested, fields: "stage: StageSlug, reviewer: string, iteration: u32, retry_pending: boolean" }
      - { kind: ReviewCompleted, fields: "stage: StageSlug, reviewer: string, iteration: u32, verdict: ReviewVerdict" }
      - { kind: PracticesAffirmed, fields: "stage: StageSlug, affirming_user: string, sections: PromotedSections, mandated: RuleLines, forbidden: RuleLines（workspace::PracticesPromotion と同じ FCC 型）" }
    constraints:
      - "次ステージ・jump 方向と差分集合・revision_count は apply が導出。phase_boundary は RMU が導出"
      - "Started の計画は歴史の材料であり、別集約の現在状態を埋め込むこととは異なる"
    relationships:
      - { to: IntentExecution, cardinality: "many-to-one", direction: "IntentExecutionEvent → IntentExecution", description: "aggregate_id が事実の対象を指す" }
      - { to: StageEntries, cardinality: "one-to-one", direction: "Started → StageEntries", description: "誕生時計画の自己完結した履歴（Intent.stages と同じ型）" }

referenced_types:
  - { owner: "core-command-domain::workflow_definition", types: [StageSlug, PhaseId, PlanAction, StageGraph, ScopeGrid, ScopeMetadata, StageNumber, ReviewPolicy] }
  - { owner: "core-command-domain::workspace", types: [CheckboxState, HumanTurns, PracticesPromotion, PromotedSection, PromotedSections, RuleLines] }
  - { owner: "core_infrastructure::collections", types: [FirstClassCollection, Collection, NonEmptyCollection], note: "FCC の共通契約（2026-09-06。trait は len / at / fold_left / filter。combine / divide / map は型ごとの契約であり、共通 trait へ盛り込む方向はオーナーの最終方針として積み残し — Q4a）。StageGraph / ScopeGrid（workflow_definition）と Checkboxes / BoltRefs / AuditFields / OrderedAuditEvents（workspace）は実装済み（P10）。StageEntries / StageSlots / ArtifactPaths / StageSlugSet / PromotedSections / RuleLines は本設計で新設し code-generation で実装する" }
  - { owner: "core-command-domain::orchestration", types: [StartRequest, WorkspaceScan, StageDisplay, Status, AutonomyMode, SkeletonStance, ReviewAttempt, ReviewVerdict, NextRequest, NextDecision, GateDecision, JumpDirection, IntentEvent, Created] }
  - { owner: "chrono", types: ["DateTime<Utc>"] }
persistence_boundary:
  snapshot: "ある通番時点の IntentExecution 自身を基底にする。WorkflowExecutionState / WorkflowExecutionSnapshot のドメイン双子型は存在しない"
  transport: "command interface-adapter の IntentExecutionDto と封筒が直列化・復号を所有。RMU は専用 DTO を所有"
  replay: "最新 snapshot の seq_nr より大きい差分だけを昇順適用。差分空なら基底を返す"
  collections: "リードモデル側（read-model-updater / クエリ側）は FCC を使わず自前の平坦な表現へ写す（オーナー裁定 2026-09-07）。DTO・リードモデル境界への要素列挙は fold_left を優先し、イテレータ公開は理由を記した最後の手段"
```

## 2. 要約

| 所有者 | 所有する情報・責務 |
|---|---|
| WorkflowDefinition | 定義 ID・内容版・グラフ・グリッド・スコープ |
| Intent | 定義への ID 参照、依頼、静的 StageEntry 列、開始時スキャン |
| IntentExecution | Intent への ID 参照、位置ごとの記録の列 StageSlots（添字・実効計画・進捗・承認・受領証）、cursor / park / 権限・通番・読取版 |
| FCC（StageEntries / StageSlots / StageIndexSet / ArtifactPaths / StageSlugSet / PromotedSections / RuleLines） | コマンド側ドメインモデルの配列部分。不変条件と操作（at / filter / fold_left / map / combine / divide + 業務操作）を型が持ち、生の配列を外へ出さない（BR5.5） |
| IntentExecutionEvent | 独立 UUIDv7 のイベント ID、aggregate_id、起きた事実。Started は計画の履歴 |
| アダプタ | 封筒、永続化 DTO、最新スナップショットと差分の取得、版の受け渡し |
