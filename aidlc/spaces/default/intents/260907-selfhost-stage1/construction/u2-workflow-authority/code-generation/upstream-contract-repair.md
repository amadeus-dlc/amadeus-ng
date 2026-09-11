# 公開契約テストの修復記録 — `modules/app/aidlc/tests/upstream_271_contract.rs`

- 作業日: 2026-09-09
- 担当: aidlc-developer-agent（Unit `u2-workflow-authority`、Code Generation Step 4 以降）
- 正本: 本家 2.7.1 固定コミット `a277af218f0df7f325d3b8be7b6d90fce2c5bd40`（`git -C vendor/aidlc-workflows show a277af21:<path>` で読んだバイト。行番号はその版のもの）
- 判断規則: 本家の実装と一致する側を残す。テストの期待が本家と食い違えばテストを直し、実装が食い違えば実装を直す。実装に合わせて期待を書き換えるだけの緩和はしない。
- 対象: `cargo test -p aidlc --test upstream_271_contract` の失敗 21 件（8 種の直接失敗 + 共通 fixture `workspace_at_code_generation()` 経由の 13 件）

## 再現（red）

`cargo test -p aidlc --test upstream_271_contract --no-fail-fast` の結果は 54 成功 / 21 失敗（ログ: scratchpad `run1-upstream271.log`）。失敗は次の 3 群に分類できた。

| 群 | 内容 | 件数 |
| --- | --- | --- |
| A | `aidlc-log answer` 拒否時の stderr 形式と監査追記の期待違い | 4 |
| B | `report --result awaiting-approval` が reverse-engineering の pipeline 受領証不足で業務拒否されていた（終了 0 の `{"kind":"error"}`）のを、テストが終了コードしか見ておらず隠れていた | 4 + fixture 13 |
| C | fixture が requirements-analysis のレビュー要求に必要な `produces` 文書を書いていなかった（B を直した後に露出） | fixture 内 |

fixture の実際の stdout は、fixture へ一時的に `eprintln!` を入れて `--nocapture` で採取した（ログ: `run2-repro.log`）。

```text
{"kind":"error","message":"Cannot present \"reverse-engineering\" for approval because these pipeline handoffs have not been recorded for the current run: aidlc-developer-agent, aidlc-architect-agent. After each link returns, run `bun .claude/tools/aidlc-log.ts link --stage reverse-engineering --link <agent>`. Set AIDLC_DISABLE_ENSEMBLE_EVIDENCE=1 only to recover a legitimately-run in-flight pipeline."}
```

## 各件の原因・直した側・本家の根拠

### 群 A — 拒否の出力形式と監査追記（直した側: テスト）

| テスト | 原因 | 本家の根拠 |
| --- | --- | --- |
| `a_dismissed_widget_is_not_an_answer_and_does_not_consume_presence` | stderr を平文で期待していた。実装は `{"error":"..."}` を出す | `core/tools/aidlc-log.ts:574-580` は `error()` を呼ぶ。`error()`（`:2313-2317`）は `emitError` へ委譲し、`core/tools/aidlc-lib.ts:22208-22246` の末尾 `console.error(JSON.stringify({ error: msg })); process.exit(1)` 以外の出力経路は無い |
| `a_summary_question_requires_a_human_response_after_its_prompt` | 同上 | `aidlc-log.ts:693-697` → `error()` → `emitError` |
| `an_answer_consumes_one_recorded_human_response_across_processes` | 同上 | `aidlc-log.ts:792-796` → `error()` → `emitError` |
| `a_handwritten_human_audit_entry_cannot_authorize_an_answer` | 拒否後に監査が不変である前提で全文比較していた。本家は状態ファイルがあれば拒否のたびに `ERROR_LOGGED` を監査へ追記する | `aidlc-lib.ts:22219-22245`（`existsSync(stateFilePath(projectDir))` なら `appendAuditEntry("ERROR_LOGGED", { Tool, Command, Error })`）。実装側の対応は `modules/app/aidlc/src/runtime/log_failure.rs::finish` |

変更内容:

- 補助関数 `upstream_error(message)` を追加し、期待値を `JSON.stringify({ error: msg }) + "\n"` の形にした（serde_json の compact 出力は ASCII 文言では JavaScript と同一）。
- 手書き監査のテストは「手書き行を含む元の内容が接頭辞として残り、末尾に `ERROR_LOGGED` がちょうど 1 件（`Tool: aidlc-log`、`Error: Cannot record this answer because no new human reply ...`）加わり、`QUESTION_ANSWERED` は現れない」へ改めた。状態ファイル不変の確認は維持。

### 群 B — pipeline 受領証の前提（直した側: テスト。実装は本家どおり）

`reverse-engineering` は配布グラフで `mode: pipeline`（lead `aidlc-developer-agent`、support `aidlc-architect-agent`）である。本家はゲートを開く前に当該試行のリンク受領証 `PIPELINE_LINK_COMPLETED` を全リンク分要求し、欠ければ `Cannot present "<slug>" for approval because these pipeline handoffs have not been recorded for the current run: ...` を返す。

- 根拠: `core/tools/aidlc-orchestrate.ts:7198-7227`（`checkPipelineLinkEvidence`。`mode !== "pipeline"` か `AIDLC_DISABLE_ENSEMBLE_EVIDENCE=1` のときだけ免除）、`core/tools/aidlc-state.ts:3787-3814`（`verifyPipelineLinkPrecondition`）。
- developer リンクは handoff 成果物 `--artifact <record>/inception/reverse-engineering/developer-scan.md` を要し、当該試行開始以降に書かれた通常ファイルでなければならない: `aidlc-log.ts:895-979`。support リンクは lead の受領証の後にのみ記録できる（順序）: `aidlc-log.ts:877-893`。
- 実装側は `modules/app/aidlc/src/runtime/pipeline_link.rs`（未追跡の新規ファイル）と `report` の `with_pipeline_observation` が同じ文言で拒否していた。実装は本家と一致しているのでテスト側を直した。

| テスト | 変更 |
| --- | --- |
| `an_approval_answer_without_a_human_reply_has_the_upstream_refusal` | `report` の前に `complete_reverse_engineering_pipeline()` を呼ぶ。開いた後に状態が `- [?] reverse-engineering` であることを積極確認（業務拒否が再び隠れないため）。期待 stderr は群 A と同じく JSON 形へ |
| `an_approval_choice_is_acknowledged_without_consuming_the_human_response` | 同上。ゲートが開くと集約は `GateOpened` 適用時にその stage の未回答質問を消す（`intent_execution.rs:2430`）ので、ゲート前に記録した決定は本家 `hasPendingDecisionAtGate`（`aidlc-log.ts:351-403`: 直近 `STAGE_AWAITING_APPROVAL` より後の `DECISION_RECORDED` だけを数える）と同じく無視され、`{"skipped":"QUESTION_ANSWERED",...,"reason":"approval-gate-report-owned"}` になる |
| `stop_allows_an_open_human_gate_without_starting_a_no_progress_streak` | 同上。ゲートが開けば集約の `continuation_wait()` が `GateOrRevision` を返し、Stop は無音で許可される（本家 `core/hooks/aidlc-continue-workflow.ts:440-470` `isHumanWaitStop`、`:1442-1450` の carve-out）。待機は RMU が block-count.json を書かない（`workflow_continuation_read_model_updater.rs` の `no_write`） |
| `different_scopes_preserve_report_results_through_another_intents_publication_failure` | 最初の intent（bugfix）で同じ前提を追加。ゲートが開けば `read_report_result` に行が生まれ、`QueryReturnedNoRows` は消える。読取スキーマ側の変更は不要だった |
| fixture `workspace_at_code_generation()`（13 テスト） | 現在 stage が reverse-engineering のとき同じ前提を満たす。あわせて `report` の stdout に `"kind":"error"` が含まれないことを検査し、終了コードだけでは見えない業務拒否を音を立てて落とすようにした |

補助関数 `Workspace::complete_reverse_engineering_pipeline()` は `<record>/inception/reverse-engineering/developer-scan.md` を書き、`aidlc-log link --stage reverse-engineering --link aidlc-developer-agent --artifact <相対パス>` → `--link aidlc-architect-agent` の順で記録し、各出力が `{"emitted":"PIPELINE_LINK_COMPLETED",...}` であることを確認する。`AIDLC_DISABLE_ENSEMBLE_EVIDENCE=1` の免除は使わない（本家が「正当に走った in-flight pipeline の回復にだけ使う」と限定しているため）。

### 群 C — レビュー要求の `produces` 前提（直した側: テスト fixture）

fixture は requirements-analysis で `review`（要求）→ `review --verdict READY`（完了）を打つが、本家のレビュー要求は stage の `produces` 全文書（`requirements` / `requirements-analysis-questions`）の実在を要し、欠ければ `Cannot start review for "<slug>": a required output document is missing or unreadable. ...` で拒否する（`aidlc-log.ts:1990-2001`）。完了は要求時に束ねたバイトの後ろへ加えた `## Review` 付録だけを許す（`:2135-2160`）。

- 変更: `Workspace::write_requirements_documents(reviewed)` を追加。要求前に 2 文書を書き、完了前に `requirements.md` の末尾へ `## Review`（Reviewer / Verdict READY / Iteration 1 / Findings None.）を加える。内容はゴールデン `tests/golden/upstream-a277af21/stage1/cases.json` の `review/request` / `review/completed` の `initial_files` と同じ。

### 実装側の変更

なし。`modules/app/aidlc/src` の 8 種すべてについて、実装の出力・拒否・遷移は本家と一致していた。`modules/core/read-model-updater` も変更していない。

## 検証（green）

- `cargo test -p aidlc --test upstream_271_contract`（対象 9 件の先行実行 `run3-targeted.log` / fixture 再実行 `run4-fixture.log`）: 群 A・B の 8 件成功、fixture 1 件成功。
- 全体の結果は末尾の「最終結果」を参照。

## 申し送り

- `cargo fmt --all --check` は本担当外の `modules/app/aidlc/tests/journal_protocol_conformance.rs` と `modules/app/aidlc/tests/jump_contract.rs` で差分を報告する。親が並行編集中のため本担当は触っていない（`rustfmt` を当てるだけで解消する差分）。
- `modules/app/aidlc/tests/upstream_271_contract.rs` と `modules/app/aidlc/src/runtime/pipeline_link.rs` は git 未追跡（`??`）の新規ファイルである。コミット時に追加漏れに注意。
- `upstream_271_contract.rs` 内で `report --result awaiting-approval` を打つ箇所は 6 か所で、いずれも今回の対象に含まれていた。他のテストに同種の隠れた業務拒否は無い。
