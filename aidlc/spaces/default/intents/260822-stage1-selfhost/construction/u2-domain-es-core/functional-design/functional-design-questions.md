# functional-design-questions — U2 ドメイン ES コア（`u2-domain-es-core`）

> Functional Design（Construction 3.1）の質問票（Unit: U2、kind: library、規模 L、Bolt: B3）。出典:
> `../../../inception/units-generation/unit-of-work.md`（U2 — ES 形 FSM、decide / apply 分離、version / seq_nr、
> `next_decision`、畳み込みの集約メソッド化、PlanAction の完全移動）、`../../../inception/units-generation/
> unit-of-work-story-map.md`（FR8.3 / FR8.4、FR1〜FR3 の土台、FR3.3 の層配置確認）、`../../../inception/requirements-analysis/
> requirements.md`（FR1.3 / FR2.1 / FR3.1 / FR3.3 / FR8.3 / FR8.4、NFR1 / NFR3）、`../../../inception/domain-design/
> decisions.md`（ADR-001〜007）と `components.md`（OrchestrationEngine / WorkflowDefinitionModel / WorkspaceModel /
> PersistenceGateways「ワイヤ構造体（serde）はこの層に閉じる」）、`../../../inception/contract-design/contract-summary.md`
> （C3 Repository ポート、C5 ドメインイベント語彙 11 変種と投影規則、C6 スキーマ）、設計監査
> `knowledge/aidlc-shared/design-audit-2026-08-22.md`（R1 / R2、C13 / C14 / C17 / C18、B-2 繰延: StageIndex E1）、
> 実コード `modules/core/domain/src/orchestration/workflow_execution.rs`（現行 FSM: 12 コマンド・`EngineSignal` 4 値・
> `usize` ステージ索引）、`formal/orchestration/engine_loop.qnt`（契約正本、ITF 準拠テスト）、`docs/specs/10-orchestration.md`
> §2.3（`next_decision` = 21 分岐ラダー、書込なし）、`docs/specs/research/orchestration-next-ladder.md`（分岐の完全列挙）。
>
> 設計が一意に決まらない 3 点を問う。それ以外は前提 P1〜P6 として確認する。

## 質問

### Q1. `next_decision`（次に何をするかの判断、21 分岐ラダー）のうち集約が持つ範囲

upstream の `next` ラダー 21 分岐のうち、**ワークフロー状態（集約）に依存する**のは「park 中か（2.5 / 2.6）」「稼働中の
ステージが進行中か・SKIP 不整合か・次の in-scope ステージか・完了か（10）」「ジャンプの方向と帰属（7）」「`--resume` で
再開メニューを出すべきか（6）」「稼働中に自由記述が来た（9c）」です。残り（read-only フラグ 1、名詞トークン 1b〜1d、
`--stage`/`--phase` 併用 2、scope 検証 3b / 4、compose 4c、`--new-intent` 4a、`--single` 4b、設定変更 5、state なしの
birth 7b / 8 / 9a / 9b）は**集約が存在する前**か**フラグだけ**で決まり、集約の状態を読みません。

- A. 状態依存の分岐だけを `WorkflowExecution::next_decision(&self, &WorkflowDefinition, &NextRequest) -> NextDecision` に
  閉じ込め、状態非依存の分岐はユースケース前段の「要求分類」（入力検証とルーティング — 判断ロジックではなく前処理）と
  位置づける。FR3.3 の「ユースケース層に判断ロジックを置かない」は「ワークフロー状態に関する判断」と読む — 推奨
- B. 21 分岐すべてを集約側に置く: 集約が無いケースも扱う純関数 `decide_next(Option<&WorkflowExecution>, &WorkflowDefinition,
  &NextRequest)` を orchestration コンテキストに置く（ユースケースは本当に配線だけ）
- C. 状態依存でも `--resume` メニュー（6）とジャンプ解決（7）はユースケースに残し、集約は in-flight / park / done の
  最小判断のみ持つ（現行 `next()` の 4 値 `EngineSignal` に近い）
- X. Other (please specify)

[Answer]: A

### Q2. `StageIndex`（ステージ位置を表す型 — 範囲の不変条件を型で守る E1 型）を本 Unit で導入するか

現行の集約は `usize` でステージを指し、範囲外は `# Panics` 漏れの原因（設計監査 C17 / C18、恒久解として B-2 = 本 Unit に
繰延）。ES 化でイベントのペイロードにもステージ位置が乗るため、型を決める好機です。

- A. 導入する — `StageIndex`（`stage_count` 未満であることを構築時に保証する Always Valid 型）を orchestration に新設し、
  集約 API・イベント・`NextDecision` で `usize` を置き換える。ITF 準拠テストは変換で吸収。U2 の規模が少し増える — 推奨
- B. 今回は見送り、`usize` のまま（範囲検査は実行時 `Err`）。StageIndex は後続 Unit（U6）で導入
- X. Other (please specify)

[Answer]: A

### Q3. 非ゲートステージ（stage 0 = initialization）の完了を表すイベント名

C5 の 11 変種（Started / GateOpened / GateApproved / GateRejected / StageRevised / StageSkipped / Jumped / Parked /
Unparked / Recomposed / AutonomyModeSet）には「ゲートを経ない完了」が無い。現行 FSM では stage 0 の `report_forward` が
承認なしで完了し、Quint モデル（engine_loop.qnt）もその形です。

- A. `StageCompleted`（ゲート無し完了）を第 12 変種として追加する。C5 は「11 変種程度」なので追加で整合 — 推奨
- B. `GateApproved` に `gated: false` を持たせて畳む（11 変種のまま。監査行の投影は `gated` で分岐）
- C. `Started` が stage 0 を完了済みにする（Quint モデルと ITF 準拠テストの改訂が要る — 非推奨）
- X. Other (please specify)

[Answer]: A

## 前提（確認事項）

- P1. **ES 形の集約**: `WorkflowExecution`（identity = `IntentId`）は現行 FSM の状態（plan / overlay / conditional / checkbox /
  cursor / status / parked_at / autonomy / approved）に `version`（楽観）と `seq_nr`（集約内の単調増加）を足し、
  `stages: Vec<StageSlug>`（索引 → slug）を `Started` イベントから保持する。コマンドは decide（`&mut self`、単一イベントを
  `Result` で返し、自身にも適用）、`apply_event(&mut self, &WorkflowExecutionEvent)` がリプレイと通常実行を同一経路にする。
  `start` は `&WorkflowDefinition` と scope から plan（グリッド）と conditional（`execution: CONDITIONAL`）を解決し、
  **`Started` のペイロードに解決済みの (slug, plan_action, conditional) 列を載せる**（定義が後で変わってもリプレイが決定的）。
- P2. **1 コマンド 1 イベント**: `jump` の複合遷移（複数 checkbox のリセット / スキップ）も単一 `Jumped` に全差分
  （direction / source / target / stages_reset / stages_skipped）を載せる。`stale_report` は状態を変えないので
  クエリのまま（イベントなし）。`next_decision` もクエリ（書込なし）。
- P3. **PlanAction の完全移動（FR8.3）**: `orchestration/plan_action.rs` を `workflow_definition/plan_action.rs` へ移し、
  `orchestration` からの再輸出は置かない。呼出側（`scope_grid.rs` / `workflow_definition.rs` / `execution_kind.rs` の doc /
  `workflow_execution.rs` / ITF 準拠テスト / `core-interface-adapter` の `workflow_definition_repository_impl.rs` とテスト 2 本 /
  `golden_parity_test.rs`）を同一 Bolt で一斉修正。
- P4. **畳み込みの移設（FR8.4）**: `WorkflowDefinition::effective_plan_action` と `next_in_scope_stage` の「サフィックス（recompose）
  がグリッドに勝つ」合成を集約の `effective_plan(stage)`（overlay）に一本化し、`WorkflowDefinition` にはグリッド照会
  （`plan_action_in_grid(scope, slug)` = 既存 `grid.action`）のみ残す。ES ではサフィックス = `Recomposed` イベントの適用結果
  （overlay）なので状態ファイルを読む合成は不要。
- P5. **serde はドメインに入れない**（components.md PersistenceGateways「ワイヤ構造体（serde）はこの層に閉じる」、ADR-006
  「ドメインは純粋・同期」）: 集約は `snapshot() -> WorkflowExecutionSnapshot`（プレーンな値オブジェクト、アクセサ公開）と
  `from_snapshot(...)` を提供し、イベントはアクセサで材料を公開する。JSON への変換（C6 の payload）は U3 のワイヤ構造体。
  `occurred_at` は封筒（C5）の項目で、集約はクロックを持たずユースケースが渡す。
- P6. **Quint / ITF 維持**: `engine_loop.qnt` の意味論は変えない。ITF 準拠テストは「Quint の action → decide、状態射影の突合せ」
  に書き換え、`EngineSignal` は `NextDecision` から導出する（または `next_decision` の最小射影として維持）。新規の
  イベント／リプレイ性質は PBT（`apply_event` のリプレイ = decide 後の状態、1 コマンド 1 イベント）で固定する。

## Consolidated Summary Confirmation

- Q1 = A: `next_decision` は状態依存の分岐（park 2.5/2.6、進行中・SKIP 不整合・次 in-scope・完了 = 10、ジャンプ方向 7、
  resume メニュー要否 6、稼働中の自由記述 9c）を集約のクエリとし、状態非依存の分岐（フラグ・scope 検証・birth 前）は
  ユースケース前段の「要求分類」（入力検証・ルーティング）とする。FR3.3 は「ワークフロー状態に関する判断がユースケースに無い」と読む
- Q2 = A: `StageIndex`（範囲不変条件を型で守る）を導入し、集約 API・イベント・`NextDecision` の `usize` を置き換える
- Q3 = A: 非ゲート完了イベント `StageCompleted` を第 12 変種として追加（C5 の 11 変種 + 1）
- P1〜P6: ES 形の集約（version / seq_nr / stages、decide / apply 分離、`Started` に解決済み plan を搭載）、1 コマンド 1 イベント
  （`Jumped` に全差分、`stale_report` / `next_decision` はクエリ）、PlanAction 完全移動の呼出側一覧、畳み込みの移設
  （`WorkflowDefinition` はグリッド照会のみ）、serde はドメインに入れない（snapshot 値オブジェクト + アクセサ、JSON は U3）、
  Quint / ITF 維持（`EngineSignal` は `NextDecision` から導出）

Does this all look correct before I generate the artifact?

- Looks correct
- Request changes

[Answer]: Looks correct
