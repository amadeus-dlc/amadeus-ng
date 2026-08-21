> 抽出元: upstream as-built 仕様 (awslabs/aidlc-workflows v2 @ 3c3146cf, v2.6.40)。2026-08-22 の 3 エージェント精密抽出。10-orchestration.md と engine_loop.qnt の執筆材料。

以下が抽出結果です(引用元は as-built 仕様のファイル番号+節番号、`file:line` は upstream ソースの行番号。英語の逐語契約は原文のまま)。

# タスク3: Directive カタログと運転系動詞の完全列挙

## 1. Directive カタログ — 10 種定義 / 8 種構築

出典: 02-orchestration-engine.md §4.1 (`aidlc-directive.ts:71-81`, `VALID_KINDS :419-430`)。判別子 allowlist はカタログ順で:

```text
"load-steering", "run-stage", "dispatch-subagent", "invoke-swarm",
"present-gate", "ask", "print", "error", "done", "parked"
```

`present-gate` と `dispatch-subagent` は placeholder(`aidlc-orchestrate.ts` ではコメント `:1031-1034` のみ)。`SKILL.md:89` は "Do not implement those two placeholder behaviours speculatively." と明示【02§4.1】。実測 8 種の構築内訳: error 15 / done 7 / load-steering 2 / invoke-swarm 2 / ask 2 / run-stage 1 / print 1 / parked 1 箇所【02 Measurement notes】。

| kind | 構築 | 意味(スキーマコメント) | 必須フィールド | 出典 |
| --- | --- | --- | --- | --- |
| `load-steering` | ○ | "one bounded part of the active stage's deterministic rule bundle"; 適用後ただちに `continue <continue_token>` | `stage`, `bundle`, `part`, `parts`, `rules_content[]{path,text}`, `continue_token` | 02§4.1; directive.ts:83-87 |
| `run-stage` | ○ | ルール読込→エージェント読込→`consumes` 読込→本体実行→`produces` 書出→`memory.md` 維持 | §2 参照 | 02§4.1; :138-143 |
| `dispatch-subagent` | ×(placeholder) | run-stage フィールド + `worker`(`Task` する named worker) | run-stage 共有 + `worker` | 02§4.1; :261-263 |
| `invoke-swarm` | ○ | "fan out N parallel workers across N worktrees for a build batch" | `units[]`(+任意 `stage`, `stage_file`, `reviewer`, `reviewer_max_iterations`, `review_class`, `protocol_modules`, `repo`) | 02§4.1; :288-289 |
| `present-gate` | ×(placeholder) | §13 learnings ritual → 承認ゲート描画 | `stage`, `phase`, `memory_path` | 02§4.1; :320-321 |
| `ask` | ○ | 構造化質問の描画; 2 サブタイプ(下記) | `question` | 02§4.1, §4.5 |
| `print` | ○ | "print verbatim and stop (status / help / doctor / version)" — 実際は run-then-continue / run-then-stop 形も | `message` | 02§4.1; :358 |
| `error` | ○ | "stop with an error … shown to the user verbatim" | `message` | 02§4.1; :366-367 |
| `done` | ○ | "stop the loop (workflow or single-stage complete)" | `reason` | 02§4.1; :375-376 |
| `parked` | ○ | "the workflow was intentionally parked mid-flow … Distinct from `done` … a parked workflow has in-scope stages still pending" | `reason`, `stage` | 02§4.1; :384-389 |

- `narration` は**全 kind** で合法。`withNarration`(directive.ts:520-544)が中央で全 allowed-key set に折込み。"a presentation field: it carries no routing meaning, every kind may omit it, and dropping it changes nothing about what the framework does"(:40-43)【02§4.1】。
- `ask` サブタイプ【02§4.5, directive.ts:335-356】: 通常の `ReportAskDirective`(回答は `report --user-input` 経由)と `NewWorkRoutingAskDirective`(`ask_type: "new-work-routing"`, `response_route: "next"`, `new_work_description`, `proposed_scope` を保持)。"its answer routes through `next` and must never be recorded as a stage report"(:332-334)。

### 1.1 検証規則 (`validateDirective`, directive.ts:553-701)【02§4.2】

`{valid:true,data}` か `{valid:false,errors[]}` を返し、最初のエラーで throw せず全フィールドエラーを収集。順序:

| # | 規則 | エラーメッセージ(逐語) | 行 |
| --- | --- | --- | --- |
| 1 | Shape | `expected object, got <null\|array\|typeof>`(単一エラー) | :557-561 |
| 2 | 判別子(短絡) | `missing or non-string required field: kind` / `unknown kind: "<k>" (expected one of <kinds joined by " \| ">)` | :566-576 |
| 3 | 未知キー | `<kind>: unknown key: <key>` | :579-585 |
| 4 | 型/必須 | `<kind>: missing required field: <f>` / `<kind>: <f> must be string, got <desc>`; 正整数・`{path,text}[]`・`{path,expected}[]`・`pipeline` オブジェクト・`protocol_modules` enum・ネスト `wave` の特殊検査 | :764-777, :829-1199 |
| 5 | cross-field | `load-steering`: `part must be less than or equal to parts`(:603-611); `review_class` 保持 kind で reviewer 欠落: `<kind>: review_class requires reviewer`(:630-632, :740-742); `ask` サブタイプ: `ask_type must be one of new-work-routing` / `new-work-routing response_route must be "next"` / `<field> requires ask_type "new-work-routing"`(:647-670) | — |

- `checkGate`(:783-799): boolean **または**リテラル `"unresolved"` のみ受理。他は `<kind>: gate must be boolean or "unresolved", got <desc>`。
- 成功時は**同一参照**を返す — コードベース唯一の trust-boundary cast(:693-700)。
- 自己検査 `bun core/tools/aidlc-directive.ts`: 全 10 kind を覆う 12 例、各 `<kind>: VALID`、全通過で exit 0(:1239-1362)【02§4.2】。
- コメントドリフト: directive.ts:566 の "one of the 8"、:1241 の "10 examples" はいずれもコメントのみ誤り【02§14】。

### 1.2 28KiB 上限と emission 規律【02§3.2】

パイプライン: `prepareEmission`(orchestrate.ts:233-304)→ `validateDirective` → serialize → size check → `writePrepared` が **stdout に JSON 1 行のみ**(:306-308)。ハード拒否 2 種(いずれも `process.exit(1)` + stderr):

- `aidlc-orchestrate: refusing to emit a malformed directive: <errors joined by "; ">`(:259-262)
- `aidlc-orchestrate: refusing to emit a directive larger than ${DIRECTIVE_MAX_BYTES} bytes`(:266-268)

定数(orchestrate.ts:1140-1143): `DIRECTIVE_MAX_BYTES = 28 * 1024`(28 672 bytes, "the common 28 KiB harness floor"); `STEERING_TEXT_TARGET_BYTES = 20 * 1024`; `CONTEXT_WARNINGS_MAX_BYTES = 6 * 1024`; `INLINE_CONTEXT_PATHS_MAX_BYTES = 8 * 1024`。チャンク化は Markdown 見出し境界→ JSON ワイヤサイズ実測でコードポイント境界分割(:2170-2243)、20KiB 目標で pack(:2245-2260)。分割不能な節は `A rule section could not be split below the directive transport limit. Shorten the affected heading section, then run a fresh \`next\`.`(:2544-2548)【02§10】。

## 2. `run-stage` 封筒の全フィールド【02§4.3, `RUN_STAGE_FIELDS` directive.ts:442-470】

`DISPATCH_SUBAGENT_FIELDS` = 同一リスト − `single`, `wave`, `protocol_modules`, `swarm_settled` + `worker`(:484-493)。

### ルーティング

| フィールド | 型 | 決定元 | 出典 |
| --- | --- | --- | --- |
| `stage`, `phase`, `lead_agent`, `support_agents`, `mode`, `sensors_applicable`, `stage_file` | routing | コンパイル済みグラフノードから直読 | orchestrate.ts:2044-2071 |
| `mode` | enum | `inline\|subagent\|pipeline\|mob\|agent-team`; `agent-team` は予約・未生成 | directive.ts:435; orchestrate.ts:2050-2054 |
| `next_stage` | `string\|null`? | エンジンが解決 — "resolved by the engine so the approval gate's Approve option can read 'Continue to <next_stage>' verbatim"; `null` = 最終 in-scope ステージ | directive.ts:217-224; orchestrate.ts:2092-2093 |

### コンテキスト

| フィールド | 型 | 決定元 | 出典 |
| --- | --- | --- | --- |
| `inline_context_paths` | string[] | インライン担当分のペルソナ+知識ファイル(inline: lead+supports / mob: lead のみ / subagent・pipeline: 空) | directive.ts:161-166 |
| `context_warnings` | string[]? | 非致命的 roster 問題; 6KiB 上限。ルール読込失敗は blocking `error` に昇格 | orchestrate.ts:1971-2000; directive.ts:167-171 |
| `rules_in_context` | string[] | 先行 `load-steering` 連鎖で配送済みルールテキストの順序付きパス manifest | orchestrate.ts:2489-2491 |
| `memory_path` | string | `<recordPrefix>/<phase>/<slug>/memory.md`、per-unit は `<recordPrefix>/construction/<unit>/<slug>/memory.md` | orchestrate.ts:1086-1098 |
| `conductor_persona` | string? | `aidlc-common/conductor.md` 全文。"Decision D-E: bake the conductor persona into the FIRST run-stage of the workflow"(:2132-2133); `forcePersona \|\| isFirstRunStageOfWorkflow(...)` で添付(:2139-2143)、以後省略 | orchestrate.ts:1121-1129 |

### 成果

| フィールド | 型 | 決定元 | 出典 |
| --- | --- | --- | --- |
| `consumes` | string[] | 宣言入力のうち**emit 時にディスク上に存在するもののみ** | directive.ts:177-180 |
| `consumes_absent` | `{path,expected}[]?` | emit 時欠落の必須入力。`expected:true` = 生産ステージがスコープ経路外("absence is by design; substitute available context, do not invent the artifact"); `expected:false` = 経路上なのにファイル欠落(回復プロトコル入力) | directive.ts:246-258 |
| `produces` | string[] | 解決済みパス。`produces_kinds` で kind フィルタ、`optional_produces` 含む | orchestrate.ts:1705-1732 |

### 統制

| フィールド | 型 | 決定元 | 出典 |
| --- | --- | --- | --- |
| `gate` | `boolean\|"unresolved"` | `computeGate`(§6 参照): initialization → `false`; skeleton-gate ステージで stance 未記録 → `"unresolved"`; 他は `true` | orchestrate.ts:1756-1771 |
| `reviewer`, `review_class`, `reviewer_max_iterations` | 任意 | ステージが reviewer を宣言**かつ**解決 class ≠ `none` のときのみ。`advisory` は iterations=1 固定、`adversarial` はデフォルト 2。`none` はブロックごと省略 | orchestrate.ts:2094-2113 |
| `protocol_modules` | enum[]? | `["reviewer","ensemble","construction","swarm"]` 上の決定的ヒント | directive.ts:62-68; orchestrate.ts:2114-2131 |
| `pipeline` | `{links,completed}`? | `mode: pipeline` の回復サーフェス | orchestrate.ts:2072-2078 |

### 反復

| フィールド | 型 | 決定元 | 出典 |
| --- | --- | --- | --- |
| `unit` | string? | 具体 Unit of Work に解決された per-unit Construction directive のみ; "a marker that this run-stage is ONE iteration of N" | directive.ts:225-236 |
| `wave` | `{batch_index,entries[]}`? | 4 つの inline per-unit 設計ステージ用 stage-major 並列サーフェス; entry 検証は duplicate-unit と `required_produces ⊆ produces` を含む | directive.ts:238-245, :1029-1199 |
| `swarm_settled` | `true`? | 全 autonomous swarm unit + reviewer receipt 収束後の gate-only 再入; "the conductor must not rerun the stage body or reviewer" | directive.ts:207-210 |
| `single` | boolean? | 隔離 stage-runner マーカー(§5 参照) | 02§9 |

### ダイジェスト束縛と continue_token【02§4.4】

| ダイジェスト | 計算 | 束縛対象 |
| --- | --- | --- |
| `bundle: "sha256:<hex>"` | `sha256(JSON.stringify(loaded.content))`(orchestrate.ts:2492) | ルールテキストバンドル |
| `directiveHash` | `sha256(JSON.stringify(directive))`(:2493) | 配送先 run-stage |
| route hash `r` | `sha256(JSON.stringify({node, scopeStages:...}))`(:2467-2474) | グラフノード+スコープのステージ membership |
| `state_sha256` / `h` | `sha256(stateContent)`(:2156, :5974) | 計算元 state ファイル |

`continue_token` = HMAC-SHA256 封筒 `{p: payload, m: mac}` base64url(:2358-2372)、decode 時 `timingSafeEqual`(:2395-2405)。payload フィールド(:1156-1175, :2438-2465): `v`(=1), `s` stage, `c` scope, `i` next part index, `b` bundle digest, `d` directive digest, `r` route hash, `a` state-aware flag, `u` unit, `k` unit kind, `f` force-persona, `g` gate, `n` next_stage, `x` single, `p` per-unit, `w` wave, `z` swarm-settled, `h` state hash。型表に合わない payload は拒否(:2409-2431)。staleness 規則(02§10): stage/bundle/directiveHash 不一致・`i > chunks.length`・state digest 不一致・stage がグラフから消失・route hash 不一致 — いずれも fail-closed で "Run a fresh \`next\`" 系の逐語メッセージ。

## 3. Jump — resolve/execute 分離【02§8】

- **分離**: `aidlc-jump.ts` は `resolve`(純読取: target+direction, `:108-217`)と `execute`(mutating commit, `:221-479`)。`next` の `emitJumpDirective`(orchestrate.ts:4530-4646)は resolve を shell し、commit は mutation なので `print` で命名: `` Run `bun <harness>/tools/aidlc-jump.ts execute --target <slug> --direction <dir> --scope <scope>` to perform the jump, then re-run `next` to continue from the jump target. ``(:4577-4579)。
- **方向計算**: `Current Stage` とのインデックス比較 → `forward` / `backward` / `redo`(jump.ts:175-181)。
- **拒否条件**:
  - `--stage`+`--phase` 併用: `Cannot use --stage and --phase together. Use one or the other.`(orchestrate.ts:2780-2784)【02§5 Branch 2】
  - init ガード(エンジン側で施行 — resolve は init を有効ターゲット扱いするため): `INIT_JUMP_ERROR` = `Cannot jump to initialization stages. The Initialization phase runs automatically when you start a workflow (describe what to build, e.g. /aidlc "build the auth service").`(:4527-4528, :4521-4526)
  - スコープ外: `Stage "<slug>" is skipped for scope "<scope>". Choose a different stage or change scope.`(jump.ts:141-144; single と同一文言を意図的共有)
  - resolve payload 不正: `Internal: aidlc-jump.ts resolve returned no target_slug/direction for …`(:4557-4562)
- **state 無し**: resolve は direction のアンカーに state を要するため、直接グラフ検索で "start here" の素の `run-stage` を emit(スコープ membership ガードは with-state と同文言、:4583-4645)。
- **execute の効果**(jump.ts:221-479; すべて*実効*プラン基準 — state suffix override が scope grid に勝つ, :34-40):

| 方向 | チェックボックス効果 | 監査 |
| --- | --- | --- |
| `forward` | 介在する in-flight ステージ → `skipped`; 現ステージも in-flight かつ非 pending なら skipped | skip 毎に `STAGE_SKIPPED` 1 行 |
| `backward` | ターゲット+下流の全 EXECUTE ステージ(`completed/in-progress/awaiting-approval/revising/skipped`)→ `pending` | — |
| `redo` | ターゲット → `pending` | — |

全ケース共通: ターゲットを `in-progress` にし、`Lifecycle Phase`, `Current Stage`, `Next Stage`, `Active Agent`, `Status=Running`, `Last Updated`, `In Progress`, `Next Action`, `Completed`, `Last Completed Stage` を書換え(:342-414)。フェーズ境界越えは `PHASE_COMPLETED` + `PHASE_VERIFIED` + `PHASE_STARTED` を emit し Phase Progress 行を書換え(:378-442)。毎回 `STAGE_JUMPED`(Direction/Source/Target/Scope/Details)+ ターゲットの `STAGE_STARTED`; **監査 emit は `writeStateFile` より先**で、emit 失敗は write を中止(:416-463)。

## 4. Park / `parked` directive / 再開【02§11】

- **park**: `handlePark`(orchestrate.ts:5937-5957)→ `aidlc-state.ts park` を shell。同 park は: autonomous 下で拒否 — `Refusing to park: Construction Autonomy Mode is autonomous. An unattended autonomous run has no human to resume it and must keep moving - do not park it.`(aidlc-state.ts:796-800); 完了済み workflow を拒否; `Current Stage` 必須; `WORKFLOW_PARKED` emit; `Parked` / `Parked At Stage` runtime フィールド書込(:811-815)。エンジンは terminal `parked` directive を emit、narration: `` Pausing here with everything saved. Run `/aidlc --resume` when you want to pick it back up. ``(:662-672)。非ゼロ exit は `Cannot park the workflow: <detail>`。
- **parked branch(next Branch 2.5)**: `Parked` セット済みかつ `Parked At Stage === Current Stage`、再入フラグなしで `parked` emit: `Workflow parked at "<slug>". Resume with /aidlc --resume.`(:2830-2848)【02§5】。
- **自己無効化フラグ**: park branch は `!flags.resume`, `!flags.stage`, `!flags.phase`, `!flags.review`, `!flags.newIntent` を要求(:2830-2838)し、**stale-by-progress**(`Parked At Stage === Current Stage` の間のみ発火, :2839-2848)。
- **再開**: parked 上の `--resume`(Branch 2.6)はまず `aidlc-state.ts unpark`(`WORKFLOW_UNPARKED` emit, aidlc-state.ts:825-839)を `print` で命名 → 次の `next` で resume `ask`(Branch 6): `An existing workflow was found (currently at "<slug>"). How would you like to proceed? Resume from last checkpoint, redo the current stage, jump to a stage, or start fresh.`(:3084-3087)。`handleResumeReport`(:5383-5457)は `--stage` を拒否(`A resume-choice report is not a stage transition; omit --stage.`)、`--user-input` 必須、数字メニュー 1–4 正規化、mutate せず**route**: redo → `aidlc-jump.ts execute --direction redo`; jump → ステージ質問後 `next --stage <slug>`; start fresh → `next --new-intent --scope <s> "<desc>"`; resume → `next` 再実行。
- **`parked` が独立 kind である理由**(directive.ts:384-389): "The Stop hook treats `parked` as a terminal allow, so the conductor can end its turn at a clean inter-stage boundary instead of rubber-stamping stages to reach `done`."。ただし hook 側(aidlc-continue-workflow.ts:1273-1280)は **autonomous Construction 下では parked allow を拒否**し cap-bounded block へ落ちる【07§7.4 carve-out 3】。

## 5. Single-stage mode の不変条件【02§9】

**不変条件**(orchestrate.ts:4418-4439, :5232-5260): **`--single` 実行はメイン workflow の `Current Stage` に決して触れない。**

- **Emission**(`emitSingleRunStage` :4443-4489; Branch 4b — scope-change・jump 分岐より前なので mutation 経路に到達不能)。ガード順(逐語・live 検証済):
  1. `Cannot use --single with --phase. --single runs one stage; pass --stage <slug>.`
  2. `--single requires --stage <slug>. A stage-runner runs exactly one named stage.`
  3. `Unknown stage "<slug>". Run /aidlc --help for the full list.`
  4. `SINGLE_INIT_ERROR`: `Cannot run an initialization stage with --single. Initialization is bootstrap (it creates the intent + state); it runs automatically when you start a workflow …`
  5. `Stage "<slug>" is skipped for scope "<scope>". Choose a different stage or change scope.`(jump 経路と同一文言・意図的)
- directive 構築は `stateContent: null`("no main state read, no skeleton round-trip, no main-pointer persona signal")、`single = true`, `gate = false`, `next_stage = null`、ペルソナ強制添付(:4469-4488)。
- **Commit**(`handleSingleReport` :5261-5361; report ガード順の #2 で最優先解決 — state-mutating subcommand へ絶対に fall-through しない, :5490-5499)。forward verdict のみ受理; `--stage` **必須**:

  ```text
  report --single must not advance the main workflow. Pass --stage <slug> to commit the
  single stage's synthetic-id pair; --single never writes the main workflow's Current Stage.
  ```

  shell 先は `aidlc-audit.ts append-batch` のみ(:4899-4931)— `advance`/`approve`/`complete-workflow` は決して呼ばず "mechanically incapable of advancing the main workflow"。書込ペアは `STAGE_STARTED {Stage, Agent, Workflow}` + `STAGE_COMPLETED {Stage, Details, Workflow}` で `Workflow` は **synthetic id** `single-stage:<slug>`(`syntheticWorkflowId` :5017-5019)。"can never satisfy the MAIN workflow's guard"(:5254-5260); practices-affirmation floor scan は `Workflow` が `single-stage:` で始まる `STAGE_STARTED` 行を明示的にスキップ(:4806-4810)。終端は `done`: `Single-stage run of "<slug>" committed under synthetic workflow "<wf>". The main workflow's Current Stage is untouched.`
- conductor 側束縛(SKILL.md:66): `directive.single === true` を通常ゲート処理より**先に**分岐、本体+reviewer 実行、`report --single … --result completed` を正確に 1 回、返る `done` は terminal — "Do not run the workflow learnings ritual, report `awaiting-approval`, present a workflow gate, call main-workflow `next`, or park."
- runner 生成(aidlc-runner-gen.ts): runnable = 非 initialization の全コンパイル済みステージ(:101-117)、当該コミットで 30 個。ドリフトガードは本体中の `--stage <slug> --single` リテラル対 `/--stage\s+([a-z][a-z0-9-]*)\s+--single/`(:413-417)で stage runner を識別。

## 6. Recompose の 8 ガードと `effectivePlanAction`【01§9.7, 09§4.2】

`recompose` は `aidlc-utility.ts` の決定的 in-flight write(`core/tools/aidlc-utility.ts:5106-5340`)。live state ファイル上の per-stage EXECUTE/SKIP suffix を flip し、これが読取時に scope grid を override する — `effectivePlanAction`(orchestrate.ts:2562-2571): **state ファイルの per-stage EXECUTE/SKIP suffix(recomposition)が静的 scope grid に勝つ**【02§5.1】。ガード順:

| # | ガード | 逐語/根拠 | 行 |
| --- | --- | --- | --- |
| 1 | flip を最低 1 つ命名 | `Usage: recompose [--skip <slug,...>] [--add <slug,...>] - name at least one flip.` | :5120 |
| 2 | 実行中 workflow 必須・`Status` = `Running` | `recompose re-shapes a RUNNING workflow; start one first.`(state 無し時) | :5129, :5161 |
| 3 | Autonomy ガード — Construction autonomy 有効中は拒否 | — | :5141-5148 |
| 4 | 命名 slug は全てコンパイル済み | — | :5189-5191 |
| 5 | 命名 slug のチェックボックスは **pending** | `` its checkbox is not pending ([${state}]). Only a PENDING stage's plan can be re-shaped; completed/in-progress/skipped stages are frozen. `` | :5195 |
| 6 | 命名 slug は全てカーソルより前方 | "In-flight recompose only reaches forward; re-running the past is out of scope." | :5199 |
| 7 | walking-skeleton アンカーを動かさない | (01§6.4) | :5210-5225 |
| 8 | strict 検証を **diff** として: flip 後に新規出現した strict error のみ拒否; `[x]` 済ステージは両 grid で EXECUTE 扱いにし偽の starvation を防ぐ | strict モード suffix: `"Strict (recompose) mode rejects a starved required input."`(01§7 の `opts.strict`) | :5228-5245 |

補足(09§4.2 :189): flip は `withAuditLock` 下、派生フィールド再構築、`RECOMPOSED` 監査(:5104-5116)。同一 slug が `--skip` と `--add` 両方に出ると拒否(:5124)。エンジン側 Branch 4c: `Cannot combine compose with --stage/--phase. Compose re-shapes the plan; jump moves the cursor. Run them separately.`(orchestrate.ts:2943)【02§5】。

## 7. Construction Autonomy Mode と `set-autonomy`【09§5.6, 03§(discrepancy), 02§5.2】

- 2 値フィールド `autonomous` / `gated`。唯一の書込 verb は `aidlc-bolt.ts set-autonomy --mode autonomous|gated`。decision ladder / `decide-question` は upstream tree に存在しない(git grep 0 件)【09§5.6】。
- `handleSetAutonomy`(aidlc-bolt.ts:804-859)の規則:
  1. 全処理が単一 `withAuditLock` 内 — "One lock covers presence check -> audit consume -> state write. Otherwise two grants, or a grant racing approval, can both observe one fresh turn"(:813-814)。
  2. **昇格のみ** human-presence ガード: `autonomous` への切替は `humanActedSinceGate(pd)` 必須(`humanPresenceGuardDisabled()` 時除く)。`gated` への降格は "restores gates without presence"(:816-818)。
  3. 拒否逐語(:825-829): `Refusing to switch Construction to autonomous: a real human has not acted since the last gate resolution, and autonomous mode is granted only by the human's ladder-prompt answer (it waives every later gate, so the grant itself needs a fresh human turn). Ask the human to confirm autonomous mode in a typed message, then retry. Do not log the ladder choice via aidlc-log answer; the choice is recorded by set-autonomy itself.`
  4. その後 `setFieldStrict("Construction Autonomy Mode", mode)` → `AUTONOMY_MODE_SET` emit → state 書込(audit-first)。
  - 無効 mode は前段拒否: `Invalid --mode: <m>. Must be 'autonomous' or 'gated'.`(:808)。
- **既知の不整合(03 M12)**: state テンプレートは `Construction Autonomy Mode` を宣言するが birth emitter は書かない。読み手は `getField` → `null` = 非 autonomous で安全劣化。だが唯一の書き手 `set-autonomy` は `setFieldStrict` を使い `setOrInsertField` サイトが無いため、生まれたての state ファイルでは `State update failed: Field not found in state file: "Construction Autonomy Mode". …` で失敗する【03§discrepancies】。
- **autonomous が効く箇所**(完全列挙):
  - swarm arm 発火条件(02§5.2): Construction ステージ+`for_each: unit-of-work`+`mode: subagent`+非 skeleton-gate+`Construction Autonomy Mode` が正確に `autonomous`(orchestrate.ts:3400-3410)。
  - park 拒否(上記 §4)。
  - report の human-presence ガード免除(02§7.2 #13: gated 未完ステージ・autonomy 非 autonomous・`AIDLC_SKIP_HUMAN_PRESENCE_GUARD !== "1"` のとき blank `--user-input` を拒否 → autonomous なら免除)。
  - recompose ガード 3(上記 §6)。
  - Stop hook: cap 8(下記)、parked allow 拒否、carve-out 1/6/7/8/9 の autonomy ガード、autonomous Code Generation gate(09§8.6)。

## 8. Stop フック forwarding loop【07§7】

`aidlc-continue-workflow.ts`(1421 行)。目的: "when the conductor tries to end its turn, this hook runs the engine (`aidlc-orchestrate next`) and, if a directive is still PENDING, blocks the stop and injects the directive back via `reason`"(:14-19)。

### 8.1 プローブ【07§7.3】

- `ENGINE_TIMEOUT_MS = 10_000`(:194)で時間制限、`AIDLC_STOP_HOOK_PROBE=1`(`STOP_HOOK_PROBE_ENV`)を付けて spawn(:939)。この env は load-bearing: `markEngineTouch` がこれを見て no-op し、プローブが自分の conversational carve-out を無効化しない(:926-933)。
- 非ゼロ exit / 空 stdout / JSON parse 不能 → `null` で stop を allow。
- `runEngineNextDirective`(:944-1021)は engine stdout から fingerprint 用フィールドを防御的に parse(型不一致フィールドは drop)。

### 8.2 `decision:block`【07§1.1, §7.1】

stdout に `{"decision":"block","reason":…}`(:206)。reason は**on-task continuation** であり override 形でない(:22-27)。一般形(:1062-1071):

> `` The AIDLC workflow has a pending step (a <kind> directive for "<stage>"). You have not finished the workflow loop yet. Run `bun <harness>/tools/aidlc-orchestrate.ts next`, do what the step it prints asks, then run `aidlc-orchestrate report --stage <stage> --result <outcome>` to record the outcome. Repeat until it answers `done`. If you meant to pause this workflow instead and pick it up in a later session, run `bun <harness>/tools/aidlc-orchestrate.ts park` to stop cleanly between stages - never mark a stage complete just to end the turn. ``

他 4 形: `rehydrate` 変種(fresh `next` を 1 回要求・旧 continuation token 再利用禁止, :1042-1044); Copilot session-owned path 用の retained `load-steering` / retained `run-stage` 変種(:1045-1050); `rules_content` JSON 全文と `continue "<token>"` コマンドをインラインし "Do not summarise or narrate these rule chunks to the user" と指示する `load-steering` 変種(:1051-1061)。

### 8.3 No-progress cap【07§7.2】

- 2 つの bound: payload の `stop_hook_active`(前 block の産物である信号)と、耐久 no-progress カウンタ `<record>/.aidlc-stop-hook/block-count.json`(`{signature, count}`, :232-234)。
- `blockCap`(:171-186): `CLAUDE_CODE_STOP_HOOK_BLOCK_CAP`(正整数)が勝つ; さもなくば `Construction Autonomy Mode: autonomous` → `AUTONOMOUS_BLOCK_CAP = 8`、それ以外 → `INTERACTIVE_BLOCK_CAP = 2`。非数値・非正の override はモード既定に fallback(ガード無効化はしない)。
- `decideBlock`(:340-377): 現 signature == 保存 signature → `count + 1`; 記録なしだが `stop_hook_active` → **2 で seed**(進行中シーケンスへの合流); それ以外 → 1。記録は判定**前**に書込。`nextCount >= cap` で release。`resetGuard`(:382-390)は `done`・`parked`・fresh-session handoff 境界でゼロ化。
- **progress signature**(`progressSignature` :247-284)= `"<stage>::<stateSha256>::<directiveFingerprint>"`:
  - stage = `Current Stage` slug。
  - stateSha256 = **`- **Last Updated**:` 行を strip した** state ファイルの SHA-256(:249-253; v2.6.40: "status-only timestamp writes" が stuck loop をリセットしないため)。
  - directiveFingerprint = `kind`, `stage`, `unit`, `part`, `parts`, `continue_token_sha256`, `rules_content_sha256`, `units`, `worker`, `repo`, `wave_sha256` の JSON オブジェクトの SHA-256(:254-282; v2.6.40: chunk/wave/batch の前進が streak をリセットするように)。

### 8.4 Carve-out 固定順【07§7.4】(各項は allow のみ可能、block は起こさない)

| # | 条件 | 根拠 | autonomy ガード |
| --- | --- | --- | --- |
| 0 | Fresh post-create session handoff | `SESSION_INTENT_HANDOFF_TTL_MS`(5 分)内の receipt、`from`/`to` UUID が session stamp と live cursor に一致(:1148-1170) | n/a |
| 1 | **Resume wait**(v2.6.40) | `hasCurrentSharedResumeWait` — **`next` プローブより先に**読む(:1209-1229)。marker が `version===2`・`owner_session` が `"sessionless:"` 始まり・`state_sha256` 一致・`kind==="ask"`・`resume.status==="waiting"`・非 autonomous のときのみ true(aidlc-lib.ts:3005-3022) | yes |
| 2 | `kind === "done"` | engine directive(:1253-1256) | n/a |
| 3 | `kind === "parked"` | engine directive(:1273-1284) | **yes** — autonomous run は parked allow を拒否し fall-through |
| 4 | `kind === "ask"` | engine directive(:1289-1291) | no |
| 5 | Human-wait gate | 現ステージのチェックボックスが `[?] awaiting-approval` か `[R] revising`(`isHumanWaitStop` :428-438) | no |
| 6 | Pending mid-stage question | `<slug>-questions.md` に `/\[Answer\]:[ \t]*_*[ \t]*$/m` 一致の未回答 `[Answer]:` タグ(:474-511) | yes — 例外: unit-major `code-generation` の Plan Approval は必須(:527-537) |
| 7 | Pending logged decision | 現ステージの `DECISION_RECORDED` で後続 `QUESTION_ANSWERED` なし(`isPendingDecisionStop` :560-573 → `hasPendingDecision(projectDir, slug, "STAGE_STARTED")`) | yes |
| 8 | Pending compose proposal | `aidlc/.aidlc-compose-pending` marker が `COMPOSE_MARKER_TTL_MS` = 24h 未満; stale marker は unlink して無視(:603-629) | yes |
| 9 | Conversational turn | transcript(Claude JSONL / Codex rollout)または turn marker 比較(:869-890) | yes |

- carve-out 9 の 2 証拠経路(:77-101): *transcript path* は最新の genuine human prompt が engine call ゼロで応答済みであることを要求、合成 user turn(`isMeta:true`, `tool_result` 配列、hook 自身の再注入 nudge — `Stop hook feedback:` または `The AIDLC workflow has a pending step` で始まり `workflow loop` を含むテキスト, :669-676)を除外。*marker path* は `.aidlc-human-turn` と `.aidlc-engine-touch` の mtime 比較で human が厳密に新しいときのみ true(aidlc-lib.ts:6065-6088)。marker path は `aidlc-jump`/`aidlc-bolt`/`aidlc-swarm`/mutating `aidlc-state` verb に盲目という非対称が明記(:95-101)。
- 順序要件の明文(:104-106): "we must read this latch BEFORE probing `next`, because the probe publishes its own sessionless directive and can overwrite the `ask` kind"。
- Copilot session-owned path(:1202-1235): `AIDLC_COPILOT_SESSION_ID === session_id` のとき probe せず `copilotStopEvidence` を読む。`foreign`/`resume` → 即 allow; `contended` → drop 付き allow; `directive` → retained directive; それ以外 → `{kind:"rehydrate", retained:true}` 合成。カウントは token/state digest・resume status/action・owner session/epoch を pipe-join した identity で `updateCopilotStopCount`(:1381-1388)。
- プローブ前の usage folding: Claude transcript を `flush-all` で fold(:1182-1198)、Codex rollout は放置、throw は swallow。

## 補遺: 関連ファイルパス

- docs/upstream/specs/02-orchestration-engine.md(§3.2 emission、§4 protocol、§5 next ladder、§8 jump、§9 single、§11 park)
- docs/upstream/specs/01-workflow-model.md(§9.7 recompose 8 ガード)
- docs/upstream/specs/09-cli-tools.md(§5.6 set-autonomy、§5 bolt、§6 swarm)
- docs/upstream/specs/07-hooks.md(§7 Stop hook)
- docs/upstream/specs/03-state-audit-runtime.md(Construction Autonomy Mode の M12 不整合)