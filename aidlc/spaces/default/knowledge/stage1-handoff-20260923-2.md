# stage-1 引き継ぎ その 2（2026-09-23 夕方 〜 2026-09-24）

前の引き継ぎ（`stage1-handoff-20260923.md`）の後の 2 セッション分の記録。native が配布 2.8.2 と同じ手順列で bugfix 1 周を完走するところまで直し、PR 6 本のスタックにした。独立レビューで見つかった問題を直してから、順に main へマージした。**この文書が main に入った時点で、スタックはすべてマージ済み**である。切替そのものの手順は `stage1-switch-runbook-20260923.md` にある。

---

## 1. いまの状態（最初に確認すること）

### 1-1. スタックはマージ済み

| PR | 内容 | main のコミット |
|---|---|---|
| [#142](https://github.com/amadeus-dlc/amadeus-ng/pull/142) | 再生ハーネス | `a5bddd79` |
| [#143](https://github.com/amadeus-dlc/amadeus-ng/pull/143) | レビュー受領の 2.8.2 形・runtime-graph・承認と差し戻しの人間の返答ガード | `36c1ca5b` |
| [#144](https://github.com/amadeus-dlc/amadeus-ng/pull/144) | 計画承認の 2 行タグ・run-stage 指示の文脈・要約確認ガード | `1c7641a5` |
| [#145](https://github.com/amadeus-dlc/amadeus-ng/pull/145) | plan-approval-guard の native 化 | `681dc4d2` |
| [#146](https://github.com/amadeus-dlc/amadeus-ng/pull/146) | 再生に PreToolUse / Stop フックを追加 | `7247a05f` |
| [#147](https://github.com/amadeus-dlc/amadeus-ng/pull/147) | 切替の手順書と、この文書 | — |

#147 のコミットは、この文書を書いた時点ではまだ決まっていない。`git log --oneline origin/main` で確かめること。

### 1-2. 独立レビューの結果（マージ前に直したもの）

#143〜#145 の差分をマージ前に独立レビューにかけ、blocker 2 件を含めて直した。

- #143: `REVIEW_COMPLETED` をコミットした後で所見表を解析していた。表が壊れていると「CLI は失敗と言うが判定は確定済み、記録は無く、打ち直しは拒否」になっていた（blocker）。所見表の解析をドメイン（`ReviewFinding` / `ReviewFindings`）へ移し、記録前に拒否する
- #143: autonomous なら全フェーズで承認・差し戻しのガードが外れていた。2.8.2 と同じく Construction に限った。差し戻しの返答を Request Changes の選択と照合し、監査シャードが読めないときは拒否側に倒す
- #144: 指紋は射影した計画を束ねるのに、作業ブリーフは原文を渡していた。承認後に付録へ足した手順や印を書き換えた手順が、指紋を変えないまま作業者へ届いていた（blocker）。ブリーフも射影した計画を渡す
- #144: 要約確認で "Request changes" でも確認済みになっていた。レビュー階級 `none` でも指示にレビュー欄が出ていた。定義グラフが読めないと要約確認ガードが黙って外れていた
- #145: 経路に symlink があっても外と判定しなかった。`sed -i … src/*.rs` や `echo x > $F` が承認前に通っていた。契約の印が `sha256:` 以外の値も数えていた

マージ直前の CodeRabbit の指摘で、さらに次を直した。

- #144: `memory.md` を書き切れなかったとき、途中までのファイルが残って二度と作り直されなかった。失敗したら消す
- #145: `PlanApprovalState` の欄が `pub(super)` で、ガードが欄を直接読んでいた。欄は private にし、`is_current` / `approves_contract` / `refusal_reason` のクエリで渡す
- #146: 再生の `pretool` が終了コード 2 しか失敗に数えていなかった。また `printf … | write` はサブシェルで動き、その中で数えた失敗が合計に出ていなかった。どちらも直し、native・配布 2.8.2 とも 102/102 のままであることを確かめた

**直さずに後続へ回したものは Issue [#148](https://github.com/amadeus-dlc/amadeus-ng/issues/148) にまとめてある**（拒否文言の包み文、Unit を切る scope、レビュー方針の出どころの一本化など）。

### 1-3. 作業場所

- worktree: `.claude/worktrees/stage1`（スタックを作った場所）。レビュー修正は `.claude/worktrees/{pr143,pr144,pr145}` で行った。どれも `mise trust` 済みで、マージ後は消してよい
- 本体の作業ツリー（`docs-stage1-handoff-followup` ブランチ = PR [#141](https://github.com/amadeus-dlc/amadeus-ng/pull/141)）の `.claude/settings.json` は、ユーザーがフックを外した状態のまま触っていない。コミットしないこと
- #141（前の引き継ぎメモの追記）はマージできる状態で残っている。マージするかはオーナーの判断待ち

---

## 2. スタックのマージで踏んだこと

- main は squash マージなので、下の PR がマージされたら、上の PR を**載せ直してから** base を付け替える: `git rebase --onto origin/main <古い下の先端> <上のブランチ>`。古い先端は、載せ直す前に控えておく
- **base の付け替えだけでは CI が起動しない**（`pull_request` の `edited` は CI の起動条件に無い）。先に `gh pr edit N --base main` で付け替えてから push する。push 済みなら PR を閉じて開き直す
- CodeRabbit のスレッドを解決する前に走った `CI Review Thread Gate` は失敗のまま残る。CI の run が終わってから `gh run rerun <run-id> --failed` で再実行する
- マージは GraphQL の `enqueuePullRequest`（`gh pr merge` は auto-merge 無効で弾かれる）。キューの CI は 20 分前後
- push が `Permission denied (publickey)` で落ちることが 1 度あった。再試行で通った
- `delete_branch_on_merge` は無効なので、マージ後のブランチは残る

---

## 3. 到達点（数値）

- 再生ハーネス: native・配布 2.8.2 とも **102/102**。run-stage 指示の差分 0。session-start の文脈と deliver-stage-rules の配送内容も一致
- 再生の数値は、独立レビューの修正を入れた後のスタック最上段でも同じだった
- `target/release/aidlc --doctor`（レビュー修正前の #146 の先端、クリーンな clone）: **28 passed / 0 failed**。main の先端で取り直すこと（§6）
- スタック最上段（レビュー修正後）で `PROPTEST_RNG_SEED=20260823 cargo test --workspace --no-fail-fast`: 4057 passed / 1 failed。clippy・`cargo lint`・fmt も通過
- **macOS のローカルでは、子プロセスが signal 9（出力なし）で終わってテストが落ちることがある。** 落ちるテストは毎回違い、単独では通る。複数の worktree で同時にテストを回すと頻度が上がる。CI（Linux）では観測していない（#148 に記録）

再生の回し方:

```bash
cargo build --release -p aidlc
scripts/aidlc-selfhost/e2e/replay-bugfix.sh --engine ~/.local/share/aidlc/versions/2.8.2/aidlc --out <dir>/dist
scripts/aidlc-selfhost/e2e/replay-bugfix.sh --engine target/release/aidlc --out <dir>/native
scripts/aidlc-selfhost/e2e/compare-directives.sh <dir>/dist <dir>/native
```

---

## 4. 暫定裁定（オーナーの確認待ち）

ユーザーから「裁定は推奨で暫定決定し、PR でフィードバックする」と指示を受けて決めたもの。詳細は各 PR 本文にある。

- 追従先は 2.8.2 のまま（2.9.0 は stage-1 の後）
- レビューの試行 ID は「試行の最初の依頼の Request Id」の指紋（2.8.2 は監査行の床の指紋）。`--review-file` と監査行の `Review Record` 欄は未対応
- 計画承認の指紋は `sha256:<hex>` のまま（`v3` を名乗らない）。native は発行エポックとソース床も束ねるため。射影は「進捗の印」と「末尾の Review 節」だけを消す
- Change Control は未実装
- plan-approval-guard は Unit を切らない scope（bugfix など）だけ正しく判定する。書込先を特定できないシェルと未知の工具は通す（2.8.2 は止める）。監査行 `PLAN_APPROVAL_BLOCKED` は書かない
- 要約確認ガードは、`upstream_271_contract` の 2 fixture でだけ `AIDLC_SKIP_SUMMARY_CONFIRMATION_GUARD=1` で外した
- 承認が付随するだけの試験は `tests/support/human_reply.rs` で `HUMAN_TURN` を代役として書き、`Approve` で答える
- 差し戻しのフィードバックは 2.8.2 の report と同じく `--reason` を優先し、保護されたゲートでは `--user-input` をフィードバックにしない
- UTF-8 でない監査シャードは置換文字で読む（2.8.2 の `toString("utf-8")` と同じ）
- 定義グラフが読めないときの報告は error 指示（exit 0）で止める（2.8.2 は stderr と exit 1）
- plan-approval-guard は、変更系コマンドやリダイレクトの書込み位置が展開・グロブで確定しなければ止める（2.8.2 と同じ）。`<記録>/link/../x` のようにリンクを `..` で戻る綴りは、2.8.2 と同じく「中」と判定する
- 承認前の `/dev/null` へのリダイレクトは止める（2.8.2 と同じ）

---

## 5. 上流側で観測したこと

- 配布 2.8.2 の plan-approval-guard は、ライフサイクルの報告も承認の対象にする。code-generation の承認待ちを開いた後は自分の active directive を失い、`report --result approved` まで止める。配布 2.8.2 のまま実地で回すと、CG の承認で詰まる可能性がある。native では起きない
- 配布 2.8.2 でも、誕生から最初の `report` までは runtime compile が発火しない。そのため reverse-engineering の `learnings surface` は失敗する（儀式は advisory なので続行してよい）

---

## 6. マージ後にやること

1. main の先端で `doctor_pass` と `all_ci_jobs_pass` の証拠を採り、`host-binding.json` へ書く（手順書 §2-1・§2-2）
2. 実地スモーク（人間の操作が必要）。手順書 §2-3〜§2-6 のとおりにする: `mise.local.toml` の `_.path` で `aidlc` を native に向け、フック登録を戻し（run-sensors だけ配布 TS）、完全に再起動し、Issue #134 を題材に `/aidlc bugfix`
3. その後の候補: Issue #148 の後続、Unit を切る scope への対応（plan-approval-guard の Unit 判定、Issue #137 の 6 入口）、2.9.0 への追従調査
