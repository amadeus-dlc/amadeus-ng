# b48 設計 — レビュー受領証（`REVIEW_REQUESTED` / `REVIEW_COMPLETED`）を `IntentExecution` のイベントで、`aidlc-log review` 動詞、approve の段 11 ガード（2026-09-04）

対象: #7 キュー 5 の残り（B10 レシート鮮度、#51 = A）。前段: [`b47-single-skeleton/design.md`](../b47-single-skeleton/design.md)。
ピン: upstream `3c3146cf`（`core/tools/aidlc-log.ts` `handleReview` `:648-1180`、`aidlc-lib.ts` `freshReviewReceipts` / `resolveReviewClass` / `terminalReviewVerdict`、`aidlc-state.ts` `verifyReviewerPrecondition` `:1775-2040` と `reviewerPreconditionError` `:2028-2039`、`handleApprove` の前提スタック）。

## 0. 裁定（オーナー、着手前に質問）

| 日付 | 問い | 裁定 |
| --- | --- | --- |
| 2026-09-04（#73 で質問） | 受領証の置き場 | **A**: 集約 `IntentExecution` のイベント。verification の独立クレートは置かず、鮮度の判断は集約に閉じる |
| 2026-09-04（同上） | 鮮度の基準 | **(i) 順序だけ**: 受領証がそのステージの直近の開始（`STAGE_STARTED` 相当）・差し戻し（`GATE_REJECTED` 相当）より後であること。成果物ハッシュ（Artifact / Source Fingerprint）の照合は凍結検査に属し後続 intent へ |
| 2026-09-04（本 Bolt で質問） | 受領証の形 | **A**: 依頼と判定の**対**で取り込む — `request_review` → `ReviewRequested`、`record_review_verdict` → `ReviewCompleted`（イベント 15 変種）。反復の通し番号（ordinal）と上限（budget）の検査も集約が持つ。理由: upstream の承認ガードは「同じ試行の中で依頼 → 判定が対になり、判定が終端」の受領証しか数えない（`freshReviewReceipts` は `pendingRequests` に無い `REVIEW_COMPLETED` を捨てる）ので、判定だけを取り込むと鮮度判定そのものが upstream と食い違う |
| 2026-09-04（同上） | 段 12（practices-discovery の `PRACTICES_AFFIRMED` 受領証 + `practices-promote` 動詞） | **A**: **b49 として分ける**。b48 は B10 の射程（レビュー受領証 + `aidlc-log review` + approve ガード）に閉じる |

用語（初見向け）: **受領証** = 「レビュアーがそのステージを見て READY / NOT-READY を出した」という事実の記録。**試行（attempt）** = そのステージの直近の開始・差し戻し以降の区間。**終端（terminal）判定** = それ以上レビューを回さない判定（READY、または反復上限に達した NOT-READY。advisory は 1 回で終端）。**通し番号（ordinal / iteration）** = 何回目の依頼か。**上限（budget）** = advisory 1 回 / adversarial `reviewer_max_iterations`（既定 2）/ 実効クラス none は 0。

## 1. 原則からの導出

- **コマンド側 = 集約と判断**。受領証は「この実行のどのステージが何回目にどう判定されたか」という**実行の事実**なので `IntentExecution` のイベント。鮮度（試行の区切り）は同じ集約の状態遷移（開始・差し戻し・ジャンプ）が決めるので、鮮度の判断は集約のクエリに閉じる（監査台帳を読み返す upstream の `freshReviewReceipts` は、我々では集約状態の再生に置き換わる）。
- **レビューの方針（誰がレビュアーか・実効クラス・上限・per-unit か）は定義の静的材料**。`WorkflowDefinition` のクエリ `review_policy` が値オブジェクト `ReviewPolicy` を返す（`resolveReviewClass` の min() は定義集約の判断）。`IntentExecution` は intent を ID で参照し材料は引数で受け取る規律どおり、`Option<&ReviewPolicy>` を引数で受ける。
- **approve のガードは FSM のガード**（`approve_gate` の `Err`）。`report_dispatch` には足さない — upstream も `verifyReviewerPrecondition` を遷移ハンドラ（`aidlc-state.ts approve`）の中に置き、report の 13 段より内側で効かせている。
- **RMU は監査 2 行を描くだけ**。状態ファイルの欄もチェックボックスも `read_*` 表も動かさない（upstream の `aidlc-log review` は `emitAudit` しかしない）。クエリ側に消費者は無い（`next` の directive は reviewer 宣言を定義から引く現状のまま）。
- **`aidlc-log review` は新しい面 `Face::Log`**。合成ルートは構文（フラグ文法・値検証・逐語）だけを持ち、判断は集約へ。
- **繰延（記録して進む）**: Artifact / Source Fingerprint（凍結）、stale-receipt recovery（凍結の派生）、`--unit`（per-unit 受領証 — unit ライフサイクルは slice 2）、`--single`（隔離実行の受領証 — 試行の区切りが `Workflow` 別になる）、autonomous swarm 例外（cap / override を無視する code-generation の受領証要求 — slice 2）、unit-major の `STAGE_STARTED` 非フロア（slice 2）。

## 2. ドメイン（`modules/core/command/domain`）

### 2.1 `workflow_definition::ReviewPolicy`（値オブジェクト、新規 `review_policy.rs`）

`{ reviewer: String, effective: ReviewCapValue, max_iterations: u32, per_unit: bool }`。

- `WorkflowDefinition::review_policy(&self, stage: &StageSlug, scope: &str, override: Option<ReviewCapValue>) -> Result<Option<ReviewPolicy>, UnknownStage>`（`UnknownStage` は新規 1 型 1 ファイル。`Err` = 定義がその slug を知らない、`Ok(None)` = reviewer 宣言なし）。
  - declared = `node.review_class()` を `ReviewCapValue` へ（reviewer あり・class 無しは **adversarial** — upstream `stage.review_class ?? "adversarial"`）。
  - effective = min(declared, `scope_metadata(scope).review_cap()`, override)。順位 none 0 < advisory 1 < adversarial 2（`REVIEW_RANK`）。cap / override は**下げるだけ**。
  - `max_iterations = node.reviewer_max_iterations().unwrap_or(2)`、`per_unit = node.for_each() == Some("unit-of-work")`。
- `budget(&self) -> u32`: none → 0、advisory → 1、adversarial → `max_iterations`。
- `requires_receipt(&self) -> bool`: `effective != None`。
- `is_terminal(&self, verdict: ReviewVerdict, iteration: u32) -> bool`（`terminalReviewVerdict` の写し）: none → false、READY → true、NOT-READY → advisory ∨ `iteration >= max_iterations`。

### 2.2 `orchestration::ReviewVerdict`（新規）

`Ready | NotReady`。`parse`（大小無視 — upstream は `toUpperCase()` 後に集合照合）、`as_str` = `READY` / `NOT-READY`。既存の `Verdict`（report の結末）とは別物。

### 2.3 `orchestration::ReviewAttempt`（値オブジェクト、新規）

ステージ 1 つの**現在の試行**: `{ requests: u32, pending: BTreeSet<u32>, closed: Vec<ReviewClosure> }`（`ReviewClosure { iteration: u32, verdict: ReviewVerdict }` は同ファイル内の private 型か、1 型 1 ファイルで別ファイル）。

- `request_count()` / `is_pending(i)` / `pending()` / `closed()`。
- `has_terminal(&self, policy: &ReviewPolicy) -> bool` = `closed` のいずれかが `policy.is_terminal`。upstream で非終端 NOT-READY が受領証を無効化するのは fingerprint が使えるときだけ（`if (verdict !== "NOT-READY" || !fingerprintUsable) continue;`）なので、fingerprint を持たない本 build では非終端判定は**読み飛ばし**（無効化しない）。
- `reset()` → 空。

### 2.4 `IntentExecution` の状態・コマンド・適用

- 状態に `review_attempts: Vec<ReviewAttempt>`（計画と同じ長さ。genesis は全て空）。スナップショット DTO・再構成に載せる（b47 の `skeleton_stance` と同じ経路）。
- **フロア（適用側で試行を空にする）**: `advance_from` で次ステージに立ったとき（`GateApproved` / `StageSkipped` の次ステージ = `STAGE_STARTED` 相当）その 1 ステージ、`GateRejected(s)` の s、`Jumped` は**全ステージ**（upstream は `STAGE_JUMPED` をステージ非依存のフロアにする — 「fail-closed over precise」）。`StageRevised` はフロアでは**ない**（`freshReviewReceipts` の FLOOR は WORKFLOW_STARTED / STAGE_STARTED / STAGE_JUMPED / GATE_REJECTED の 4 つ。practices 側の `STAGE_REVISING` は b49）。`Recomposed` / `Parked` / `Unparked` / `AutonomyModeSet` / `SingleStageRunCommitted` / `SkeletonStanceRecorded` は触らない。
- **`request_review(&mut self, intent: &Intent, stage: &StageSlug, policy: Option<&ReviewPolicy>, reviewer: &str, iteration: u32, retry_pending: bool, occurred_at) -> Result<IntentExecutionEvent, CommandError>`**。ガード順は `handleReview`（`:648-1105`）どおり:
  1. 取り違え → `IntentMismatch`。slug が添字帳に無い → `UnknownStage(String)`（新変種）。
  2. `policy == None` → `NoDeclaredReviewer(StageIndex)`。
  3. `reviewer != policy.reviewer` → `ReviewerMismatch { stage, declared }`。
  4. `retry_pending`: `pending` に `iteration` が無い → `NoPendingReview { stage, iteration }`。あれば `ReviewRequested { retry: true }`（**適用はフレーム空** — `Retry: pending-request` 行は `requestCount` に数えない）。
  5. 通常: `expected = requests + 1`。`iteration > budget` → `ReviewBudgetExceeded { stage, ordinal: iteration, budget }`；`expected > budget` → 同 `{ ordinal: expected }`；`iteration != expected` → `ReviewOutOfSequence { stage, iteration, expected }`。通れば `ReviewRequested { retry: false }`；適用は `requests += 1`、`pending.insert(iteration)`。
  - 本流の状態（Running / Parked / Completed / autonomous）は**見ない** — upstream の `handleReview` に status ガードは無い（`--single` と同じ受理集合）。
- **`record_review_verdict(&mut self, intent, stage, policy, reviewer, iteration, verdict: ReviewVerdict, occurred_at)`**: ガード 1〜3 同じ、4: `pending` に無い → `NoPendingReview`。適用: `pending.remove`、`closed.push`。
- **`approve_gate(&mut self, intent, policy: Option<&ReviewPolicy>, user_input, occurred_at)`**（引数追加）: `require_checkbox` の**後**（upstream も state 検査の後・変更の前）に、`policy.is_some_and(requires_receipt) && !attempt.has_terminal(policy)` → `ReviewReceiptMissing { stage, reviewer }`。
- クエリ: `review_attempt(&self, stage: StageIndex) -> Option<&ReviewAttempt>`（テスト・ITF 準拠・RMU が使う）。
- イベント 2 変種（新規ファイル `intent_execution_event/review_requested.rs` / `review_completed.rs`、いずれも `id` + `aggregate_id` を持つエンティティ）: `ReviewRequested { stage: StageSlug, reviewer: String, iteration: u32, retry: bool }`、`ReviewCompleted { stage, reviewer, iteration, verdict }`。C5 のイベント表を 15 変種へ。
- `CommandError` 新変種 6: `UnknownStage(String)` / `NoDeclaredReviewer(StageIndex)` / `ReviewerMismatch { stage: StageIndex, declared: String }` / `ReviewBudgetExceeded { stage, ordinal: u32, budget: u32 }` / `ReviewOutOfSequence { stage, iteration: u32, expected: u32 }` / `NoPendingReview { stage, iteration: u32 }` / `ReviewReceiptMissing { stage, reviewer: String }`（Display は材料だけ。逐語は app）。

## 3. Quint（`formal/orchestration/engine_loop.qnt`、v2.5）

状態変数（静的 nondet は `init` で選び全アクションで凍結）: `reviewed: int -> bool`（stage 0 は常に false）、`advisory: int -> bool`、`reqCount: int -> int`、`pending: int -> Set[int]`、`terminal: int -> bool`、スナップショット `prevReqCount` / `prevPending` / `prevTerminal`。`pure val BUDGET = 2`、`def budget(s) = if (advisory.get(s)) 1 else BUDGET`、`def attemptEmpty(s) = reqCount.get(s) == 0 and pending.get(s) == Set() and not(terminal.get(s))`。

抽象化の対応（ヘッダに表を書く）: 実効クラス none / reviewer 宣言なし = `reviewed(s) == false`（Rust は `policy == None` か `requires_receipt() == false`。どちらも request は budget 0 で拒否され approve は受領証を要求しない）。判定の値は `terminal` の 1 bit に射影（READY、または advisory / 上限到達の NOT-READY で true）。retry-pending はフレーム空アクション。

アクション:
- `actRequestReview`: nondet s、`reviewed.get(s)`、`reqCount.get(s) + 1 <= budget(s)`；`reqCount' = reqCount.set(s, reqCount.get(s) + 1)`、`pending' = pending.set(s, pending.get(s).union(Set(reqCount.get(s) + 1)))`、他はフレーム、`lastAction' = "request_review"`。
- `actRetryReview`: nondet s、nondet i ∈ `pending.get(s)`；全変数フレーム、`lastAction' = "retry_review"`。
- `actRecordVerdict`: nondet s、nondet i ∈ `pending.get(s)`、nondet ready: Bool；`pending' = pending.set(s, pending.get(s).exclude(Set(i)))`、`terminal' = terminal.set(s, terminal.get(s) or ready or advisory.get(s) or i >= BUDGET)`、`lastAction' = "record_verdict"`。
- フロア: `actReportForward` に**ガード** `(gated(s) and reviewed.get(s)) implies terminal.get(s)` を足し、`nxt != -1` なら nxt の試行を空に（`reqCount' = reqCount.set(nxt, 0)` 等）；`actReportSkipped` も nxt を空に；`actReject` は cursor を空に；`actJumpForward` / `actJumpBackward` / `actJumpRedo` は全ステージを空に；`actRevise` は空に**しない**；他はフレーム。
- `snapshot` に 3 変数を追加。`step` に 3 アクションを追加。

不変条件 4 本（各 1 つの mutation で検出力を証明し §9 に記録する）:

| 名前 | 内容 | 検出する mutation |
| --- | --- | --- |
| `approve_requires_terminal_receipt` | `(lastAction == "report_forward" and gated(prevCursor) and reviewed.get(prevCursor)) implies prevTerminal.get(prevCursor)` | `actReportForward` の受領証ガードを外す |
| `review_attempt_floor` | 差し戻し → cursor が空、ジャンプ 3 種 → 全ステージが空、forward / skipped でカーソルが動いた → 新カーソルが空 | `actReject` のリセットを外す |
| `review_budget` | `STAGES.forall(s => reqCount.get(s) <= budget(s) and pending.get(s).forall(i => i >= 1 and i <= reqCount.get(s)))` | `actRequestReview` の上限ガードを外す |
| `review_frame` | `request_review` / `record_verdict` / `retry_review` は本流の 8 変数を動かさない；`retry_review` は 3 変数も動かさない | `actRetryReview` で `reqCount` を +1 する |

witness 4 本（負形式）: `w_review_requested`、`w_verdict_terminal = lastAction == "record_verdict" and STAGES.exists(s => terminal.get(s))`、`w_approved_reviewed = lastAction == "report_forward" and gated(prevCursor) and reviewed.get(prevCursor)`（ガードの空虚成立を防ぐ）、`w_retry_review`。`scripts/quint-gate.sh` の invariants 列と witness 列に追加。

ITF 準拠（`modules/core/command/domain/tests/engine_loop_conformance.rs`）: `parse_state` に 5 変数、`assert_projection` に `reqCount` / `pending`（ソート済み Vec で比較）/ `terminal`（`policy_of(s)` を `reviewed` / `advisory` から組み `has_terminal` で射影）。駆動: `request_review` は `reqCount` の差分でステージを特定；`record_verdict` は `pending` の差分で (s, i) を特定し、verdict は「`terminal[s]` が false → true に変わったら READY、それ以外は NOT-READY」（既に true なら値はどちらでも射影が一致する）；`retry_review` は `pending` が非空なステージを 1 つ選び `retry_pending = true` で打つ；`report_forward` は `policy_of(cursor)` を `approve_gate` に渡す。合成 `ReviewPolicy` は `reviewer = "r"`、`effective = advisory ? Advisory : Adversarial`、`max_iterations = 2`。フィクスチャは 11 本すべて採り直し、`trace-0x606`（`not(w_approved_reviewed)`）と `trace-0x707`（`not(w_retry_review)`）を追加（採取コマンドは §9 に記録）。

## 4. ユースケース（`modules/core/command/use-case`）

- `CommitVerdictUseCase<E, I, D: WorkflowDefinitionRepository>`（ポート 1 つ追加）。`Approve` 段（`[Approve]` / `[GateStartRecovered, Approve]`）だけ定義を引く: `definition = D.find_by_id(intent.definition_id())`、`override = intent.review().map(ReviewCapValue::parse)`（`--review` は閉集合で受けているので失敗は壊れた歴史 → `CommitError::CorruptReviewOverride`）、`policy = definition.review_policy(stage, intent.scope(), override)`（`Err(UnknownStage)` → `CommitError::UnknownDefinitionStage { stage }`）、`approve_gate(intent, policy.as_ref(), …)`。他の段は定義を読まない（I/O を増やさない）。`CommitError` に `DefinitionRepository(RepositoryError<WorkflowDefinitionId>)` / `UnknownDefinitionStage` / `CorruptReviewOverride` を追加。
- 新規 `RecordReviewUseCase<E, I, D>`（`aidlc-log review`）: 入力 `ReviewLogRequest { stage: StageSlug, reviewer: String, iteration: u32, kind: ReviewLogKind }`、`ReviewLogKind = Request { retry_pending: bool } | Verdict(ReviewVerdict)`。定型 3 手 + 楽観競合 1 回再試行（`RecordSkeletonStanceUseCase` と同型）。結末 `ReviewLogOutcome { emitted: EventType(ReviewRequested | ReviewCompleted), retried: bool }`。`ReviewLogError { Repository, IntentRepository, DefinitionRepository, UnknownStage(StageSlug), Command { stage: StageSlug, error: CommandError } }`。
- `test_support` に定義側の in-memory（既存 `InMemoryWorkflowDefinitionRepository`）で reviewer 付き / cap 付きの定義を組むヘルパを足す。

## 5. RMU（`modules/core/read-model-updater`）

- `projection.rs`: `ReviewRequested` → `## Review Requested` 1 行、フィールド順 `Stage` / `Reviewer` / `Iteration` / （retry のとき）`Retry: pending-request`（upstream `fields` の構築順 `:1103-1108` + `:1175` + `:1186`）。`ReviewCompleted` → `## Review Completed` 1 行、`Stage` / `Reviewer` / `Iteration` / `Verdict`（`:1129-1136`。`Artifact Fingerprint` / `Source Fingerprint` は繰延）。状態ファイル・`Last Updated`・チェックボックス・`read_*` 表は**触らない**。`mod key` に `REVIEWER` / `ITERATION` / `VERDICT` / `RETRY`。
- イベント DTO 2 変種を RMU 側と command interface-adapter 側の両 DTO 集合に追加（`dto_vocabulary` に verdict の綴り、`ReviewRequested` の `retry` は bool）。command 側スナップショット DTO に `review_attempts`（ステージ順の配列 `{ requests, pending: [u32], closed: [{ iteration, verdict }] }`。欄不在の歴史は全て空で読む）。
- `read_execution` 行に列は足さない（消費者なし）。

## 6. app（`modules/app/aidlc`）

- `cli/face.rs`: `Face::Log`（`aidlc-log`）。`cli/request.rs`: `(Face::Log, Some("review"))` → `Request::LogReview(ReviewArgs)`；`decision` / `answer` / `link` → `Request::LogNotWired { verb }`（own wording、b46 の「not wired in this build」層）；未知 → `Request::UnknownLogVerb { given }` → stderr `Unknown subcommand: <sub>. Valid: decision, answer, link, review`（exit 1）。
- `cli/review_args.rs`: `parseFlags`（`:92-115`）の写し — `--single` / `--retry-pending` は値なし真偽、他の `--x` は値必須（`<flag> expects a value, got end of arguments.` / `<flag> expects a value, got another flag: "<val>". Did you forget the value?` は stderr exit 1）。`--project-dir` は `Invocation` が剥がす。
- `runtime::log_review`（順序は `handleReview` どおり）: `Missing --stage <slug>` → `Missing --reviewer <agent>` → `--intent` / `--space` があれば `The review command does not accept --intent/--space selectors. Switch to the target workspace first.` → 実行カーソル不在 `Cannot resolve the active intent for review logging.`（読めない・壊れているは不在と混ぜず `unreadable_execution_cursor`）→ `--unit` / `--single` は not-wired 拒否（own wording）→ 依頼形: `REVIEW_REQUESTED requires --iteration <positive integer>.`；判定形: `--retry-pending cannot be combined with --verdict.` → `REVIEW_COMPLETED requires --iteration <positive integer>.` → `Unknown --verdict "<v>". Accepted: READY, NOT-READY.` → ユースケース → `catch_up` → stdout JSON 1 行 `{"emitted":"REVIEW_REQUESTED","stage":"<slug>"}`（retry は `,"retry":"pending-request"` を追加）/ `{"emitted":"REVIEW_COMPLETED","stage":"<slug>"}`。失敗はすべて **stderr + exit 1**（`Completion::refused` — upstream `error()` は directive を出さない）。upstream の `emitError` が描く `ERROR_LOGGED` 行は本 build では描かない（既存の拒否と同じ扱い、逸脱台帳）。
- 集約の拒否 → 逐語（`wording.rs`）: `NoDeclaredReviewer` / `UnknownStage` → `Cannot record review: stage "<slug>" has no declared reviewer.`；`ReviewerMismatch` → `Cannot record review for "<slug>": reviewer "<r>" does not match the declared reviewer "<declared>".`；`ReviewBudgetExceeded` → `reviewBudgetMessage`（budget 1 の文と それ以外の文の 2 形）；`ReviewOutOfSequence` → `Refusing REVIEW_REQUESTED for "<slug>": iteration <n> is out of sequence; expected <m> from the current audit attempt.`；`NoPendingReview` は動詞で分かれる — 判定形 `Refusing REVIEW_COMPLETED for "<slug>": no unmatched REVIEW_REQUESTED iteration <n> exists in the current audit attempt.`、retry 形 `Refusing review retry for "<slug>": no unmatched REVIEW_REQUESTED iteration <n> exists in the current audit attempt.`。
- `report`: `CommitVerdictUseCase::new(E, I, WorkflowDefinitionRepositoryImpl::open(&store))`。`CommitError::Transition { error: ReviewReceiptMissing { reviewer, .. }, stage, .. }` → `transition_rejected_by("approve", slug, reviewer_precondition(slug, reviewer))`（aidlc-state の stderr 逐語を orchestrate の包み文で包む — b46 の既存形）。`reviewer_precondition` は `:2028-2039` の逐語:
  `Refusing to complete "<slug>": it declares a reviewer (<reviewer>) but no fresh REVIEW_COMPLETED is recorded for it. Invoke the reviewer (stage-protocol-reviewer.md §12a) and record the verdict with \`aidlc-log.ts review --stage <slug> --reviewer <reviewer> --verdict <READY|NOT-READY>\` before completing. Terminal ordering: apply any fixes FIRST, then run the reviewer, record the receipt, and stop editing produces[] artifacts - a later write to one invalidates the receipt and re-opens this refusal. Do not apply suggestions riding on a READY verdict; surface them at the gate instead.`
- `journal_protocol_conformance.rs` / `intent_lifecycle.rs` の網羅 match を 15 変種へ。

## 7. テスト（TDD、層ごとに red → green）

- ドメイン: `ReviewPolicy` の min() 表（declared × cap × override）・`budget` / `is_terminal` 表；`request_review` の拒否 6 形と受理（retry のフレーム空を `==` で固定）；`record_review_verdict`；フロア（forward / skipped / reject / jump 3 種で空になる、revise では残る、`SingleStageRunCommitted` は触らない）；`approve_gate` の受領証ガード（none クラスは要求しない、advisory の NOT-READY 1 回で通る、adversarial の NOT-READY 1 回目は通らず 2 回目で通る、READY は常に通る）；再構成（スナップショット → 再生）で試行が復元される。
- Quint: `scripts/quint-gate.sh` 全緑、mutation 4 件の記録、ITF 13 本の準拠。
- ユースケース: `CommitVerdictUseCase` の定義読込は Approve 段だけ（in-memory の lookup 回数で固定）、受領証欠落の伝播、`RecordReviewUseCase` の 3 手と競合再試行。
- RMU: 監査 2 行のフィールド順（retry 有無・verdict）、状態ファイル不変。
- interface-adapter: DTO 往復 2 変種 + スナップショット `review_attempts`。
- app: パーサ（真偽フラグ・値欠落 2 形）、逐語カタログ、`Workspace` ハーネスで end-to-end — 依頼 → 判定 → `report --result approved` が通る／受領証なし・NOT-READY 1 回目（adversarial）は `Transition rejected by aidlc-state.ts approve for "<slug>": Refusing to complete …`／advisory の NOT-READY で通る／retry-pending／上限超過／順序違反／未知動詞。監査シャードに 2 行が並ぶことも読んで固定。
- ゴールデン `cli/report/approved`（practices-discovery = reviewer なし）は影響を受けない — 回帰で確認。
- カバレッジ相対ゲート（base ≧ 99.11%）を割らない。

## 8. 仕様・記録

- `docs/specs/10-orchestration.md`: B10 行に本裁定（A: 対、b49 分割）を追記；§3 ユースケースに `RecordReview`（`RequestReview` / `RecordReviewVerdict`）；§6 に I18「reviewer 宣言ステージ（実効クラス ≠ none）の承認は現在の試行の終端受領証を要する。試行は開始・差し戻し・ジャンプで区切る」E4 = `engine_loop::{approve_requires_terminal_receipt, review_attempt_floor, review_budget, review_frame}`；§9 に v2.5；§10 に b48 の実装ノート（段 11 のうちレビュアー述語を配線、段 12 は b49）。
- `docs/specs/deviations.md`: fingerprint 2 欄と stale-receipt recovery の繰延、`--unit` / `--single` の not-wired、`ERROR_LOGGED` 非描画。
- `handoff-b48.md`、Issue #7 キュー 5 の本文（b48 完了、**残り = b49**: practices-promote + `PRACTICES_AFFIRMED` + 段 12）。

## 9. 検証記録（2026-09-04 実装 / 2026-09-05 実測、実装は Opus サブエージェント、統合レビューは Fable 5）

- **ドメイン**: `IntentExecutionEvent` を 15 変種へ（`ReviewRequested { stage, reviewer, iteration, retry }` / `ReviewCompleted { stage, reviewer, iteration, verdict }`、いずれも `id` + `aggregate_id` を持つ）。
  `IntentExecution` に `review_attempts: Vec<ReviewAttempt>`（計画と同じ長さ、完全コンストラクタが長さを検査）。コマンド `request_review`（ガード順は `handleReview` どおり: 取り違え → 未知 slug → 宣言なし → 不一致 → retry は判定待ちの存在 / 通常は予算 → 順序。**本流の状態は見ない**。retry の適用は**フレーム空**で通番以外が `==`）と `record_review_verdict`（開いている依頼にだけ対応）。`approve_gate(intent, policy, user_input, at)` の段 11 = `require_review_receipt`（checkbox 前提の**後**・変更の前、`policy.requires_receipt() && !has_terminal` → `ReviewReceiptMissing`）。
  フロア: `advance_from` で立った次ステージ（forward / skipped）、`GateRejected` のステージ、`Jumped` は**全ステージ**。`StageRevised` / `Parked` / `Unparked` / `Recomposed` / `AutonomyModeSet` / `SingleStageRunCommitted` / `SkeletonStanceRecorded` は触らない。
  `WorkflowDefinition::review_policy(slug, scope, override) -> Result<Option<ReviewPolicy>, UnknownStage>`（reviewer あり・class 無しは adversarial、cap / override は**下げるだけ**、未知 scope は上限なし）。`ReviewPolicy::{budget, requires_receipt, is_terminal}` は upstream `:966-968` / `:1810-1812` / `terminalReviewVerdict` の写し。`ReviewAttempt::has_terminal` は非終端 NOT-READY を**読み飛ばす**（fingerprint 不在時の upstream と同じ）。`CommandError` 新変種 7（`UnknownStage` / `NoDeclaredReviewer` / `ReviewerMismatch` / `ReviewBudgetExceeded` / `ReviewOutOfSequence` / `NoPendingReview` / `ReviewReceiptMissing`、Display は材料だけ）。
- **DTO**: 2 変種の永続化 DTO（command 側 / RMU 側の両集合）、`dto_vocabulary` に verdict の行の綴り（`Ready` / `NotReady` — 監査面の `READY` / `NOT-READY` とは別面）、スナップショット行に `review_attempts`（`{ requests, pending: [u32], closed: [{ iteration, verdict }] }`、欄不在は全ステージ空で読む）。ワイヤ形式 15 変種を両側のゴールデンコーパスで固定。
- **ユースケース**: `CommitVerdictUseCase<E, I, D>`（定義ポートを追加。**Approve 段だけ**が定義を読む — in-memory の lookup 回数で固定。`CommitError::{DefinitionRepository, UnknownDefinitionStage, CorruptReviewOverride}`）。新規 `RecordReviewUseCase<E, I, D>`（find → 定義 → コマンド → store、`Conflict` 1 回再試行）、入力 VO `ReviewLogRequest` / `ReviewLogKind`、結末 `ReviewLogOutcome`、封筒 `ReviewLogError`。
- **RMU**: `ReviewRequested` → `## Review Requested` 1 行（`Stage` / `Reviewer` / `Iteration` / [`Retry: pending-request`]）、`ReviewCompleted` → `## Review Completed` 1 行（`Stage` / `Reviewer` / `Iteration` / `Verdict`）。状態ファイル・`Last Updated`・チェックボックス・`read_*` 表は不変。計画を引かないので計画外 slug でも描ける。`read_tables/spelling.rs::jump_refusal` に 7 変種の綴り（閉集合の網羅）。
- **app**: `Face::Log`（`aidlc-log`）。`review` → `RecordReviewUseCase`、`decision` / `answer` / `link` → not-wired 拒否（own wording）、未知 → `Unknown subcommand: <sub>. Valid: decision, answer, link, review`。`cli/review_args.rs` は `parseFlags` の写し（真偽 2 フラグ、値欠落 2 形の逐語）。`runtime::log_review` の順序: フラグ文法 → `--stage` → `--reviewer` → セレクタ拒否 → 実行カーソル（不在 / 読めないを混ぜない）→ `--unit` / `--single` の not-wired 拒否 → 依頼形 `--iteration` / 判定形 `--retry-pending` 併用 → `--iteration` → `--verdict` 閉集合 → 記録 → `catch_up` → stdout JSON 1 行。失敗はすべて stderr + exit 1、`ERROR_LOGGED` は描かない。`report` の段 11 拒否は `Transition rejected by aidlc-state.ts approve for "<slug>": Refusing to complete "<slug>": it declares a reviewer (<r>) but no fresh REVIEW_COMPLETED is recorded for it. …`（`reviewerPreconditionError` `:2026-2037` 逐語）。
- **Quint v2.5**: 状態変数 5 本（`reviewed` / `advisory` は init で選んで凍結、`reqCount` / `pending` / `terminal`）+ スナップショット 3 本、アクション 3 本（`actRequestReview` / `actRetryReview` / `actRecordVerdict`）、`actReportForward` の受領証ガード、フロア（forward / skipped の新カーソル、reject のカーソル、jump 3 種の全ステージ）、不変条件 4 本（16 本へ）、witness 4 本。
  **mutation 検査 4/4（統合レビューで再実行して再確認、対照の無変異は `[ok]`）**: `actReportForward` の受領証ガードを外す → `approve_requires_terminal_receipt`；`actReject` のリセットを外す → `review_attempt_floor`；`actRequestReview` の上限ガードを外す → `review_budget`；`actRetryReview` で `reqCount` を +1 → `review_frame`。
  ITF 準拠: `parse_state` に 5 変数、`assert_projection` に `reqCount` / `pending` / `terminal`（`policy_of` で合成した方針の `has_terminal`）、駆動 3 アクション（`request_review` は `reqCount` の差分、`record_verdict` は `pending` の差分 + `terminal` の遷移で verdict を決める、`retry_review` は判定待ちの最小番号）。フィクスチャ 13 本（既存 11 本を再採取 + `trace-0x606` / `trace-0x707`）、アクション網羅 22。
- **テスト**: 新規 **108 本**（既存ファイルへ 71 + 新規ファイルに 37。`#[test]` / `#[tokio::test]` の増分）。ドメインの受領証ガード表 `the_approval_receipt_guard_follows_the_effective_class`、フロア 6 本、`the_request_guards_reject_in_the_upstream_order`、`Workspace` ハーネスの end-to-end 13 本（一巡 → `done` + 監査 2 行、受領証なしの拒否逐語、adversarial の上限到達、advisory の 1 パス、`review_cap: none`、retry、拒否 4 形、構文段 12 形、差し戻し後の積み直し）。
- `cargo fmt --all --check` / `cargo clippy --workspace --all-targets -- -D warnings` / `cargo lint` / `cargo test --workspace` — **全緑（49 スイート、1,974 本）**。`tools/lint` 自己テスト 69 本も緑
- `scripts/quint-gate.sh` — **全 PASS（26 ステップ。engine_loop の不変条件 16 本、witness 7 本）**
- `scripts/coverage.sh --base origin/main` — 絶対 **99.12% ≥ 90.0%**、相対 **99.125% ≥ 99.117 − 0.01 PASS**

### ITF フィクスチャの採取コマンド（`M=formal/orchestration/engine_loop.qnt`、`D=tests/conformance/fixtures/engine_loop`）

b47 §9 の 11 本は**同じコマンド**で再採取し、新規 2 本を足した。統合レビューで 13 本すべてを再採取し、`#meta` を除いてバイト一致することを確認した。

```
for s in 0xa1 0xb2 0xc3 0xd4 0xe5 0xf6 0x202; do quint run $M --seed $s --max-samples 1 --max-steps 40 --out-itf $D/trace-$s.itf.json; done
quint run $M --seed 0x101 --max-samples 2000 --max-steps 40 --invariant 'not(lastAction == "report_revised")' --out-itf $D/trace-0x101.itf.json
quint run $M --seed 0x303 --max-samples 2000 --max-steps 40 --invariant 'not(w_repark)'            --out-itf $D/trace-0x303.itf.json
quint run $M --seed 0x404 --max-samples 2000 --max-steps 40 --invariant 'not(w_single_run)'        --out-itf $D/trace-0x404.itf.json
quint run $M --seed 0x505 --max-samples 2000 --max-steps 40 --invariant 'not(w_stance_recorded)'   --out-itf $D/trace-0x505.itf.json
quint run $M --seed 0x606 --max-samples 2000 --max-steps 40 --invariant 'not(w_approved_reviewed)' --out-itf $D/trace-0x606.itf.json
quint run $M --seed 0x707 --max-samples 2000 --max-steps 40 --invariant 'not(w_retry_review)'      --out-itf $D/trace-0x707.itf.json
```

状態数: 素の 7 本 = 41、0x101 = 14、0x303 = 10、0x404 = 4、0x505 = 12、0x606 / 0x707 = 34（状態変数が増えたぶん、同じ seed でも b47 とは経路が変わっている）。

### 設計との差分（実装レビューで受け入れたもの）

1. `ReviewCapValue::rank` / `weaker` を新設 — 派生 `Ord` は宣言順（`Adversarial < Advisory < None`）で強度順の**逆**なので、設計 §2.1 の「`min`」はそのまま書けない。low-wins は `weaker` で書き、型 doc とテストで `min` を禁じた。
2. `ReviewAttempt::restored(requests, pending, closed)` — DTO 復号専用の再構成口（壊れた行は検査せずクラッシュする規律のまま）。
3. app の `review_log_input` は依頼形・判定形の**両方**で `--iteration` を検査してから `(u32, ReviewLogKind)` を返し、slug の文法検査より**先**に置く（upstream `:983-985` / `:1124-1134` が `loadContext` より前 — team-lead レビューで修正）。文法外の slug は upstream の `find` 空振りと同じ「宣言が無い」逐語。
4. `positive_iteration` は `u32` に収まらない値を飽和させる（JS の `Number()` は巨大値のまま予算超過で断られるので答えは同じ）。
5. RMU の `jump_refusal` 綴り表に 7 変種を足した（`jump_resolve` からは返らないが、閉集合の網羅 match を丸めない）。
6. 仕様 B10 の旧記述（`record_review_receipt` → `ReviewReceiptRecorded`）は対の裁定で `request_review` / `record_review_verdict` に置き換わった — B10 行に追記済み、旧記述は裁定の経緯として残す。
7. 統合レビューで直した点: `WorkflowDefinition::review_policy` の doc が「`min` で書ける」と書いたまま `weaker` を使っていた矛盾を訂正（コード変更なし）。
