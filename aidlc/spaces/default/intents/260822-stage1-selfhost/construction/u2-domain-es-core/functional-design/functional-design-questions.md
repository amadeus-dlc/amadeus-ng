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

## 以前に確認済みのまとめ（2026-08 の確認、2026-09-05 是正で一部上書き）

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

## 2026-09-07 再走（Modify）— 是正後の差分に関する追加質問

> 出典: 2026-09-05 の是正記録 `../correction-report.md`（残る差異 = `next_decision` の ID 照合）、オーナー裁定
> 2026-09-06 のコーディング規則 `coding-rules/first-class-collections.md`（配列・イテレータを外へ取り出す前に
> コレクション自身の操作で表す）、現行コード `modules/core/command/domain/src/orchestration/`（`Intent::stages()` /
> `IntentExecution::stage_keys()` はスライス `&[..]` を返す。消費側は command interface-adapter の DTO 3 か所、
> read-model-updater の `ResolvedPlan::of`（計画の写し）と `read_tables`（行生成・slug 引当）、および集約内の
> `IntentExecution::skeleton_gate_stage` が `intent.stages().iter().position(..)` で判断）、
> `next_decision(&Intent, &NextRequest) -> NextDecision`（全コマンドと `jump_resolve` / `skeleton_gate_stage` / `gated`
> は `matches(intent)` で ID を照合するが、`next_decision` だけ照合が無い。現行の唯一の呼出側は read-model-updater の
> `NextAnswerRow::of` で、`intent.id() == execution.intent_id()` で引いた Intent を渡す）。
> Q1〜Q3・P1〜P6 は 2026-08 の記録であり、その後の裁定（Intent / IntentExecution 分離、16 変種、誕生時初期化完了、
> 最新スナップショット + 差分再生）で上書きされた項目は是正済み本文が正である。再質問はしない。

### Q4. 静的計画と添字帳のファーストクラスコレクション化（2026-09-06 規則の U2 への適用範囲）

用語: 「静的計画」= Intent が誕生時に解決した全ステージの列（`Intent.stages`、StageEntry の並び）。「添字帳」=
実行がイベントの slug を位置へ解決するための最小の列（`IntentExecution.stage_keys`、StageKey の並び）。
「ファーストクラスコレクション」= 配列を生のまま外へ出さず、`at` / `filter` / `map` / `fold_left` と業務上の操作を
自身に持つコレクション型（2026-09-06 のオーナー規則。適用済みは StageGraph / ScopeGrid / Checkboxes など 7 型で、
上記 2 列は未適用）。現状は両方とも `&[..]` のスライスを公開しており、消費側が外で走査している。

- A. 両方を導入する — orchestration に `StageEntries`（非空・slug 一意・文書順）と `StageKeys`（同じ不変条件）を
  ファーストクラスコレクションとして新設し、`at` / `filter` / `fold_left` / `map`（slug 衝突は Result で拒否）に加え、
  実際に使われる業務操作（slug → 位置、最初の Construction かつ EXECUTE の位置 = skeleton 対象、位置以降の列、
  位置以降で最初の実効 EXECUTE）を持たせる。公開 getter はコレクション型を返し、要素列挙は DTO の符号化境界だけの
  理由付き例外とする。集約内の判断（`skeleton_gate_stage` 等）はコレクションの操作で書く。実装は U2 の
  code-generation で行い、read-model-updater 側の消費コード（`ResolvedPlan::of` / `read_tables`）の書換えも
  同じ Bolt に含めて CI を緑に保つ — 推奨
- B. A と同じ設計だが、read-model-updater 側の書換えは U4 の code-generation に繰り延べる（U2 の Bolt では
  理由付きの移行用スライス accessor を一時的に残す）
- C. 今回は導入しない — スライス公開を DTO / リードモデル境界の理由付き例外として設計に記録し、集約をまたぐ判断
  （`skeleton_gate_stage`）だけを Intent 側の操作へ移す
- X. Other (please specify)

[Answer]: X — リードモデルでは使わないでください。コマンド側でドメインモデルの配列部分があるならFCCを使ってください。

### Q5. `next_decision`（次に何をすべきかの判断）の Intent ID 照合（是正記録の残件 BR2.6）

用語: 「ID 照合」= 集約が引数で受け取った別集約（Intent）が自分の参照先（`intent_id`）と一致するかを確かめ、
不一致なら `IntentMismatch` の Err で拒否すること（`coding-rules/aggregate-references.md`）。全コマンドと
書込前ガード（`jump_resolve`）は照合済み。`next_decision` だけ照合が無く、取り違えた Intent を渡されると skeleton
ゲートの判断材料だけが別計画で計算される。是正記録は「所有範囲を越える API 整理の残件」として未解決にしている。

- A. 戻り値を `Result<NextDecision, CommandError>` にし、不一致は `IntentMismatch` で拒否する（`jump_resolve` /
  `report_dispatch` と同じ形）。呼出側（read-model-updater の `NextAnswerRow::of`）は Err を判断結果ではなく
  投影の束縛不整合として扱う。実装は U2 の code-generation、呼出側の更新も同じ Bolt — 推奨
- B. 戻り値は `NextDecision` のまま、照合は呼出側（ID で引いた Intent を渡す構成）の責務と設計に明記し、
  `aggregate-references` からの容認された逸脱として記録する
- C. `&Intent` 依存を外へ出す — 呼出側が `skeleton_gate_stage(intent)`（照合済み、不一致は None）で skeleton 対象を
  求めて渡す `next_decision(skeleton_target: Option<StageIndex>, &NextRequest)` に変える（判断材料の組立てが
  集約の外へ出るため非推奨）
- X. Other (please specify)

[Answer]: A

### Q4a. FCC の結合（combine = 和集合 / 連結）と差集合（divide）の扱い（オーナー指摘による追問）

指摘: 結合・差集合の高階操作が無い FCC は、結局イテレータで取り出して外で合成することになり、ロジックが分散する。
実測: 規則 §「結合と差集合」は `combine` / `divide` を定めるが、共通 trait `FirstClassCollection` は `len` / `at` /
`fold_left` / `filter` のみ。汎用 `Collection<T>` / `NonEmptyCollection<T>` と `BoltRefs` / `AuditFields` は
`combine` / `divide` を持ち、`StageGraph` / `ScopeGrid` / `Checkboxes` / `OrderedAuditEvents` は持たない。

- A. 型ごとの契約として、本設計の全 FCC に `combine` / `divide` を定める（列は連結 + slug 衝突は Result、集合は
  和集合 / 差集合 + Monoid 則）。共通 trait への一律化は結果型と失敗条件が型ごとに異なるため今回は行わない — 推奨
- B. 共通 trait `FirstClassCollection` にも `combine` / `divide`（結果型は関連型）を入れる方針にし、規則本文と既存 7 型の
  改修を U2 の code-generation に加える
- C. 反映案を修正する
- X. Other (please specify)

[Answer]: 1 (= A) — 最終的にはtraitに盛り込みたい

Q4a の解釈: 今回は A（型ごとの契約）で進め、`combine` / `divide`（および `map`）を共通 trait へ盛り込む方向を
オーナーの最終方針として設計と日誌に記録する（積み残し。Issue は起票しない。着手時期は別途裁定）。

## 前提（2026-09-07 再走で確認する事項）

- P7. **上流の古い再生方式の記述**: `components.md` 冒頭注記（2026-08-30）と `contract-summary.md` C3 の B13 追記
  （2026-08-30）は「ジャーナル全再生」と書くが、オーナー裁定 2026-09-05（`coding-rules/aggregate-commands.md` の
  再生方式の訂正）は「最新スナップショット + それより後の差分」である。U2 設計は 2026-09-05 裁定に従う（BR2.3）。
  上流 2 ファイルの同期は本ステージの成果物外なので、intent 記録（本質問票と日誌）に積み残しとして書く（Issue は起票しない）。
- P8. **コード冒頭説明の乖離**: `orchestration/mod.rs` の冒頭は「再構成はジャーナル全再生」「`next_decision` は
  クエリ側（`ExecutionStateView`）が所有」と書くが、実コードは `IntentExecution::next_decision` を持ち、再生は
  BR2.3 の形。これは設計変更ではなくドキュメント修正として code-generation への引継ぎ項目に載せる。
- P9. **旧レビュー節の退避**: functional-spec.md 末尾の 2026-09-05 NOT-READY レビュー節（是正前の所見）は
  `functional-spec-review-history-2026-09-05.md` へ原文のまま移し、本文冒頭から参照する。所見への対応は
  是正記録のとおり反映済みであり、今回の独立レビューはこの是正済み本文に対して行う。
- P10. **2026-09-06 のコレクション型の反映**: workflow_definition の StageGraph / ScopeGrid と workspace の
  Checkboxes 等がファーストクラスコレクション契約（`FirstClassCollection`）を実装した事実を entities.md の
  referenced_types と BR4.2 の注記に現行事実として記録する（残す 6 述語は変えない）。

## Consolidated Summary Confirmation

2026-09-07 再走（Modify）。Q1〜Q3・P1〜P6 の 2026-08 の確認と 2026-09-05 の是正済み本文は維持し、以下を追加で反映する。

- Q4 = X（オーナー回答「リードモデルでは使わないでください。コマンド側でドメインモデルの配列部分があるなら FCC を使ってください」）:
  コマンド側ドメインモデルの配列はすべてファーストクラスコレクション（FCC）にする。対象と型の設計（新規 BR5.5）:
  静的計画 `Intent.stages` / `Created.stages` / `Started.stages` → `StageEntries`（非空・slug 一意・文書順）。
  集約の位置ごとの並列 7 列（stage_keys / overlay / checkbox / approved / revision_count / review_attempts /
  practices_affirmed）→ 位置ごとの進捗記録を 1 要素に持つ 1 つの FCC `StageSlots`（非空・slug 一意・文書順、
  `StageIndex` で `at`。「長さが等しい」不変条件は型で消える）。`GateOpened.artifacts` → `ArtifactPaths`（順序保持・空可）。
  `Recomposed.skipped / added` → `StageSlugSet`（重複なし・文書順）。`PracticesAffirmed` と `PracticesPromotion` の
  sections / mandated / forbidden → `PromotedSections` / `RuleLines`（順序保持・重複なし）。`ReviewAttempt` の
  pending / closed と `ReportDecision::Commit.steps` は値オブジェクト・判断結果の内部列として FCC 化の対象に含め、
  各型が実際に使う操作（at / filter / fold_left / map + 業務操作）だけを持たせる。
- Q4a = A（オーナー追問 2026-09-07「結合・差集合の高階操作が無いと結局イテレータで取り出してロジックが分散する」）:
  本設計の全 FCC に `combine` / `divide` を型ごとの契約として定める（列は連結で slug 衝突は Result、集合は和集合 /
  差集合と Monoid 則）。jump の位置集合の合成や受領証の一括リセットはこの演算で書く。オーナーの最終方針
  「最終的には trait に盛り込みたい」（`combine` / `divide` / `map` を共通 trait `FirstClassCollection` へ）は
  積み残しとして設計と日誌に記録し、今回の Bolt には含めない。
- Q4 の境界規則: リードモデル側（read-model-updater / クエリ側）は FCC を使わず自前の平坦な表現へ写す。
  DTO・リードモデル境界への要素列挙は `fold_left`（または理由を記した最後の手段のイテレータ公開）で行う。
  集約をまたぐ判断（skeleton 対象の特定など）はコレクションの操作で書き、配列を外へ取り出さない。
- Q5 = A: `next_decision(&Intent, &NextRequest)` は `Result<NextDecision, CommandError>` を返し、ID 不一致は
  `IntentMismatch` で拒否する（`jump_resolve` / `report_dispatch` と同形）。呼出側（リードモデル更新器の
  `NextAnswerRow::of`）は Err を判断結果ではなく投影の束縛不整合として扱う。BR2.6 の「残る差異」は裁定済みになる。
- P7: 上流 `components.md` 冒頭注記と `contract-summary.md` C3 の B13 追記（いずれも 2026-08-30「ジャーナル全再生」）は
  オーナー裁定 2026-09-05（最新スナップショット + 差分）で上書きされている。U2 設計は 2026-09-05 裁定に従い（BR2.3）、
  上流 2 ファイルの同期は intent 記録の積み残しとして書く（Issue は起票しない）。
- P8: `orchestration/mod.rs` 冒頭の旧説明（全再生、`next_decision` はクエリ側）はドキュメント修正として
  code-generation への引継ぎ項目に載せる。
- P9: functional-spec.md 末尾の 2026-09-05 NOT-READY レビュー節は `functional-spec-review-history-2026-09-05.md` へ
  原文のまま移し、本文冒頭から参照する。今回の独立レビューは是正済み本文（+ 本再走の反映）に対して行う。
- P10: 2026-09-06 に StageGraph / ScopeGrid / Checkboxes 等が `FirstClassCollection` 契約を実装した事実を
  entities.md の referenced_types と BR4.2 の注記に記録する（残す 6 述語は変えない）。
- 実装はすべて U2 の code-generation（再走）で行い、リードモデル側の消費コードの追随（境界列挙の書換え・
  `next_decision` の Err 処理）も同じ Bolt に含めて CI を緑に保つ。

Does this all look correct before I generate the artifact?

- Looks correct
- Request changes

[Answer]: Looks correct
