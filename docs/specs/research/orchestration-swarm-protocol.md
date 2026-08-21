> 抽出元: upstream as-built 仕様 (awslabs/aidlc-workflows v2 @ 3c3146cf, v2.6.40)。2026-08-22 の 3 エージェント精密抽出 (slice 2)。10-orchestration.md slice 2 の執筆材料。

以下、orchestration コンテキスト仕様 slice 2 (Construction 実行機構) のための精密抽出。出典表記は「仕様書§節 + 仕様書が引用する upstream コード行 (`file:line`)」。逐語契約は原文 (英語) のまま保存。

# Swarm 3 動詞と収束プロトコルの完全列挙

## 0. アーキテクチャ前提 (責務三分割)

| 項目 | 内容 | 出典 |
|---|---|---|
| 三分割 | 逐語: "the conductor owns fan-out + loop drive (knowledge); this tool owns the convergence verdict + merge + audit (determinism); the human grants autonomy and takes the baton on the envelope (judgement)." | 09 §6.1 / `aidlc-swarm.ts:11-13` |
| worker dispatch は本ツール外 | "A bun subprocess cannot issue Task calls, so the worker-dispatch layer is NOT here" | 09 §6.1 / `:6-7` |
| fan-out 駆動 | N 並列 Task call、または `AIDLC_USE_SWARM=1` で inline Dynamic Workflow。driver 選択は conductor 側。ツールが downgrade を知るのは `prepare --degraded-from` 経由のみ | 09 §6.1 / `:28-31` |
| stateless | "no iteration counter, no persisted state" の 3 サブコマンド。retry cap 定数は不在: verdict=determinism→`check`、retry 判断=knowledge→conductor、暴走防止=determinism→harness Stop-hook 上限 | 09 §6.2 / `:15`, `:55-63` |
| check/finalize の権威根拠 | 逐語: "check is advisory, finalize is authoritative (re-verifies at the merge gate), so a red unit cannot merge even if the conductor lies or misremembers." | 09 §6.2 / `:61-63` |
| argv パーサ | `--flag value` ペアをスキップして verb を探すため `--project-dir <p> check <unit>` / `check --project-dir <p> <unit>` 両方が解決 | 09 §6.2 / `:1352-1371` |
| 未知 verb 拒否 | `Unknown subcommand: <x>. Valid: prepare, check, finalize` | 09 §6.2 / `:1385` |
| anti-tamper baseline | "The anti-tamper baseline is each worktree's OWN git fork (HEAD) — nothing is stored" — 保存されるベースラインは無い | 09 §6.3 / `:24-26` |
| 起点 directive | engine 側 `invoke-swarm` directive kind (`aidlc-directive.ts:75`)。必須 `units[]` + 任意 `stage`, `stage_file`, `reviewer`, `reviewer_max_iterations`, `review_class`, `protocol_modules`, `repo`。意味: "fan out N parallel workers across N worktrees for a build batch" | 02 §4.1 / `aidlc-directive.ts:288-289` |

## 1. `prepare`

構文: `prepare --batch <n> --units <a,b,c> [--base <branch>] [--concurrency <n>] [--degraded-from <subagent|ultracode>] [--repo <name>]` (09 §6.3)

実行順序 (`aidlc-swarm.ts:705-859`):

| # | ステップ | 判定 / 逐語拒否 | 行 |
|---|---|---|---|
| 1 | `--batch` 正整数 + `--units` 非空を検証 | (bolt 側の batch 検証正規表現は `/^[1-9][0-9]*$/`、拒否 `Invalid --batch: "<b>". Must be a positive integer.` — 09 §5.7) | `:705-` |
| 2 | **Autonomous Code Generation gate**: `Current Stage` 正規化が `code-generation` かつ `Construction Autonomy Mode` が `autonomous` のとき、全 unit が `evaluateCodeGenerationApproval` を通ること | 拒否逐語: `prepare requires a current, explicitly approved Code Generation plan for every autonomous unit before worktrees are forked: <unit> (<reason>); …` | `:730-736` |
| 3 | 権威 unit DAG 解決。malformed は fail-closed | `prepare cannot resolve the authoritative unit DAG: <reason> (<detail>). Fix unit-of-work-dependency.md before starting the swarm.` | `:740-743` |
| 4 | DAG unit と要求 unit の和集合で bolt-slug 一意性を assert | — | `:745-749` |
| 5 | construction repo 解決 (`--repo`; multi-repo intent で省略はエラー) | — | — |
| 6 | `--base` デフォルト = repo の現在ブランチ; `--concurrency` デフォルト = unit 数 | — | — |
| 7 | **attempt stamp** `{stage, floor}` 解決。不在なら拒否 | `prepare could not resolve the current stage attempt from state and audit` | `:774` |
| 8 | `--degraded-from` があれば batch-start 行の**前に** `SWARM_DEGRADED` 発行。値は `subagent` か `ultracode` のみ | — | `:778-787` |
| 9 | unit ごと: `aidlc-worktree create --slug <boltSlug> --base <base> [--repo]` → `aidlc-bolt start --worktree --slug <boltSlug> --batch <n> --name <unit> [--repo]` | — | — |
| 10 | worktree 作成と start が**両方**成功した unit のみを載せて `SWARM_STARTED` を**1 行**発行 | anti-replay 根拠逐語: "Emitting before creation would let a failed re-prepare in a later stage attempt relabel an old preserved worktree with the current attempt, allowing stale data to pass finalize's exact-attempt check." | `:842-849` |
| 11 | JSON plan を出力し `process.exit(prepared.some(p => !p.ok) ? 2 : 0)` | — | — |

## 2. `check` (advisory)

構文: `check <unit> --check-cmd <cmd> [--test-file <path>]` (09 §6.4)

| 項目 | 内容 | 行 |
|---|---|---|
| 2 信号ともディスクから再導出 | — | `:864-906` |
| **Green** | worktree 内で `--check-cmd` 実行、exit 0 = converged。逐語: "the AUTHORITATIVE green check — a worker's own claim of success is never trusted (it could fake a pass)" | `:186-188` |
| シェル選択 | POSIX で `/bin/bash` 存在時 `shell: "/bin/bash"` (bashism 保持)、それ以外 `shell: true` (win32=cmd.exe, bash 無し POSIX=/bin/sh)。60 s timeout | `:190-203`, `:211-219` |
| **Untampered** | worktree 内 `git diff --quiet HEAD -- <testFile>`。tamper 確定は status **1** のみ; "any other status (e.g. 128 — path not tracked at HEAD) is not a confirmed tamper" (`return result.status === 1;`) | `:227-228`, `:235` |
| confinement | `--test-file` は worktree 内に閉じ込め。`../` escape は設定エラーであって pass ではない: `--test-file resolves outside the unit worktree: <path>` — 理由逐語: "a `../` escape would point the guard at a file the worker never touched and silently DISABLE it" | `:261-272`, `:268` |
| 出力 JSON 形 | `{unit, converged, tampered, reason}`; tamper 時 `detail: "protected test file was modified"` | `:895-902` |
| exit code | 0 は真の収束 `converged && !tampered` のみ | `:905` |
| worktree 不在 | `` no worktree for unit "<u>" — run `prepare` first `` | `:879` |
| audit | `check` は audit を**発行しない** | 09 §6.4 |

プロトコル側逐語 (04 §7 手順 3, `stage-protocol-swarm.md`): "exit `0` = genuinely converged (the real check passed and no protected file was tampered); non-zero = not yet, and you judge retry-vs-escalate".

## 3. `finalize` (authoritative gate)

構文: `finalize --batch <n> --units <a,b,c> --claimed <a,b> --check-cmd <cmd> [--test-file <path>] [--reasons <unit>=<reason>,…]` (09 §6.5)

### 3.1 claimed unit の 6 段ガード連鎖 (lying-conductor guard)

1 本の `else if` 連鎖 (6 ガード + not-green fallthrough)、**最初のマッチ勝ち** (`aidlc-swarm.ts:966-1059`):

| 段 | 検査 | verdict | detail 逐語 | 行 |
|---|---|---|---|---|
| 1. stamp | この unit+batch の stamped `SWARM_STARTED` boundary が無い | `error` | `no stamped SWARM_STARTED boundary for this unit and batch; run prepare in the current attempt` | `:973` |
| 2. attempt 一致 | prepared attempt ≠ current attempt | `error` | `prepared swarm attempt <s>/<f> does not match the current attempt <s>/<f>` | `:981` |
| 3. worktree | 再検証時に worktree が無い | `error` | `no worktree on re-verify (prepare not run?)` | `:995` |
| 4. confinement | `--test-file` の閉じ込め失敗 | `error` | §2 の `confineError` 文字列を運搬 | `:1002-1003` |
| 5. tamper | 保護テストファイル改変 | `error` (+`tampered: true`) | `convergence rejected: protected test file was modified` | `:1004` |
| 6. green+receipt+binding | green のとき: (a) 有効な reviewer receipt 無し → `error` (receipt error §3.4)、(b) source binding 失敗 → `error` (binding error)、(c) reviewed かつ source bound → `converged` | `converged` / `error` | — | `:1012-1049` |
| 7. fallthrough | green でない | `error` | `claimed converged but the check command did not pass on re-verify` | `:1050-1058` |

### 3.2 declined unit (非 claimed)

| 項目 | 内容 | 行 |
|---|---|---|
| verdict | status `failed`、reason は `--reasons` から、デフォルト `cap-exhausted` | `:1060-1077` |
| 許容 reason | `DECLINED_REASONS` = `unsatisfiable`, `budget-exhausted`, `cap-exhausted` のみ。`error` は意図的に除外: "it is the tool's OWN verdict for a claimed-but-red / tampered unit, never a conductor-supplied attribution" | `:132`, `:130-131` |
| malformed 拒否 | `--reasons entry must be <unit>=<reason>: "<pair>"` / `--reasons reason for "<u>" must be one of: unsatisfiable, budget-exhausted, cap-exhausted` | `:945`, `:951` |
| プロトコル側逐語 | "records your attribution faithfully but never lets it override a claimed-but-red unit's `error` verdict" | 04 §7 手順 4 |

### 3.3 merge-back・audit・envelope

| 項目 | 内容 | 行 |
|---|---|---|
| merge-back | 真の pass のみを決定論のためソートして**直列**実行: unit ごと `aidlc-bolt release-merge --slug <s>` (冪等) → `aidlc-bolt complete --merge --slug <s> --batch <n> --name <u>` | `:1084-1096` |
| audit | unit ごと 1 行、failed unit ごと baton 行、最後に batch tally | `:1107-1135` |
| **「行なし」中間状態** | merge-back が失敗した converged unit は `SWARM_UNIT_CONVERGED` も `SWARM_UNIT_FAILED` も得ない。理由逐語: converged 行 "is the engine's batch-advance signal, and emitting it for a unit whose metadata never landed on main would advance the run past an unmerged unit"; unit 自体は収束したので failure envelope + exit 2 が merge 結果を運び、**行は scoped retry で着地する** (B5 サーガ材料) | `:1099-1103` |
| envelope JSON 形 (逐語) | `{batch, units, converged, failed, merge_failures}` | `:1137-1147` |
| exit | いずれかの unit failed または merge 失敗で exit 2、さもなくば 0。"Exit 2 means 'the conductor must take the baton'" | `:1137-1147` |
| BOLT_FAILED 併発 | `emitBoltFailed` が failed unit ごとに `aidlc-bolt fail` を best-effort で合成: "the swarm's own SWARM_UNIT_FAILED is the authoritative swarm signal, so a failure to emit BOLT_FAILED must not mask it." | `:695-701` (09 §6.8) |

### 3.4 reviewer receipt と reviewed-source binding (finalize 第 6 段の中身)

09 §6.7:

| 要素 | 内容 | 行 |
|---|---|---|
| `reviewerRequirement` | `Current Stage` を読み stage 定義を解決し `{stage, reviewer, reviewClass, maxIterations}` を返す。`review_class` デフォルト `adversarial`; `maxIterations` は `advisory`=1、それ以外 `reviewer_max_iterations ?? 2` | `:284-325` (interface `:276-282`) |
| `reviewerReceiptError` の floor | review が**この Bolt attempt 内**で起きた証明。floor は `BOLT_STARTED` (`STAGE_STARTED` ではない): "excludes a matching receipt inherited from main when prepare forked the worktree, while preserving a receipt across a merge retry on that worktree" | `:331-465`, `:328-330` |
| ペアリング | worktree 自身の audit shard を読み `BOLT_STARTED`/`REVIEW_REQUESTED`/`REVIEW_COMPLETED` に絞り `(timestamp, position)` でソート、各 `REVIEW_COMPLETED` を先行 `REVIEW_REQUESTED` とキー `<unit>\0<iteration>` で対にし、`Stage`・`Reviewer`・`Unit` フィールド一致を要求、`Workflow: single-stage:*` 行はスキップ。`Recovery: stale-receipt` request は verdict 判定を素の `READY`/`NOT-READY` に緩和 | 同上 |
| fingerprint 要求 | `Artifact Fingerprint` が `/^sha256:[0-9a-f]{64}$/` に一致し、かつ新規再計算した `reviewArtifactFingerprint` と等しいこと。stage が `workspace_requires` を宣言していれば `Source Fingerprint` が worktree の現在 source fingerprint と一致すること | 同上 |
| mismatch 拒否逐語 | `claimed converged but the reviewed source no longer matches its worktree's fingerprint for stage "<s>", unit "<u>" (source-fingerprint mismatch); re-invoke the reviewer against the current worktree source and record a fresh verdict before finalizing` | `:456-461` |
| bypass | `AIDLC_SKIP_SOURCE_FRESHNESS=1` は source 側のみ bypass し、binding の代わりに収束行へ `Source Freshness Bypass: true` を記録 | `:448`, `:963-964`, `:639-641` |
| `bindReviewedSource` | Bolt ブランチを動かさず reviewed アプリケーションバイトを不変 commit として実体化: 一時 `GIT_INDEX_FILE` → `read-tree HEAD` → `add -A` → submodule 検証 (dirty initialized submodule は fail-closed) → filtered path の raw-byte 再バインド → framework 所有 pathspec `:(top)aidlc/`, `:(top).aidlc/`, `:(glob)**/aidlc/spaces/*/intents/**/.aidlc-sensors/**` を `git reset -q HEAD --` で復元 (目的逐語: "restores framework-owned paths from HEAD so the later source merge carries application source only")。framework identity (`GIT_AUTHOR_NAME: "AI-DLC"`, `aidlc@localhost`) で commit し、object 書き込み後に fingerprint 再計算 (concurrent-edit window を閉じる)、`update-ref` で専用 ref に保持 | `:473-571`, `:548-553`, `:469-470` |

## 4. SwarmAttemptStamp と旧試行 replay 防止

09 §6.6:

| 項目 | 内容 | 行 |
|---|---|---|
| 型 | `SwarmAttemptStamp` = `{stage, floor}` | `:152-155` |
| 書き込み | `prepare` が一度だけ捕捉し `SWARM_STARTED` の `Stage` / `Run floor` フィールドに書く | `:588-598` |
| 前方伝搬 | `emitUnitConverged` は **prepare 時**の stamp を再計算せず持ち越す。理由逐語: "a late retry against a preserved prior-attempt worktree would otherwise be mislabeled as current" | `:616-618` |
| `preparedSwarmAttempt` | batch+unit に一致する `SWARM_STARTED` を audit shard から読み、stamped 行を優先、`(timestamp, shardIndex, pos)` でソート。最新 timestamp が複数 shard にまたがり stamp が**異なる**場合はファイル名で選ばず `null` (fail-closed): "Same-second starts in different shards are unordered. A shared stamp is harmless; differing stamps fail closed instead of picking by filename." | `:1194-1237` |
| `legacyPreparedSwarmAttempt` | 未 stamp 行の移行パス: worktree の `AUDIT_FORKED` の `Fork Boundary` byte offset と `Source Audit Hash` を main shard prefix の SHA-256 と照合し、frozen prefix 内に順序付き `SWARM_STARTED → BOLT_STARTED → STATE_FORKED` 列を要求してから worktree の `Current Stage` から stamp を導出 | `:1239-1330` |

## 5. SWARM_* 6 イベントと発行者制約

09 §6.8。**本ツールが swarm タクソノミの唯一の発行者**: "The engine is read-only and the conductor (prose) never emits audit events" (`:575-576`)。

| Event | 発行者 | フィールド |
|---|---|---|
| `SWARM_STARTED` | `prepare` | `Batch number`, `Unit names`, `Concurrency cap`, `Stage`, `Run floor` |
| `SWARM_DEGRADED` | `prepare` | `Batch number`, `Requested driver`, `Fallback driver` (常に `subagent`) |
| `SWARM_UNIT_CONVERGED` | `finalize` | `Batch number`, `Unit name`, `Stage`, `Run floor`, + `Source Fingerprint`+`Source Commit` または `Source Freshness Bypass` |
| `SWARM_UNIT_FAILED` | `finalize` | `Batch number`, `Unit name`, `Reason` |
| `SWARM_BATON_RETURNED` | `finalize` | `Batch number`, `Unit name`, `Reason` |
| `SWARM_COMPLETED` | `finalize` | `Batch number`, `Converged count`, `Failed count` |

補強 (03-state-audit-runtime.md §6.5-6.6, `:795`, `:818`): Swarm ファミリは登録簿で 6 イベント。`SWARM_STARTED` と `SWARM_UNIT_CONVERGED` は `CLI_PROTECTED_EVENT_TYPES` (18 種) に含まれ、`AIDLC_ALLOW_DIRECT_AUDIT_EVENTS=1` なしの直接 append を拒否 (拒否逐語は 03 §6.6: `Direct emission of <E> is blocked: it is an authority-bearing receipt owned by its emitting tool or hook (gate resolutions and approvals come from aidlc-orchestrate.ts report, interview answers and reviews from aidlc-log.ts, human presence from the prompt-submit hook). The audit CLI appends diagnostic events only.`)。また `MERGE_PROTECTED_EVENT_TYPES` の注記 (03 §6.6 / `aidlc-audit.ts:377-394`): "the referee's defence against a lying conductor is artifact re-verification at finalize, not delta [filtering]"。

batch 番号は join key: "The batch number is carried into the `Batch number` audit field and is the join key the swarm's `prepare`/`finalize` use to correlate a `SWARM_STARTED` boundary with a unit" (09 §5.7 / `aidlc-bolt.ts:202-204`, `:363-365`)。

## 6. engine 側 swarm アーム (02 §5.1-5.2)

| 項目 | 内容 | 行 |
|---|---|---|
| 発火順 | branch 10 (happy path) で in-flight stage → `tryEmitSwarm(...)` を**先に**試し、だめなら `emitForSlug(...)` | `aidlc-orchestrate.ts:3314-3328` |
| 発火条件 (4 条件 AND) | Construction stage かつ `for_each: unit-of-work` かつ `mode: subagent`、**skeleton-gate stage ではない**、`Construction Autonomy Mode` が正確に `autonomous` | `:3483-3589`, `:3400-3410` |
| **バッチ前進のキーイング** | `next` 1 回につき 1 Bolt batch 前進。**ディスク artifact ではなく `SWARM_UNIT_CONVERGED` audit 行にキーイング** | `:3446-3463` |
| settle 発行 | 全 unit 収束時、`swarm_settled: true` の settle `run-stage` を発行 — reviewer フィールドは strip、`protocol_modules: ["construction","swarm"]` | `:3435-3444`, `:3519-3532` |
| `settle` は swarm verb ではない | 3 verb がツール表面の全部。`settle` サブコマンドも pool 概念も無し (`grep -i -e settle -e pool` → 両ファイル 0 件)。batch→engine handshake は run-stage directive の任意フィールド `swarm_settled?: true` (`aidlc-directive.ts:210`、allow-list/検証 `:464`, `:490`, `:745`)、engine が post-swarm run-stage 再発行時に設定 (`aidlc-orchestrate.ts:3442`)、unit-attachment path ("the swarm settle", `:243`) で消費 | 09 §6.9 末尾 |
| `swarm_settled` 意味論 (directive スキーマ) | "Gate-only re-entry after every autonomous swarm unit and reviewer receipt converged; 'the conductor must not rerun the stage body or reviewer'" | 02 §4.3 / `aidlc-directive.ts:207-210` |
| continue_token payload | `z` = swarm-settled フラグを HMAC 署名 payload に含む | 02 §4.4 / `:1156-1175` |

## 7. swarm プロトコル (04 §7, `stage-protocol-swarm.md`) — conductor 側の 4 手順と終端規則

| 手順 | 内容 | 出典行 |
|---|---|---|
| 役割逐語 | "You — the live `/aidlc` session — are the conductor: you own the fan-out and the retry loop; `aidlc-swarm.ts` is the deterministic referee you consult, never a loop-owner" | `:16` |
| 1. prepare | `prepare --batch <n> --units <directive.units joined by comma> [--base main] [--repo <name>]`。`--repo` は directive の `repo`; multi-repo intent で directive が省略していれば `prepare` はエラー | — |
| 2. fan out | Claude Code では floor = 1 メッセージ内 N 並列 `Task`。`AIDLC_USE_SWARM=1` で inline Dynamic Workflow、Workflow tool 不在なら "loud-degrade to the floor" して `--degraded-from ultracode` を渡し `SWARM_DEGRADED` を発行させる。他 harness では subagent/spawn のみで `AIDLC_USE_SWARM=1` は無効 — "if it is set, say so out loud" | — |
| 3. check | 上記 §2 | — |
| 4. finalize | "a unit you wrongly claim is refused — the lying-conductor guard"。未列挙 declined unit は `cap-exhausted` デフォルト | — |
| **exit 0 分岐** | batch 収束+merge 済 → **stage を report せず** `next` を再実行。engine は次の `invoke-swarm` か、全 batch 収束後の settle `run-stage` で応答。逐語: "Reporting approved after an intermediate batch would complete the stage with later batches unbuilt." | `:16` |
| **exit 2 分岐** | failure envelope; baton を取り戻し construction module の halt-and-ask seam で停止 (halt-and-ask 自体は 04 §6.2 `:51-74`: "When a Bolt's code-generation returns failure, always halt and present the halt-and-ask prompt regardless of autonomy mode"、選択肢 Retry/Skip/Abort) | — |
| **merge_failures の scoped retry (B5)** | 逐語: converged but merge-back failed → "no `SWARM_UNIT_CONVERGED` row lands until the merge does" — ブロッカーを解決してその unit に**スコープした `finalize` を再実行** (`release-merge` は冪等)、**`prepare` は再実行しない** ("the existing worktree makes it error") | — |
| settled-swarm 再入 (自己完結規則) | 逐語: "`swarm_settled: true` is a gate-only directive emitted after every Unit body and reviewer receipt has converged. Do not run the stage body, dispatch builders, or dispatch a reviewer again. Run only the stage-level learnings ritual and approval gate, then report the human's result." | `:7-12` |

## 8. reviewer 付き claim の worktree 内 terminal REVIEW_COMPLETED 要求 (04 §7 `:18`)

| 項目 | 内容 |
|---|---|
| 原則逐語 | `invoke-swarm` が `directive.reviewer` を運ぶとき "a unit is not claimable at `finalize` merely because `check` passed." |
| 記録手順 | unit の prepared worktree 内で `REVIEW_REQUESTED` を `aidlc-log.ts review --stage "<directive.stage>" --unit "<unit>" --reviewer "<directive.reviewer>" --iteration <n> --project-dir "<worktree>"` で記録 → reviewer を `directive.stage_file` + その worktree の artefacts/contracts に対して dispatch → `--verdict <READY|NOT-READY>` で `REVIEW_COMPLETED` を記録。logger は main workspace に居て `--project-dir` が worktree を指す |
| NOT-READY ループ | 同一 worktree で lead を再起動し check を再実行、`directive.reviewer_max_iterations` まで反復 |
| recovery 消費後 | 1 回の recovery receipt が再度 invalidate されたら: claim しない、finalize しない、人間の Retry/Abort まで停止。Retry 時逐語: "return to the main workspace, abort and discard the old Bolt, then rerun the current `aidlc-swarm.ts prepare` step for that Unit with the original batch/base/repo arguments; the fresh `BOLT_STARTED` boundary resets review accounting without claiming convergence." |
| 禁止 | "Never synthesize `GATE_REJECTED`" (`stage-protocol-reviewer.md:132`) |
| finalize 側の enforcement | 上記 §3.4 (`reviewerReceiptError` の floor = `BOLT_STARTED`、`(Stage, Reviewer, Unit)` 一致、fingerprint 照合) |

参考 (04 §5.4-5.5、main workspace 側の一般 reviewer 前提条件): terminal receipt 後の `produces[]` 書き込みは receipt を invalidate ("a later write invalidates the receipt and the engine refuses the gate")、recovery は 1 回のみ (`Recovery: stale-receipt`, `aidlc-log.ts:1103`)、`aidlc-state.ts verifyReviewerPrecondition` (`:1775`) の拒否逐語 4 種は既存抽出範囲外なら 04 §5.5 参照。

## 9. autonomous Code Generation gate と §12b Plan Contract

### 9.1 prepare 側 gate (09 §6.3 手順 2 + 09 §8.6)

`evaluateCodeGenerationApproval(projectDir, unit)` (`aidlc-testing-posture.ts:925-1006`、swarm からの消費点 `aidlc-swarm.ts:727`) は `<docsRoot>/construction/<unit>/code-generation/` の 3 ファイル (`code-generation-plan.md`, `unit-test-instructions.md`, `code-generation-questions.md`) を読み、固定順・最初の失敗勝ちで検査。reason 逐語:

| # | reason (逐語) |
|---|---|
| 1 | `code-generation-plan.md is missing or empty` |
| 2 | `unit-test-instructions.md is missing or empty` |
| 3 | `code-generation-plan.md has no valid ## Testing Contract JSON block` |
| 4 | `the approved Testing Contract is stale because memory, scope, test strategy, or project type changed` (埋め込み `contract_sha256` ≠ 新規解決値) |
| 5 | `Plan Approval is not explicitly answered Approve Plan` |
| 6 | `the Plan Approval fingerprint does not match the current plan, test instructions, and Testing Contract` |
| 7 | それ以外 → `approved`、`ok: true` |

答えの正規表現: `APPROVE_PLAN_RE = /^(?:[A-Z][.)][ \t]*)?["']?Approve Plan["']?$/i` (`:98`)。最新の Plan Approval heading が勝つ (`latestPlanApproval` `:845-893`)。

### 9.2 §12b Autonomous Code Generation Plan Contract (04 §6.5 / `stage-protocol-construction.md:273-311`)

逐語: `invoke-swarm` は "changes where generation runs, not whether planning and Plan Approval happen." `aidlc-swarm.ts prepare` の前に 4 義務すべて:

1. `directive.units` の全 unit について Code Generation Part 1 を Plan Approval 準備まで **main workspace で**実行: `code-generation-plan.md` 作成、`aidlc-testing-posture.ts render` が出力した正確な `## Testing Contract` を埋め込み、`unit-test-instructions.md` 作成、現在の `[Approval Fingerprint]` を書き、当該 unit の Plan Approval 質問を提示。
2. 未回答の Plan Approval ごとに STOP。逐語: "Do not fork worktrees or dispatch implementation workers during these planning turns."
3. batch 内全 unit に現行の approval evidence が揃ってから `prepare` を呼ぶ; `prepare` は worktree 作成前に plan / test instructions / embedded contract / answer / fingerprint を検証。
4. 全 worker brief は正確に次で始まる (逐語):

   ```text
   AIDLC-UNIT: <unit>
   AIDLC-TESTING-CONTRACT: <contract_sha256 from that unit's approved plan>
   ```

   "The plan-approval guard rejects a delegated worker whose marker is missing, stale, or different from the approved plan."

## 10. converged / failed 判定の総括 (仕様執筆用まとめ)

| 終端状態 | 到達条件 | audit 行 | envelope / exit |
|---|---|---|---|
| `converged` | 6 段ガード全通過 (stamp 一致 + worktree 存在 + confinement OK + untampered + green + receipt/binding OK)、かつ merge-back 成功 | `SWARM_UNIT_CONVERGED` (prepare 時 stamp を持ち越し) | `converged` 配列、exit 0 側 |
| `error` (claimed-but-red / tampered 等) | ガード 1-5 のいずれかヒット、receipt/binding 失敗、または not-green fallthrough | `SWARM_UNIT_FAILED` + `SWARM_BATON_RETURNED` + best-effort `BOLT_FAILED` | `failed` 配列、exit 2 |
| `failed(unsatisfiable\|budget-exhausted\|cap-exhausted)` | 非 claimed unit。reason は `--reasons` から、デフォルト `cap-exhausted`。conductor は `error` を自己申告できない | 同上 | 同上 |
| **converged だが merge 失敗** | ガード全通過だが merge-back 失敗 | **行なし** (`SWARM_UNIT_CONVERGED` も `SWARM_UNIT_FAILED` も無し) — 中間状態 | `merge_failures`、exit 2。scoped `finalize` 再実行 (prepare 再実行禁止) で行が着地し、engine の batch 前進キー (`SWARM_UNIT_CONVERGED`) が初めて満たされる |
| batch 完了 | 全 unit 判定後 | `SWARM_COMPLETED {Batch number, Converged count, Failed count}` | exit: 失敗 or merge 失敗が 1 つでもあれば 2、なければ 0 |

出典ファイル (絶対パス):
- docs/upstream/specs/09-cli-tools.md (§5.7, §6.1-6.9, §8.6-8.7)
- docs/upstream/specs/02-orchestration-engine.md (§4.1, §4.3-4.4, §5.1-5.2)
- docs/upstream/specs/04-stage-protocol.md (§5.4-5.5, §6.1-6.5, §7)
- docs/upstream/specs/03-state-audit-runtime.md (§6.5-6.6: イベント登録簿と authority class)