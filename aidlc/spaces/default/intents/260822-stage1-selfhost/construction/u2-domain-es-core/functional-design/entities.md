# entities — U2 ドメイン ES コア（`u2-domain-es-core`）

> Functional Design（Construction 3.1）成果物（Unit: U2）。出典: `../../../inception/units-generation/unit-of-work.md`（U2 の責務）、
> `../../../inception/domain-design/decisions.md`（ADR-001 ES / ADR-002 集約 = FSM・1 コマンド 1 イベント・decide/apply /
> ADR-004 集約ルート・version・seq_nr / ADR-005 PlanAction 完全移動）、`../../../inception/domain-design/components.md`
> （OrchestrationEngine / WorkflowDefinitionModel / WorkspaceModel、PersistenceGateways「serde はゲートウェイ層に閉じる」）、
> `../../../inception/contract-design/contract-summary.md`（C3 Repository ポート、C5 イベント語彙と封筒、C6 スナップショット列）、
> 現行コード `modules/core/domain/src/orchestration/workflow_execution.rs`（状態 9 フィールド・12 コマンド）、
> `formal/orchestration/engine_loop.qnt`（状態変数と不変条件）、確認質問 `functional-design-questions.md`（Q1 = A / Q2 = A /
> Q3 = A、P1〜P6）。
>
> 実装言語に依存しない論理モデル。下の fenced `yaml` が正本。

## 1. エンティティ（正本）

```yaml
entities:
  - name: IntentId
    description: "集約 WorkflowExecution の識別子（intent の記録ディレクトリ名 — intents.json の dirName、実データ例 `260822-stage1-selfhost`）。Always Valid な Domain Primitive"
    attributes:
      - { name: value, type: string, required: true, unique: true, constraints: "kebab 表記（`[a-z0-9]+` を `-` で連結、前後空白は trim）。`<kebab-slug>-<id8>` も `<YYMMDD>-<slug>` もこの形の部分集合（Bolt B3 実装判断 D3 — `-<id8>` 必須は実データと不一致のため撤回）。構築時に形を検証" }
    constraints:
      - "空・不正形は構築できない（parse-don't-validate）"

  - name: WorkflowDefinitionId
    description: "集約 WorkflowDefinition（エンティティ）の識別子。内容（ピン更新・プラグイン選択・再コンパイル）が変わっても不変の系譜 ID — オーナー裁定 2026-08-23（エンティティは内容が変わっても追跡可能でなければならない。内容アドレスは値の同一性であり ID にしない）。Always Valid な Domain Primitive"
    attributes:
      - { name: value, type: string, required: true, unique: true, constraints: "このハーネスにインストールされた定義の系譜を表す。供給元は Repository 実装（harness.json の `name`、例 `claude` — 12 号 §3 の 3 入力と同じ harness dir）。空・不正形は構築できない" }

  - name: DefinitionRevision
    description: "WorkflowDefinition の内容版（値属性 — 識別子ではない）。3 入力（コンパイル済み stage-graph / scope-grid / scope identity 群）の正準 JSON（U1 canon-json hash-canonical）の sha256 ダイジェスト。同じ内容なら同じ revision、ピン更新で変わる。来歴と drift 検出の材料"
    attributes:
      - { name: value, type: string, required: true, constraints: "`sha256:<hex64>`。Repository 実装が束ね直し時に計算して載せる（ドメインは計算しない — canon-json 依存はアダプタ層）" }

  - name: WorkflowDefinition
    description: "既存の集約ルート（workflow_definition コンテキスト、12 号 §2.1）。本 Unit で **識別子 id と内容版 revision を追加**する（エンティティに ID が無かった欠落の是正 — オーナー指摘 2026-08-23）。他の内包（StageGraph / ScopeGrid / ScopeDefinition 群）は変更なし。読取専用・ES 対象外"
    attributes:
      - { name: id, type: WorkflowDefinitionId, required: true, unique: true, constraints: "Repository が付与。`WorkflowDefinition::id()` で公開" }
      - { name: revision, type: DefinitionRevision, required: true, constraints: "Repository が付与。`WorkflowDefinition::revision()` で公開" }
    constraints:
      - "WorkflowDefinitionRepository は `find_by_id(&WorkflowDefinitionId)`（C4 改訂 — 既存 `find()` は廃止、後方互換の併存なし）。要求 id がこのハーネスの定義 id と異なれば NotFound（契約上 fatal）"

  - name: StageIndex
    description: "ステージ位置（0 始まり）。ある実行の stage_count 未満であることを型で保証する E1 型（Q2 = A）"
    attributes:
      - { name: value, type: integer, required: true, min: 0, constraints: "value < 所属する WorkflowExecution の stage_count。集約だけが構築できる（`WorkflowExecution::stage_index(usize) -> Option<StageIndex>` 相当）" }
    constraints:
      - "比較・順序は value による（前後関係 = ジャンプ方向の導出に使う）"
      - "非ゲートかどうかは索引ではなく StageEntry.phase で判定する（initialization フェーズ = 非ゲート。Quint の stage 0 は ITF 用合成計画上の抽象 — BR1.3 / BR2.5）"

  - name: StageEntry
    description: "Started イベントに載る解決済みの 1 ステージ分の計画（定義が後で変わってもリプレイが決定的になるための自己完結データ — P1）"
    attributes:
      - { name: slug, type: StageSlug, required: true, unique: true }
      - { name: phase, type: PhaseId, required: true, constraints: "stages_in_scope が返す PhaseId。gated = (phase ≠ initialization) をここから導く（所見 14）" }
      - { name: plan_action, type: PlanAction, required: true, constraints: "グリッド（scope 列）から解決した EXECUTE / SKIP（None → SKIP）。initialization ステージは必ず EXECUTE" }
      - { name: conditional, type: boolean, required: true, constraints: "同じ文書順の graph().nodes()[i].execution() = CONDITIONAL（stages_in_scope は execution を返さない）。initialization ステージは必ず false" }

  - name: WorkflowExecution
    description: "集約ルート = エンジン FSM（ADR-002）。状態としてのデータ・状態遷移（decide / apply）・判断（next_decision 等のクエリ）を 1 型に閉じ込める。I/O なし・純粋・同期。~~serde に依存しない（P5）~~ → **失効**: 2026-08-27 / ADR-010 で本家 `Aggregate` trait が serde 境界を要求するため `Serialize`/`Deserialize` を derive する（`#[serde(into/try_from)]` で memento を経由、下記 constraints 参照）"
    attributes:
      - { name: intent_id, type: IntentId, required: true, unique: true }
      - { name: definition_id, type: WorkflowDefinitionId, required: true, constraints: "集約間の間接参照（ID のみ保持、WorkflowDefinition のオブジェクトは保持しない）。Started で確定し以後不変（start は記録のみ）。next_decision に渡される &WorkflowDefinition の id と一致しなければ Err(CommandError::DefinitionMismatch) — BR2.6" }
      - { name: definition_revision, type: DefinitionRevision, required: true, constraints: "start 時点の定義の内容版（来歴）。以後不変。定義側の revision が進んでも計画は Started で自己完結しているため Err にはしない（drift は観測のみ — BR2.6）" }
      - { name: stages, type: list<StageEntry>, required: true, constraints: "索引 → StageEntry（slug / phase / plan_action / conditional）。Started で確定し以後不変。長さ = stage_count ≥ 1。phase を保持するので再水和後も gated = phase ≠ initialization を再計算できる（Bolt B3 実装判断 D1 — 旧 list<StageSlug> では phase が失われ実装不能）。plan / conditional は独立列のまま持ち、from_snapshot が StageEntry との整合を検査する" }
      - { name: plan, type: list<PlanAction>, required: true, constraints: "静的グリッド由来。Started で確定し以後不変（Quint: plan）" }
      - { name: overlay, type: list<PlanAction>, required: true, constraints: "実効プランの源（Quint: overlay）。Started 時は plan の写し、Recomposed で対象要素が反転" }
      - { name: conditional, type: list<boolean>, required: true, constraints: "Started で確定し以後不変" }
      - { name: checkbox, type: list<CheckboxState>, required: true, constraints: "6 値。Started 時 stage 0 = InProgress、他 = Pending" }
      - { name: cursor, type: StageIndex, required: true }
      - { name: status, type: enum, required: true, allowed_values: [running, completed], constraints: "Quint の WorkflowParked は status = running ∧ parked_at = Some(cursor) に対応（BR2.5 の射影表）" }
      - { name: parked_at, type: StageIndex, required: false, constraints: "park 中は cursor と一致（Quint 不変条件 parked_position）" }
      - { name: autonomy, type: AutonomyMode, required: true, defaults: "gated" }
      - { name: approved, type: list<boolean>, required: true, constraints: "ゲート承認履歴。非ゲート（initialization）ステージは常に false" }
      - { name: revision_count, type: list<integer>, required: true, min: 0, constraints: "ステージごとの差し戻し回数。reject_gate で +1（GateRejected.revision_count の供給元 — レビュー所見 16 (a)）。upstream 状態ファイルの Revision Count の材料" }
      - { name: seq_nr, type: integer, required: true, min: 0, constraints: "集約内で単調増加。適用したイベント数と一致（Started = 1）。apply_event ごとに +1" }
      - { name: version, type: usize, required: true, min: 0, constraints: "【2026-08-27 改訂 / ADR-010・Bolt B6】楽観 version = **ストアが採番する不透明なトークン**。ドメインは解釈も比較もせず、`seq_nr` から導かない（BR5.3）。~~Repository の store 成功後に +1（集約は `with_version` で受け取る）~~ → **失効**: `with_version` は削除され、載せ替えの口は本家 `Aggregate::set_version` 1 本だけ（借り物の契約なので綴りも可変性も本家が所有する）。新しい version を知るのはストアだけなので、続けて書くには再水和が要る。集約内の遷移では変えない" }
      - { name: last_updated_at, type: "chrono::DateTime<Utc>", required: true, constraints: "【2026-08-27 新設 / ADR-010】最後に適用したイベントの `occurred_at`（本家 `Aggregate::last_updated_at` の要求）。集約は時計を持たないので、値は常に適用したイベントから来る" }
    constraints:
      - "stages / plan / overlay / conditional / checkbox / approved の長さはすべて stage_count に等しい"
      - "status = running のとき cursor の実効プランは EXECUTE（Quint: cursor_in_scope）"
      - "active（InProgress / AwaitingApproval / Revising）なステージは高々 1 つ（Quint: at_most_one_active）。in-flight（Pending を含む未完了）とは区別する — rules BR1.2"
      - "ゲート（phase ≠ initialization）の Completed は approved = true を伴う（Quint: no_gate_bypass）"
      - "decide は単一イベントを返し、同じイベントを自身に apply した結果が次状態（1 コマンド 1 イベント）"
    relationships:
      - { to: WorkflowDefinition, cardinality: "many-to-one", direction: "WorkflowExecution → WorkflowDefinition", description: "**ID による間接参照**（definition_id を保持。オブジェクトは保持しない — 集約間参照の規約）。start は &WorkflowDefinition の id / revision を Started に記録するだけ（検査対象の既存状態が無い）、next_decision は引数の id が definition_id と一致することを検査する（BR2.6）。リプレイは定義を要しない（Started が自己完結）" }

  - name: WorkflowExecutionEvent
    description: "ドメインイベント（コマンドと 1:1、12 変種 — C5 の 11 + StageCompleted（Q3 = A））。封筒 + ペイロード"
    attributes:
      - { name: id, type: WorkflowExecutionEventId, required: true, constraints: "【2026-08-27 改訂 / ADR-010・Bolt B6】~~intent_id / seq_nr の 2 属性~~ → **Domain Primitive `WorkflowExecutionEventId`（intent_id + seq_nr）に統合**（本家 `Event::id()` の要求）。値は同じ 2 つ組で、採番は決定的（集約 ID + seq_nr）。`seq_nr` は適用後の集約 seq_nr と一致（min: 1）、型は `usize`（本家に従う）" }
      - { name: schema_version, type: integer, required: true, defaults: 1, constraints: "C5 の予約。追加フィールドは消費側が無視。**2026-08-27 補足**: 復号時に値を検査する経路は無くなった（対応外の版も復号失敗に畳まれる — `CorruptCause::SchemaVersion` は削除）" }
      - { name: occurred_at, type: "chrono::DateTime<Utc>", required: true, constraints: "【2026-08-27 改訂 / ADR-010】~~ISO 8601 UTC（文字列）~~ → **`chrono::DateTime<Utc>`**（本家 `Event::occurred_at()` の要求。自前 ISO 8601 整形は撤去 — **NFR4.1 依存最小化の再検討対象**）。ユースケースが Clock から渡す（集約はクロックを持たない）" }
      - { name: is_created, type: "fn() -> bool", required: true, constraints: "【2026-08-27 新設 / ADR-010】genesis 判定（`Started` のみ真）。本家 `Event::is_created()` の要求で、ストアが create 経路と更新経路を分けるために使う" }
      - { name: kind, type: enum, required: true, allowed_values: [Started, StageCompleted, GateOpened, GateApproved, GateRejected, StageRevised, StageSkipped, Jumped, Parked, Unparked, Recomposed, AutonomyModeSet] }
      - { name: payload, type: record, required: true, constraints: "kind ごとに固定（下の payloads）" }
    payloads:                          # C5 の形を正本とする。ステージ参照は StageSlug（自己記述 — 投影側が索引表を要しない）。StageIndex は集約 API の内部表現
      - { kind: Started, fields: "definition_id: WorkflowDefinitionId, definition_revision: DefinitionRevision, scope, request, stages: list<StageEntry>（文書順の全ステージ）, depth?, test_strategy?" }
      - { kind: StageCompleted, fields: "stage: StageSlug（非ゲート = initialization フェーズのみ）, next_stage: StageSlug?" , note: "C5 への追加提案（Q3 = A）— 非ゲート完了。next_stage 無し = ワークフロー完了" }
      - { kind: GateOpened, fields: "stage: StageSlug, artifacts: list<path>（呼出側が渡す投影材料 — C5 どおり）" }
      - { kind: GateApproved, fields: "stage: StageSlug, user_input: string?, next_stage: StageSlug?, phase_boundary: record?（呼出側が定義から導出して渡す投影材料 — C5 どおり）" }
      - { kind: GateRejected, fields: "stage: StageSlug, feedback: string?, revision_count: integer" }
      - { kind: StageRevised, fields: "stage: StageSlug" }
      - { kind: StageSkipped, fields: "stage: StageSlug, reason: string, next_stage: StageSlug?" }
      - { kind: Jumped, fields: "direction: JumpDirection, source: StageSlug, target: StageSlug, stages_reset: list<StageSlug>, stages_skipped: list<StageSlug>" , note: "承認の消去は direction と target から決定的に導出（backward: target 以降 / redo: source）— C5 の形を保つ" }
      - { kind: Parked, fields: "stage: StageSlug" }
      - { kind: Unparked, fields: "（なし — C5 どおり。位置は parked_at から復元）" }
      - { kind: Recomposed, fields: "skipped: list<StageSlug>, added: list<StageSlug>, stages_in_scope: list<StageSlug>（適用後）" , note: "1 コマンドで複数ステージの反転をまとめて 1 イベントにする（C5 どおり）。Quint の actRecompose（1 ステージ反転）は要素数 1 の Recomposed に対応" }
      - { kind: AutonomyModeSet, fields: "mode: AutonomyMode" }
    c5_revision_proposal:
      - "追加: StageCompleted（Q3 = A）— 非ゲート（initialization フェーズ）ステージの完了"
      - "変更: Started.stages_in_scope（list<StageSlug>）→ stages（list<StageEntry> = slug + phase + plan_action + conditional、文書順の全ステージ）— P1（自己完結）と所見 14（phase の保持）の帰結"
      - "変更: Started に definition_id / definition_revision を追加 — WorkflowDefinition（エンティティ）への ID 参照と来歴（オーナー裁定 2026-08-23、ADR-008）"
      - "変更なし: 他の 10 変種のペイロードは C5 のまま（初稿の artifacts / phase_boundary の削除、Unparked.stage、Recomposed {stage, from, to} は取り下げ）"
      - "型の明示: C5 の `stage` / `next_stage` / `stages_*` は StageSlug"
      - "投影規則の改訂提案（U4 と合意）: Started は WORKFLOW_STARTED / PHASE_STARTED(initialization) / STAGE_STARTED(先頭) を描き、initialization 各ステージの STAGE_COMPLETED（+ 次の STAGE_STARTED、フェーズ境界の PHASE_COMPLETED / PHASE_STARTED）は StageCompleted ごとに描く — 1 コマンド 1 イベントの帰結（所見 14 の裁定案 A）"
    constraints:
      - "イベントは構築後不変。材料はアクセサで公開（~~serde なし — JSON 化は U3 のワイヤ構造体~~ → **失効**: 2026-08-27 / ADR-010 で本家 `Event` trait が serde 境界を要求するため derive する。ストアの payload は本家が書き、**それは契約 JSON ではない**）"

  - name: WorkflowExecutionState
    description: "**失効（2026-08-29 / Bolt B12）**: 型は `IntentExecutionSnapshot`（クレート内私有）へ改名・縮小（17 → 12 属性 — 静的側は `Intent` 構造体へ分離）。`state()`/`from_state()`/`StateError`/`Builder` は `snapshot()`/`from_snapshot()`/`SnapshotError`/`IntentExecutionSnapshotBuilder` へ（B5 の改名は巻き戻った）。検査点は実行時不変条件が `from_snapshot`、計画依存の検査は `&Intent` を受ける面の 2 か所に分割。以下は B12 以前の記録 — 集約の全状態の値オブジェクト（C6 snapshot 行の論理形）。集約 → `snapshot()`、集約 ← `from_snapshot(...)`。~~serde なし~~ → **2026-08-27 改訂**: 型名は Bolt B5 で `WorkflowExecutionSnapshot` から `WorkflowExecutionState`（`state()` / `from_state()` / `StateError` / `WorkflowExecutionStateBuilder`）へ改名済み（**2026-08-27: 正本のエンティティ名もここで追従**）。**serde を持つ**が、集約の serde がこの写しを経由する（`#[serde(into/try_from)]`）ので、**復号側の検査点は `from_state()` の 1 か所のまま**（ADR-010 / オーナー裁定 2026-08-27）"
    attributes:
      - { name: state, type: record, required: true, constraints: "WorkflowExecution の全属性（intent_id / definition_id / definition_revision / stages（StageEntry 列）/ plan / overlay / conditional / checkbox / cursor / status / parked_at / autonomy / approved / revision_count / seq_nr / version ~~の 16 属性~~ → **+ last_updated_at の 17 属性**、2026-08-27 / ADR-010）をそのまま持つ。構築は公開ビルダー（引数 17 個を避ける house style）" }
    constraints:
      - "from_state は集約不変条件を検証し、違反は Err（Corrupt 相当の材料）。**serde の復号経路もここを通る**"

  - name: NextRequest
    description: "next_decision への入力のうちワークフロー状態の判断に要る観測（状態非依存のフラグ処理はユースケース前段 — Q1 = A）"
    attributes:
      - { name: resume, type: boolean, required: true, constraints: "`--resume` 指定" }
      - { name: reentry, type: boolean, required: true, constraints: "`--stage` / `--phase` / `--review` / `--new-intent` のいずれか（park ガード 2.5 を外す再入フラグ）" }
      - { name: free_text, type: boolean, required: true, constraints: "稼働中に自由記述 prose が来た（9c）" }

  - name: NextDecision
    description: "next_decision の結果（書込なし）。状態依存の分岐だけを表す閉集合。EngineSignal（Quint の DirectiveKind 4 値）はここから導出"
    attributes:
      - { name: kind, type: enum, required: true, allowed_values: [RunStage, Done, Parked, UnparkThenResume, ResumeMenu, NewWorkRouting, RecoverSkipInconsistency, InconsistentSkip], constraints: "stale_report も Done を NextDecision で返す（EngineSignal との導出規則は BR3.1）" }
      - { name: stage, type: StageIndex, required: false, constraints: "RunStage / Parked / RecoverSkipInconsistency / InconsistentSkip" }
      - { name: gate, type: boolean, required: false, constraints: "RunStage: gated(stage) = phase ≠ initialization" }
      - { name: checkbox, type: CheckboxState, required: false, constraints: "RecoverSkipInconsistency / InconsistentSkip の観測状態" }

  - name: PlanAction
    description: "EXECUTE / SKIP（workflow_definition コンテキスト所有へ完全移動 — FR8.3 / ADR-005）"
    attributes:
      - { name: value, type: enum, required: true, allowed_values: [EXECUTE, SKIP] }

relationships:
  - { from: WorkflowExecutionEvent, to: WorkflowExecution, cardinality: "many-to-one", description: "集約 1 つのイベント列。seq_nr 順に apply すると集約が再構成される" }
  - { from: WorkflowExecutionState, to: WorkflowExecution, cardinality: "one-to-one", description: "ある seq_nr 時点の全状態" }
  - { from: WorkflowExecution, to: StageIndex, cardinality: "one-to-many", description: "cursor / parked_at / イベントのステージ参照はすべて StageIndex" }
  - { from: WorkflowExecutionEvent, to: StageEntry, cardinality: "one-to-many", description: "kind = Started のペイロードが解決済み計画（slug / plan_action / conditional）の列を持つ" }
```

## 2. 要約

- **WorkflowExecution** は現行 FSM の 9 フィールドに `intent_id` / `stages` / `seq_nr` / `version` を足した集約ルート。状態・遷移・
  判断を 1 型に閉じ、decide は単一イベントを返し apply がリプレイと通常実行を同一経路にする。
- **WorkflowExecutionEvent** は 12 変種（C5 の 11 + `StageCompleted`）。封筒（~~intent_id / seq_nr~~ → **`id`
  （`WorkflowExecutionEventId` = intent_id + seq_nr、2026-08-27 / ADR-010 で新設した Domain Primitive）** /
  schema_version / occurred_at）+ 変種ごとのペイロード。`Started` は解決済み計画（StageEntry 列）を自己完結で持つ。
- **StageIndex** が `usize` を置き換え、範囲不変条件を型で守る（Q2 = A）。**IntentId** が集約識別子。
- **WorkflowExecutionState**（旧 `WorkflowExecutionSnapshot`、B5 で改名。2026-08-27: 正本のエンティティ名も追従）と **NextDecision / NextRequest** は値オブジェクト。
  ~~serde はドメインに入れない。~~ → **失効（2026-08-27 / ADR-010・Bolt B6）**: 集約・ドメインイベント・集約識別子は
  serde を持つ（本家 trait の境界要求）。ただし集約の復号は memento を経由するので検査点は 1 か所のまま。
- **PlanAction** は workflow_definition へ完全移動（再輸出なし）。
