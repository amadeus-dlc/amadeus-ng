# ハンドオフ — b48: レビュー受領証（`REVIEW_REQUESTED` / `REVIEW_COMPLETED`）を `IntentExecution` のイベントで、`aidlc-log review` 動詞、approve の段 11 ガード（2026-09-05）

対象: #7 キュー 5 の残り（B10 レシート鮮度、#51 = A）。設計書: [`b48-review-receipts/design.md`](b48-review-receipts/design.md)。
前段: [`handoff-b47.md`](handoff-b47.md)。オーナー裁定（着手前に質問、設計 §0）: 受領証の置き場は `IntentExecution` のイベント（A）、
鮮度の基準は**順序だけ**（i）、受領証の形は依頼と判定の**対**（A）、段 12（practices-discovery の `PRACTICES_AFFIRMED` + `practices-promote`）は **b49 に分ける**（A）。

用語（初見向け）: **受領証** = 「レビュアーがそのステージを見て READY / NOT-READY を出した」事実の記録。**試行** = そのステージの直近の開始・差し戻し・ジャンプ以降の区間。
**終端判定** = それ以上レビューを回さない判定（READY、または反復上限に達した NOT-READY。advisory は 1 回で終端）。**予算** = advisory 1 回 / adversarial `reviewer_max_iterations`（既定 2）/ 実効 none は 0。

## やったこと
- **ドメイン**: `IntentExecutionEvent` を 15 変種へ（`ReviewRequested { stage, reviewer, iteration, retry }` / `ReviewCompleted { stage, reviewer, iteration, verdict }`）。
  `IntentExecution` に `review_attempts: Vec<ReviewAttempt>`（`requests` / `pending` / `closed`）。コマンド `request_review`（ガード順は upstream `handleReview` どおり、
  本流の状態は見ない、retry の適用はフレーム空）と `record_review_verdict`（開いている依頼にだけ対応）。`approve_gate` に `policy: Option<&ReviewPolicy>` を足し、
  段 11 = checkbox 前提の後・変更の前で `requires_receipt() && !has_terminal()` → `ReviewReceiptMissing`。フロア（試行を空へ戻す位置）は「前進 / 読み飛ばしで立った
  次ステージ」「差し戻しのステージ」「ジャンプは全ステージ」（upstream `freshReviewReceipts` の FLOOR 4 種の写し。`StageRevised` はフロアではない）。
  `WorkflowDefinition::review_policy(slug, scope, override) -> Result<Option<ReviewPolicy>, UnknownStage>`（low-wins は `ReviewCapValue::weaker`）。
- **DTO**: 2 変種 + スナップショット行の `review_attempts`（欄不在は全ステージ空）。行の面の verdict 綴りは `Ready` / `NotReady`（監査面の `READY` / `NOT-READY` とは別面）。
- **ユースケース**: `CommitVerdictUseCase<E, I, D>`（定義ポート追加、**Approve 段だけ**が読む）、新規 `RecordReviewUseCase<E, I, D>`（`Conflict` 1 回再試行）。
- **RMU**: 監査 2 行だけ（`REVIEW_REQUESTED`: Stage / Reviewer / Iteration / [Retry: pending-request]、`REVIEW_COMPLETED`: Stage / Reviewer / Iteration / Verdict）。
  状態ファイル・`read_*` 表・`Last Updated` は不変。
- **app**: 新しい面 `aidlc-log`（`Face::Log`）。`review` の構文段は `parseFlags` の写し + upstream 逐語 10 形、集約の拒否 6 形の逐語（`NoPendingReview` は判定形 / retry 形で言い回しが分かれる）、
  成功は stdout JSON 1 行（`{"emitted":"REVIEW_REQUESTED","stage":"<slug>"[,"retry":"pending-request"]}` / `{"emitted":"REVIEW_COMPLETED",…}`）、失敗はすべて stderr + exit 1。
  `decision` / `answer` / `link` は not-wired 拒否（own wording）。`report` の段 11 拒否は `aidlc-state.ts approve` の `reviewerPreconditionError` 逐語を包み文に入れる。
- **Quint v2.5**: 状態変数 5 本 + スナップショット 3 本、アクション 3 本、`actReportForward` の受領証ガード、フロア、不変条件 4 本（16 本へ）+ witness 4 本。
  mutation 4/4（設計 §9）。ITF フィクスチャ 13 本（既存 11 本を同じコマンドで再採取 + `trace-0x606`（`not(w_approved_reviewed)`）/ `trace-0x707`（`not(w_retry_review)`）、採取コマンドは設計 §9）。
- **仕様**: `docs/specs/10-orchestration.md` B10 行に対の裁定と b49 分割、§3 に `RecordReview`、§6 に I18（E4 = 4 不変条件）、§9 に v2.5、§10 に実装ノート。`docs/specs/deviations.md` #5（fingerprint 2 欄・stale-receipt recovery・`--unit` / `--single`・swarm 例外・unit-major・`ERROR_LOGGED` の繰延）。
- **テスト**: 新規 108 本。全ゲート緑（49 スイート 1,974 本、Quint 26 ステップ PASS、カバレッジ 99.12%）。

- **設計との差分 7 点**（設計 §9「設計との差分」）: `ReviewCapValue::weaker`（派生 `Ord` が強度順の逆）、`ReviewAttempt::restored`、`--iteration` 検査を slug 文法より先に、
  `positive_iteration` の飽和、`jump_refusal` 綴り表の網羅、仕様 B10 旧記述の位置づけ、`review_policy` doc の訂正。

## 積み残し（記録のみ、起票しない）
- **b49（裁定済み）**: 段 12 — practices-discovery の `PRACTICES_AFFIRMED` 受領証と `practices-promote` 動詞（`STAGE_REVISING` のフロアも含む）。#7 キュー 5 の残り。
- **繰延（deviations #5）**: `Artifact Fingerprint` / `Source Fingerprint` と stale-receipt recovery（凍結検査の後続 intent、#51 = A）、`--unit`（per-unit 受領証 — unit ライフサイクルは slice 2）、
  `--single`（`Workflow` 別の試行）、autonomous swarm 例外（slice 2）、unit-major の `STAGE_STARTED` 非フロア、`aidlc-log` 失敗時の `ERROR_LOGGED` 行（クリティカルパス 5）。
- 段 11 のうち completion-evidence は slice 2、turn-shape marker は CP5（変わらず）。
- `aidlc-log` の `decision` / `answer` / `link` は not-wired（配線はクリティカルパス 4 のディスパッチャ配線と合わせて）。

## 次
b49（段 12）→ #7 キュー 6 以降（#72 set-autonomy → #71 WorkspaceScanner → …）。
