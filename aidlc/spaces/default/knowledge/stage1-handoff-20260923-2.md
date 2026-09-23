# stage-1 引き継ぎ その 2（2026-09-23 夕方）

前の引き継ぎ（`stage1-handoff-20260923.md`）の後のセッションの終了時点。native が配布 2.8.2 と同じ手順列で bugfix 1 周を完走するところまで直し、PR 6 本のスタックにした。**いまはそのスタックを main へ順にマージしている途中**である。切替そのものの手順は `stage1-switch-runbook-20260923.md` にある。

---

## 1. いま進行中のもの（最初に確認すること）

### 1-1. #142 はマージキューに入っている

2026-09-23 夕方に #142 をマージキューへ投入した（`QUEUED`, position 1）。キューの CI は 20 分前後かかる。

```bash
gh pr view 142 --json state,mergedAt --jq .
```

- `MERGED` なら §2 の手順で #143 へ進む
- キューから外れていたら、落ちたジョブを確認する。coverage が Issue #134 のテスト（`pipeline_link_contract::concurrent_duplicate_completions_persist_only_one_receipt`）で落ちたなら、既知の flake なので再投入でよい。`check` が「Text file busy」（ETXTBSY）で落ちたのも既存の flake である

### 1-2. #143〜#145 の独立レビューは結果を回収できていない

マージ前の自己点検として、#143〜#145 の差分をレビュー担当のエージェントに読ませた。ただし結果を受け取る前にセッションを閉じた。**新しいセッションでやり直すこと**（観点は正しさ・パス脱出・ガードの抜け道・状態の戻し忘れ・DTO の復号・2.8.2 との挙動差）。

### 1-3. 作業場所

- worktree: `.claude/worktrees/stage1`（`mise trust` 済み）。スタックのブランチはすべてここで作った
- 本体の作業ツリー（`docs-stage1-handoff-followup` ブランチ）の `.claude/settings.json` は、ユーザーがフックを外した状態のまま触っていない。コミットしないこと
- 砂場や一時ファイルは scratchpad に置いた。残す必要のあるものは無い

---

## 2. スタックのマージ手順

| PR | ブランチ | base | 内容 |
|---|---|---|---|
| [#142](https://github.com/amadeus-dlc/amadeus-ng/pull/142) | `feat/selfhost-e2e-replay` | main | 再生ハーネス（キュー投入済み） |
| [#143](https://github.com/amadeus-dlc/amadeus-ng/pull/143) | `feat/selfhost-review-and-approval-2-8-2` | #142 | レビュー受領の 2.8.2 形・runtime-graph・承認と差し戻しの人間の返答ガード |
| [#144](https://github.com/amadeus-dlc/amadeus-ng/pull/144) | `feat/selfhost-plan-approval-2-8-2` | #143 | 計画承認の 2 行タグ・run-stage 指示の文脈・要約確認ガード |
| [#145](https://github.com/amadeus-dlc/amadeus-ng/pull/145) | `feat/selfhost-native-plan-approval-guard` | #144 | plan-approval-guard の native 化 |
| [#146](https://github.com/amadeus-dlc/amadeus-ng/pull/146) | `feat/selfhost-replay-pretool-guards` | #145 | 再生に PreToolUse / Stop フックを追加 |
| [#147](https://github.com/amadeus-dlc/amadeus-ng/pull/147) | `docs-stage1-switch-runbook` | #146 | 切替の手順書（このファイルもここに入る） |

注意点:

- **base が main 以外の PR には CI が走らない**（CodeRabbit も走らない）。#143 以降は、下がマージされて base を main へ付け替えた時点で初めて CI が走る
- main は squash マージなので、下の PR の元コミットは main に入らない。そのまま base を付け替えると差分に下の PR の分が残る。**付け替えの前に main へ載せ直す**:

```bash
git fetch origin
# 例: #142 がマージされた後の #143
git rebase --onto origin/main feat/selfhost-e2e-replay feat/selfhost-review-and-approval-2-8-2
git push --force-with-lease origin feat/selfhost-review-and-approval-2-8-2
gh pr edit 143 --base main
```

- 上の PR はさらにその上で `git rebase --onto <載せ直した下のブランチ> <古い下のブランチの先端> <上のブランチ>` の要領で積み直す（または下がマージされるたびに同じ手順）
- マージは GraphQL の `enqueuePullRequest` で入れる（`gh pr merge` は auto-merge 無効で弾かれる。前の引き継ぎの §4-4）
- `delete_branch_on_merge` は無効なので、マージ後のブランチは残る

---

## 3. 到達点（数値）

- 再生ハーネス: native・配布 2.8.2 とも **102/102**。run-stage 指示の差分 0。session-start の文脈と deliver-stage-rules の配送内容も一致
- `target/release/aidlc --doctor`（#146 の先端、クリーンな clone）: **28 passed / 0 failed**
- #145 のブランチで `PROPTEST_RNG_SEED=20260823 cargo test --workspace --no-fail-fast`: 4025 passed / 0 failed。clippy・`cargo lint`・fmt も通過

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

---

## 5. 上流側で観測したこと

- 配布 2.8.2 の plan-approval-guard は、ライフサイクルの報告も承認の対象にする。code-generation の承認待ちを開いた後は自分の active directive を失い、`report --result approved` まで止める。配布 2.8.2 のまま実地で回すと、CG の承認で詰まる可能性がある。native では起きない
- 配布 2.8.2 でも、誕生から最初の `report` までは runtime compile が発火しない。そのため reverse-engineering の `learnings surface` は失敗する（儀式は advisory なので続行してよい）

---

## 6. マージ後にやること

1. main の先端で `doctor_pass` と `all_ci_jobs_pass` の証拠を採り、`host-binding.json` へ書く（手順書 §2-1・§2-2）
2. 実地スモーク（人間の操作が必要）。手順書 §2-3〜§2-6 のとおりにする: `mise.local.toml` の `_.path` で `aidlc` を native に向け、フック登録を戻し（run-sensors だけ配布 TS）、完全に再起動し、Issue #134 を題材に `/aidlc bugfix`
3. その後の候補: Unit を切る scope への対応（plan-approval-guard の Unit 判定、Issue #137 の 6 入口）、2.9.0 への追従調査
