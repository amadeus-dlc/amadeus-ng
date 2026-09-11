<!-- INVARIANT: examples are single-line HTML comments so a fresh template parses to total=0 (MEMORY_EMPTY). Do NOT un-comment or split across lines. t100 guards this. -->
> This file is kept up to date automatically while the stage runs. Add observations at the review step, not by editing here directly.

## Interpretations
<!-- example: 2026-05-29T10:14:32Z — chose REST over GraphQL; the consuming team only needs CRUD, revisit if subscriptions land -->
- 2026-09-10T18:35:00Z — 裁定 jump-redo-guard Q1 = A（モデルを実装へ揃える）の射程を、集約 `jump_execute_guard` が別 scope 無しの跳躍へ課す `jump_plan(None)` = 実効 EXECUTE と読んだ。よって Quint 側のガードは `inScope(cursor)` 一本とし、`synced` と `foreign` を個別に除外する形にはしていない。両方とも「実効計画の外にカーソルがある」という同じ帰結を持つためである。
- 2026-09-10T11:05:00Z — `.claude/settings.json` の `env` / `model` / `effortLevel` 22 行の削除は、利用者の回答「BEDROCKは使いません」と、ファイルの更新時刻（2026-09-10 05:23Z、短いセッションが連続した直後で、どのセッションの監査にも書込みが無い）から、利用者自身の意図した変更と読んだ。同ファイルは同期ツールの保持設定（`PRESERVE`）で、ツールが書き替えたものではない。作業ツリーの内容をそのまま B1 に含め、U2 の実装変更ではなく利用者の設定変更として code-summary に明記する。`CLAUDE_CODE_USE_BEDROCK` は未設定でも既定で無効であり、Bedrock を使わない方針と矛盾しない。
- 2026-09-10T05:55:00Z — 委譲 brief への規則束の逐語貼付けを、`.claude/settings.json` に登録済みの `aidlc-deliver-stage-rules.ts`（Task の PreToolUse で同じ束を逐語で付ける）に委ねた。同じ束が 2 度届くのを避けるためであり、brief には規則束の所在（`implementation-brief.md`）と、承認済み計画・テスト手順の全文、`AIDLC-UNIT` / `AIDLC-TESTING-CONTRACT` の先頭 2 行を載せた。

## Deviations
<!-- example: 2026-05-29T10:14:32Z — skipped the optional caching layer the stage prose suggested; the dataset is small enough that it adds risk -->
- 2026-09-10T13:40:00Z — 前セッション（Runtime Session `9d1d85aa` の前、`27e03be0-…`）が 2 担当（`u2_classic_parity` / `u2_hook_parity`）の作業途中（21:02 頃）で止まり、報告書は未作成のまま作業ツリーに両担当の未完変更が残った。再開でディレクティブが再発行され承認（指紋 `f1d7f6e0…`）が失効したため、計画本文・Testing Contract・テスト手順を無変更のまま指紋 `sha256:b83cf443…` で承認を取り直した。1 回目の `decision` に SessionStart の Runtime Session ではなく別の識別子を渡したため受領が拒否され、同じ質問を 2 回提示した（利用者はいずれも Approve Plan）。再開直後の実測: `cargo check --workspace --all-targets` 成功、`cargo fmt --all` で 5 ファイルを整形、`cargo test --workspace --no-fail-fast` は 3,098 件成功・25 件失敗（全文ログは scratchpad `ws-test-1.log`、後で step9-logs へ写す）。
- 2026-09-10T13:40:00Z — 25 件の失敗が両担当の所有ファイルにまたがり（`directive_drawing` / `turn` / `classic_corpus_contract` と `claude_hook_contract` / `upstream_271_contract` の stop 系 / `task_update_contract` / `next_branches` / `learnings_contract` / `runtime_graph_contract` / `intent_lifecycle`）、並行させると再び同じテストファイルで衝突するため、残作業を 1 担当（`u2_parity_closeout`）へ直列でまとめ直した。
- 2026-09-10T10:40:00Z — 利用量上限で前セッションが止まり、再開でディレクティブが再発行されて計画承認（指紋 `a22159d4…`）が失効した。計画本文・Testing Contract・テスト手順を無変更のまま `[Answer]:` を空へ戻し、指紋 `sha256:f1d7f6e0…` で承認を取り直した（Runtime Session `27e03be0-…`、AskUserQuestion の選択が `HUMAN_TURN` として記録され、`PLAN_APPROVAL_RECORDED` まで通った）。前セッション終盤の 3 担当（classic 是正・フック差是正・差し向け記録）は資料読込の段階で止まっており、作業ツリーへの変更は 1 件も無かった（各 subagent transcript に Edit/Write が無いことを確認）。再開直後の fmt / clippy / `cargo lint` / verify-corpus / aidlc-sync --check はすべて成功。
- 2026-09-10T18:35:00Z — ベースラインの `cargo test --workspace --no-fail-fast` を `tail -120` で捕捉したため、全スイートの件数を採取できていない。exit 0 は確認済みだが、Step 9 の最終記録には全文を残す実行をやり直す必要がある。
- 2026-09-10T05:55:00Z — セッション再開でディレクティブが再発行され計画承認が失効したため、計画本文・Testing Contract・テスト手順を無変更のまま `[Answer]:` を空へ戻し、指紋 `sha256:a22159d4…` で承認を取り直した（Runtime Session `8f466f39-…`）。今回は構造化回答（AskUserQuestion）の選択が `HUMAN_TURN` として記録され、チャットへの直接入力なしで `PLAN_APPROVAL_RECORDED` まで通った。前セッションの「構造化回答では受領へ結ばれない」という記録は現在のフックでは再現しない。
- 2026-09-10T05:55:00Z — Step 7 の未完了 3 スライス（fold-usage 実装、deliver-stage-rules と reviewer-scope の着地記録）と Step 8 の棚卸しを 4 名へ並行委譲した。前セッションの記録（共有署名の変更でコンパイル停止）を踏まえ、`runtime.rs` と `intent_execution_event.rs` の所有を固定し、他担当のファイルへ触れないよう brief に明記した。

## Tradeoffs
<!-- example: 2026-05-29T10:14:32Z — picked TDD over BDD this run; the team is unit-first and the domain is well-understood -->
- 2026-09-10T10:40:00Z — 前セッションの 3 担当分を 2 担当へまとめ直した。フック差の是正（監査値の `<project-dir>` 置換）と差し向け記録の逐語強制は、どちらも `ReviewerScopeBlock` の `Target` / `Stage` / `Unit` と `review_guards_contract.rs` の期待値を書き替えるため、並行させると同じテストファイルで衝突する。Step 8 の classic 是正は別担当のまま、`steering_source.rs`（`bundle` の接頭辞と規則 BOM の両方が触る）も classic 側の所有に寄せた。R7（旧ピン・逸脱台帳を根拠にする注記）は是正、R9（intent 記録内 fixture の `include_str!`）は `tests/golden/selfhost-stage1/` へ移す、と親が判断した。
- 2026-09-10T18:35:00Z — 残る Step 7 のフック接続を 2 担当の並行委譲にした。前セッションで共有署名の変更がコンパイル停止を招いた記録があるため、3 担当以上へは広げず、`run_hook` のフック名一覧を同時に触ることを両者へ明示した。競合の解消は親が行う。
- 2026-09-10T07:20:00Z — 本家との差を「契約 C1 / C2 / C3 で是正が決まるもの」と「契約だけでは向きが決まらないもの」に分け、前者は裁定なしで担当へ是正を出し、後者だけを構造化質問で人間へ回した。全件を裁定に回すと U2 の着地が遠のき、全件を独断で直すと Mandated「上流との不一致を人間へ提示」に反するためである。
- 2026-09-10T07:20:00Z — 2.7.1 コーパスの未駆動ケースは、先頭 4 ケース（前提を probe で再現できたもの）だけを U2 でテストへ固定し、承認・jump・skip を挟む連鎖が要る残りは切替条件 2 の判定材料として一覧で記録する案を推奨し、利用者が選んだ。U2 の着地を優先した判断であり、Step 8「駆動できないままの必要ケースを見直す」を「見直して分類し、判定前に駆動するかを決める」と読んだ。

## Open questions
<!-- example: 2026-05-29T10:14:32Z — confirm the retention window with compliance before the next stage hardens the schema -->
- 2026-09-10T10:50:00Z — 作業ツリーの `.claude/settings.json` は `main` に対して `env`（`CLAUDE_CODE_USE_BEDROCK` 等 17 変数）・`model`・`effortLevel` の 22 行が削除されている。誰がいつ消したかは記録に無く、U2 の実装対象でもない。B1 の PR に含める前に利用者へ提示して裁定を求める（`settings.local.json` へ移した意図的な変更か、担当の誤編集か）。
- 2026-09-10T10:50:00Z — U2 は U1 が申告した経路（`scripts/goldens/capture-cli.ts`、`.github/workflows/ci.yml`、`tests/golden/upstream-a277af21/` の再封印と `reviewer-scope/` 追加、記録用フック 3 本）を U1 完了後に変更している。U1 の終端レビュー受領がこの書込みで失効しているかは、Unit 完了・工程ゲートの時点でエンジンの判定に従い、失効していれば U1 の回復レビューを 1 回起票する。
- 2026-09-10T18:35:00Z — 計画外カーソルへの redo について、本家 2.7.1 の該当観測は未確認のままである。裁定 A は「実装を正とする」判断であって上流観測との突き合わせではない。切替条件 2（状態・監査の upstream 互換）を判定する前に、この 1 点を実観測で確かめるかどうかは未決である。
- 2026-09-10T07:20:00Z — 委任エージェントは `aidlc-state-transition-guard` により本家の `aidlc-orchestrate.ts` も本 build の `aidlc next` も走らせられない（一時ワークスペース対象でも一律拒否）。本家との A/B 実走行は親が主セッションで行うしかなく、今回は probe スクリプトで代行した。ガードの判定に `--project-dir` の宛先を見せるかは配布元の変更であり、本 intent では扱わない。
