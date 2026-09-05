# functional-spec — U2 ドメイン ES コア（`u2-domain-es-core`）

> 現行本文は2026-09-05の是正を反映する。構造の正本は [entities.md](entities.md) の YAML、
> 規則の正本は [rules.md](rules.md) の YAML。以下はその API・手順・導出ビューである。
> 末尾の Review は是正前のレビュー記録をそのまま保持する。是正の範囲・検証・残件は
> [correction-report.md](../correction-report.md) に記録し、正式承認とは区別する。
> 根拠となる共有成果物は [unit-of-work.md](../../../inception/units-generation/unit-of-work.md)、
> [requirements.md](../../../inception/requirements-analysis/requirements.md)、
> [decisions.md](../../../inception/domain-design/decisions.md)、
> [components.md](../../../inception/domain-design/components.md)、
> [contract-summary.md](../../../inception/contract-design/contract-summary.md)。
> 後続裁定との世代差は是正記録に示す。以後のコードパスはリポジトリルート基準。

## 1. 概要

`core-command-domain` の `Intent` は依頼・定義参照・静的計画を、
`IntentExecution` は進捗・overlay・ゲート・受領証と状態に基づく判断を所有する。
集約間は ID で参照し、必要な判断材料は引数で渡す。
通常コマンドは単一イベントを返し、同じ `apply_event` が通常実行とリプレイを担う。
ドメインに serde・ストア trait・復号用 memento 型を置かない。時刻は呼出側から渡す。
イベントの固有 UUIDv7 をコマンド内で生成することは裁定済みの例外である。

`PlanAction` は `workflow_definition` が所有し（FR8.3）、
実効計画の畳み込みは `IntentExecution` に閉じる（FR8.4）。
永続化と復号は U3、投影は U4、呼出順と入力の分類は U5 / U6 が担当する。

## 2. インターフェイス（現行 API）

`modules/core/command/domain/src/orchestration/intent.rs` の生成は次の形である。

```text
Intent::create(id: IntentId, definition: &WorkflowDefinition,
  start_request: StartRequest, scan: WorkspaceScan, occurred_at: DateTime<Utc>)
  -> Result<(Intent, IntentEvent), IntentError>
```

`intent_execution.rs` の公開境界は次のとおり。以下の `t` は `DateTime<Utc>`、
特記しない通常コマンドの戻り値は `Result<IntentExecutionEvent, CommandError>` を表す。

| API | 戻り値・意味 |
|---|---|
| start(id: IntentExecutionId, intent: &Intent, t) | (IntentExecution, IntentExecutionEvent)。計画検査済みなので Result ではない |
| open_gate(&Intent, artifacts, t) | GateOpened |
| approve_gate(&Intent, Option<&ReviewPolicy>, user_input, t) | GateApproved。phase_boundary は引数・ペイロードに持たない |
| reject_gate(&Intent, feedback, t) / revise_stage(&Intent, t) | GateRejected / StageRevised |
| skip_stage(&Intent, reason, t) | StageSkipped |
| jump(&Intent, StageIndex, t) / jump_resolve(&Intent, StageIndex) | Jumped / Result<JumpDirection, CommandError> |
| park(&Intent, t) / unpark(&Intent, t) / recompose(&Intent, &[StageIndex], t) | Parked / Unparked / Recomposed |
| switch_autonomy(&Intent, mode, &HumanTurns, guard: bool, t) | AutonomyModeSet |
| record_single_stage_run(&Intent, StageIndex, t) | SingleStageRunCommitted |
| record_single_stage_run_named(&Intent, &StageSlug, t) | Result<IntentExecutionEvent, NamedStageRunError>。名指しの対象を自己の添字帳で解決して隔離実行を記録 |
| record_skeleton_stance(&Intent, stance, t) | Result<IntentExecutionEvent, SkeletonStanceRefusal>。拒否時はstage・scope・CommandErrorを文脈として返す |
| apply_report(&Intent, &ReportRequest, &[TransitionStep], Option<&ReviewPolicy>, t) | Result<IntentExecutionEvent, ReportCommitError>。報告適用の段分岐と入力の正規化を所有し、成功時は単一イベント |
| report_dispatch(&Intent, &ReportRequest) | ReportDecisionのCommit / NoOpに、判断対象のscopeも含めて返す |
| request_review(&Intent, &StageSlug, policy, reviewer, iteration, retry_pending, t) | ReviewRequested |
| record_review_verdict(&Intent, &StageSlug, policy, reviewer, iteration, verdict, t) | ReviewCompleted |
| affirm_practices(&Intent, &PracticesPromotion, affirming_user, t) | PracticesAffirmed |
| stale_report(StageIndex) | Result<(), CommandError>。書込を伴わない冪等完了の受理ガード |
| next_decision(&Intent, &NextRequest) | NextDecision。&Intent を skeleton ゲートの判断に使用 |
| replay(snapshot: IntentExecution, events: IntoIterator<(usize, DateTime<Utc>, IntentExecutionEvent)>) | IntentExecution。最新 snapshot と後続差分を畳み込む |
| apply_event(seq_nr: usize, t, &IntentExecutionEvent) | ()。壊れた履歴は panic |
| new(全状態材料) | Result<IntentExecution, IntentExecutionError>。DTO 境界の検査付き完全コンストラクタ |
| with_version(usize) / version() | 読取済みの不透明な版の受け渡し。本家 trait の set_version ではない |
| stage_index(usize) | Option<StageIndex>。公開の位置解決 |

`complete_stage` / `StageCompleted` / `set_autonomy` / `state` / `from_state` /
`WorkflowExecutionState` は現行 API ではない。
`next_decision` が `DefinitionMismatch` を返すという旧宣言も失効している。
コマンドは `IntentMismatch` を返すが、クエリの参照照合には残る差異がある（BR2.6、是正記録）。

2026-09-05のTell, Don't Ask是正により、`ReportRequest::for_retry_at(StageSlug)` が他の観測を保持して再試行対象を固定する。
`Intent::resolve_review_policy(&WorkflowDefinition, &StageSlug)` が定義IDを照合し、依頼のスコープ・レビュー指定・定義の規則から方針を判断する。
ユースケースは入力getterでこれらを組み立て直さず、操作を依頼する。関連取得はRepositoryの `find_for_execution` / `find_for_intent` を使い、参照IDを読むのはadapter内に限定する。

## 3. ワークフロー

### W1 — Intent の作成と実行の誕生

1. `Intent::create` が scope を検査し、`stages_in_scope` の全ステージを文書順に解決する。
   `None → SKIP`、conditional は同じ順序の `graph().nodes()[i].execution()`、
   display はノードの番号・表題・担当から取得する。
2. 計画の非空・slug 一意・先頭 EXECUTE・initialization の EXECUTE かつ非 conditional、
   表示材料の単一行制約を検査する。不正は `IntentError`。定義 ID と revision は Intent の来歴となる。
3. `IntentExecution::start` は別型の実行 ID、`&Intent`、時刻から `Started` を作る。
   `Started` は自分のイベント ID、aggregate_id、intent_id、StageEntry 列を運ぶ。
   `From<(Started, occurred_at)>` が seq_nr=1、version=0 の誕生状態を導出する。
4. initialization は全て Completed、承認は false。最初の実効対象の実ステージだけ InProgress。
   実ステージがなければ Running、cursor=0、active=0 の縮退形になり、通常 next は Done を返す。
   初期化完了用の追加コマンド・追加ドメインイベントは発行しない。
5. 各集約と誕生イベントをそれぞれの Repository に保存する。監査面の初期化完了行は U4 の誕生投影が描く。

### W2 — コマンド実行

1. 対象 Intent の照合とコマンド別ガードを通す。承認では状態前提の後、昇格受領証、レビュー終端受領証を順に検査する。
2. 自前の UUIDv7 イベント ID と aggregate_id、必要な事実だけでイベントを作る。
   artifacts / user_input / feedback 等は引数の値を使う。
3. `apply_event(current_seq + 1, occurred_at, event)` が次位置・差分・回数を導出し状態を進める。
   受領証の試行境界は前進先、差し戻し先、各 jump では全ステージで更新する。
4. 単一イベントを返す。Repository はイベントと適用後集約、および読み取った version を使って保存する。
   version は apply で増やさない。store は () を返すため、続けて書く場合は再読込した版を使う。

事後条件は「同じ旧状態に、返された同じイベントと通番・時刻を適用した結果と同値」。
イベント ID が異なる別コマンド実行同士のイベント同値を前提にはしない。

### W3 — 最新スナップショットと差分から再構成

1. Repository が同じ集約 ID の最新 snapshot を取得する。アダプタの DTO 復号と検査付き変換で
   `IntentExecution::new` を通し、保存済み状態を基底とする。
2. snapshot の通番より大きいイベントだけを取得する。ストアの inclusive な取得 API には
   `snapshot.seq_nr + 1` を渡す。通番順で封筒・集約 ID・復号結果を検査する。
3. `replay(base, delta)` を呼ぶ。差分が空なら基底の状態が結果。
   `apply_event` は通番・時刻・ドメインイベントを受け、イベントを新規生成しない。
4. ストアが返した読取版を `with_version` で保持する。

genesis の `From<(Started, occurred_at)>` は誕生状態の導出として存在するが、
通常の `find_by_id` を毎回ジャーナル先頭からの全再生へ戻す根拠ではない。
`replay` / `apply_event` は Result を返さない。復号・封筒の不正を返すアダプタ境界と、
型変換後の壊れた歴史でクラッシュするドメイン境界を分ける。

### W4 — next_decision

0. 呼出境界は対象実行に対応する Intent を渡す。現行メソッドに ID 不一致の Err 経路は無いため、
   この前提を満たすことと集約内の照合を同一視しない（BR2.6）。
1. park 活性かつ非再入 → resume 指定なら UnparkThenResume、それ以外 Parked。
2. resume → ResumeMenu。free_text → NewWorkRouting。Completed → Done。
3. cursor が in-flight で実効 SKIP → InProgress / Revising なら RecoverSkipInconsistency、
   Pending / AwaitingApproval なら InconsistentSkip。
4. cursor が in-flight → RunStage。それ以外は次の in-scope があれば RunStage、なければ Done。
5. RunStage の gate は GateDecision。initialization は Ungated、skeleton 対象で stance 未記録なら
   Unresolved、それ以外は Gated。static な skeleton 対象の特定に Intent を使う。

判断は集約が所有する。RMU が同じ判断を投影し、クエリ側ユースケースは DAO から答えを取得する。
`stale_report` は別の書込前ガードで、成功時の冪等な完了応答は呼出側が組み立て、イベントを作らない。

### W5 — ジャンプ

`jump_resolve` が対象と方向を検査し、`jump` は `Jumped{target}` を記録する。
apply が方向・スキップ・巻き戻しを導出する。

- forward: 介在位置は **in-scope かつ in-flight** のみ Skipped。現 cursor は in-flight かつ非 Pending のみ Skipped。
- backward: **target+1 以降**の in-scope 非 Pending を Pending に戻す。target 以降の承認を消す。
- redo: cursor の承認を消す。
- いずれも target は InProgress、cursor=target。全ステージのレビュー試行・昇格受領証を消す。target 自身を Pending と記述しない。

### W6 — 計画・park・権限・受領証

recompose は後続 Pending の反転対象を全件検査し、一括で skipped / added を記録する。
静的計画は変えない。再 park は同じ位置の再スタンプとして受理する。
自律切替、隔離実行、レビュー会計、実践昇格は本流の Running / park 状態を共通ガードにしない。
それぞれの対象・権限・順序ガードは第4.2節のとおり。

### W7 — PlanAction の所有と畳み込み

現行の所有元は `modules/core/command/domain/src/workflow_definition/plan_action.rs` とそのファサード。
`orchestration` は利用側であり、定義・再輸出を持たない。
旧10ファイル移動一覧は B3 当時の実績で、現在の全呼出側の監査結果ではない。

検査は現行ディレクトリの存在を先に確認し、不在・検索エラーを成功扱いしない。
`rg -n 'enum PlanAction|pub use .*PlanAction' modules/core/command/domain/src/orchestration`
の一致0件を確認する。改変時には複数行の再輸出を含めファサードと全参照も確認する。
`WorkflowDefinition` の畳み込み2メソッドを再導入せず、BR4.2 の既存6述語を保持する。

## 4. 状態遷移

### 4.1 ステージの checkbox

| 対象・現在 | イベント | 前提 | 適用後 |
|---|---|---|---|
| initialization | Started | 誕生 | Completed、approved=false |
| 最初の実効対象実ステージ | Started | 存在する場合 | InProgress |
| 後続実ステージ | GateApproved / StageSkipped | 次の in-scope | InProgress、試行を空にする |
| InProgress | GateOpened | 非 initialization | AwaitingApproval |
| InProgress / AwaitingApproval | GateApproved | 昇格・レビュー受領証を含む承認条件成立 | Completed、approved=true |
| InProgress / AwaitingApproval | GateRejected | 非 initialization | Revising、回数を飽和加算で+1、試行・昇格受領証を消す |
| Revising | StageRevised | BR1.0 | AwaitingApproval |
| InProgress / Revising | StageSkipped | conditional または実効 SKIP | Skipped |
| forward の介在位置 | Jumped | in-scope かつ in-flight | Skipped |
| forward の旧 cursor | Jumped | in-flight かつ非 Pending | Skipped |
| backward の target+1 以降 | Jumped | in-scope かつ非 Pending | Pending |
| jump の target | Jumped | forward / backward / redo の対象検査成立 | InProgress |
| 通過済み Completed | stale_report | BR1.9 | Completed のまま、イベントなし |

### 4.2 コマンド別ガード

すべての `&Intent` を受けるコマンドは、まず ID 一致を検査する。
共通条件 `accepts_commands` は位置・ゲート・計画を変更する行だけに適用する。

| コマンド | 状態条件 | 固有条件・変更 |
|---|---|---|
| open / approve / reject / revise / skip / jump | Running かつ非 park | checkbox・対象・受領証等は BR1.3〜BR1.6 |
| recompose | Running かつ非 park | Gated、後続 Pending の非空集合 |
| park | Running（park 活性も可） | Autonomous を先に拒否。位置不変で再スタンプ可 |
| unpark | park 活性 | マーカー除去、位置復元 |
| switch_autonomy | 状態条件なし | Autonomous への設定で guard 有効なら human presence 必須 |
| record_single_stage_run | 状態条件なし | 対象は計画内の非 initialization。本流の状態不変 |
| record_skeleton_stance | 状態条件なし | cursor が静的計画の skeleton 対象。再記録可 |
| request_review | 状態条件なし | 対象・レビュアー宣言・一致、通常は予算と順序、retry は判定待ち必須 |
| record_review_verdict | 状態条件なし | 対象・レビュアー宣言・一致、対応する判定待ち必須 |
| affirm_practices | 状態条件なし | 計画内に practices-discovery が必要。再昇格可 |
| stale_report（クエリ） | Running かつ非 park | cursor より前の Completed のみ。状態不変 |

後続が無い承認・skip は status を Completed にする。
Completed は本流の前進の終端であり、権限変更や受領証等を含む全コマンドの禁止状態ではない。
隔離実行で「不変」とするのは本流の位置・計画・checkbox・承認等であり、イベントの記録に伴う通番・時刻は進む。

## 5. エラーと失敗境界

| 境界 | 失敗 | 扱い |
|---|---|---|
| Intent の作成 | IntentError、計画・表示属性の不正 | 集約誕生前に拒否。None → SKIP 自体は不正ではないが initialization は EXECUTE が必要 |
| コマンド | IntentMismatch / NotRunning / CheckboxPrecondition / InvalidTarget / NotSkippable / RefusedUnderAutonomy 等 | 状態不変の Err |
| 承認・会計・人間の操作 | ReviewReceiptMissing / PracticesReceiptMissing / NoPendingReview / ReviewBudgetExceeded / HumanPresenceRequired 等 | 状態不変の Err。正確な材料は CommandError が所有 |
| stale_report | NotRunning / NotStale | 書込なしで拒否 |
| DTO → 集約基底 | IntentExecutionError、DTO 復号失敗 | RepositoryError::Corrupt 等のアダプタ側エラーへ写す |
| 封筒・通番の境界検査 | 集約 ID・通番・manifest 等の不整合 | Repository の Corrupt。検査対象は最新 snapshot と後続差分 |
| replay / apply_event | 型変換後の壊れた歴史 | Result を返さず panic。回復用 ApplyError を公開 API に戻さない |
| next_decision | ID 不一致 Err なし | BR2.6 の残件。DefinitionMismatch を返すと宣言しない |

## 6. 導出ビュー — 所有関係（entities.md が正本）

旧 ER 図の存在しない Snapshot / State 型への参照は、同じ YAML から導く以下の表に置き換える。

| 参照元 | 参照先 | 関係 |
|---|---|---|
| Intent | WorkflowDefinition | 多対1。definition_id のみ保持 |
| Intent | StageEntry | 1対多。静的計画を所有 |
| IntentExecution | Intent | 多対1。intent_id のみ保持 |
| IntentExecution | StageKey | 1対多。適用の添字帳を所有 |
| IntentExecution | StageIndex | cursor / parked_at の位置 |
| IntentExecutionEvent | IntentExecution | 多対1。aggregate_id で対象を指す |
| Started | StageEntry | 誕生時の計画を歴史として運ぶ |

スナップショットはある通番時点の同じ集約の状態であり、ドメインに双子の Snapshot エンティティを追加しない。
表で参照する値型と既存型の所有元は entities.md の referenced_types にある。

## 7. 導出ビュー — 規則要約

BR1.0〜BR1.9 は受理条件、1コマンド1イベント、進捗、誕生時の初期化完了、ゲート、skip、
jump、再 park、自律切替、stale の無変更ガード。
BR2.1〜BR2.6 はイベント固有 ID と封筒の分離、計画の所有、最新 snapshot と差分の再生、
16変種、Quint v2.7 の射影、集約間の ID 参照。
BR3.1〜BR3.3 は判断と呼出境界、BR4.1〜BR4.2 は PlanAction と overlay の所有、
BR5.1〜BR5.4 は位置型、永続化中立、読取版、コーディング規則。
個別の正確な条件は rules.md の同じ ID を参照する。

## 8. トレーサビリティ

[traceability.json](traceability.json) の coverage が要求ごとの対応表。
FR8.3 → BR4.1、FR8.4 → BR4.2、FR2.1 → BR1.0 / 1.1 / 1.3 / 1.4 / 1.5 / 1.9、
FR3.1 / FR3.3 → BR3.1〜BR3.3、FR1.3 → BR2.1 / 2.3 / 2.6 / 5.2 / 5.3。
NFR1 は遷移・射影・再生の前提、NFR3 は自己完結した履歴と差分再生へ対応する。
coverage の OK は要求と設計箇所の対応を表し、実装の全面適合や正式承認を表さない。

## Review

**Verdict:** NOT-READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-09-05T07:04:22Z
**Iteration:** 1
**Request Challenge:** review:61b1c22701501c1e4c090dd54ec689a9

### Findings

旧レビューの番号 1〜20 を保持する。Resolved は旧所見そのものの解消を示し、後続裁定への同期完了を意味しない。新たな差異は R-21 以降へ分けた。過去の質問回答・pending-revision は履歴として読み、撤去された API や旧方式の再導入指示には用いていない。

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| 1 | Critical | aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u2-domain-es-core/functional-design/rules.md > BR1.0 | 旧所見の park 中の位置変更拒否は規則に明記され、現行コードも guard_running_for で位置変更を拒否する。後続の再 park・自律切替の許可は別論点 R-24。 | 旧修正を維持し、明示的な例外は R-24 として同期する。 | Resolved |
| 2 | Major | aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u2-domain-es-core/functional-design/rules.md > BR2.2 | 全ステージを文書順で扱うことが明記され、現行 Intent::create も stages_in_scope を同じ順序で使う。 | 追加対応なし。責務の移動は R-21。 | Resolved |
| 3 | Major | aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u2-domain-es-core/functional-design/entities.md > c5_revision_proposal | 旧所見の残件だった Started.stages の変更宣言は追記され、C5 にも反映されている。後続のイベント設計は R-22。 | 追加対応なし。 | Resolved |
| 4 | Major | aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u2-domain-es-core/functional-design/rules.md > BR1.6 | 現カーソルでは Pending を飛ばさず、介在位置では Pending も対象にする非対称は明記済み。現行モデルも維持する。後から追加された inScope 条件の未反映は R-24。 | 旧修正を維持する。 | Resolved |
| 5 | Major | aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u2-domain-es-core/functional-design/rules.md > BR1.2 | active と in-flight の集合を分離済み。現行の状態射影・準拠テストも成功した。 | 追加対応なし。 | Resolved |
| 6 | Major | aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u2-domain-es-core/functional-design/rules.md > BR4.2 | 削除対象を畳み込みの 2 メソッドに限定済み。現行 WorkflowDefinition には残す 6 述語があり、削除 2 メソッドの公開定義はない。 | 追加対応なし。 | Resolved |
| 7 | Major | aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u2-domain-es-core/functional-design/rules.md > BR2.2 | None を SKIP に畳む規則が明記され、現行の a_missing_grid_cell_folds_to_skip_outside_initialization テストも成功した。 | 追加対応なし。 | Resolved |
| 8 | Minor | aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u2-domain-es-core/functional-design/functional-spec.md > 第 2 節 stale_report | 旧設計内の戻り値は統一されたが、現在の stale_report は Result<(), CommandError>。NextDecision を返す宣言は現行 API と一致しない。 | 現行の戻り値と、完了応答を組み立てる境界を同期する。 | Unresolved |
| 9 | Minor | aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u2-domain-es-core/functional-design/rules.md > BR4.1.logic | 正当な利用を除く検出式にはなったが、対象パス modules/core/domain は旧配置。現行の modules/core/command/domain/src/orchestration では定義・再輸出は 0 件だった。 | 検出先を現行の配置へ直し、パス不在を成功と扱わない条件を明記する。 | Unresolved |
| 10 | Minor | aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u2-domain-es-core/functional-design/rules.md > BR4.1.statement | 「実測 10 ファイル」は旧 WorkflowExecution・旧クレート配置の一覧で、現在の移動対象一覧として使えない。元の移動は現行ファサードで確認できるが、現在の全呼出側を網羅したという証拠ではない。 | 10 ファイルを過去実績と明示し、次の変更では現行配置の影響範囲を別途確認する。 | Unresolved |
| 11 | Minor | aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u2-domain-es-core/functional-design/rules.md > BR2.5 | 射影表は追加済みだが、現行 engine_loop.qnt v2.7 の初期化完了状態・受領証状態・受理条件を反映していない。ITF の現行 assert_projection / assert_signal との全対応表ではなくなっている。 | 現行モデル版と射影対象・モデル外の条件を明記する。古いモデルへ戻さない。 | Unresolved |
| 12 | Minor | aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u2-domain-es-core/functional-design/rules.md > BR3.1 | 冒頭では定義 ID 検査を必須とする一方、末尾では定義引数を未使用と説明する。現行 next_decision は &Intent を受け NextDecision を返すため、予約引数という説明も古い。 | 未使用注記を撤去し、R-21 の現行境界に合わせて入力と検査を記述する。 | Unresolved |
| 13 | Minor | aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u2-domain-es-core/functional-design/entities.md > relationships、および functional-spec.md > 第 6 節 | 旧所見の既存参照型の所有注記はなお不足する。さらに ER 図は WorkflowExecutionSnapshot を参照するが、YAML の見出しは WorkflowExecutionState で、そこも B12 以前の履歴とされている。派生図の参照が現行正本へ解決しない。 | 既存型の所有元を明記し、正本の同期後に ER 図と要約を同じ型集合から導出する。 | Unresolved |
| 14 | Critical | aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u2-domain-es-core/functional-design/rules.md > BR1.3、および entities.md > StageEntry.phase | phase を保持し initialization 全体を非ゲートとする修正は反映済み。現行の initialization 3 段のテストも成功する。旧 Critical の索引 0 限定という原因は解消。誕生状態の後続変更は R-23。 | フェーズ判定を維持し、誕生状態は R-23 で同期する。 | Resolved |
| 15 | Major | aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u2-domain-es-core/functional-design/functional-spec.md > 第 2 節・W2 | artifacts / phase_boundary の供給引数は旧指摘後に追記済み。現行 open_gate も artifacts を受ける。ただし phase_boundary は現在イベントへ載せない形に変わっており、その同期は R-22。 | 旧不足は閉じ、現行ペイロードを R-22 で整理する。 | Resolved |
| 16 | Major | aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u2-domain-es-core/functional-design/entities.md > WorkflowExecution.revision_count | ステージごとの回数と加算規則を追加済み。現行 reject_gate_increments_the_revision_count テストも成功し、供給元不在という旧問題は解消。現行では apply が回数を導出する。 | 回数の存在を維持し、イベントへの搭載有無は R-22 で同期する。 | Resolved |
| 17 | Major | aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u2-domain-es-core/functional-design/entities.md > c5_revision_proposal | Started.stages_in_scope から stages: list<StageEntry> への変更と phase 追加が宣言され、共有 C5 も stages を記載する。 | 追加対応なし。 | Resolved |
| 18 | Minor | aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u2-domain-es-core/functional-design/functional-spec.md > 第 4.1 節 backward・redo | InProgress と redo 行は追加された。ただし backward 行は「target 以降」を Pending とし、BR1.6 / W5 の「target+1 以降を Pending、target は InProgress」となお一致しない。 | 状態表を target とその後続に分け、target を Pending と読める記述を訂正する。 | Unresolved |
| 19 | Minor | aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u2-domain-es-core/functional-design/rules.md > BR2.2.logic | conditional の取得元を graph().nodes()[i].execution() と明記済み。現在は Intent::create が同じ組み立てを担う。 | 追加対応なし。所有の更新は R-21。 | Resolved |
| 20 | Minor | aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u2-domain-es-core/functional-design/functional-spec.md > 第 5 節 StartError | initialization を除く旨と Empty の条件は明記済み。旧エラー条件説明の不足は解消している。開始時検査の所有移動は R-21。 | 追加対応なし。 | Resolved |
| R-21 | Major | aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u2-domain-es-core/functional-design/entities.md > IntentId・WorkflowExecution・WorkflowDefinition、および functional-spec.md > 第 2 節・W1・W4 | 保存済み設計は dirName を IntentId とし、定義参照・依頼・静的計画を WorkflowExecution に保持する。現行裁定は Intent / IntentExecution を分離し、両 ID は別型、IntentId は UUIDv7。現行 intent.rs が計画解決と定義参照を所有し、IntentExecution::start は実行 ID と &Intent を受ける。WorkflowDefinition の「読取専用・ES 対象外」も現行 cqrs-boundaries / gateway-taxonomy の通常 ES Repository と一致しない。これは名称変更だけではなく、再構成と判断へ渡す情報の所有変更である。 | 旧記録を履歴としたうえで、現行の集約・ID・静的計画・実行状態の所有表と、開始／判断 API を同期する。pending-revision の kebab ID 案は再採用しない。 | New |
| R-22 | Major | aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u2-domain-es-core/functional-design/rules.md > BR2.1・BR2.4・BR5.2・BR5.3、および functional-spec.md > W2・W3・第 5 節 | serde・本家 Aggregate/Event trait・memento・複合イベント ID・失敗を返す apply_event を現行設計として指示しているが、domain-persistence-neutrality / aggregate-commands はそれらを撤去済み。現行 Cargo.toml に serde / ESA の直接依存はなく、イベントは自前 UUIDv7 の id と aggregate_id を分離し、通番と時刻は apply の引数。replay / apply_event は Result を返さない。C3 の v3/B13 追記とも世代が違い、phase_boundary / revision_count 等のペイロードも現行実装と不一致。 | ドメインのイベント内容とアダプタの封筒・DTO を分離した現行契約へ同期し、復号失敗と壊れた履歴の扱いを区別する。再生方式は確定済みの「最新スナップショット＋それより後の差分」を維持する。共有契約の古い全再生指定を再導入しない。 | New |
| R-23 | Major | aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u2-domain-es-core/functional-design/functional-spec.md > W1 手順 3・4・第 4 節、および rules.md > BR1.3 | W1 は初期化を InProgress で始め、birth が complete_stage を 3 回呼ぶ。一方、後続裁定と現行 engine_loop.qnt v2.2 以降は誕生時に初期化完了済みとし、現在のイベント enum に StageCompleted はなく、集約に complete_stage もない。現行 From<(Started, occurred_at)> は初期化を Completed、最初の実効対象を InProgress にする。設計の呼出順は現 API では実行不能で、旧 API を復活させると初期化完了の二重記録へ戻る。 | W1・状態表・イベント一覧から旧非ゲート完了経路を履歴へ移し、誕生時完了と対象実ステージがない縮退形の状態を現行の根拠に合わせて明記する。 | New |
| R-24 | Major | aidlc/spaces/default/intents/260822-stage1-selfhost/construction/u2-domain-es-core/functional-design/rules.md > BR1.0・BR1.3・BR1.6・BR1.7・BR1.8、および functional-spec.md > W5・W6・第 4.2 節 | コマンド受理条件が後続の互換裁定を反映していない。現行モデル v2.1 は forward の介在位置に inScope を要求、v2.3 は再 park を許可、v2.7 は park 中／Completed の自律切替を許可する。現行 approve_gate はレビュー・実践昇格の受領証ガードを持ち、switch_autonomy は人間の操作の確認を伴う。旧設計はそれらを省略し「park 中は unpark 以外拒否」「completed は終端」と一括規定するため、許可／拒否が現行と逆になる場面がある。 | 現行モデル版に対応したコマンド別ガード表を作り、位置変更と再スタンプ・権限変更・受領証記録を区別する。承認前提、forward の inScope 条件、モデル外の人間操作確認も明記する。 | New |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| aidlc-sensor-required-sections.ts（--stage functional-design、各 --output-path） | PASS: entities / rules / functional-spec、所見 0 | 追記前の H2 は 2 / 2 / 8。契約の世代差は検出しない。 |
| aidlc-sensor-upstream-coverage.ts（consumes 5 件、deliverables 3 件を明示） | PASS: unreferenced 0 | 共有入力への参照は揃う。内容の現行性は別途照合した。 |
| aidlc-sensor-traceability.ts | FAIL: missing_from_upstream_ids 32 件。他の gaps / orphans / missing_from_table / invalid_entries / invalid_targets は空 | 欠落一覧は共有 story-map 上の U2 担当外。FR8.3 / FR8.4 と、明示された横断の前提要求の BR 参照は解決する。ただし OK は現行実装への同期完了を証明しない。 |
| linter / type-check の適用判定 | 対象外・未実行 | 対象成果物に TS/JS/TSX の実行コードや該当スニペットはない。 |
| cargo test --locked -p core-command-domain --lib orchestration::intent_execution | PASS: 166 件、失敗・無視 0 | 初期化完了・差分再生・イベント ID・再 park・自律切替・受領証・ジャンプ等の関連テストを実行。検証結果は現行実装の証拠であり、旧設計への適合証明ではない。 |
| cargo test --locked -p core-command-domain --test engine_loop_conformance | PASS: 1 件、失敗・無視 0 | コミット済み全トレースの遷移と観測を現行集約へ再生。Quint ソルバーの新規実行・トレース再採取・全ワークスペース検証は今回行っていない。 |
| PlanAction 所有・公開面の静的検査 | PASS: 現行 orchestration 内の定義／再輸出 0 件 | workflow_definition/plan_action.rs とそのファサードに存在。WorkflowDefinition の残す 6 述語も実在する。旧 10 ファイル一覧の全件再監査とは区別する。 |
| 永続化依存・現行 API・モデルヘッダの照合 | 差異を確認 | domain Cargo.toml、Intent / IntentExecution / IntentExecutionEvent、engine_loop.qnt v2.7 を使用。R-21〜R-24 は実装失敗の報告ではなく、保存済み正本の同期不足である。 |
| ER 図と YAML の参照照合 | 不一致: WorkflowExecutionSnapshot / WorkflowExecutionState | 所見 13。Mermaid パーサ検査は未実行。 |

### Summary

旧 Critical 2 件は原因を解消済みだが、未解消の Major 4・Minor 7 が残るため ADVISORY 判定は NOT-READY。関連実測 167 テストは成功しており、今回確認したのは現行実装の不具合ではなく、後続裁定と実測に追従していない設計記録の問題である。過去の回答と改訂候補を保存しつつ、現行の実装指示となる正本を同期する必要がある。
