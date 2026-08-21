> 抽出元: upstream as-built 仕様 (awslabs/aidlc-workflows v2 @ 3c3146cf, v2.6.40)。2026-08-22 の 3 エージェント精密抽出 (slice 2)。10-orchestration.md slice 2 の執筆材料。

抽出に必要な本文をすべて確認した。以下が抽出結果である。

---

# Bolt 8 動詞の完全抽出 (orchestration 側)

出典表記: `09§x.y (Lnn)` = `docs/docs/upstream/specs/09-cli-tools.md` の節と行番号、`04§x.y (Lnn)` = 同 `04-stage-protocol.md`、`03 (Lnn)` = 同 `03-state-audit-runtime.md`、`02 (Lnn)` = 同 `02-orchestration-engine.md`。`bolt:NNN` 等は upstream ソース `core/tools/aidlc-bolt.ts` の行番号 (仕様文書内の引用)。

## 1. Bolt の定義と所有権 (09§5.1, L210-216)

| 項目 | 内容 | 出典 |
| --- | --- | --- |
| Bolt の定義 | "A bolt is one execution of stages 3.1-3.5 for a Unit (or small group of dependency-linked Units)." (逐語) | 09§5.1 (L212), bolt:3-4 |
| 所有 audit イベント (4 種) | `BOLT_STARTED`, `BOLT_COMPLETED`, `BOLT_FAILED`, `AUTONOMY_MODE_SET` | 09§5.1 (L214); 03 (L789) で「Construction Bolt: 4」として集計 |
| `BOLT_ABORTED` 不在規則 | `abort` は `BOLT_ABORTED` 型を導入せず、`BOLT_FAILED` + `Reason: aborted` フィールドを再利用する。理由 (逐語): "keeps the audit count stable and uses field taxonomy for sub-classification" | 09§5.1 (L214), bolt:7-9 |
| 合成のみで重複禁止 | sibling primitive (`aidlc-state.ts fork/merge`, `aidlc-audit.ts audit-fork/audit-merge`, `aidlc-runtime.ts fragment-fork/fragment-merge`, `aidlc-worktree.ts discard`) を合成するが、決して重複実装しない。ヘッダ不変条件 (逐語): "Never duplicate state mutations the sibling primitives already own (Bolt Refs, Worktree Path) — this is the t48 emitter-pairing rule" | 09§5.1 (L216), bolt:36-38 |
| sibling 起動の実行形 | `compiledExecutable()` が非 null なら `<executable> <noun> <verb> …`、null なら `bun <toolsDir>/aidlc-<tool>.ts …`。bolt はさらに私的サブコマンド名 `audit-fork`/`audit-merge` を dispatcher 公開動詞 `audit fork`/`audit merge` に翻訳する | 09§2.4 (L67), bolt:117-160, bolt:130-135 |

## 2. 8 サブコマンド総覧 (09§5.2, L218-235)

router は 8 動詞を列挙 (bolt:881-910)。未知動詞拒否 (逐語, bolt:907-909):

```
Unknown subcommand: <x>. Valid: start, complete, fail, abort, set-autonomy, dispatch-event, hold-merge, release-merge
```

| サブコマンド | 必須フラグ | 任意フラグ | 発行イベント | 出典 |
| --- | --- | --- | --- | --- |
| `start` | `--name`, `--batch` | `--walking-skeleton`, `--worktree --slug`, `--repo`, `--intent`, `--space` | `BOLT_STARTED`; `--worktree` 付きは加えて `STATE_FORKED` + `AUDIT_FORKED` + fragment fork を駆動 | 09§5.2 (L224) |
| `complete` | `--name`, `--batch` | `--merge --slug` | `BOLT_COMPLETED`; `--merge` 付きは加えて `STATE_MERGED` + `AUDIT_MERGED` + fragment merge を駆動 | 09§5.2 (L225) |
| `fail` | `--name`, `--error` | `--slug`, `--succeeded-siblings` | `BOLT_FAILED` | 09§5.2 (L226) |
| `abort` | `--name`, `--slug`, `--reason` | `--discard` | `BOLT_FAILED` (`Reason: aborted` 付き) | 09§5.2 (L227) |
| `set-autonomy` | `--mode autonomous\|gated` | — | `AUTONOMY_MODE_SET` + state フィールド書込 | 09§5.2 (L228) |
| `dispatch-event` | `--event`, `--slug` + 変種別フラグ | — | `MERGE_DISPATCH_*` 3 種のいずれか 1 つ | 09§5.2 (L229) |
| `hold-merge` | `--slug` | — | (audit 発行なし) | 09§5.2 (L230) |
| `release-merge` | `--slug` | — | (audit 発行なし) | 09§5.2 (L231) |

### 2.1 フラグ解析規則 (09§5.2, L233-235)

| 規則 | 内容 (逐語拒否含む) | 出典 |
| --- | --- | --- |
| boolean フラグは 3 つのみ | `--worktree`, `--merge`, `--discard`。strict な値必須パーサの前に `splitBooleanFlags` で除去される | 09§5.2 (L233), bolt:97-110 |
| フラグ直後にフラグが来た場合の拒否 | `--x expects a value, got another flag: "--y". Did you forget the value?` | 09§5.2 (L233), bolt:172 |
| CSV `--name` × `--worktree` 拒否 | `--worktree requires a single bolt name; got csv: "<n>". Issue one start --worktree per bolt.` | 09§5.2 (L235), bolt:215-217 |
| CSV `--name` × `--merge` 拒否 | 対称形 `--merge requires a single bolt name; …` | 09§5.2 (L235), bolt:375-377 |
| 並列バッチの発行形 | 並列バッチは slug ごとに N 回の `start --worktree` を発行する (1 呼び出し 1 Bolt) | 09§5.7 (L290), bolt:194-196 |

### 2.2 `--batch` の検証と join key (09§5.7, L288-290)

| 項目 | 内容 | 出典 |
| --- | --- | --- |
| 検証正規表現 | `/^[1-9][0-9]*$/` (正の整数)。`start` と `complete` の双方で検証 | 09§5.7 (L290), bolt:202-204, bolt:363-365 |
| 逐語拒否 | `Invalid --batch: "<b>". Must be a positive integer.` | 09§5.7 (L290) |
| audit フィールド | `Batch number` フィールドとして audit に運ばれる | 09§5.7 (L290) |
| join key の役割 | swarm の `prepare`/`finalize` が `SWARM_STARTED` boundary と unit を相関させる結合キー (09§6.6 の attempt stamp `{stage, floor}` は prepare 時に捕捉され `SWARM_STARTED` の `Stage`/`Run floor` フィールドに書かれる) | 09§5.7 (L290), 09§6.6 (L382-388) |

## 3. ガード順序 (ordering discipline) — 09§5.3 (L237-245)

3 種の固定順序が、それぞれ理由付きで記録されている。

### 3.1 `start --worktree` の三層 fork 固定順序 (09§5.3, L241; bolt:224-335)

| 順 | ステップ | 失敗時 | 出典 |
| --- | --- | --- | --- |
| 1 | state ファイル形状の検証 (読み取り) | `state-read-failed` → failJson | bolt:224-335 |
| 2 | `BOLT_STARTED` 発行 | `audit-emit-failed` → failJson | 同上 |
| 3 | state-fork (`aidlc-state.ts fork`) | 失敗時: リカバリ `BOLT_FAILED` を発行してから fail | 同上 |
| 4 | audit-fork (`aidlc-audit.ts audit-fork`) | 同上 | 同上 |
| 5 | fragment-fork (`aidlc-runtime.ts fragment-fork`) | 同上 | 同上 |

- 検証が emit に先行する理由 (逐語): "so a missing state file doesn't leave an orphan BOLT_STARTED" (09§5.3 L241 / 09§15 原則 2 (L1047), bolt:221-223)。
- 各 fork 段の失敗はリカバリ `BOLT_FAILED` を発行してから失敗する (09§5.3, L241)。
- fork boundary は `BOLT_STARTED` + `STATE_FORKED` + `AUDIT_FORKED` の三重証明、merge boundary は `BOLT_COMPLETED` + `STATE_MERGED` + `AUDIT_MERGED` (逐語引用: "the fork boundary is already triple-attested by BOLT_STARTED + STATE_FORKED + AUDIT_FORKED, the merge boundary by BOLT_COMPLETED + STATE_MERGED + AUDIT_MERGED"; 03 (L1145-1146), `aidlc-runtime.ts:1104-1107`)。

### 3.2 `complete --merge` の固定順序 (09§5.3, L242; bolt:387-489)

| 順 | ステップ | 備考 | 出典 |
| --- | --- | --- | --- |
| 1 | **hold-merge チェック** (最前段) | `Merge-Held: true` なら逐語拒否 (下記 §5 参照)、reason `merge-held` | bolt:387-489, 拒否文 bolt:392 |
| 2 | `BOLT_COMPLETED` 発行 | audit-first | 同上 |
| 3 | state-merge | 失敗: `state-merge-failed` / `state-merge-timeout` | 同上 |
| 4 | audit-merge | 失敗: `audit-merge-failed` / `audit-merge-timeout` | 同上 |
| 5 | fragment-merge | 失敗: `fragment-merge-failed` / `fragment-merge-timeout` | 同上 |

### 3.3 `abort --discard` の反転順序 (09§5.3, L243; bolt:562-586)

| 順 | ステップ | 出典 |
| --- | --- | --- |
| 1 | discard を**先に**実行 (`aidlc-worktree.ts discard`) | bolt:562-586 |
| 2 | audit (`BOLT_FAILED` + `Reason: aborted`) を**後で**発行 | 同上 |

順序反転の理由 (コメントに記録された知見、逐語): 先に emit すると "would claim the Bolt was aborted-and-cleaned-up while the worktree directory still existed on disk and the slug remained in main's Bolt Refs" (09§5.3, L243)。09§15 原則 1 (L1046) も同旨: audit-first の kill-9 窓の一般原則に対し、`abort --discard` は「逆方向の失敗モードの方が悪い」ため意図的に反転 (bolt:562-569)。

### 3.4 sibling spawn の 30 秒タイムアウトと reason 選択 (09§5.3, L245)

| 規則 | 内容 | 出典 |
| --- | --- | --- |
| タイムアウト値 | 全 sibling spawn に一律 30 s | 09§5.3 (L245), bolt:150-151 |
| 判別方法 | `signal === "SIGTERM"` でタイムアウトと exit-code 失敗を区別 | bolt:150-151, bolt:277-278 |
| reason 選択 | タイムアウトなら `*-timeout` 系 reason enum (例: `state-fork-timeout`)、それ以外は `*-failed` 系 | 同上 |

## 4. 失敗エンベロープ (09§5.4, L247-255)

worktree 系パスの非 `error()` 失敗は機械可読エンベロープを印字し exit 1 (`failJson`, bolt:946-966)。JSON 形 (逐語):

```json
{"ok": false, "slug": "…", "stage": "…", "reason": "…", "detail": "…"}
```

| フィールド | 取り得る値 (逐語・完全列挙) | 出典 |
| --- | --- | --- |
| `stage` | `start-worktree`, `complete-merge`, `abort-discard`, `hold-merge`, `release-merge` (5 値) | 09§5.4 (L255) |
| `reason` | `state-read-failed`, `audit-emit-failed`, `state-fork-failed`, `state-fork-timeout`, `audit-fork-failed`, `audit-fork-timeout`, `fragment-fork-failed`, `fragment-fork-timeout`, `merge-held`, `state-merge-failed`, `state-merge-timeout`, `audit-merge-failed`, `audit-merge-timeout`, `fragment-merge-failed`, `fragment-merge-timeout`, `discard-failed`, `discard-timeout` (17 値) | 09§5.4 (L255) |

`failJson` は `error()` と明示的に区別される。`error()` は `emitError` → `ERROR_LOGGED` audit 行に経路する (09§5.4 L255, bolt:943-945, bolt:916-920)。

swarm 側の exit-code 分岐 (orchestration 連携): `finalize` の exit `2` = failure envelope → 「baton を取り戻し construction モジュールの halt-and-ask seam を通じて halt する」(04§7 L444)。

## 5. HOLD-MERGE (09§5.5, L257-270) — orchestration 側

(worktree 側の保存 API は既存抽出 `workspace-lock-fork-worktree.md` 済み。以下は orchestration 協調に関する部分。)

| 項目 | 内容 | 出典 |
| --- | --- | --- |
| 保存先 | per-Bolt **forked** state ファイル `<projectDir>/.aidlc/worktrees/bolt-<slug>/…/aidlc-state.md` の `Merge-Held` フィールド | 09§5.5 (L259), bolt:620-621; 03 (L522): `Merge-Held` (`true`/`false`) は `## Project Information` 配下・per-Bolt forked state のみ, bolt:692 |
| 冪等性 | hold/release 双方向で冪等 | 09§5.5 (L261), bolt:622-633 |
| フィールド挿入 | 初回 hold 時に `## Project Information` 配下へ挿入 (state テンプレートの版上げ不要) | 09§5.5 (L262) |
| audit 非発行の理由 (逐語) | "Merge-Held is internal coordination state, not a user-visible event." | 09§5.5 (L263) |
| forked state 不在時の読み取り | not held と読む (`forkedStateFilePath` が `null` → `isMergeHeld` false) | 09§5.5 (L264), bolt:661-682 |
| forked state 不在時の `setMergeHeld` | ハードエラー (逐語): ``No per-Bolt forked state file for slug "<s>" — was `aidlc-bolt start --worktree --slug <s>` run?`` | 09§5.5 (L264), bolt:687-689 |

**強制点は `complete --merge`**。逐語拒否 (09§5.5 L266-268, bolt:392):

> `` Merge held by HOLD-MERGE invariant; resolve the failed-sibling halt-and-ask sequence and run `aidlc-bolt release-merge --slug <slug>` before retrying. ``

**協調プロトコルの根拠** (09§5.5 L270, bolt:379-386): 複数失敗の halt-and-ask シーケンスは、失敗 sibling の質問を 1 つでも描画する**前に**、**成功した全 sibling** に `Merge-Held: true` を設定する。これによりシーケンス途中で merge が着地できない。逐語: "This refusal pins that invariant in tooling so an orchestrator that forgets the prose contract cannot land a merge mid-AUQ-sequence."

**release-merge の冪等性の運用** (04§7 L444): `merge_failures` unit (収束済みだが merge-back 失敗; "no `SWARM_UNIT_CONVERGED` row lands until the merge does") では、ブロッカーを解消して当該 unit にスコープした `finalize` を再実行する — `release-merge` は冪等 — が、`prepare` は再実行**しない** (既存 worktree がエラーになるため)。swarm の merge-back 直列化順も参照: unit ごとに `aidlc-bolt release-merge --slug <s>` (冪等) → `aidlc-bolt complete --merge --slug <s> --batch <n> --name <u>` (09§6.5 L376, swarm:1084-1096)。

## 6. halt-and-ask シーケンス (orchestration 側手順) — 04§6.2 (L385)

| 項目 | 内容 (逐語含む) | 出典 |
| --- | --- | --- |
| 発火条件 | "When a Bolt's code-generation returns failure, **always halt and present the halt-and-ask prompt regardless of autonomy mode**." | 04§6.2 (L385), construction:51-74 |
| autonomous モードで人間に諮る 2 例 | (1) この Bolt 失敗 halt-and-ask、(2) Build-and-Test loop-back の exhausted rung | 04§6.2 (L385) |
| 単独失敗 | `BOLT_FAILED` を `--slug` 付きで発行 | 04§6.2 (L385) |
| 並列バッチ失敗 | 全タスクの完了を待ち、成功 Bolt の成果物を保存し、`BOLT_FAILED` を `Succeeded=[names]` 付きで発行 (CLI 側は `fail --succeeded-siblings`) | 04§6.2 (L385), 09§5.2 (L226) |
| 3 選択肢 | Retry (既存 worktree 内で再実行) / Skip (`[S]` マーク、worktree 保存) / Abort (worktree 保存) | 04§6.2 (L385) |
| プロンプト素材の決定的取得 | 質問組立て前に `aidlc-worktree.ts info --slug <slug>` から `<path>` と `<branch_name>` を取得。`info` は最新 `WORKTREE_CREATED` ブロックの `Worktree path` と `Branch name` を印字し、schema は `knowledge/aidlc-shared/worktree-info-schema.md` に固定 | 04§6.2 (L385); 09§7.6 (L551), worktree:1039-1049 |
| レンダリング先 | halt-and-ask on Bolt failure は harness の question-rendering annex の対象サイトの 1 つ (approval gates / interaction-mode choice / ladder prompt / halt-and-ask / consolidated-summary / §13 learnings gate) | 02§12 (L429) |
| 事後検証 | `aidlc-worktree.ts verify` が「orchestrator の決定的 post-dispatch backstop」: event + `Bolt slug` の最新 audit ブロックを鮮度窓 (既定 **60 秒**, `--max-age-seconds`) で検査。3 結果: `{verified:true, event, slug, audit_timestamp}` (exit 0) / `{verified:false, …, reason:"absent"}` (exit 1) / `{verified:false, …, reason:"stale (last seen <ts>)"}` (exit 1) | 09§7.6 (L549) |

Build-and-Test loop-back 側の halt-and-ask 2 変種 (04§6.3 L399, construction:191-226): impact-estimated 変種 (Retry with fix / Accept failure / Abort、各記述に effort・financial cost・risk 必須) と、候補 fix が無い場合に "Retry with fix" を**丸ごと省略**する no-fix 変種。理由 (逐語): 候補 fix なしで提示することは "would itself be the impact-unestimated give-up option this protocol forbids in the other direction (a fabricated fix to retry with)"。テンプレートのスロットを placeholder や捏造内容で埋めて形だけ保つことは禁止。

## 7. `set-autonomy` (09§5.6, L272-286)

| # | ガード / ステップ (順序どおり) | 内容 (逐語含む) | 出典 |
| --- | --- | --- | --- |
| 0 | mode 検証 (全ての前) | `Invalid --mode: <m>. Must be 'autonomous' or 'gated'.` | 09§5.6 (L286), bolt:808 |
| 1 | 単一 `withAuditLock` | "One lock covers presence check -> audit consume -> state write. Otherwise two grants, or a grant racing approval, can both observe one fresh turn" | 09§5.6 (L278), bolt:813-814 |
| 2 | human-presence ガード (**昇格のみ**) | `autonomous` への切替は `humanActedSinceGate(pd)` が必要 (`humanPresenceGuardDisabled()` の場合を除く)。`gated` への降格は "restores gates without presence" | 09§5.6 (L279), bolt:816-818 |
| 3 | 逐語拒否 | `Refusing to switch Construction to autonomous: a real human has not acted since the last gate resolution, and autonomous mode is granted only by the human's ladder-prompt answer (it waives every later gate, so the grant itself needs a fresh human turn). Ask the human to confirm autonomous mode in a typed message, then retry. Do not log the ladder choice via aidlc-log answer; the choice is recorded by set-autonomy itself.` | 09§5.6 (L280-282), bolt:825-829 |
| 4 | 検証済みコンテキスト内で audit-first | `setFieldStrict("Construction Autonomy Mode", mode)` で state フィールド検証 → `AUTONOMY_MODE_SET` 発行 → state ファイル書込 | 09§5.6 (L284), handleSetAutonomy = bolt:804-859 |

| 補足 | 内容 | 出典 |
| --- | --- | --- |
| 唯一の autonomy 動詞 | `decide-question` サブコマンドは存在せず、autonomy decision ladder は upstream ツリーのどこにも無い。`git grep -F -e "decide-question" -e "decideQuestion" -- core plugins harness` → 0 件。autonomy は単一動詞で書かれる 2 値フィールド (`autonomous`/`gated`) | 09§5.6 (L274), 09 Measurement notes (L1080) |
| ladder prompt との連携 | 実 walking skeleton の gate 承認直後に**ちょうど 1 回**発火 (skeleton-off / zero-Unit では発火しない)。2 択: "Continue autonomously" / "Gate every Bolt"。回答は `aidlc-bolt.ts set-autonomy --mode <choice>` で記録 (`AUTONOMY_MODE_SET` は set-autonomy 自身が発行)。先に interview answer としてログすると人間の fresh turn を消費し mode switch が拒否される: "logging the choice as an interview answer first would consume that turn and the mode switch would refuse" (construction:44)。resume 時、mode が `unset` かつ skeleton `[x]` ならプロンプト再発火 | 04§6.2 (L383), construction:27-45 |
| engine 側 presence guard との関係 | engine の report 側にも別の human-presence ガードあり: gated・未完了 stage・autonomy ≠ `autonomous`・`AIDLC_SKIP_HUMAN_PRESENCE_GUARD !== "1"` のとき、空 `--user-input` は拒否: `report --result <r> for "<slug>" requires --user-input with the human's exact approval choice.` | 02§7 (L292), `aidlc-state.ts:5786-5797` |
| **文書化済み不整合** | state テンプレートは `Construction Autonomy Mode` を宣言 (`state-template.md:61`) するが birth emitter は書かない (`aidlc-utility.ts:4271-4276`)。読み手は `getField` (null → not-autonomous に安全縮退)。しかし唯一の書き手 `set-autonomy` は `setFieldStrict` (bolt:837) を使い `setOrInsertField` サイトが無いため、生まれたての state ファイルでは `State update failed: Field not found in state file: "Construction Autonomy Mode". …` で失敗する。テスト fixture は製品経路でなく regex で行を注入 (`t186:205`, `t215:250`) | 03 §discrepancies (L631), 03 (L505, L1246) |

## 8. `dispatch-event` (09§5.8, L292-302)

| 項目 | 内容 (逐語含む) | 出典 |
| --- | --- | --- |
| 意味論 | emit-only: "no state mutation, no spawn. Pure audit emission so doctor can reconcile orphan INVOKED rows" | 09§5.8 (L294), bolt:716-717 |
| 実装形 | Map lookup ではなく 3 つのリテラル `emitAudit(pd, "EVENT_NAME", …)` 呼び出し。理由: grep ベースのテストがリテラル emitter pairing を検証するため | 09§5.8 (L302), bolt:719-722 |

3 変種と変種別必須フラグ (bolt:732-796):

| `--event` | 必須フラグ | audit フィールド | 出典 |
| --- | --- | --- | --- |
| `MERGE_DISPATCH_INVOKED` | `--practices-excerpt` | `Bolt slug`, `Practices section excerpt` | 09§5.8 (L298) |
| `MERGE_DISPATCH_RETURNED` | `--strategy` (∈ squash\|merge\|rebase), `--target`, `--confidence` (∈ [0,1]), `--notes` | `Bolt slug`, `Strategy`, `Target branch`, `Confidence`, `Notes` | 09§5.8 (L299) |
| `MERGE_DISPATCH_FALLBACK` | `--reason`, `--defaults` | `Bolt slug`, `Fallback reason`, `Defaults applied` | 09§5.8 (L300) |

(03 (L792) の audit 分類でも「Merge Dispatch: 3」`MERGE_DISPATCH_INVOKED` `MERGE_DISPATCH_RETURNED` `MERGE_DISPATCH_FALLBACK` として集計。)

## 9. zero-Unit directive の特例 (04§6.1, L373-375)

| 項目 | 内容 (逐語含む) | 出典 |
| --- | --- | --- |
| ガード位置 | construction モジュール冒頭 (construction:5-11) | 04§6.1 (L375) |
| 適用条件 | Bolt / walking-skeleton / ladder / autonomy / per-Unit ceremony は "apply only when the engine resolved a real non-empty Unit DAG" | 04§6.1 (L375) |
| 判定フィールド | `directive.unit` または `directive.wave` = Unit 作業、`directive.swarm_settled` = autonomous 実行の gate-only 終端 | 04§6.1 (L375) |
| 特例規則 (逐語) | "A zero-Unit directive has none of those fields: run it once as an ordinary stage, with no Bolt, skeleton, ladder, or swarm ceremony." | 04§6.1 (L375) |
| ladder への波及 | ladder prompt は zero-Unit 実行では一切発火しない | 04§6.2 (L383) |

## 10. Bolt ライフサイクル状態機械 (統合ビュー)

上記の一次出典から合成した遷移表 (各行に出典付き):

| 遷移 | トリガ動詞 | worktree | 効果 | 出典 |
| --- | --- | --- | --- | --- |
| (未開始) → 実行中 | `start` | なし | `BOLT_STARTED` のみ | 09§5.2 (L224) |
| (未開始) → 実行中 (隔離) | `start --worktree --slug` | 作成 (三層 fork) | 検証 → `BOLT_STARTED` → `STATE_FORKED` → `AUDIT_FORKED` → fragment fork。各 fork 失敗はリカバリ `BOLT_FAILED` → failJson | 09§5.3 (L241) |
| 実行中 → 完了 | `complete` | なし | `BOLT_COMPLETED` のみ | 09§5.2 (L225) |
| 実行中 (隔離) → 完了 (merge 済) | `complete --merge --slug` | merge して回収 | hold チェック → `BOLT_COMPLETED` → `STATE_MERGED` → `AUDIT_MERGED` → fragment merge。`Merge-Held` なら reason `merge-held` で拒否 | 09§5.3 (L242), 09§5.5 (L266) |
| 実行中 → 失敗 | `fail --name --error [--slug] [--succeeded-siblings]` | **保存** (halt-and-ask の Retry が既存 worktree 内再実行を前提) | `BOLT_FAILED` (並列時 `Succeeded=[names]`) | 09§5.2 (L226), 04§6.2 (L385) |
| 実行中 → 中止 (worktree 保存) | `abort --name --slug --reason` | 保存 | `BOLT_FAILED` + `Reason: aborted` | 09§5.2 (L227), 09§5.1 (L214) |
| 実行中 → 中止 (worktree 破棄) | `abort --discard` | 破棄 | discard **先** → audit **後** (`BOLT_FAILED` + `Reason: aborted`) | 09§5.3 (L243) |
| merge 保留 ⇄ 解除 | `hold-merge` / `release-merge` | forked state に `Merge-Held` | audit 非発行・双方向冪等 | 09§5.5 (L259-263) |
| autonomy 切替 | `set-autonomy --mode` | — | `AUTONOMY_MODE_SET` + state 書込 (昇格のみ presence ガード) | 09§5.6 |

**fail と abort の違い**: `fail` は失敗事実の記録 (`--error` 必須、`--slug` 任意、並列成功者を `--succeeded-siblings` で記録)。`abort` は意図的中止 (`--reason` 必須、`--slug` 必須) で、audit 型は増やさず `BOLT_FAILED` + `Reason: aborted` を再利用し、`--discard` 指定時のみ worktree を破棄しかつ audit 順序を反転する (09§5.1 L214, 09§5.2 L226-227, 09§5.3 L243)。halt-and-ask の 3 選択肢 (Retry/Skip/Abort) はいずれも worktree を保存する (04§6.2 L385)。

**swarm 経由の失敗経路**: swarm の `emitBoltFailed` (swarm:695-701) は失敗 unit ごとに `aidlc-bolt fail` を best-effort で合成する — "the swarm's own SWARM_UNIT_FAILED is the authoritative swarm signal, so a failure to emit BOLT_FAILED must not mask it" (09§6.8 L417)。swarm reviewer 経路の Retry では "return to the main workspace, abort and discard the old Bolt, then rerun the current `aidlc-swarm.ts prepare` step for that Unit with the original batch/base/repo arguments; the fresh `BOLT_STARTED` boundary resets review accounting without claiming convergence" (04§7 L446)。また `BOLT_STARTED` は reviewer receipt 有効性の floor である ("`BOLT_STARTED` — not `STAGE_STARTED` — is the floor"、prepare fork 時に main から継承した receipt を除外しつつ merge retry を跨いだ receipt は保存する; 09§6.7 L394, swarm:328-330)。

## 11. 出力形のまとめ

| 経路 | 形 | 出典 |
| --- | --- | --- |
| worktree 系パスの非 error 失敗 | `{"ok": false, "slug": "…", "stage": "…", "reason": "…", "detail": "…"}` + exit 1 (`failJson`) | 09§5.4 (L249-253), bolt:946-966 |
| `error()` 失敗 | `emitError` → `ERROR_LOGGED` audit 行 | 09§5.4 (L255), bolt:916-920, bolt:943-945 |
| 逐語拒否 (パース段) | §2.1 の各拒否文 | 09§5.2 |
| 成功時 | as-built 仕様 §5 に成功時 stdout の明示規定なし (audit 発行 + exit 0 が契約。事後確認は `aidlc-worktree verify` の JSON が担う) | 09§5, 09§7.6 (L549) |

## 12. 仕様執筆時の注意点 (as-built が明記する境界)

| 論点 | 内容 | 出典 |
| --- | --- | --- |
| `settle` 動詞・pool 概念は bolt/swarm に存在しない | `grep -c -i -e settle -e pool core/tools/aidlc-swarm.ts core/tools/aidlc-bolt.ts` → 両ファイル 0。batch→engine の settle handshake は run-stage directive の任意フィールド `swarm_settled?: true` (directive.ts:210, orchestrate.ts:3442) | 09§6.9 (L441), 09 Measurement notes (L1094) |
| bolt LOC / 動詞数 | `aidlc-bolt.ts` は 970 行・8 動詞 | 09§3 (L92) |
| ExitPlanMode 系フックの盲点 | marker path は `aidlc-jump`/`aidlc-bolt`/`aidlc-swarm` と mutating `aidlc-state` 動詞に blind (transcript path は engagement として数える) | 07 (L309) |
| worktree 命名は導出であり引数でない | worktree dir = `<projectDir>/.aidlc/worktrees/bolt-<slug>`、branch = `bolt-<slug>` (既存抽出と重複するが、bolt 側の `--slug` 意味論の前提) | 09§7.1 (L462) |

---

主な出典ファイル (絶対パス):
- docs/upstream/specs/09-cli-tools.md (§5: L208-304, §2.4: L67, §6.5-6.9: L376-441, §7.6: L549-551, §15: L1046-1047, Measurement notes: L1080, L1094)
- docs/upstream/specs/04-stage-protocol.md (§6: L371-427, §7: L431-467)
- docs/upstream/specs/03-state-audit-runtime.md (L505, L522, L631, L789-792, L1145-1146)
- docs/upstream/specs/02-orchestration-engine.md (L292, L429)