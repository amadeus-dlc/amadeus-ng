# 継続トークン拒否 3 経路の実装記録（u2_continuation_rejection）

対象は承認済み実装計画 Step 4 の残り「継続トークンを不透明な入力として渡し、古い/不正/別対象のトークンを拒否する」である。
本家 2.7.1 が `continue` に対して持つ 4 種類の拒否のうち、未実装だった 3 種類を実装した。

## 1. 実装した変更

### 1.1 逐語 3 本（`modules/app/aidlc/src/wording.rs`）

既存の `INVALID_CONTINUATION_TOKEN` と同じ節へ 3 定数を追加した。いずれも本家とバイト一致である。

| 定数 | 本家の結論 | 本家の行 |
| --- | --- | --- |
| `CONTINUATION_TOKEN_SUPERSEDED` | `superseded` | `aidlc-orchestrate.ts:8488` |
| `CONTINUATION_CONTEXT_CHANGED` | `drift` | `aidlc-orchestrate.ts:8489` |
| `CONTINUATION_COORDINATION_BUSY` | `ActiveDirectiveLockContendedError` | `aidlc-orchestrate.ts:8493` 付近 |

### 1.2 カーソル照合（新規 `modules/app/aidlc/src/runtime/continuation_cursor.rs`）

本家は `handleContinue` → `advanceContinuationCursor` の 1 トランザクションの中でこの 3 つを出す。
それを `Verdict`（`Current` / `Superseded` / `ContextChanged` / `Busy`）へ写した。

判定は本家の順序をそのまま保つ。

1. 入口で採った `Snapshot` と publish 直前の実測が違えば `ContextChanged`。
2. marker がいまの文脈そのもの（本家 `exactContext`）で、かつ `kind != "load-steering"` または
   `continue_token_sha256 != sha256(提示トークン)` なら `Superseded`。
3. それ以外は `Current`。

本家 `exactContext` の 4 面（`project_sha256` / `intent_uuid` / `state_present` / `state_sha256`）と
`version == 2` をそのまま比較している（`aidlc-lib.ts:5546-5551` の実バイト）。

marker が不在・壊れている・v1・別文脈のときは `Superseded` にしない。本家が
それらを「1 度だけの復旧継続」として通すからである（CHANGELOG 2.6.51:
`Missing, malformed, oversized, v1, cross-harness, and pre-state bare-space markers permit one natively validated recovery continuation under the same transaction; a mismatched token is stale, not a bootstrap opportunity.`）。

判定の中核 `verdict()` は I/O を持たない純粋関数にし、ワークスペースの実測は `inspect()` に閉じた。

### 1.3 結線（`modules/app/aidlc/src/runtime.rs`）

`resume`（`continue` の合成ルート）に 2 点を足した。

- 継続を組み立てる**前**に `Snapshot::capture` で作業文脈を控える（drift の突き合わせ相手）。
- 組み立て済みの directive を出す**前**に `inspect` を通し、拒否ならその逐語の
  `Directive::Error` を返して `publish_directive` を呼ばない。

publish を呼ばないことは `Busy` の逐語が負う約束そのものである
（`This call did not commit a cursor change.`）。テストで marker のバイト不変を固定した。

## 2. Red の再現

### 2.1 Red 1 — 消費済みトークンの再提示（本来の red-first）

```
cargo test -p aidlc --test steering_across_processes a_reused_continuation_token_is_refused
```

生ログ: [continuation-rejection-logs/red-01-reused-token.log](continuation-rejection-logs/red-01-reused-token.log)

```
thread 'a_reused_continuation_token_is_refused' panicked at modules/app/aidlc/tests/steering_across_processes.rs:326:5:
assertion `left == right` failed
  left: "load-steering"
 right: "error"
```

コンパイルは成功したうえでの失敗である。**現行 build は同じトークンの再提示に対して後続の部を
もう一度返していた**。親が本日 TypeScript エンジンで踏んだ差（8488 の文言）と同じ現象を、
このリポジトリ側から実測で固定できた。

### 2.2 Red 2 — ロック競合（mutation で確認した red）

この経路は**実装が先になった**。red-first を守れていないので、テストに力があることを
mutation で確かめた。`inspect` のロック取得失敗を `Verdict::Busy` から `Verdict::Current` へ
差し替えて実行した。

生ログ: [continuation-rejection-logs/red-02-contention-mutation.log](continuation-rejection-logs/red-02-contention-mutation.log)

```
assertion `left == right` failed
  left: "exclusive file lock timed out"
 right: "Continuation coordination is busy. This call did not commit a cursor change. Retry the current token; if it is reported superseded, run a fresh `next`."
```

この出力は副産物として重要な事実を示す。**本変更の前は、競合時に基盤の生文字列
`exclusive file lock timed out` が漏れていた**（`publish_directive` → `PlanApprovalAccess::open` の
ロック取得失敗）。本家の逐語ではなく、境界でのエラー文言変換もされていなかった。

### 2.3 Red 3 — 準備中の文脈変化（mutation で確認した red）

こちらも実装が先である。`verdict` の drift 分岐を無効化して実行した。

生ログ: [continuation-rejection-logs/red-03-drift-mutation.log](continuation-rejection-logs/red-03-drift-mutation.log)

```
test ..._a_state_that_disappeared_while_preparing_is_context_changed ... FAILED
test ..._an_intent_that_moved_while_preparing_is_context_changed ... FAILED
test ..._a_state_that_moved_while_preparing_is_context_changed ... FAILED
  left: Current
 right: ContextChanged
test result: FAILED. 8 passed; 3 failed
```

superseded 系のテストは mutation 下でも通っており、判定表が経路ごとに分離できていることも示す。

## 3. Green の一覧

| テスト | 置き場 | 固定した内容 |
| --- | --- | --- |
| `a_reused_continuation_token_is_refused` | `tests/steering_across_processes.rs` | 1 回目の `continue` は通り、同じトークンの再提示は superseded の逐語で拒否される |
| `a_contended_cursor_refuses_without_moving_it` | `tests/steering_across_processes.rs` | 競合時に busy の逐語を返し、marker が 1 バイトも動かない |
| `the_current_token_is_current` | `src/runtime/continuation_cursor.rs` | 現行トークンは通る |
| `a_token_that_is_no_longer_current_is_superseded` | 同上 | 後続が発行済みなら前のトークンは superseded |
| `a_settled_run_stage_supersedes_the_token` | 同上 | `kind` が `run-stage` に着地していれば superseded |
| `a_state_that_moved_while_preparing_is_context_changed` | 同上 | 状態が動けば drift |
| `an_intent_that_moved_while_preparing_is_context_changed` | 同上 | intent が入れ替われば drift |
| `a_state_that_disappeared_while_preparing_is_context_changed` | 同上 | 状態が消えれば drift |
| `a_missing_marker_permits_one_recovery_continuation` | 同上 | marker 不在は復旧継続として通す |
| `a_marker_from_another_context_is_not_superseded` | 同上 | 別文脈の marker は superseded にしない |
| `a_v1_marker_is_not_an_exact_context` | 同上 | v1 marker は照合対象外 |
| `every_refusal_carries_its_upstream_wording` | 同上 | 3 逐語のバイト一致 |
| `a_malformed_marker_is_not_parsed` | 同上 | 壊れた marker は照合対象外 |

既存の `steering_across_processes.rs` 7 件（`the_steering_chain_survives_across_invocations` を含む）は
そのまま通っている。1 回目の `continue` は現行トークンなので影響を受けない。

生ログ: [continuation-rejection-logs/green-01-reused-token.log](continuation-rejection-logs/green-01-reused-token.log)、
[continuation-rejection-logs/green-full-workspace.log](continuation-rejection-logs/green-full-workspace.log)

## 4. 本家との突き合わせ

### 4.1 確認できたこと

固定コミット `a277af218f0df7f325d3b8be7b6d90fce2c5bd40` の実バイトを読んだ。

```
git -C vendor/aidlc-workflows diff --stat a277af21... HEAD -- dist/claude/.claude/tools/aidlc-orchestrate.ts
```

差分なし。**vendor の作業ツリーは固定コミットとバイト一致**であり、以下の行番号はその
コミットのものとして読んでよい。

- `aidlc-orchestrate.ts:8360-8496` — `handleContinue` の全体。
- `aidlc-lib.ts:5352-5620` — `advanceContinuationCursor`。3 結論の出どころ。
- `aidlc-lib.ts:5546-5551` — `exactContext` の 5 条件。
- `CHANGELOG.md:597` — 2.6.51。この振る舞いを導入した版。

CHANGELOG 2.6.51 の逐語で本実装が根拠にした文:

- `presenting the same continuation token twice now returns an error directive ("This continuation token is no longer current for this workflow. Run a fresh `next`; do not reuse an earlier token.") instead of repeating the successor.`
- `Concurrent uses of one token have exactly one winner on every harness; recovery is always a fresh `next`.`
- `The exactly-one-winner guarantee requires the marker and lock paths on one local filesystem (atomic rename, exclusive directory creation, coherent cross-process visibility); NFS/SMB/FUSE-style shares are unsupported`

### 4.2 確認できなかったこと — 本家を走らせられなかった

**本家エンジンの実走による A/B 比較は行えなかった。** 使い捨ての一時ワークスペースへ
`dist/claude` を複写して実行しようとしたが、プロジェクトのフックが拒否した。

```
bun .claude/tools/aidlc-orchestrate.ts next --project-dir .
```

```
PreToolUse:Bash hook error: [bun ".claude/hooks/aidlc-state-transition-guard.ts"]:
Delegated agent "u2_continuation_rejection" cannot run aidlc-orchestrate.ts next because only
the main workflow session can change stage status or routing.
```

このガードは `.claude/hooks/aidlc-state-transition-guard.ts:912-921` でスクリプト名
（`aidlc-orchestrate.ts`）と動詞（`next` / `continue` / `report` / `park`）だけを見ており、
`--project-dir` の指す先を見ない。したがって本件のように**本リポジトリと無関係な一時
ワークスペースを対象にした実行も一律で拒否される**。

複写したスクリプトを別名にすればパターン照合は外れるが、それはガードの回避であり行わなかった。
`shell-write-targets-verification.md` が採った「本家を実際に走らせて差分比較する」手本は、
この単位では再現できていない。**本実装の根拠は実バイトの読解と CHANGELOG の逐語であり、
実走による出力比較ではない。**

なお親は本日、消費済みトークンの再提示に対して TypeScript エンジンが 8488 の文言を返すことを
自ら観測している（依頼原文に記載）。本記録はそれを引用するが、私自身の観測ではない。

### 4.3 同時提示の勝者が 1 つであること — 実装したが検証していない

CHANGELOG の `Concurrent uses of one token have exactly one winner on every harness` について。

実装上、カーソル照合は `aidlc/.aidlc-runtime.lock` の排他ロックの中で行うので、
2 つの `continue` が同時に来ても marker の読取と判定は直列化される。負けた側は
ロック待ちの後に marker の `continue_token_sha256` が入れ替わっているのを見て
`Superseded` になる。

ただし**この「勝者が 1 つ」を実測するテストは書いていない**。理由は 2 つある。

1. 現在の `resume` は照合と publish が別のロック取得に分かれている（照合は
   `continuation_cursor::inspect`、publish は `PlanApprovalAccess::open`）。本家のように
   1 つのトランザクションで「照合して後続を publish する」までを閉じてはいない。
   したがって照合通過から publish までの間に別呼び出しが割り込む窓が理論上残る。
2. その窓を踏む決定的なテストは、同一プロセス内の 2 スレッド競合になり、
   本質的に不安定な検査になる。

**「勝者が 1 つ」は現時点で保証できていない。** 完全な保証には照合と publish を 1 つの
排他区間へ畳む改修が要る。これは本スライスの範囲外として残課題に挙げる。

## 5. 範囲外とした分岐とその理由

`handleContinue` の 8460-8496 には `legacy-plan-approval-*` の 4 分岐がある。
いずれも**今回の範囲外**とした。

| 分岐 | 判断 |
| --- | --- |
| `legacy-plan-approval-owned` | 範囲外 |
| `legacy-plan-approval-recovery-required` | 範囲外 |
| `legacy-plan-approval-reissued` | 範囲外 |
| `legacy-plan-approval-transport` | 範囲外 |

理由は実バイトで確定できる。4 分岐はいずれも `advanceContinuationCursor` の
`legacyPlanApprovalSession` が渡ったときにだけ到達する。その値を作る
`attachLegacyKiroPlanApprovalChoices` は冒頭で早期 return する
（`aidlc-orchestrate.ts:467-473`）。

```js
if (
  !projectDir ||
  prepared.marker?.stage !== "code-generation" ||
  installedHarnessName(projectDir) !== "kiro-ide"
) {
  return { prepared };
}
```

`installedHarnessName(projectDir) !== "kiro-ide"` が条件である。本 build が名乗るハーネスは
`claude` である（`modules/app/aidlc/src/turn.rs` の `DEFINITION_ID`、および marker の
`cursor_harness` に `claude` を書く `active_directive.rs:22`）。したがって**この 4 分岐は
Claude ハーネスでは到達不能**であり、依頼原文の除外「Kiro 専用や自律実行の未接続分岐」に
そのまま該当する。

## 6. 検査の結果

| 検査 | 結果 |
| --- | --- |
| `cargo fmt --all --check`（自分のファイル） | 差分なし |
| `cargo clippy -p aidlc --all-targets --no-deps`（自分のファイル） | 指摘なし |
| `cargo test -p aidlc --lib continuation_cursor` | 11 件成功・0 失敗 |
| `cargo test -p aidlc --test steering_across_processes` | 9 件成功・0 失敗 |
| `cargo test --workspace` | 別記（下記 7.3） |

`cargo fmt --all --check` と `cargo clippy --workspace --all-targets -- -D warnings` は
**workspace 全体では現在赤い**。ただし指摘は全て他担当が作業中のファイルである。

- `modules/core/command/domain/src/orchestration/mod.rs`、`reviewer_scope_paths.rs`、`reviewer_scope.rs`
- `modules/harness/claude/src/stage_rule_bundle.rs`、`reviewer_scope_envelope.rs`
- `modules/harness/infrastructure/src/reviewer_scope_segments.rs`
- `modules/core/command/domain/src/orchestration/exempt_paths.rs`、`inspected_command.rs`、
  `inspected_command_step.rs`、`reviewer_scope_candidates.rs`

これらは `u2_reviewer_scope` と `u2_stage_rules` の担当範囲であり、**私は触っていない**。
`cargo clippy --workspace` は `harness-claude` のコンパイル段で止まるため、私の crate まで
到達しない。そのため `-p aidlc --no-deps` で自分のファイルだけを検査した。

閾値の緩和は行っていない。カバレッジ床 90.0% と相対ゲート `head >= base - 0.01` に手を触れていない。

## 7. 残課題

### 7.1 照合と publish を 1 つの排他区間へ畳む

4.3 のとおり「同時提示の勝者が 1 つ」は保証できていない。本家は
`transactActiveDirectiveTarget` の中で照合と後続 marker の書込みを閉じている。
本 build は照合（`continuation_cursor::inspect`）と publish（`PlanApprovalAccess::publish`）で
ロックを 2 回取る。裁定と改修方針の指示を求める。

### 7.2 本家実走による A/B 比較

4.2 のとおり `aidlc-state-transition-guard.ts` により実行できなかった。
ガードがスクリプト名と動詞だけで判定し `--project-dir` を見ないことが原因である。
一時ワークスペース対象の実行を許すかどうかは人間の裁定事項として残す。
私はガードの回避を行っていない。

### 7.3 workspace 全体の通し実行

`cargo test --workspace` は他担当の作業中コードを含むため、基準線（97 スイート・
2,921 件成功・0 失敗）との比較は同時作業が収束してから行う必要がある。
本記録の作成時点の実行結果は
[continuation-rejection-logs/green-full-workspace.log](continuation-rejection-logs/green-full-workspace.log) に置いた。

### 7.4 TDD の順序を守れなかった箇所

Red 1 は red-first を守った。Red 2・Red 3 は**実装が先**になり、事後に mutation で
テストの力を確かめた。手順としては後退であり、記録に残す。

## Sources

- `vendor/aidlc-workflows/dist/claude/.claude/tools/aidlc-orchestrate.ts`（固定コミット `a277af21` とバイト一致を確認）
- `vendor/aidlc-workflows/dist/claude/.claude/tools/aidlc-lib.ts`
- `vendor/aidlc-workflows/CHANGELOG.md:597` — 2.6.51
- `.claude/hooks/aidlc-state-transition-guard.ts`
- `.claude/hooks/aidlc-plan-approval-guard.ts`
- `modules/app/aidlc/src/runtime.rs`、`src/turn.rs`、`src/wording.rs`
- `modules/core/read-model-updater/src/workspace/active_directive.rs`

## Assumptions & Open Questions

- 7.1 の「照合と publish を 1 区間へ畳む」改修範囲は未裁定である。
- 7.2 のガード運用（一時ワークスペース対象の本家実走を許すか）は未裁定である。
