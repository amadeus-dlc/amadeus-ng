# functional-spec — U2 ドメイン ES コア（`u2-domain-es-core`）

> 現行本文は2026-09-05の是正を反映する。構造の正本は [entities.md](entities.md) の YAML、
> 規則の正本は [rules.md](rules.md) の YAML。以下はその API・手順・導出ビューである。
> 是正前の 2026-09-05 レビュー記録は [functional-spec-review-history-2026-09-05.md](functional-spec-review-history-2026-09-05.md)
> へ原文のまま退避した。是正の範囲・検証・残件は [correction-report.md](../correction-report.md) に記録し、正式承認とは区別する。
> 2026-09-07 再走（Modify）は質問票 Q4 / Q4a / Q5・P7〜P10 を反映する: コマンド側の配列は FCC（BR5.5）、
> `next_decision` の ID 照合（Q5 = A）、コード側への引継ぎ（第 9 節）。実装は U2 の code-generation 再走で行う。
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
コマンド側ドメインモデルの配列部分はすべてファーストクラスコレクション（FCC）にし、
不変条件と操作（`at` / `filter` / `fold_left` / `map` / `combine` / `divide` + 業務操作）を型が持つ（BR5.5）。
リードモデル側は FCC を使わず、境界で `fold_left` により自前の平坦な表現へ写す（オーナー裁定 2026-09-07）。
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
| open_gate(&Intent, ArtifactPaths, t) | GateOpened |
| approve_gate(&Intent, Option<&ReviewPolicy>, user_input, t) | GateApproved。phase_boundary は引数・ペイロードに持たない |
| reject_gate(&Intent, feedback, t) / revise_stage(&Intent, t) | GateRejected / StageRevised |
| skip_stage(&Intent, reason, t) | StageSkipped |
| jump(&Intent, StageIndex, t) / jump_resolve(&Intent, StageIndex) | Jumped / Result<JumpDirection, CommandError> |
| park(&Intent, t) / unpark(&Intent, t) / recompose(&Intent, StageIndexSet, t) | Parked / Unparked / Recomposed。recompose の入力は位置集合（現行は `&[StageIndex]`、code-generation で同期） |
| switch_autonomy(&Intent, mode, &HumanTurns, guard: bool, t) | AutonomyModeSet |
| record_single_stage_run(&Intent, &StageSlug, t) | Result<IntentExecutionEvent, SingleStageRunRefusal>。名指しの対象を自己の添字帳で解決して隔離実行を記録 |
| record_skeleton_stance(&Intent, stance, t) | Result<IntentExecutionEvent, SkeletonStanceRefusal>。拒否時はstage・scope・CommandErrorを文脈として返す |
| apply_report(&Intent, &ReportRequest, &TransitionSteps, Option<&ReviewPolicy>, t) | Result<IntentExecutionEvent, ReportCommitError>。報告適用の段分岐と入力の正規化を所有し、成功時は単一イベント。TransitionSteps は ReportDecision::Commit.steps と同じ FCC（現行は `&[TransitionStep]`、BR5.5） |
| report_dispatch(&Intent, &ReportRequest) | ReportDecisionのCommit / NoOpに、判断対象のscopeも含めて返す |
| request_review(&Intent, &StageSlug, policy, reviewer, iteration, retry_pending, t) | ReviewRequested |
| record_review_verdict(&Intent, &StageSlug, policy, reviewer, iteration, verdict, t) | ReviewCompleted |
| affirm_practices(&Intent, &PracticesPromotion, affirming_user, t) | PracticesAffirmed |
| stale_report(StageIndex) | Result<(), CommandError>。書込を伴わない冪等完了の受理ガード |
| next_decision(&Intent, &NextRequest) | Result<NextDecision, CommandError>。ID 不一致は IntentMismatch（Q5 = A）。&Intent を skeleton ゲートの判断に使用 |
| replay(snapshot: IntentExecution, events: IntoIterator<(usize, DateTime<Utc>, IntentExecutionEvent)>) | IntentExecution。最新 snapshot と後続差分を畳み込む |
| apply_event(seq_nr: usize, t, &IntentExecutionEvent) | ()。壊れた履歴は panic |
| new(全状態材料) | Result<IntentExecution, IntentExecutionError>。DTO 境界の検査付き完全コンストラクタ |
| with_version(usize) / version() | 読取済みの不透明な版の受け渡し。本家 trait の set_version ではない |
| stage_index(usize) | Option<StageIndex>。公開の位置解決 |
| slots() / stage_key(StageIndex) | &StageSlots / Option<&StageKey>。旧 stage_keys() のスライス公開は廃止し、境界の列挙は fold_left で行う |
| Intent::stages() | &StageEntries。旧 &[StageEntry] のスライス公開は廃止 |

`complete_stage` / `StageCompleted` / `set_autonomy` / `state` / `from_state` /
`WorkflowExecutionState` は現行 API ではない。
`next_decision` が `DefinitionMismatch` を返すという旧宣言も失効している。
コマンド・書込前ガード・`next_decision` はすべて `IntentMismatch` を Err で返す（BR2.6、Q5 = A。
現行コードの `next_decision` は `NextDecision` を直接返しており、code-generation で同期する）。

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
   `Started` は自分のイベント ID、aggregate_id、intent_id、`StageEntries` を運ぶ。
   `From<(Started, occurred_at)>` が `StageEntries` から `StageSlots` を導出し、seq_nr=1、version=0 の誕生状態を作る。
4. initialization は全て Completed、承認は false。最初の実効対象の実ステージだけ InProgress。
   実ステージがなければ Running、cursor=0、active=0 の縮退形になり、通常 next は Done を返す。
   初期化完了用の追加コマンド・追加ドメインイベントは発行しない。
5. 各集約と誕生イベントをそれぞれの Repository に保存する。監査面の初期化完了行は U4 の誕生投影が描く。

### W2 — コマンド実行

1. 対象 Intent の照合とコマンド別ガードを通す。承認では状態前提の後、昇格受領証、レビュー終端受領証を順に検査する。
2. 自前の UUIDv7 イベント ID と aggregate_id、必要な事実だけでイベントを作る。
   artifacts / user_input / feedback 等は引数の値を使う。
3. `apply_event(current_seq + 1, occurred_at, event)` が次位置・差分・回数を導出し状態を進める。
   位置ごとの更新は `StageSlots.with_slot` / `with_slots(StageIndexSet, ..)` を通し、対象位置は
   `StageIndexSet` の集合演算（`range` / `combine` / `divide` と `positions(述語)`）で求める。
   受領証の試行境界は前進先、差し戻し先、各 jump では `clear_receipts` で全ステージを更新する。
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

0. まず `&Intent` の ID を照合し、不一致は `IntentMismatch` の Err で拒否する（BR2.6、Q5 = A）。
   呼出側（リードモデル更新器の `NextAnswerRow::of`）は Err を判断結果ではなく投影の束縛不整合として扱う。
1. park 活性かつ非再入 → resume 指定なら UnparkThenResume、それ以外 Parked。
2. resume → ResumeMenu。free_text → NewWorkRouting。Completed → Done。
3. cursor が in-flight で実効 SKIP → InProgress / Revising なら RecoverSkipInconsistency、
   Pending / AwaitingApproval なら InconsistentSkip。
4. cursor が in-flight → RunStage。それ以外は次の in-scope があれば RunStage、なければ Done。
5. RunStage の gate は GateDecision。initialization は Ungated、skeleton 対象で stance 未記録なら
   Unresolved、それ以外は Gated。skeleton 対象は `StageEntries.first_of(Construction, EXECUTE)` で特定し、
   次の in-scope は `StageSlots.next_effective_execute_after(cursor)` で求める（配列を外へ取り出さない）。

判断は集約が所有する。RMU が同じ判断を投影し、クエリ側ユースケースは DAO から答えを取得する。
`stale_report` は別の書込前ガードで、成功時の冪等な完了応答は呼出側が組み立て、イベントを作らない。

### W5 — ジャンプ

`jump_resolve` が対象と方向を検査し、`jump` は `Jumped{target}` を記録する。
apply が方向・スキップ・巻き戻しを導出する。

- forward: 介在位置は **in-scope かつ in-flight** のみ Skipped。現 cursor は in-flight かつ非 Pending のみ Skipped。
  対象集合 = `range(cursor+1, target)` ∩ `positions(in_scope)` ∩ `positions(in_flight)`（StageIndexSet の combine / divide）。
- backward: **target+1 以降**の in-scope 非 Pending を Pending に戻す。target 以降の承認を消す。
  対象集合 = `range(target+1, end)` ∩ `positions(in_scope)` ∖ `positions(Pending)`。
- redo: cursor の承認を消す。
- いずれも target は InProgress、cursor=target。全ステージのレビュー試行・昇格受領証を消す。target 自身を Pending と記述しない。

### W6 — 計画・park・権限・受領証

recompose は後続 Pending の反転対象（入力の位置集合を `StageIndexSet` に集合化）を全件検査し、
添字帳で slug へ写した `StageSlugSet` として一括で skipped / added を記録する。
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
| next_decision（クエリ） | 状態条件なし | ID 一致のみ（不一致は IntentMismatch）。状態不変 |

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
| next_decision | IntentMismatch | 状態不変の Err（Q5 = A）。DefinitionMismatch を返すと宣言しない |

## 6. 導出ビュー — 所有関係（entities.md が正本）

旧 ER 図の存在しない Snapshot / State 型への参照は、同じ YAML から導く以下の表に置き換える。

| 参照元 | 参照先 | 関係 |
|---|---|---|
| Intent | WorkflowDefinition | 多対1。definition_id のみ保持 |
| Intent | StageEntries | 1対1。静的計画（FCC）を所有 |
| IntentExecution | Intent | 多対1。intent_id のみ保持 |
| IntentExecution | StageSlots | 1対1。位置ごとの記録（添字・実効計画・進捗・承認・受領証）の FCC を所有 |
| IntentExecution | StageIndex | cursor / parked_at の位置 |
| IntentExecutionEvent | IntentExecution | 多対1。aggregate_id で対象を指す |
| Started | StageEntries | 誕生時の計画を歴史として運ぶ（Intent.stages と同じ型） |
| GateOpened / Recomposed / PracticesAffirmed | ArtifactPaths / StageSlugSet / PromotedSections・RuleLines | ペイロードの列は FCC |

スナップショットはある通番時点の同じ集約の状態であり、ドメインに双子の Snapshot エンティティを追加しない。
表で参照する値型と既存型の所有元は entities.md の referenced_types にある。

## 7. 導出ビュー — 規則要約

BR1.0〜BR1.9 は受理条件、1コマンド1イベント、進捗、誕生時の初期化完了、ゲート、skip、
jump、再 park、自律切替、stale の無変更ガード。
BR2.1〜BR2.6 はイベント固有 ID と封筒の分離、計画の所有、最新 snapshot と差分の再生、
16変種、Quint v2.7 の射影、集約間の ID 参照。
BR3.1〜BR3.3 は判断と呼出境界、BR4.1〜BR4.2 は PlanAction と overlay の所有、
BR5.1〜BR5.5 は位置型、永続化中立、読取版、コーディング規則、コマンド側配列の FCC 化。
個別の正確な条件は rules.md の同じ ID を参照する。

## 8. トレーサビリティ

[traceability.json](traceability.json) の coverage が要求ごとの対応表。
FR8.3 → BR4.1、FR8.4 → BR4.2、FR2.1 → BR1.0 / 1.1 / 1.3 / 1.4 / 1.5 / 1.9、
FR3.1 / FR3.3 → BR3.1〜BR3.3、FR1.3 → BR2.1 / 2.3 / 2.6 / 5.2 / 5.3。
NFR1 は遷移・射影・再生の前提、NFR3 は自己完結した履歴と差分再生へ対応する。
BR5.5 は要求 ID を持たない横断規律として reverse に記す。
coverage の OK は要求と設計箇所の対応を表し、実装の全面適合や正式承認を表さない。

## 9. code-generation への引継ぎ（2026-09-07 再走）

本設計と現行コード（`modules/core/command/domain/src/orchestration/`）の差分。U2 の code-generation 再走で実装する。

| # | 差分 | 出典 |
|---|---|---|
| 1 | FCC の新設: `StageEntries` / `StageSlots`（旧 7 並列列の統合）/ `StageIndexSet` / `ArtifactPaths` / `StageSlugSet` / `PromotedSections` / `RuleLines`。各型は不変条件と at / filter / fold_left / map / combine / divide + 業務操作を持ち、`Vec` / `&[..]` の公開を廃止する。`ReviewAttempt` の内部列と `ReportDecision::Commit.steps` も対象 | BR5.5、Q4 / Q4a |
| 2 | `next_decision` を `Result<NextDecision, CommandError>` にし ID 不一致を `IntentMismatch` で拒否する。呼出側 `NextAnswerRow::of`（read-model-updater）は Err を投影の束縛不整合として扱う | BR2.6 / BR3.1、Q5 |
| 3 | 境界の追随: command interface-adapter の DTO（Started / Created / IntentExecution）と read-model-updater（`ResolvedPlan::of`、`read_tables` の行生成・slug 引当）の要素列挙を `fold_left` へ書き換える。リードモデル側は FCC を使わず自前の平坦な表現のまま | BR5.5、オーナー裁定 2026-09-07 |
| 4 | `orchestration/mod.rs` 冒頭説明の修正: 「再構成はジャーナル全再生」→ 最新スナップショット + 差分（BR2.3）、「`next_decision` はクエリ側 `ExecutionStateView` が所有」→ `IntentExecution::next_decision`（BR3.1 / 2026-09-02 オーナー規律）。`intent_execution.rs` 冒頭の「memento」「panic しない」の旧説明も同様 | P8、correction-report |
| 5 | 積み残し（本 Bolt に含めない、Issue は起票しない）: (a) 上流 `components.md` 冒頭注記と `contract-summary.md` C3 の B13 追記（2026-08-30「ジャーナル全再生」）を 2026-09-05 裁定へ同期する。(b) `combine` / `divide` / `map` を共通 trait `FirstClassCollection` へ盛り込む（オーナーの最終方針、着手時期は別途裁定） | P7、Q4a |

同じ Bolt で CI 3 ジョブ（check / quint / coverage）を緑に保つ。Quint モデル v2.7 と ITF 準拠テストの射影（rules.md 第 3 節）は
FCC 化で意味論を変えない（`StageSlots.at` で同じ観測を読む）。

## Review

**Verdict:** NOT-READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-09-06T15:41:19Z
**Iteration:** 1

### Findings

| ID | Severity | Location | Finding | Required action |
|---|---|---|---|---|
| R-01 | Major | `construction/u2-domain-es-core/functional-design/entities.md` > 第 1 節 entities、`rules.md` > BR5.5 statement、`functional-spec.md` > 第 2 節 apply_report 行・第 9 節 #1 | BR5.5 は 9 系統の配列を FCC 化対象に挙げるが、entities.md が不変条件と操作を定義しているのは StageEntries / StageSlots / StageIndexSet / ArtifactPaths / StageSlugSet の 5 型だけ。PromotedSections / RuleLines は referenced_types に型名が並ぶのみで属性・不変条件・操作が無い。`ReportDecision::Commit.steps` の FCC は functional-spec 第 2 節に `TransitionSteps` という名前だけが現れ定義が無く、`ReviewAttempt` の pending / closed は型名すら無い。`coding-rules/first-class-collections.md` は型ごとに不変条件・filter / map の結果型・combine / divide の規則を定めることを要求しており、この状態では実装者が発明するほかない。加えて現行の生産コード `modules/core/command/use-case/src/orchestration/commit_verdict_use_case.rs:212` は `steps.contains(&TransitionStep::Approve)` を使うが、BR5.5 の操作集合（at / filter / fold_left / map / combine / divide + 業務操作）に対応する操作が示されていない | TransitionSteps、ReviewAttempt の pending / closed、PromotedSections、RuleLines を entities.md に他の 5 型と同じ粒度（不変条件・操作・結果型）で定義する。TransitionSteps には `contains` 相当の業務操作を明示する |
| R-02 | Major | `functional-spec.md` > 第 9 節 #3 | 追随対象が command interface-adapter の DTO 3 本と read-model-updater だけになっている。実測では `core-command-use-case` の生産コードが影響を受ける（`commit_verdict_use_case.rs:212,218` が `ReportDecision::Commit.steps` を `Vec` として `contains` し `apply_report(.., &steps, ..)` へ渡す。`test_support.rs:114,856,889` は `Started::new(.., original.stages().to_vec())`）。ドメインクレートの ITF 準拠テスト `modules/core/command/domain/tests/engine_loop_conformance.rs` も `next_decision`（:356）・`open_gate(.., Vec::new(), ..)`（:449）・`recompose(.., &[index], ..)`（:488）を呼ぶ。第 9 節は「同じ Bolt で CI 3 ジョブを緑に保つ」と宣言しているため、この 2 か所の欠落はそのまま CI 赤になる | 第 9 節 #3 に `core-command-use-case`（`commit_verdict_use_case.rs` / `test_support.rs`）と `engine_loop_conformance.rs` を追随対象として追加する |
| R-03 | Major | `entities.md` > StageSlugSet | 不変条件に「重複なし、文書順」を置き、操作に combine（和集合）と「空集合を単位元とする Monoid として試験する」を課している。しかし `StageSlug` は `modules/core/command/domain/src/workflow_definition/stage_slug.rs:19` のとおり `String` の newtype で順序情報を持たず、StageSlugSet 自身も静的計画（StageEntries）や添字帳への参照を持たない。したがって任意の 2 値を combine した結果を文書順へ並べ直す手段が型内に無く、仕様どおりには実装できない。文書順を保証しているのは生成経路（StageIndexSet から添字帳で写す）だけで、combine / divide はその保証の外にある | 表現を位置つき（StageIndex を伴う）にする、combine の順序規則を明示する（例: 合成は StageIndexSet 上で行い最後に写す）、または不変条件から文書順を外す、のいずれかを選んで記載する |
| R-04 | Minor | `entities.md` > StageEntries / StageSlots | 両型は非空を不変条件に持ちつつ filter と「divide（他方に含まれる slug を除いた列、空可の型へ戻る）」を操作に挙げるが、その「空可の型」の名前が本設計のどこにも無い。`FirstClassCollection` trait は `type Filtered` の具体型を要求する（`modules/core/infrastructure/src/collections/first_class_collection.rs:10`）。referenced_types の `Collection<T>` が候補だが、両型は slug 一意という追加不変条件を持つため自明に決まらない | filter / divide の結果型を名前で指定する |
| R-05 | Minor | `rules.md` > 第 3 節 射影表「stage 1以降 / skeletonGateStage」行 | 同行は「静的計画で最初の非 init EXECUTE 位置を skeleton 対象にする」と書くが、BR3.1・functional-spec W4-5・entities.md はいずれも `StageEntries.first_of(Construction, EXECUTE)` であり、現行コード `intent_execution.rs:544-554` も `phase() == PhaseId::Construction && plan_action() == PlanAction::Execute` である。出荷グラフでは initialization と Construction の間に inception 系フェーズが入るため、両者は一致しない | 当該行を「Construction かつ EXECUTE」に統一し、合成計画に限った記述であることを明記する |
| R-06 | Minor | `rules.md` > BR5.5 violation、`functional-spec.md` > 第 9 節 #3 | violation は「リードモデル側での FCC 使用は違反」と書くが、第 9 節 #3 と entities.md の persistence_boundary.collections は read-model-updater に「要素列挙を fold_left へ書き換える」ことを指示している。fold_left は FCC の操作なので、文言どおりでは RMU が `slots().fold_left(..)` を書くこと自体が違反になり、指示と矛盾する | 「リードモデル側は FCC 型を定義・保持しない（境界での読取操作の呼出は除く）」へ言い換える |
| R-07 | Minor | `rules.md` > BR5.5 statement / violation | statement は全 FCC に combine / divide を型ごとの契約として課す一方、violation は「使われない共通メソッド群の機械的追加も違反」と書き、`coding-rules/first-class-collections.md` 第「検証と適用」節も同じ禁止を置く。設計本文が用途を示しているのは StageIndexSet の combine / divide（jump の対象集合合成）だけで、StageEntries / StageSlots / ArtifactPaths の combine / divide には用途の記載が無く、実装者がどちらの規則に従うか判断できない | Q4a = A の裁定が当該禁止に優先することを BR5.5 に明記するか、用途の無い型を combine / divide の対象から外す |
| R-08 | Minor | `functional-spec.md` > 第 2 節「インターフェイス（現行 API）」 | 見出しは現行 API を掲げるが、表の複数行は設計後の API である。recompose / apply_report / next_decision / slots / `Intent::stages` には現行との差の注記があるのに、`open_gate(&Intent, ArtifactPaths, t)` には注記が無い（現行は `intent_execution.rs:821-825` のとおり `artifacts: Vec<String>`） | 見出しを「API（設計。現行との差は第 9 節）」等に改めるか、open_gate 行にも現行型の注記を入れる |
| R-09 | Minor | `functional-spec.md` > 第 9 節 #1、`rules.md` > BR5.5 logic | `coding-rules/first-class-collections.md` は既存 7 型の trait 適合を `modules/core/command/domain/tests/collection_contract_test.rs` と `modules/core/infrastructure/tests/collections_test.rs` で検査すると定める。本設計は集合型の Monoid 則試験に触れるが、新設 7 型をこの既存ハーネスへ登録する指示が引継ぎに無い | 第 9 節 #1 に既存のコレクション契約テストへの登録を明記する |
| R-10 | Info | `functional-spec.md` > 第 9 節 #3、上流 `inception/units-generation/unit-of-work.md` U2「境界」 | U2 の宣言境界は「`core-command-domain` クレート内 … Repository・ストア・投影は持たない（U3 / U4）」だが、第 9 節 #3 は read-model-updater と interface-adapter の改修を U2 の Bolt に含める。U2 責務欄の PlanAction 完全移動に「呼出側パスの一斉修正を同 Unit に含む」という先例があり、質問票の `## Consolidated Summary Confirmation` もオーナー承認済みなので裁定としては成立するが、越境の根拠が設計本文に書かれていない | 第 9 節に「呼出側一斉修正の先例と 2026-09-07 の裁定による」旨を 1 行加える |

### Validation Tool Results

| ツール | 結果 | 解釈 |
|---|---|---|
| `aidlc-sensor-required-sections`（entities.md） | `pass: true`、H2 2 本、findings 0 | 構造上の欠落なし |
| `aidlc-sensor-required-sections`（rules.md） | `pass: true`、H2 3 本、findings 0 | 構造上の欠落なし |
| `aidlc-sensor-required-sections`（functional-spec.md） | `pass: true`、H2 9 本、findings 0 | 構造上の欠落なし |
| `aidlc-sensor-traceability`（traceability.json） | `pass: false`、`gaps` / `orphans` / `invalid_targets` / `invalid_entries` はすべて空、`missing_from_upstream_ids` 32 件 | 32 件はすべて他 Unit 所管の FR ID（FR1 / FR2 / FR4〜FR7 / FR9 系）による既知ノイズ。U2 の対応関係そのものに欠落・孤児・不正 target は無い |
| BR ID の網羅照合（手検査） | 定義 26 本（BR1.0〜1.9 / BR2.1〜2.6 / BR3.1〜3.3 / BR4.1〜4.2 / BR5.1〜5.5）に対し coverage 22 本 + reverse 4 本 = 26 本 | 過不足なし |
| 現行コード実測（`grep` / `sed` による読取のみ） | `intent.rs:260` `stages() -> &[StageEntry]`、`intent_execution.rs:441` `stage_keys() -> &[StageKey]`、`:821` `open_gate(.., Vec<String>, ..)`、`:1060` `recompose(.., &[StageIndex], ..)`、`:1897` `next_decision(..) -> NextDecision`、`:2024` `apply_report(.., &[TransitionStep], ..)`、`mod.rs` 冒頭は「ジャーナル全再生」「next_decision はクエリ側が所有」 | 第 9 節 #1 / #2 / #4 の差分記述は実測と一致する。追随対象の列挙だけが不足（R-02） |
| 上流の古い記述（P7）の実否 | `inception/domain-design/components.md:4` と `inception/contract-design/contract-summary.md:225-226` に「ジャーナル全再生」を確認 | P7 の扱い（2026-09-05 裁定で上書き済みとし、同期を積み残しに明記）は正確で、黙った読み替えは無い |

### Summary

裁定の反映（Q4 のリードモデル除外、Q4a の型ごと combine / divide、Q5 の `IntentMismatch`、P7〜P10）と現行コードとの差分記述（第 9 節 #1 / #2 / #4）は実測と一致しており、上流の古い再生方式も黙って読み替えず明示して扱っている。判定を NOT-READY にしたのは、FCC 化という今回の中心変更が設計として閉じていない 3 点による: BR5.5 が対象に挙げた配列のうち 4 系統（TransitionSteps、ReviewAttempt の pending / closed、PromotedSections、RuleLines）に型定義が無く（R-01）、追随が必要な呼出側から `core-command-use-case` の生産コードと ITF 準拠テストが漏れており CI 緑の宣言と両立しない（R-02）、StageSlugSet の「文書順」が `StageSlug` の表現力では combine で維持できない（R-03）。いずれも設計本文への追記で閉じられる範囲であり、集約の状態機械・イベント語彙・再生方式そのものに構造的な欠陥は見つからなかった。
