# rules — U2 ドメイン ES コア（`u2-domain-es-core`）

> Functional Design（Construction 3.1）成果物（Unit: U2）。出典: `../../../inception/units-generation/unit-of-work.md`（U2）、
> `../../../inception/units-generation/unit-of-work-story-map.md`（FR8.3 / FR8.4、FR2.1 / FR3.1 / FR3.3 の土台）、
> `../../../inception/requirements-analysis/requirements.md`（FR1.3 / FR2.1 / FR3.1 / FR3.3 / FR8.3 / FR8.4、NFR1 / NFR3）、
> `../../../inception/domain-design/decisions.md`（ADR-002 / 004 / 005 / 007）、`../../../inception/contract-design/contract-summary.md`
> （C3 / C5 / C6）、`formal/orchestration/engine_loop.qnt`（不変条件 run 27 本のうち orchestration 分: no_run_stage_for_skip /
> cursor_in_scope / no_gate_bypass / gate_lifecycle_preconditions / parked_position / unpark_restores_position /
> stale_rereport_yields_done / stale_rereport_frame / at_most_one_active）、現行コード `workflow_execution.rs` のガード、
> `docs/specs/research/orchestration-next-ladder.md`（分岐 2.5 / 2.6 / 6 / 7 / 9c / 10）、確認質問 `functional-design-questions.md`
> （Q1 = A / Q2 = A / Q3 = A、P1〜P6）、`aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/`（domain-equality /
> field-visibility / module-visibility / tell-dont-ask）。
>
> 下の fenced `yaml` が正本。BR1.x = 集約の不変条件と遷移ガード、BR2.x = イベントとリプレイ、BR3.x = next_decision、
> BR4.x = PlanAction 移動と畳み込み、BR5.x = 型・スナップショット・コーディング規則。

## 1. 規則（正本）

```yaml
rules:
  # --- BR1: 集約の不変条件と遷移ガード（engine_loop.qnt と 1:1） ---
  - id: BR1.0
    statement: "コマンド受理述語 accepts_commands = (status = running) ∧ (parked_at ≠ Some(cursor))。この述語が偽のとき unpark 以外のすべての decide コマンド（complete_stage / open_gate / approve_gate / reject_gate / revise_stage / skip_stage / jump / park / recompose / set_autonomy）は NotRunning を返し状態を変えない。park 中に jump で暗黙に park を解除することはない"
    category: constraint
    applies_to: [WorkflowExecution]
    trigger: "すべての decide コマンド（unpark を除く）"
    logic: "IF ¬accepts_commands THEN Err(NotRunning)。unpark は park 中（parked_at = Some(cursor)）のみ受理"
    violation: "Err（状態不変）。現行 `running()` ガードと Quint（park = WorkflowParked で他 action が発火しない）と 1:1"
    source: "engine_loop.qnt actPark / unpark_restores_position, 現行 workflow_execution.rs running(), レビュー所見 1"
  - id: BR1.1
    statement: "1 コマンド 1 イベント — 各 decide コマンドは成功時にちょうど 1 つの WorkflowExecutionEvent を返し、同じイベントを自身に apply した結果が次状態になる。失敗時（Err）は状態を変えずイベントも返さない"
    category: constraint
    applies_to: [WorkflowExecution, WorkflowExecutionEvent]
    trigger: "start / complete_stage / open_gate / approve_gate / reject_gate / revise_stage / skip_stage / jump / park / unpark / recompose / set_autonomy"
    logic: "IF ガード成立 THEN event を構築し apply_event(event) を呼び seq_nr を +1 して Ok(event) ELSE Err(材料) かつ状態不変"
    violation: "Vec<Event> 返し・複数イベント・副作用のある Err は設計違反（PBT: decide 後の状態 == 旧状態 + apply_event(event)）"
    source: "ADR-002 ①②, unit-of-work U2 実装ノート"
  - id: BR1.2
    statement: "用語: active = {InProgress, AwaitingApproval, Revising}（at_most_one_active 用）、in-flight = {Pending, InProgress, AwaitingApproval, Revising}（= 未完了、next_decision とジャンプ用）。cursor は稼働中（status = running）つねに in-scope（実効プラン = EXECUTE）のステージを指し、active なステージは高々 1 つ"
    category: constraint
    applies_to: [WorkflowExecution]
    trigger: "すべての遷移の後"
    logic: "不変条件 cursor_in_scope ∧ at_most_one_active（Quint）"
    violation: "違反する遷移は設計上存在しない（テストと PBT で確認）"
    source: "engine_loop.qnt cursor_in_scope / at_most_one_active"
  - id: BR1.3
    statement: "gated(stage) = (phase(stage) ≠ initialization)（Started の StageEntry.phase から。Quint slice-1 の gated(s) = s ≠ 0 はこの抽象で、ITF 準拠テストは initialization 1 ステージの合成計画で駆動する — BR2.5）。ゲートの完了は必ず承認を経る — approve_gate は AwaitingApproval または InProgress（open_gate 省略経路）からのみ成立し approved[stage] = true を立てる。非ゲート（initialization フェーズの各ステージ）は complete_stage（StageCompleted）で完了し approved は立てない。birth のユースケースは start 直後に initialization の各ステージを complete_stage で順に完了させる（1 コマンド 1 イベント — upstream の birth 時自動完了を 3 イベントで再現）"
    category: constraint
    applies_to: [WorkflowExecution]
    trigger: "approve_gate / complete_stage"
    logic: "IF ¬gated(cursor) THEN complete_stage のみ許可（approved は立てない）ELSE approve_gate のみ許可（complete_stage をゲートに呼ぶと InvalidTarget）。checkbox ∉ {InProgress, AwaitingApproval} なら Err(CheckboxPrecondition)"
    violation: "Err を返す。不変条件 no_gate_bypass: Completed ∧ gated ⇒ approved"
    source: "engine_loop.qnt no_gate_bypass / gate_lifecycle_preconditions, Q3 = A"
  - id: BR1.4
    statement: "ゲートの生存期間は InProgress → AwaitingApproval（open_gate）→ {Completed（approve_gate）| Revising（reject_gate）} → Revising から AwaitingApproval（revise_stage）に限る。reject_gate は InProgress / AwaitingApproval から、revise_stage は Revising からのみ"
    category: constraint
    applies_to: [WorkflowExecution]
    trigger: "open_gate / reject_gate / revise_stage"
    logic: "前提 checkbox が一致しなければ Err(CheckboxPrecondition{stage, actual})。非ゲート（initialization）への open_gate / reject_gate は Err(InvalidTarget)。reject_gate は revision_count[stage] を +1 し GateRejected.revision_count に載せる（所見 16 (a)）"
    violation: "Err を返す（状態不変）"
    source: "engine_loop.qnt gate_lifecycle_preconditions, 現行 FSM"
  - id: BR1.5
    statement: "skip_stage は accepts_commands（BR1.0）のもとで InProgress / Revising のステージにのみ成立し、そのステージが conditional か実効プランが SKIP である場合に限る。完了・スキップ後は次の in-scope ステージへ cursor を進め、無ければ status = completed"
    category: constraint
    applies_to: [WorkflowExecution]
    trigger: "skip_stage / approve_gate / complete_stage"
    logic: "next_in_scope(cursor) が Some(s) なら checkbox[s] = InProgress, cursor = s ELSE status = completed（イベントの next_stage に記録。next_stage 無し = 完了）"
    violation: "NotSkippable(stage) / CheckboxPrecondition"
    source: "engine_loop.qnt actReportSkipped / actReportForward, C5 StageSkipped"
  - id: BR1.6
    statement: "jump は accepts_commands（BR1.0）のもとで受理。target は stage_count 未満、forward / backward は target が非 initialization（gated）かつ in-scope、redo は cursor が非 initialization。forward のスキップ集合は Quint と同じ 2 条件 — (a) 介在ステージ（cursor < u < target）は in-flight（Pending を含む）ならすべて Skipped、(b) 現ステージ cursor は in-flight かつ非 Pending のときのみ Skipped。backward は target+1 以降の in-scope 非 Pending（InProgress を含む — cursor 自身も対象）を Pending に戻し target 以降の approved を消す、redo は cursor の approved を消す。target は InProgress、cursor = target"
    category: constraint
    applies_to: [WorkflowExecution]
    trigger: "jump(target)"
    logic: "direction = JumpDirection::of(cursor, target)。上記の差分を Jumped{direction, source, target, stages_reset, stages_skipped}（C5 の形、ステージは slug）に載せる（単一イベント）。承認の消去は apply 側が direction / target から決定的に導出（backward: target 以降、redo: source）"
    violation: "InvalidTarget(target) / NotRunning"
    source: "engine_loop.qnt actJumpForward / actJumpBackward / actJumpRedo, C5 Jumped"
  - id: BR1.7
    statement: "park は accepts_commands（BR1.0）かつ autonomy = gated のときのみ（autonomous 下は RefusedUnderAutonomy）。parked_at = cursor。unpark は park 中のみで位置を復元する（cursor 不変、Unparked のペイロードは空 — C5）"
    category: constraint
    applies_to: [WorkflowExecution]
    trigger: "park / unpark"
    logic: "不変条件 parked_position: parked ⇒ parked_at == cursor。unpark_restores_position"
    violation: "RefusedUnderAutonomy / NotRunning"
    source: "engine_loop.qnt actPark / actUnpark"
  - id: BR1.8
    statement: "recompose は accepts_commands（BR1.0）かつ gated のみ。引数は反転の集合（1 件以上）で、各対象は cursor より後ろの Pending ステージに限り、overlay の当該要素を反転（EXECUTE ⇄ SKIP）する — 1 コマンドで 1 つの Recomposed{skipped, added, stages_in_scope}（C5）。plan（静的グリッド）は不変。set_autonomy は accepts_commands のもとで mode を置き換える（setter。Quint の actSetAutonomy はトグル — 射影は BR2.5）"
    category: constraint
    applies_to: [WorkflowExecution]
    trigger: "recompose(flips) / set_autonomy(mode)"
    logic: "いずれかの対象が stage <= cursor ∨ checkbox[stage] ≠ Pending なら全体を Err（部分適用しない）。Recomposed{skipped, added, stages_in_scope}、AutonomyModeSet{mode}"
    violation: "InvalidTarget / CheckboxPrecondition / RefusedUnderAutonomy / NotRunning"
    source: "engine_loop.qnt actRecompose / actSetAutonomy, C5"
  - id: BR1.9
    statement: "stale_report（cursor より前の Completed ステージへの再報告）はクエリ（イベントなし・状態不変）で NextDecision::Done を返す（戻り値型は Result<NextDecision, CommandError>）。accepts_commands でなければ NotRunning、条件を満たさなければ NotStale"
    category: constraint
    applies_to: [WorkflowExecution]
    trigger: "stale_report(stage)"
    logic: "stage < cursor ∧ checkbox[stage] = Completed ⇒ Ok(Done) ELSE Err"
    violation: "Err"
    source: "engine_loop.qnt stale_rereport_yields_done / stale_rereport_frame, P2"

  # --- BR2: イベントとリプレイ ---
  - id: BR2.1
    statement: "【2026-08-27 改訂 / ADR-010・Bolt B6】イベント封筒は id（= `WorkflowExecutionEventId` = intent_id + seq_nr の Domain Primitive、B6 で新設）/ schema_version = 1 / occurred_at（`chrono::DateTime<Utc>` — 本家 `Event` trait の要求）を持ち、seq_nr は集約内で 1 から単調増加（Started = 1）。**値は従来と同じ 2 つ組**で、型がまとまっただけである。apply_event は seq_nr が現在値 + 1 でなければ Err（順序違反）。イベント ID の採番は決定的（集約 ID + seq_nr）"
    category: constraint
    applies_to: [WorkflowExecutionEvent, WorkflowExecution]
    trigger: "apply_event"
    logic: "IF event.seq_nr ≠ self.seq_nr + 1 THEN Err(SequenceGap{expected, actual})"
    violation: "Err（リプレイ不能の材料）"
    source: "C5 envelope, ADR-004 seq_nr, C6 UNIQUE(aggregate_id, seq_nr)"
  - id: BR2.2
    statement: "Started は自己完結 — definition_id / definition_revision（参照した定義の ID と内容版 — BR2.6）、scope / request と解決済み StageEntry 列（slug, phase, plan_action, conditional）を持ち、リプレイは WorkflowDefinition を参照しない。列は stages_in_scope(scope) が返す全ステージを文書（グラフ）順に並べたもので、plan_action はグリッドの 3 値 Option<PlanAction> を『None → SKIP』で 2 値に畳む（現行 next_in_scope_stage が == Some(Execute) 以外を in-scope 外とする挙動と等価。畳んだうえで Recomposed により EXECUTE 化できる点も 3 値契約と等価）。conditional は StageNode.execution = CONDITIONAL。stages[0] は EXECUTE かつ非 conditional でなければ start は Err"
    category: constraint
    applies_to: [WorkflowExecutionEvent, WorkflowExecution]
    trigger: "start / apply_event(Started)"
    logic: "start(&definition, scope) は is_valid_scope → stages_in_scope（文書順、全ステージ、PhaseId 付き）→ None→SKIP 畳み込み → conditional は同じ文書順の graph().nodes()[i].execution() から索引一致で取る（stages_in_scope は execution を返さない）→ StageEntry(slug, phase, plan_action, conditional) 列。UnknownScope / Empty（コンパイル済みグラフが空のときのみ — 防御的）/ InitializationMustExecute（initialization ステージが SKIP に畳まれた）/ InitializationMustBeUnconditional は Err"
    violation: "StartError"
    source: "P1, 現行 StartError, ADR-002 ②"
  - id: BR2.3
    statement: "リプレイの決定性 — from_snapshot(S) に seq_nr 以降のイベントを順に apply した集約と、通常実行で同じコマンド列を decide した集約は同値（PartialEq）。decide 後の状態は旧状態 + apply_event(event) と同値"
    category: validation
    applies_to: [WorkflowExecution]
    trigger: "PBT / ITF 準拠テスト"
    logic: "∀ コマンド列: replay(events) == execute(commands)"
    violation: "テスト失敗 = 実装を直す"
    source: "ADR-001 / ADR-002, NFR3（書く側の前提）"
  - id: BR2.4
    statement: "ドメインイベントは 12 変種（C5 の 11 + StageCompleted）で、ペイロードは C5 の形を正本とする（ステージ参照は StageSlug）。C5 への改訂提案は entities.md の c5_revision_proposal に列挙したもの（StageCompleted の追加 / Started.stages の形 / Started.definition_id・definition_revision の追加 / stage 系フィールドの型 = StageSlug / 投影規則）に限り、他の 10 変種のペイロードは変えない（GateOpened.artifacts / GateApproved.phase_boundary は呼出側が投影材料として渡す）。upstream 監査行語彙（86 語）とは別物で、1 イベント → N 監査行の描画は ReadModelUpdater（U4）の規則"
    category: policy
    applies_to: [WorkflowExecutionEvent]
    trigger: "イベント設計"
    logic: "変種の追加は C5 の改訂を伴う"
    violation: "レビューで差し戻し"
    source: "C5, ADR-003, Q3 = A"

  - id: BR2.5
    statement: "Quint（engine_loop.qnt）状態 ↔ 集約状態の射影: status=Running ⇔ status = running ∧ parked_at ≠ Some(cursor); WorkflowParked ⇔ status = running ∧ parked_at = Some(cursor); WorkflowCompleted ⇔ status = completed; parkedAt = -1 ⇔ parked_at = None、それ以外 ⇔ Some(index); autonomous ⇔ autonomy = autonomous; actSetAutonomy（トグル）⇔ set_autonomy(現在値の反転); actRecompose（1 ステージ反転）⇔ recompose({stage})（要素数 1）; lastDirective ⇔ EngineSignal（BR3.1 の導出）; Quint の stage 0（非ゲート）⇔ initialization フェーズ 1 ステージだけを持つ合成計画の索引 0（ITF 準拠テストは from_snapshot 相当の合成 Started で集約を作る）。ITF 準拠テストはこの表で射影を突き合わせる"
    category: validation
    applies_to: [WorkflowExecution]
    trigger: "ITF 準拠テスト"
    logic: "表のとおり（現行 assert_projection と同じ）"
    violation: "テスト失敗"
    source: "P6, NFR1, engine_loop_conformance.rs assert_projection, レビュー所見 11"

  - id: BR2.6
    statement: "集約間の依存は ID による間接参照 — WorkflowExecution は WorkflowDefinition を definition_id（WorkflowDefinitionId）で参照し、オブジェクトを保持しない。WorkflowDefinition はエンティティ（集約ルート、12 号 §2.1）なので内容が変わっても不変の id を持ち、内容版は revision（DefinitionRevision、値属性）で表す — 内容アドレスを ID にしない（オーナー裁定 2026-08-23）。start は def.id() / def.revision() を無条件に Started に記録する（比較対象となる既存状態が無い静的コンストラクタ — 検査しない）。Started 適用後に &WorkflowDefinition を受け取るクエリ／コマンド（現時点では next_decision のみ）は、引数の id が definition_id と一致しなければ Err(CommandError::DefinitionMismatch{expected, actual}) を返す。revision の差は Err にしない（計画は Started で自己完結 — upstream も dist 更新をまたいでワークフローを継続する）"
    category: constraint
    applies_to: [WorkflowExecution, WorkflowDefinition, WorkflowExecutionEvent]
    trigger: "start（記録のみ）/ next_decision（検査）/ apply_event(Started)"
    logic: "start: Started.definition_id = def.id(), Started.definition_revision = def.revision()（戻り値型は StartError のまま — DefinitionMismatch は持たない）。next_decision: IF definition.id() ≠ self.definition_id THEN Err(CommandError::DefinitionMismatch{expected: self.definition_id, actual: definition.id()})。WorkflowDefinitionId / DefinitionRevision は Repository 実装（U3 側 `WorkflowDefinitionRepositoryImpl`）が harness.json の name と 3 入力の正準 JSON ダイジェスト（canon-json）から付与し、C4 は find_by_id(&WorkflowDefinitionId) に改訂（find() は廃止、後方互換なし — Bolt B3 で trait / impl / 呼出側を同時修正）"
    violation: "DefinitionMismatch（状態不変）。定義 ID を持たない集約・オブジェクト参照を保持する集約はレビューで差し戻し"
    source: "オーナー裁定 2026-08-23（エンティティと ID 参照）、ADR-008、C4 / C5 改訂、12 号 §2.1"

  # --- BR3: next_decision（状態依存の分岐） ---
  - id: BR3.1
    statement: "next_decision は書込なしのクエリで、まず引数の定義の id を検査し（不一致は Err(DefinitionMismatch) — BR2.6、戻り値型は Result<NextDecision, CommandError>）、状態依存の分岐だけを次の優先順で判定する: (1) park 中（parked_at = cursor）かつ再入フラグなし → Parked / resume 指定なら UnparkThenResume、(2) resume 指定（非 park）→ ResumeMenu、(3) 稼働中の自由記述 → NewWorkRouting、(4) completed → Done、(5) cursor が in-flight（Pending を含む未完了）で実効プラン SKIP → InProgress / Revising なら RecoverSkipInconsistency、それ以外（Pending / AwaitingApproval）は InconsistentSkip、(6) cursor が in-flight → RunStage{cursor, gate = gated(cursor)}、(7) 次の in-scope → RunStage、無ければ Done。第 2 引数 &WorkflowDefinition は FR3.3 の合格基準が固定する契約上の引数で、Started 自己完結化により現時点の分岐では参照しない（将来の分岐のための予約 — 実装は `_definition` で未使用警告を抑える）"
    category: calculation
    applies_to: [WorkflowExecution, NextRequest, NextDecision]
    trigger: "next_decision(&self, &WorkflowDefinition, &NextRequest)"
    logic: "上記の順に最初に成立した分岐を返す。EngineSignal（Quint の 4 値）は RunStage → DRunStage、Done → DDone、Parked → DParked、InconsistentSkip / RecoverSkipInconsistency → DError、UnparkThenResume / ResumeMenu / NewWorkRouting → DDone（Quint の語彙に対応語が無い『ステージを走らせない・park でもエラーでもない停止』— ITF は踏まない。Bolt B3 実装判断 D4）で導出 — 8 値すべてに定義する"
    violation: "該当なし（ITF 準拠テストと分岐テーブルテストで検出）"
    source: "Q1 = A, orchestration-next-ladder 分岐 2.5 / 2.6 / 6 / 9c / 10, engine_loop.qnt no_run_stage_for_skip"
  - id: BR3.2
    statement: "状態非依存の分岐（read-only フラグ、名詞トークン、--stage/--phase 併用、scope 検証、compose、--new-intent、--single、設定変更、state なしの birth）は集約のクエリではなく、ユースケース前段の要求分類（入力検証・ルーティング）に属する — U6 が所有"
    category: policy
    applies_to: [NextRequest]
    trigger: "U6 の設計"
    logic: "集約が存在しない / フラグだけで決まる判断をドメインに置かない"
    violation: "レビューで差し戻し（FR3.3 の確認対象）"
    source: "Q1 = A, FR3.3"
  - id: BR3.3
    statement: "jump_resolve(target) は書込なしのクエリで、BR1.6 と同じ検証を行い JumpDirection を返す（`aidlc-jump resolve` の純読取に対応）。jump（コマンド）は jump_resolve が Ok の場合に限り Jumped を返す"
    category: calculation
    applies_to: [WorkflowExecution]
    trigger: "jump_resolve / jump"
    logic: "resolve と execute の分離（10 号 §2.3 jump_resolve）"
    violation: "Err(InvalidTarget)"
    source: "orchestration-next-ladder 分岐 7, 10 号 §2.3"

  # --- BR4: PlanAction の完全移動と畳み込み ---
  - id: BR4.1
    statement: "PlanAction は workflow_definition コンテキストが所有する。orchestration に定義も再輸出も置かず、全参照は workflow_definition::PlanAction を指す。実測 10 ファイルを同一 Bolt で一斉修正: core-domain の orchestration/plan_action.rs（移動対象）/ orchestration/mod.rs（再輸出の削除）/ orchestration/workflow_execution.rs / workflow_definition/scope_grid.rs / workflow_definition/workflow_definition.rs / workflow_definition/execution_kind.rs（doc）/ tests/engine_loop_conformance.rs、core-interface-adapter の src/orchestration/workflow_definition_repository_impl.rs / tests/workflow_definition_repository_impl_test.rs / tests/golden_parity_test.rs"
    category: policy
    applies_to: [PlanAction]
    trigger: "コンパイル・grep"
    logic: "IF orchestration に `PlanAction` の定義 or `pub use` が残る THEN FR8.3 不合格。判定式: grep -rnE 'enum PlanAction|pub use .*PlanAction' modules/core/domain/src/orchestration が 0 件（workflow_execution.rs の正当な利用は対象外）"
    violation: "CI（grep + ビルド）で検出。module-visibility の再エクスポート禁止"
    source: "FR8.3, ADR-005 改訂, coding-rules/module-visibility"
  - id: BR4.2
    statement: "有効プランの畳み込み（サフィックス = recompose ∨ グリッド）は WorkflowExecution の effective_plan(stage)（overlay）に一本化する。WorkflowDefinition から削除するのは『畳み込み』だけ — effective_plan_action と、その合成に依存する next_in_scope_stage（次の in-scope 判定は集約の next_decision が担う）。グリッド照会 grid().action(scope, slug)（= plan_action_in_grid）と、畳み込みを含まない既存の述語（is_valid_scope / valid_scopes / scope_metadata / subgraph_for_scope / stages_in_scope / first_in_scope_stage_of_phase）は残す"
    category: policy
    applies_to: [WorkflowExecution, WorkflowDefinition]
    trigger: "FR8.4"
    logic: "ES ではサフィックスが Recomposed イベントの適用結果なので、状態ファイルを読む合成は存在しない"
    violation: "WorkflowDefinition にオーバレイ（サフィックス）合成が残れば FR8.4 不合格。逆に畳み込みを含まない述語を消すのは過剰（レビュー所見 6）"
    source: "FR8.4, ADR-002 ⑤, 設計監査 C14"

  # --- BR5: 型・スナップショット・コーディング規則 ---
  - id: BR5.1
    statement: "ステージ位置は StageIndex（stage_count 未満を構築時に保証）で表し、集約 API・イベント・NextDecision・Snapshot で usize の生値を公開しない。StageIndex は当該集約の stage_index(usize) -> Option<StageIndex> でのみ構築"
    category: constraint
    applies_to: [StageIndex]
    trigger: "API 設計"
    logic: "範囲外は Option::None（Err の材料）で表し、panic しない（# Panics なし）"
    violation: "clippy missing_panics_doc / レビュー"
    source: "Q2 = A, 設計監査 C17 / C18"
  - id: BR5.2
    statement: "【2026-08-27 改訂 / ADR-010・Bolt B6】~~集約は serde に依存しない~~ → **失効**: 本家 `Aggregate` / `Event` trait が `Serialize` / `Deserialize` を境界に要求するため、集約・ドメインイベント・集約識別子は serde を持つ（Conformist、腐敗防止層なし）。**ただし serde は状態の写し（memento）を経由する** — 集約に `#[serde(into = \"WorkflowExecutionState\", try_from = \"WorkflowExecutionState\")]` を置き、直列化は `state()`、復号は `from_state()` へ委ねる。したがって**復号側の検査点は 1 か所のまま**で、不変条件は serde 経路でも破れない（オーナー裁定 2026-08-27）。~~イベント・スナップショットの JSON 化は U3 のワイヤ構造体が行う~~ → **失効**: ワイヤ構造体は削除され、ストアの payload は本家が書く（**それは契約 JSON ではない**）。upstream 観測面のワイヤ形式がアダプタ層のままである点は不変なので、BR5.2 の趣旨（観測互換をドメインの都合で動かさない）は保たれている"
    category: policy
    applies_to: [WorkflowExecution, WorkflowExecutionState, WorkflowExecutionEvent, WorkflowExecutionEventId]
    trigger: "永続化との境界"
    logic: "from_state の不変条件違反は Err(StateError::InvariantViolation{..})。serde の復号は try_from 経由でこの検査点を必ず通る"
    violation: "~~domain クレートに serde 依存が入れば設計違反~~ → **失効**。現在の違反は『集約の Deserialize が `from_state` を経由せず derive されている』こと"
    source: "P5, components.md PersistenceGateways, ~~ADR-006~~ → ADR-010"
  - id: BR5.3
    statement: "version はストアが採番する楽観ロック用の不透明な値で、ドメインは解釈も比較もしない（seq_nr と混ぜない — オーナー裁定 2026-08-27）。seq_nr は apply_event ごとに +1 するドメインの通番。復元はストアが返した値をそのまま保持する"
    category: constraint
    applies_to: [WorkflowExecution]
    trigger: "store / replay"
    logic: "Conflict 判定はストア（event-store-adapter-rs）の責務。ドメイン・Repository は version を前提条件に使わない（旧 statement の with_version(version + 1) と『version = 最後の seq_nr』は自前ストア時代の採番規則がドメイン契約へ漏れたもので、ADR-010 の Conformist 化で撤回。with_version は B6 委任 1 で削除済み）"
    violation: "該当なし"
    source: "ADR-004, C3, C6 snapshot.version"
  - id: BR5.4
    statement: "集約・イベント・値オブジェクトのフィールドは private でアクセサ経由、同値は PartialEq（ドメイン同値）で表し名前付き比較メソッドを置かない。エラーは手実装 enum + Display（材料のみ、文言はアダプタ層）、std::error::Error 手実装可（Bolt B1 ゲートの house style）"
    category: policy
    applies_to: [WorkflowExecution, WorkflowExecutionEvent, NextDecision]
    trigger: "実装"
    logic: "coding-rules 正本どおり"
    violation: "cargo lint / レビュー"
    source: "coding-rules field-visibility / domain-equality / module-visibility, U1 ゲート裁定"
```

## 2. 規則の要約

| ID | 区分 | 一言 | 出典 |
|---|---|---|---|
| BR1.1 | constraint | 1 コマンド 1 イベント、Err は状態不変 | ADR-002 |
| BR1.2 | constraint | cursor は in-scope、進行中は高々 1 | Quint |
| BR1.3 | constraint | ゲート完了は承認経由、非ゲート（initialization）は StageCompleted | Quint / Q3 |
| BR1.4 | constraint | ゲート生存期間（open / approve / reject / revise） | Quint |
| BR1.5 | constraint | skip の条件と次ステージへの前進 | Quint / C5 |
| BR1.6 | constraint | jump の検証と差分（単一 Jumped） | Quint / C5 |
| BR1.7 | constraint | park / unpark（gated のみ、位置復元） | Quint |
| BR1.8 | constraint | recompose（後ろの Pending のみ反転）/ set_autonomy | Quint / C5 |
| BR1.9 | constraint | stale_report はクエリ | Quint / P2 |
| BR2.1 | constraint | 封筒（`id` = intent_id + seq_nr、2026-08-27 / ADR-010）と seq_nr の単調性 | C5 / C6 |
| BR2.2 | constraint | Started は自己完結（解決済み計画） | P1 |
| BR2.3 | validation | リプレイの決定性（PBT / ITF） | ADR-001/002 |
| BR2.4 | policy | 12 変種、監査行は投影（U4） | C5 / ADR-003 |
| BR2.6 | constraint | 集約間は ID 参照（definition_id）、WorkflowDefinition に id / revision、不一致は DefinitionMismatch | オーナー裁定 / ADR-008 |
| BR3.1 | calculation | next_decision の状態依存分岐と優先順 | Q1 / ラダー |
| BR3.2 | policy | 状態非依存分岐は U6 の要求分類 | Q1 / FR3.3 |
| BR3.3 | calculation | jump_resolve と jump の分離 | ラダー 7 |
| BR4.1 | policy | PlanAction 完全移動・再輸出なし | FR8.3 |
| BR4.2 | policy | 畳み込みは集約へ、定義はグリッド照会のみ | FR8.4 |
| BR5.1 | constraint | StageIndex で範囲を型保証 | Q2 |
| BR5.2 | policy | ~~serde なし~~ → **serde は memento 経由**（2026-08-27 / ADR-010）、`state()` / `from_state()` | P5 |
| BR5.3 | constraint | version / seq_nr の責務 | ADR-004 / C3 |
| BR5.4 | policy | private + アクセサ、PartialEq、手実装エラー | coding-rules |
