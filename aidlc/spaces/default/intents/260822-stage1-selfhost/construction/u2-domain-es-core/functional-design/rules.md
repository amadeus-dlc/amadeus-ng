# rules — U2 ドメイン ES コア（`u2-domain-es-core`）

> 2026-09-05 是正。以下の YAML が規則の正本。第2節と functional-spec.md 第7節は同じ BR ID から導出する。
> 上流要求は [requirements.md](../../../inception/requirements-analysis/requirements.md) と
> [unit-of-work-story-map.md](../../../inception/units-generation/unit-of-work-story-map.md)、
> 集約モデルは [entities.md](entities.md)。裁定の優先順位と根拠は [是正記録](../correction-report.md) に示す。
> コード・モデルのパスは特記がなければリポジトリルート基準。
>
> 2026-09-07 再走（Modify、質問票 Q4 / Q4a / Q5・P7〜P10）: BR5.5（コマンド側の配列は FCC、結合・差集合を型ごとの契約に含める）を
> 追加し、BR1.2 / BR2.2 / BR2.6 / BR3.1 / BR4.2 / BR5.4 を同期した。実装は U2 の code-generation 再走で行う。

## 1. 規則（正本）

```yaml
rules:
  - id: BR1.0
    category: constraint
    applies_to: [IntentExecution]
    trigger: "コマンド受理"
    statement: "accepts_commands = Running かつ park 非活性。位置・ゲート・計画を動かす open_gate / approve_gate / reject_gate / revise_stage / skip_stage / jump / recompose に適用する。全コマンド共通の禁止条件ではない"
    logic: "コマンドは入口で Intent ID を照合する。park は再スタンプ可、unpark は park 活性時のみ。switch_autonomy / single / レビュー・昇格記録に Running ガードを追加しない。詳細は functional-spec 第4.2節"
    violation: "IntentMismatch / NotRunning。拒否は状態不変"
    source: "Quint v2.3〜v2.7、後続互換裁定"
  - id: BR1.1
    category: constraint
    applies_to: [IntentExecution, IntentExecutionEvent]
    trigger: "decide / start"
    statement: "通常コマンドは成功時に単一イベントを自身へ適用して返す。誕生ファクトリは集約と誕生イベントを対で返す。拒否ではイベントも状態変更もない"
    logic: "生成するイベントの ID は UUIDv7。commit は通番枯渇を検査し、apply_event(current_seq + 1, occurred_at, event) を呼ぶ。再生で新しい ID を生成しない"
    violation: "複数イベント返し、状態変更後の Err、再生時の採番は違反"
    source: "aggregate-commands 2026-08-29 / 09-02"
  - id: BR1.2
    category: constraint
    applies_to: [IntentExecution]
    trigger: "各遷移後"
    statement: "active = InProgress / AwaitingApproval / Revising。in-flight = active + Pending。Running の cursor は実効 EXECUTE、active は高々1"
    logic: "位置ごとの記録は StageSlots の 1 要素であり、旧 7 列の長さ一致は型で保証される（BR5.5）。active の数は StageSlots.active_count。誕生の縮退形では cursor 0、active 0、Running のまま通常 next が Done"
    violation: "構築時は IntentExecutionError、壊れた履歴の適用は panic"
    source: "engine_loop.qnt cursor_in_scope / at_most_one_active"
  - id: BR1.3
    category: constraint
    applies_to: [IntentExecution]
    trigger: "誕生 / approve_gate"
    statement: "initialization は誕生時に Completed、approved は false。実ステージの Completed は approved を伴う。initialization の逐次完了コマンドは存在しない"
    logic: "最初の実効対象の実ステージを InProgress にする。approve_gate は非 initialization かつ InProgress / AwaitingApproval、昇格受領証、レビュー終端受領証の順に検査"
    violation: "InvalidTarget / CheckboxPrecondition / PracticesReceiptMissing / ReviewReceiptMissing"
    source: "Quint v2.2 / v2.5 / v2.6、初期化完了の後続裁定"
  - id: BR1.4
    category: constraint
    applies_to: [IntentExecution]
    trigger: "open / approve / reject / revise"
    statement: "open は InProgress → AwaitingApproval、approve は InProgress または AwaitingApproval → Completed、reject は同じ前提から Revising、revise は Revising → AwaitingApproval"
    logic: "reject の apply が revision_count を飽和加算で +1 し、そのステージのレビュー試行・昇格受領証を消す。前進時は新カーソルの試行を空にする"
    violation: "CheckboxPrecondition / InvalidTarget"
    source: "Quint gate_lifecycle_preconditions / review_attempt_floor / practices_receipt_floor"
  - id: BR1.5
    category: constraint
    applies_to: [IntentExecution]
    trigger: "skip_stage / 前進"
    statement: "skip は InProgress / Revising、かつ conditional または実効 SKIP の場合だけ。conditional は引数の Intent から得る"
    logic: "apply が次の実効 EXECUTE 位置を導出し InProgress にする。後続がなければ Completed。next_stage はイベントへ載せない"
    violation: "NotSkippable / CheckboxPrecondition"
    source: "Quint actReportSkipped / actReportForward"
  - id: BR1.6
    category: constraint
    applies_to: [IntentExecution]
    trigger: "jump_resolve / jump"
    statement: "forward / backward は非 initialization かつ in-scope の target。redo は cursor が非 initialization。forward の介在 cursor < u < target は in-scope かつ in-flight のみ Skipped。現 cursor は in-flight かつ非 Pending のみ Skipped"
    logic: "Jumped は target のみを運び、apply が方向と差分を導出。backward は target+1 以降の in-scope 非 Pending を Pending に戻し、target 以降の承認を消す。target は InProgress。redo は cursor の承認を消して InProgress。全方向の jump で全ステージのレビュー試行・昇格受領証を消す"
    violation: "InvalidTarget / NotRunning。実効 SKIP の介在 Pending は変更しない"
    source: "Quint v2.1、upstream 観測互換裁定"
  - id: BR1.7
    category: constraint
    applies_to: [IntentExecution]
    trigger: "park / unpark"
    statement: "park は ID 照合後、Autonomous なら拒否、次に非 Running を拒否。既に park 中でも同じ位置を再スタンプする。unpark は ID 一致かつ park 活性時のみ"
    logic: "Parked.stage = cursor の slug。Unparked は共通 ID のみで、apply は保存位置のままマーカーを消す"
    violation: "RefusedUnderAutonomy / NotRunning。再 park を NotRunning にしない"
    source: "Quint v2.3、再 park 裁定"
  - id: BR1.8
    category: constraint
    applies_to: [IntentExecution]
    trigger: "recompose / switch_autonomy"
    statement: "recompose は Running・非 park・Gated、非空の反転対象全件が cursor より後かつ Pending。switch_autonomy は park 中・Completed でも許可"
    logic: "recompose は重複を集合化し全件検査後に skipped / added を1イベントへ載せる。自律切替は Intent ID 照合後、Autonomous への設定かつ guard 有効なら HumanTurns と last_gate_resolution_at で人間の操作を確認する"
    violation: "InvalidTarget / CheckboxPrecondition / RefusedUnderAutonomy / HumanPresenceRequired"
    source: "Quint v2.7、I11 はモデル外"
  - id: BR1.9
    category: constraint
    applies_to: [IntentExecution]
    trigger: "stale_report"
    statement: "stale_report は書込の受理ガードで Result<(), CommandError>。accepts_commands かつ対象が cursor より前の Completed なら Ok(())"
    logic: "呼出側はイベントを起こさず冪等な完了応答を組み立てる。次判断を返す API ではない"
    violation: "NotRunning / NotStale"
    source: "Quint stale_rereport_frame / stale_rereport_yields_done"
  - id: BR2.1
    category: constraint
    applies_to: [IntentExecutionEvent, IntentExecution]
    trigger: "イベント適用"
    statement: "イベント自身の UUIDv7 id と aggregate_id を分離する。seq_nr と occurred_at はジャーナル封筒から apply の引数へ渡す。schema_version はドメイン属性ではない"
    logic: "Repository がイベントの aggregate_id と対象 ID を照合したうえで apply_event(seq_nr, occurred_at, event) に渡す。apply は通番 current+1 を要求。内部の検査違反は panic、公開の Result は返さない"
    violation: "アダプタの復号・封筒不整合は RepositoryError::Corrupt。型変換後の壊れた歴史は回復しない"
    source: "aggregate-commands / domain-persistence-neutrality"
  - id: BR2.2
    category: constraint
    applies_to: [Intent, StageEntry, IntentExecutionEvent]
    trigger: "Intent::create / IntentExecution::start"
    statement: "Intent が定義参照・依頼・全ステージを文書順に解決する。grid の None は SKIP。conditional と display は同じ順序の graph.nodes から取る。initialization は EXECUTE かつ非 conditional"
    logic: "create は StageEntries.check_plan の計画検査後に Intent と Created を返す。start は実行ID・&Intent・時刻を受け、Started に intent_id と StageEntries を記録する。From<(Started, 時刻)> が StageEntries から StageSlots を導出して誕生状態を作り、定義を要しない"
    violation: "IntentError / PlanError。実行 start 自体は Result ではない"
    source: "後続 Intent 分離裁定、aggregate-commands の自己完結 genesis"
  - id: BR2.3
    category: validation
    applies_to: [IntentExecution]
    trigger: "replay"
    statement: "同じ集約の最新スナップショットを基底に、event.seq_nr > snapshot.seq_nr の差分を昇順で適用する。空差分は基底と同じ状態"
    logic: "replay(snapshot, events) は Self を返す。通常コマンドと replay は同じ apply 経路。基底より前の履歴を再適用しない。比較時は保存後の version と新規イベント ID を区別し、同じイベント列で状態を比較する"
    violation: "全履歴を常時再生する方式、反映済みイベントの重複適用は違反"
    source: "2026-09-05 オーナーの差分再生訂正、NFR3"
  - id: BR2.4
    category: policy
    applies_to: [IntentExecutionEvent]
    trigger: "イベント語彙"
    statement: "現行16変種と各ペイロードは entities.md の payloads が正本。StageCompleted は廃止。次位置・jump 差分・差し戻し回数は apply、phase_boundary は RMU の導出"
    logic: "保存 DTO と RMU 専用 DTO の対応は横断適合テストで固定。1イベントから複数監査行への描画は U4。共有 C5 の旧世代記述は後続裁定と同期して扱う"
    violation: "C5 の旧12変種・旧封筒を再導入しない"
    source: "後続イベント痩身 / イベントID / 初期化 / 受領証裁定"
  - id: BR2.5
    category: validation
    applies_to: [IntentExecution]
    trigger: "ITF準拠"
    statement: "モデルは engine_loop.qnt v2.7。射影表は本ファイル第3節。単なる旧状態9列の対応ではなく、stance・レビュー会計・昇格受領証・指令の観測を含む"
    logic: "assert_projection / assert_signal と射影表を照合。human presence の入力・時刻・イベントID・ストア版はモデル外であり、モデル成功をその検証と呼ばない"
    violation: "射影差異は不具合として実測し、古いモデルへ戻さない"
    source: "formal/orchestration/engine_loop.qnt v2.7、engine_loop_conformance.rs"
  - id: BR2.6
    category: constraint
    applies_to: [Intent, IntentExecution, WorkflowDefinition]
    trigger: "集約参照"
    statement: "Intent → WorkflowDefinition は definition_id、IntentExecution → Intent は intent_id の ID 参照。定義の内容版は Intent の来歴であり、実行に定義オブジェクトや静的計画を埋め込まない"
    logic: "コマンド・書込前ガード（jump_resolve / stale_report の対象解決）・next_decision はすべて &Intent を照合し IntentMismatch で拒否する。next_decision は Result<NextDecision, CommandError> を返す（Q5 = A、2026-09-07。現行コードは NextDecision を直接返しており、code-generation で同期する）。呼出側（リードモデル更新器）は Err を判断結果ではなく投影の束縛不整合として扱う"
    violation: "next_decision が DefinitionMismatch を返すと記載しない。ID 不一致で判断結果を返すのは違反"
    source: "aggregate-references、Intent / IntentExecution 分離裁定、質問票 Q5"
  - id: BR3.1
    category: calculation
    applies_to: [IntentExecution, NextRequest, NextDecision]
    trigger: "next_decision(&Intent, &NextRequest) -> Result<NextDecision, CommandError>"
    statement: "ID 照合の後、優先順は park 活性かつ非再入 → resume → free_text → Completed → cursor の in-flight / 実効 SKIP → 次の in-scope → Done。&Intent は skeleton ゲートの静的計画判断（StageEntries.first_of(Construction, EXECUTE)）に使用する"
    logic: "RunStage は GateDecision を返す。initialization は Ungated、skeleton 対象かつ stance 未記録なら Unresolved、他は Gated。SKIP不整合は InProgress / Revising なら RecoverSkipInconsistency、それ以外なら InconsistentSkip。次の in-scope は StageSlots.next_effective_execute_after(cursor)"
    violation: "IntentMismatch（BR2.6）。未使用予約引数という説明を置かない"
    source: "next ラダー、cqrs-boundaries、Quint v2.4"
  - id: BR3.2
    category: policy
    applies_to: [NextRequest]
    trigger: "ルーティング / 投影"
    statement: "フラグによる状態非依存の分類・birth・single の要求処理は U6。状態依存の判断は集約が所有し、RMU が結果を投影する。クエリ側は DAO から View を取得する"
    logic: "コマンド側は最新集約を Repository から読む。クエリ側へドメイン判断を移動しない"
    violation: "CQRS 境界違反"
    source: "FR3.3、cqrs-boundaries 2026-09-02以降"
  - id: BR3.3
    category: calculation
    applies_to: [IntentExecution]
    trigger: "jump_resolve"
    statement: "jump_resolve(&Intent, target) は BR1.6 の受理検査と方向導出を行う書込なしクエリ。jump は同じガードを経て Jumped を返す"
    logic: "resolve は Result<JumpDirection, CommandError>。jump は事実 target だけを記録"
    violation: "IntentMismatch / NotRunning / InvalidTarget"
    source: "next ラダー分岐7、10号§2.3"
  - id: BR4.1
    category: policy
    applies_to: [PlanAction]
    trigger: "所有検査"
    statement: "PlanAction は core-command-domain::workflow_definition が所有。orchestration に定義・再輸出を置かない。旧10ファイル一斉移動一覧は B3 当時の実績であり現行影響範囲一覧ではない"
    logic: "検査前に modules/core/command/domain/src/orchestration の存在を確認し、不在は失敗とする。rg で enum PlanAction と pub use の PlanAction 再輸出を検出し0件を要求。正当な use は除く。変更時は現配置の全参照を改めて検索する"
    violation: "検索対象不在や検索エラーを0件成功と扱わない"
    source: "FR8.3、module-visibility"
  - id: BR4.2
    category: policy
    applies_to: [IntentExecution, WorkflowDefinition]
    trigger: "実効計画"
    statement: "畳み込みの所有者は IntentExecution.effective_plan(stage) = overlay。静的計画の所有者は Intent。WorkflowDefinition はグリッド照会と畳み込みを含まない述語を保持"
    logic: "WorkflowDefinition の削除対象は effective_plan_action / next_in_scope_stage。残す述語は is_valid_scope / valid_scopes / scope_metadata / subgraph_for_scope / stages_in_scope / first_in_scope_stage_of_phase。2026-09-06 に StageGraph / ScopeGrid が FirstClassCollection 契約（at / filter / fold_left / map）を実装した事実は現行事実として記録するのみで、述語の増減はしない（P10）"
    violation: "既存述語の過剰削除、定義への overlay 再導入は違反"
    source: "FR8.4、ADR-002 / ADR-005"
  - id: BR5.1
    category: constraint
    applies_to: [StageIndex]
    trigger: "公開位置"
    statement: "集約の公開位置は StageIndex。stage_index(usize) は範囲内だけ Some。コマンド入口でも他実行の位置を検査する"
    logic: "検査付きの完全コンストラクタ new は DTO 境界から cursor / parked_at の整数を受けて検査する。公開APIに整数が一切無いとは宣言しない"
    violation: "不正位置の無検査利用は違反"
    source: "Q2の型保証目的、現行 StageIndex"
  - id: BR5.2
    category: policy
    applies_to: [IntentExecution, IntentExecutionEvent]
    trigger: "永続化境界"
    statement: "domain に serde / ESA 直接依存・ストア trait・復号用 memento 双子型を置かない。DTO と封筒はアダプタが所有する"
    logic: "DTO の検査付き変換は IntentExecution::new を通して基底を得る。その後 replay(base, delta) を行う。new の Err と replay / apply の panic は境界が異なる。過去の『genesis以外からの状態復元禁止』を最新 snapshot 禁止へ読み替えない"
    violation: "復号失敗は Corrupt。ドメイン型へ永続化知識を戻すのは違反"
    source: "domain-persistence-neutrality、aggregate-commands 2026-09-05訂正"
  - id: BR5.3
    category: constraint
    applies_to: [IntentExecution]
    trigger: "store / find_by_id"
    statement: "version はストア採番の不透明な usize。未保存は UNPERSISTED_VERSION=0。Repository が読取版を with_version で渡し次回 store の期待版として使う。seq_nr とは別"
    logic: "apply は version を増やさない。store の戻り値は () なので、続けて書く場合は再読込した版を次回書込へ引き継ぐ。古い Aggregate::set_version 直実装を使わない"
    violation: "seq_nr から version を算出、あるいはストア trait を domain へ戻すのは違反"
    source: "2026-08-30読取版保持裁定、C3現行契約"
  - id: BR5.4
    category: policy
    applies_to: [IntentExecution, IntentExecutionEvent, NextDecision]
    trigger: "実装規律"
    statement: "private フィールド、所有ファサード、PartialEq / Eq、手実装エラー、decide / apply 分離、コマンド側配列の FCC 化（BR5.5）を守る。イベント UUIDv7 生成と壊れた歴史の panic は明示裁定の射程で許可する"
    logic: "coding-rules の最新裁定を優先し、古いコード doc や過去回答をその代用にしない"
    violation: "cargo lint / 型 / レビューで検査"
    source: "coding-rules README と各規則"
  - id: BR5.5
    category: policy
    applies_to: [Intent, IntentExecution, IntentExecutionEvent, StageEntries, StageSlots, StageIndexSet, ArtifactPaths, StageSlugSet]
    trigger: "コマンド側ドメインモデルの配列部分"
    statement: "コマンド側ドメインモデルの配列はすべてファーストクラスコレクション（FCC）にする。対象: Intent.stages / Created.stages / Started.stages → StageEntries、IntentExecution の位置ごとの旧 7 並列列 → StageSlots、jump・recompose の位置集合 → StageIndexSet、GateOpened.artifacts → ArtifactPaths、Recomposed.skipped / added → StageSlugSet、PracticesAffirmed と PracticesPromotion の sections / mandated / forbidden → PromotedSections / RuleLines、ReviewAttempt の pending / closed と ReportDecision::Commit.steps も内部列として FCC 化する。リードモデル側（read-model-updater / クエリ側）は FCC を使わず自前の平坦な表現へ写す"
    logic: "各 FCC は不変条件（非空・一意・順序）と at / filter / fold_left / map に加え combine / divide を型ごとの契約として持つ。文書順の列（StageEntries / StageSlots）の combine は連結で slug 衝突は Result で拒否、divide は他方に含まれる slug を除き空可の型へ戻る。集合（StageIndexSet / StageSlugSet）の combine は和集合・divide は差集合で空集合を単位元とする Monoid 則を試験する。順序付き列（ArtifactPaths / RuleLines）の combine は連結で重複を消さない。業務判断（skeleton 対象、次の実効 EXECUTE、jump の Skipped / Pending 戻し対象、受領証の一括リセット）はコレクションの操作と集合演算で書き、配列やイテレータを集約の外へ取り出さない。DTO・リードモデル境界への要素列挙は fold_left を優先し、イテレータ公開は理由を記した最後の手段"
    violation: "生の Vec / スライスの公開、集約外での配列走査による判断、リードモデル側での FCC 使用は違反。使われない共通メソッド群の機械的追加も違反"
    source: "coding-rules/first-class-collections.md（オーナー裁定 2026-09-06）、質問票 Q4 / Q4a（オーナー回答 2026-09-07）。combine / divide / map の共通 trait への一律化はオーナーの最終方針として積み残し（今回の Bolt に含めない）"
```

## 2. 規則の要約

| ID | 要約 |
|---|---|
| BR1.0 | accepts_commands = Running かつ park 非活性 |
| BR1.1 | 通常コマンドは成功時に単一イベントを自身へ適用して返す |
| BR1.2 | active = InProgress / AwaitingApproval / Revising |
| BR1.3 | initialization は誕生時に Completed、approved は false |
| BR1.4 | open は InProgress → AwaitingApproval、approve は InProgress または AwaitingApproval → Completed、reject は同じ前提から Revising、revise は Revising → AwaitingApproval |
| BR1.5 | skip は InProgress / Revising、かつ conditional または実効 SKIP の場合だけ |
| BR1.6 | forward / backward は非 initialization かつ in-scope の target |
| BR1.7 | park は ID 照合後、Autonomous なら拒否、次に非 Running を拒否 |
| BR1.8 | recompose は Running・非 park・Gated、非空の反転対象全件が cursor より後かつ Pending |
| BR1.9 | stale_report は書込の受理ガードで Result<(), CommandError> |
| BR2.1 | イベント自身の UUIDv7 id と aggregate_id を分離する |
| BR2.2 | Intent が定義参照・依頼・全ステージを文書順に解決する |
| BR2.3 | 同じ集約の最新スナップショットを基底に、event.seq_nr > snapshot.seq_nr の差分を昇順で適用する |
| BR2.4 | 現行16変種と各ペイロードは entities.md の payloads が正本 |
| BR2.5 | モデルは engine_loop.qnt v2.7 |
| BR2.6 | Intent → WorkflowDefinition は definition_id、IntentExecution → Intent は intent_id の ID 参照。next_decision も IntentMismatch を Err で返す |
| BR3.1 | ID 照合の後、優先順は park 活性かつ非再入 → resume → free_text → Completed → cursor の in-flight / 実効 SKIP → 次の in-scope → Done（戻り値は Result） |
| BR3.2 | フラグによる状態非依存の分類・birth・single の要求処理は U6 |
| BR3.3 | jump_resolve(&Intent, target) は BR1.6 の受理検査と方向導出を行う書込なしクエリ |
| BR4.1 | PlanAction は core-command-domain::workflow_definition が所有 |
| BR4.2 | 畳み込みの所有者は IntentExecution.effective_plan(stage) = overlay |
| BR5.1 | 集約の公開位置は StageIndex |
| BR5.2 | domain に serde / ESA 直接依存・ストア trait・復号用 memento 双子型を置かない |
| BR5.3 | version はストア採番の不透明な usize |
| BR5.4 | private フィールド、所有ファサード、PartialEq / Eq、手実装エラー、decide / apply 分離、FCC 化を守る |
| BR5.5 | コマンド側ドメインモデルの配列はすべて FCC（at / filter / fold_left / map / combine / divide + 業務操作）。リードモデル側は FCC を使わない |

## 3. Quint v2.7 との射影

`formal/orchestration/engine_loop.qnt` と
`modules/core/command/domain/tests/engine_loop_conformance.rs` の
`assert_projection` / `assert_signal` を対応させる。

| モデル | 集約・テストの対応 |
|---|---|
| plan / conditional | 合成 Intent の静的 StageEntries の要素（StageEntry）。実行の保持状態には複製しない |
| checkbox / overlay / approved | 各位置の StageSlot（checkbox / plan_action / approved）を StageSlots.at で読む |
| cursor / parkedAt | cursor、parked_at（None は -1） |
| Running / WorkflowParked / WorkflowCompleted | Running かつ非 park / park 活性 / Completed |
| autonomous | autonomy().is_autonomous()。actSetAutonomy は現在値の反転を switch_autonomy に渡す |
| stage 0 | initialization 1段の合成。実グラフの initialization 全体は誕生時完了 |
| stage 1以降 / skeletonGateStage | 合成計画では Construction。静的計画で最初の非 init EXECUTE 位置を skeleton 対象にする |
| stanceRecorded | skeleton_stance().is_some()。stance 値の分類そのものはモデル外 |
| reviewed / advisory | ReviewPolicy の受領証要否 / 実効 advisory クラス |
| reqCount / pending / terminal | ReviewAttempt の request_count / pending / has_terminal(policy) |
| practicesStage / affirmed | practices_stage() / その位置の practices_affirmed。他位置は false |
| actSingleRun | 非 init 対象の record_single_stage_run。本流の状態は不変（通番・時刻・イベントはモデル外） |
| actRequestReview / actRetryReview / actRecordVerdict | request_review / retry_pending / record_review_verdict。retry は会計を増やさない |
| actPromotePractices | affirm_practices。内容の文書・規則行はモデル外 |
| prev* / lastAction | モデルの遷移前観測・フレーム条件用。集約の永続状態に追加しない |
| lastDirective | NextDecision から EngineSignal へ。RunStage → DRunStage、Done → DDone、Parked → DParked、不整合2種 → DError |
| resume / free_text 分岐 | UnparkThenResume / ResumeMenu / NewWorkRouting は EngineSignal で Done。ITF の通常 next の射程外 |
| human presence (I11) | モデル外。HumanTurns と last_gate_resolution_at によるガードを境界・単体・end-to-end で検証 |
| seq_nr / version / occurred_at / イベント UUID | モデル外。Repository・再生・識別子のテストで検証 |

モデルは実装全体の代用ではない。レビュー予算・終端判定は ReviewPolicy の抽象に対応し、完全な文字列や監査投影バイトはモデル外に残る。
