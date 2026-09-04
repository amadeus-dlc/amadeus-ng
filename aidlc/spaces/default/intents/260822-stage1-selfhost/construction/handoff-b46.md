# ハンドオフ — b46: report の 13 段ガード連鎖（本流）— `report_dispatch` を集約のクエリへ、逐語 3c3146cf 準拠、完了投影の是正（2026-09-04）

対象: GitHub #73（#7 キュー 5）の本流部分 + b45 の所見（完了がリード面へ投影されない）。設計書: [`b46-report-guards/design.md`](b46-report-guards/design.md)。
前段: [`handoff-b45.md`](handoff-b45.md)。

## やったこと
- **集約 `IntentExecution::report_dispatch`（クエリ、`&self`）**: 仕様 §2.3 の `report_dispatch` を独立ドメインサービスではなく
  集約のクエリに置いた（`next_decision` と同じ形）。判定順はピン `handleReport`（`:5545-5860`）と同順 — 対象の解決（明示
  `--stage` かカーソル）→ `skipped` の受理 5 条件（全 completion ガードより先）→ gate 系 3 語の前提 → 段 13 human presence →
  forward 表（`[S]`/`[R]` 拒否・`[ ]` 拒否・`[x]` の no-op 2 形・`[-]` は明示 `--stage` 必須で `gate-start --recovered + approve`・
  `[?]` は `approve`）。新設の値: `ReportRequest`（観測 5 つ）、`ReportDecision`（`Commit { stage, steps }` / `NoOp`）、
  `TransitionStep`（8 段と upstream の綴り）、`ReportNoOp`（3 形）、`ReportRefusal`（材料付き 13 形 — 設計の 12 + `RoutedVerdict`）。
  `CheckboxState::spelling()` を正本にして RMU の綴り表もそこへ委譲。`Verdict::is_gate_lifecycle()` を追加。
- **ユースケース `CommitVerdictUseCase`**: find → `report_dispatch` → 決まった段を打つ → store。`CommitOutcome`（`Committed { stage, scope, steps }` /
  `NoOp`）を返し、`CommitError` は `Repository` / `IntentRepository` / `Refused(ReportRefusal)` / `Transition { step, stage, error }` /
  `UnwiredTransition`（`advance` / `complete-workflow` — b42 で撤去済み、縮退計画でのみ到達）。`ReportedTransition` と
  `is_stale_re_report` / `gate_is_already_open` は `report_dispatch` に吸収して削除（後方互換なし）。`Conflict` 1 回再試行は維持。
- **クエリ側**: `StateFileDao`（record の状態ファイルを生テキストで引く DAO。不在 `Ok(None)`、0 バイト `Ok(Some(""))`）+
  `FindStateFileUseCase` + `StateFileDaoImpl`。段 1 の state-version guard の材料。
- **domain `StateVersionClassification`**: `version()` を追加し、一致判定を**綴りの一致**へ是正（upstream `v === CURRENT_STATE_VERSION` —
  数値に畳むと `008` が `ok` になり、upstream が `past` として拒む状態を通してしまう）。
- **RMU の完了投影（b45 の所見の是正）**: `leave_for(None)` の素の `WORKFLOW_COMPLETED` 1 行を `complete_workflow` に置換 —
  状態 7 欄（`Completed` 数え直し / `Status: Completed` / `Last Updated` / `In Progress: none` / `Next Stage: none` /
  `Next Action: Workflow complete` / 最終フェーズ `Verified`）と監査 3 行（`PHASE_COMPLETED` `To phase: (end)` →
  `PHASE_VERIFIED` `<phase> → end` → `WORKFLOW_COMPLETED` `Scope` + `Details`）。経路（承認 / 読み飛ばし）で `Details` / `Reason` が
  違うので型 `Completion` で運ぶ。`STAGE_COMPLETED` は承認経路が既に書いているので再 emit しない（upstream `:2498` と同じ）。
- **app**: `report` を 13 段ラダーへ再構成 — 段 1（state-version、0 バイトも「在る」）、段 2 `--single` と段 3 `--skeleton-stance` は
  構文検証と逐語（本体は b47。「not wired in this build」の `error` directive で止め、本流は絶対に進めない = I10）、段 4 resume
  ルーティング（数字 1〜4 の正規化、redo は逸脱台帳 #1 の写像で `aidlc-jump execute ...`）、段 5〜7・9 の構文半分、段 13 の env。
  成功 `Committed <subs joined by " + "> for "<slug>" (scope: <scope>)` / `Committed skip ...` / `Recorded <result> for "<slug>".`（print）、
  no-op 3 形、拒否 13 形、`Transition rejected by aidlc-state.ts <sub> for "<slug>": <detail>` を `wording.rs` に逐語で（40 本超）。
  現行の「reported <raw>」「Transition rejected: …」「report requires --result <outcome>.」「No workflow execution to report against.」は
  すべて逐語へ置換。
- **テスト**: 新規 77 本（`#[test]` / `#[tokio::test]` の増分）。domain の表テスト `the_dispatch_table_pins_every_verdict_against_every_checkbox`
  （6 checkbox × 4 verdict = 24 組合せ）+ 軸別（gated=false / 計画外の名指し / 明示 `--stage` / final ∧ Completed / moved-on の有無 /
  段 13 と抜け道 2 つ / skipped 5 条件 / Resume / 非ゲート 3 形 / 構成不能の記録）、use-case の `CommitOutcome` 3 形と `Refused` /
  `Transition` / `UnwiredTransition`、クエリ側 DAO 4 本、RMU の完了投影 2 経路、app 結合の段 1〜13 逐語（state-version 3 形 × 3 経路、
  `--single` 4 形、`--skeleton-stance`、resume 4 経路 + 拒否、skipped 5 条件、gate 3 系、human-presence、forward の no-op と拒否、
  `Committed ... + ...`）。各レイヤーで赤を先に確認（domain: `ReportRequest` 未存在でコンパイルエラー / use-case: 新 API で先に
  書き替え / RMU: `Field not found: "Status"` で赤 / app: 旧逐語の 8 本が赤）。

- **設計との差分 12 点**（`design.md` §9「設計との差分」）: 綴り一致の state-version、`ReportDecision` が slug、拒否 13 形、`UnwiredTransition` の創作逐語、`StateFileDao::find`、ポート失敗の中継形、env 抜け道は集約側で固定、`catch_up` をガード前に、`Last Updated` は完了時のみ、`[S]` skip 腕は到達不能、`NoOp` の二重 stage、完了投影の監査 3 行。
- **ゴールデン**: 5 ケースは slug 1 語の置換でバイト一致（`the_report_directives_match_the_recorded_cases_after_the_slug_substitution`）。`completed-ungated` は構成不能、`approved-across-phases` は合成グラフに対応位置が無く `approved` と同型のためそちらが覆う。

## 積み残し（記録のみ、起票しない）
- **b47**: `--single`（`SingleStageRun` — synthetic-id pair、遷移ポート非注入 = I10）と `--skeleton-stance`（新コマンド・イベント・skeleton-gate アンカー）。b46 は構文検証と逐語だけを持ち、本体は「not wired in this build」で止めている。
- **b48（裁定済み）**: B10 のレシート鮮度（#51 = A）と段 12 の practices-discovery 受領証。オーナー裁定 2026-09-04（b46 中に質問）: **(A) 受領証は集約 `IntentExecution` のイベント**として取り込む（`record_review_receipt` → `ReviewReceiptRecorded`、監査行は RMU が投影、鮮度の判断は集約のクエリに閉じる）、**(i) 鮮度は順序だけ**（受領証が直近の開始・差し戻しより後。成果物ハッシュの照合は凍結検査として後続 intent）。受領証を書く動詞（`aidlc-audit append` 相当）の配線もこの Bolt に含める。
- 段 11（completion-evidence）は slice 2 の Construction 実行機構と一緒に。段 1 の turn-shape marker は Stop フック（クリティカルパス 5）で置き場ごと決める。
- **到達不能の 2 段**: `advance` / `complete-workflow` に対応する集約コマンドは無い（b42、#85 = A）。初期化ステージだけが in-scope の
  縮退計画でのみ `report_dispatch` がこれらを名指しし、ユースケースは `CommitError::UnwiredTransition` で断る（逐語は
  `Transition rejected by aidlc-state.ts <sub> for "<slug>": ... not wired in this build`）。
- **`STAGE_COMPLETED` の `Final stage <Name> completed` 行**は report からは到達しない（承認経路は `approve` が既に書き、読み飛ばし
  経路は `STAGE_SKIPPED` のみ）。`complete-workflow` を直に叩く経路がこの build に無いため。
- **`.coderabbit.yaml` のパースエラー**（PR #101 で CodeRabbit が警告。b46 対象外、設定の整備は別途）。

## 次
b47（`--single` / `--skeleton-stance`）→ #7 キュー 6 以降（#72 set-autonomy → #71 WorkspaceScanner → …）。
