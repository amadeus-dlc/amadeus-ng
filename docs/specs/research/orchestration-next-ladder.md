> 抽出元: upstream as-built 仕様 (awslabs/aidlc-workflows v2 @ 3c3146cf, v2.6.40)。2026-08-22 の 3 エージェント精密抽出。10-orchestration.md と engine_loop.qnt の執筆材料。

以下、02-orchestration-engine.md (as-built 仕様、upstream commit `3c3146cf` v2.6.40) からの精密抽出。行番号表記 `:NNNN` は仕様が引用する upstream ソース (`core/tools/aidlc-orchestrate.ts` 等) の行番号、`§x.y` は 02-orchestration-engine.md の節番号。逐語契約はすべて原文のまま。

# タスク 1 抽出結果: `next` 決定ラダーと周辺プロトコル

## 1. `handleNext` の 21 分岐ラダー完全列挙 (§5)

`handleNext` は `aidlc-orchestrate.ts:2587-3357` の**フラットな 21 ラベル付き分岐ラダー**。実行順に列挙する (仕様 §5 の表を完全転記+補足)。ラベル `—` の行はラベルなしの前置ガードで、21 のカウントに含まれない。

**21 ラベルの正確な集合** (§Measurement notes, `grep -cE '^  // Branch ...'` = 21):
`0, 1, 1b, 1c, 1d, 2, 2.5, 2.6, 3b, 4, 4a, 4b, 4c, 5, 6, 7, 7b, 8, 9, 9c, 10`
(注意: §5 の表は `9a`/`9b` を別行で示すが、これらはラベル `9` の 2 つの腕であり、計測上は 1 ラベル)

| # | 条件 (観測される状態) | 出力 | 備考 (ミューテーション・逐語契約) | upstream 行 |
|---|---|---|---|---|
| — | ターンシェイプマーカー | — | `touchEngineMarker`。read-only/workspace モードでは実行しない | `:2605` |
| — | `flags.parseError` | `error` | 例: `--review requires <adversarial\|advisory\|none>.` | `:806` |
| — | `--review` を他モードと併用 | `error` | `Cannot combine --review with read-only, workspace, compose, single-stage, jump, or resume modes. Apply /aidlc --review <class> first, then run the other command.` | `:2629-2631` |
| 0 | Kiro roll-forward ラッチ: `.aidlc-readonly-latch` と同一ターンカウンタでの真に裸の `next` | `done` | advisory、fail-open (失敗しても続行) | `:2635-2681` |
| 1 | read-only フラグ (`--status`/`--help`/`--doctor`/`--version`) | `print` | `aidlc-utility.ts <sub>` を名指し。"This is a read-only utility, NOT workflow work: do NOT run `next`" | `:2697-2709` |
| 1b/1c/1d | workspace / plugin / knowledge の名詞トークン | `print`/`error` | 先頭トークン意味論のみ (leading-token semantics only) | `:2711-2775` |
| 2 | `--stage` と `--phase` の併用 | `error` | `Cannot use --stage and --phase together. Use one or the other.` | `:2780-2784` |
| — | state バージョンガード | `error` | `classifyStateVersion` の判定をカーソル読み取り**前**に中継 | `:2789-2803` |
| 2.5 | `Parked` セット済みかつ `Parked At Stage === Current Stage`、再入フラグなし | `parked` | `Workflow parked at "<slug>". Resume with /aidlc --resume.` ガードは `!flags.resume && !flags.stage && !flags.phase && !flags.review && !flags.newIntent` を要求 (§11, `:2830-2838`)。stale-by-progress: `Parked At Stage === Current Stage` の間だけ発火 (`:2839-2848`) | `:2830-2848` |
| 2.6 | parked ワークフローに対する `--resume` | `print` | `aidlc-state.ts unpark` を名指しし、その後 `next --resume` を再実行させる (unpark は `WORKFLOW_UNPARKED` を記録; §11) | `:2856-2868` |
| 3b | 無効な明示 `--scope` | `error` | `Unknown scope "<s>". Valid scopes: <list>.` — **state がラダーで勝つ場合でも無条件に検証される** | `:2880-2896` |
| 4 | scope が env 由来 | `error` | `aidlc-utility.ts resolve-env-scope` を spawn し、その逐語 stderr `Invalid AWS_AIDLC_DEFAULT_SCOPE "…". Valid scopes: …` を中継 | `:2898-2911` |
| — | 解決不能な scope | `error` | 同じ `Unknown scope` 文言 | `:2921-2925` |
| 4c | `compose` / `--new-scope` / `--report` | `print` | Composer ディスパッチ。front vs in-flight は state 有無で分岐。`Cannot combine compose with --stage/--phase. Compose re-shapes the plan; jump moves the cursor. Run them separately.` (文字列は `:2943`) | `:2940-2949` |
| 4a | `--new-intent` | `print`/`error` | 空白でない description が必須。scope は**明示 `--scope` のみ**を使い、ラダーを使わない | `:2966-2982` |
| 4b | `--single` | `run-stage`/`error` | §9 (単一ステージ隔離モード)。scope-change/jump 分岐より**前**に処理されるため、`--single` 下では変異パスに到達不能 | `:3004-3021` |
| 5 | state あり + 有効で異なる `--scope` / depth / test-strategy / review | `print` | `aidlc-utility.ts scope-change` または `config-change` を名指し | `:3028-3065` |
| 6 | state ありでの `--resume` | `ask` | `An existing workflow was found (currently at "<slug>"). How would you like to proceed? Resume from last checkpoint, redo the current stage, jump to a stage, or start fresh.` 発出前に `markActiveDirectiveResumeWaiting` をスタンプ (§11, `:3074-3088`) | `:3084-3087` |
| 7 | `--stage`/`--phase` (jump) | `print`/`run-stage`/`error` | §8 参照。init ガード `INIT_JUMP_ERROR`: `Cannot jump to initialization stages. The Initialization phase runs automatically when you start a workflow (describe what to build, e.g. /aidlc "build the auth service").` (`:4527-4528`)。state ありなら `aidlc-jump.ts resolve` (純読み取り) → `print` で `execute` コマンドを名指し (`:4577-4579`); state なしなら直接グラフ検索で `run-stage` (`:4583-4645`) | `:4530-4646` |
| 7b | 位置引数 scope、state なし | `print`/`ask` | Birth print、または fresh-clone の intent-pick `ask` | `:3111-3127` |
| 8 | 自由記述 prose、state なし | `ask` | キーワードヒット → コスト条項付き scope 確認; それ以外 → compose 提案 | `:3148-3183` |
| 9a | 明示 `--scope`、state なし | `print` | Birth (intent-create の名指し) | `:3196-3210` |
| 9b | state なし、名指し scope なし | `error` | `No workflow state found (no active intent). Start one by describing what to build (/aidlc "build the auth service") or by naming a scope (/aidlc --scope <scope>).` | `:3220-3227` |
| 9c | ワークフロー稼働中の自由記述 prose | `ask` (`new-work-routing` サブタイプ) | conductor の分類に対するエンジン側バックストップ。回答は `next` 経由でルーティングされ、**stage report として記録してはならない** (§4.5, `aidlc-directive.ts:332-334`) | `:3241-3261` |
| 10 | ハッピーパス | `run-stage` / `invoke-swarm` / `done` / `error` / `print` | §5.1 (下記) | `:3266-3348` |

**Birth の不変条件** (§5 末尾, `:876-916`, `:1001-1020`): `next` は birth を**決して自身で実行しない**。`createPrintDirective` が `bun <harness>/tools/aidlc-utility.ts intent-create --scope <s> [--arguments <json>] --label "<2-3 word kebab essence>"` を名指しする。`--new-intent` 変種はさらに新セッションへのハンドオフ指示を付す。重複 birth ガード `intentPickPromptIfRecordsExist` は「records は存在するが active-intent カーソルなし」を第 2 intent の鋳造ではなく `ask` に変換する。

### 1.1 Branch 10 (ハッピーパス) の内部順序 (§5.1)

| 手順 | 条件 | 出力 | upstream 行 |
|---|---|---|---|
| 1 | `Current Stage` が欠落 | `error`: `State file has no Current Stage field — cannot determine the next stage.` | `:3266-3271` |
| 2 | in-flight 判定 = checkbox state ∈ {pending, in-progress, awaiting-approval, revising} または欠落 | — | `:3281-3286` |
| 3 | Plan/cursor 不整合: in-flight ステージの effective plan action が `SKIP` | `in-progress`/`revising` なら回復手順の名指し (`report --stage <slug> --result skipped --reason "stage is SKIP in the approved workflow plan"`); それ以外は `error`: `Stage "<slug>" is SKIP in the approved workflow plan but its active cursor state is "<state>". Refusing to emit run-stage; repair the inconsistent state before continuing.` | `:3293-3312` |
| 4 | in-flight → `tryEmitSwarm(...)` を先に試し、だめなら `emitForSlug(...)` | `invoke-swarm` または `run-stage` | `:3314-3328` |
| 5 | completed/skipped → `nextInScopeStage(currentSlug, scope, stateContent)`; `null` なら | `done` reason: `Workflow complete — no in-scope stage remains after <slug> (scope: <scope>).` + `NEW_WORK_HINT` サフィックス (`:853-857`) | `:3332-3348` |

`effectivePlanAction` (`:2562-2571`): state ファイルのステージ別 EXECUTE/SKIP サフィックス (recomposition 由来) が静的 scope グリッドに**勝つ**。

---

## 2. scope 解決ラダー (§2.1, §5 分岐 3b/4; 01-workflow-model.md §5.5)

**優先順位** (02 §2.1 第 1 行, `aidlc-orchestrate.ts:1041-1073`):

```
state > --scope > positional > AWS_AIDLC_DEFAULT_SCOPE > default
```

| 順位 | ソース | 詳細 | 出典 |
|---|---|---|---|
| 1 | state ファイルの `Scope` フィールド | 稼働中ワークフローでは state が常に勝つ。ただし無効な明示 `--scope` は **state が勝つ場合でも無条件に検証**され `Unknown scope "<s>". Valid scopes: <list>.` で error (分岐 3b) | 02 §5 `:2880-2896` |
| 2 | 明示 `--scope <name>` | `validScopes()` (ファイル存在) に対して検証 | 01 §5.5 手順 1 |
| 3 | 位置引数 / キーワード推論 | `inferScopeFromText` (`aidlc-utility.ts:5563-5602`): キーワードは語境界 regex `new RegExp(`\b${tokens.join("\\s+")}\b`, "i")` にコンパイル (`:5578`)。scope は**アルファベット順**にスキャンし最初のマッチが決定的に勝つ (`:5574`, `:5596-5601`)。**5 語超のテキストでは推論を抑止** ("keyword + >5 words → likely a project description containing the keyword incidentally", `:5586-5594`) し `source: "freeform"` でデフォルトへ。`enterprise`/`feature`/`classic` は `keywords: []` で推論不能 (名指し必須) | 01 §5.5 手順 2 |
| 4 | `AWS_AIDLC_DEFAULT_SCOPE` | 設定済みかつ有効なとき (`envDefaultScope`, `aidlc-lib.ts:8902-8908`)。無効値はエンジンが `aidlc-utility.ts resolve-env-scope` を spawn しその逐語 `Invalid AWS_AIDLC_DEFAULT_SCOPE "…". Valid scopes: …` を中継して error (02 §5 分岐 4, `:2898-2911`) | 01 §5.5 手順 3 |
| 5 | デフォルト定数 | `export const DEFAULT_SCOPE = "classic";` (`aidlc-lib.ts:8896`)。優先名が enabled でない場合、`selectionAwareDefaultScope` は (a) `freeform_default: true` を自己指名する scope、次に (b) 唯一の enabled plugin の先頭 scope へフォールバック (`aidlc-lib.ts:8910-8947`) | 01 §5.5 手順 3 |

補足: `--new-intent` (分岐 4a) は例外的に**明示 `--scope` のみ**を使いラダーを使わない (`:2966-2982`)。解決不能な scope は同じ `Unknown scope` 文言で error (`:2921-2925`)。

---

## 3. `next` の読み取り専用不変条件と 2 つの例外 (§3.1)

**不変条件**: `next` はワークフロー状態 (`aidlc-state.md`、checkbox、audit 行) を決して書かない。Birth・jump・scope-change・config-change はすべて実行されず、`print` directive として conductor に**名指し**される (`:10-14`, `:2863-2867`, `:3040-3063`, `:4571-4579`)。

明示的に切り出された機械ローカル書き込みは 2 つ:

| # | 例外 | 内容 | upstream 行 |
|---|---|---|---|
| 1 | **steering MAC キー** | `.aidlc-steering-token-key`。intent の gitignore 済み `.aidlc-*` ファミリまたは `aidlc/.aidlc-sessions/` 配下に遅延鋳造 (lazily minted)。"machine-local runtime state, not a project-derived value an untrusted continuation can recompute" (`:2288-2292`) | `:2275-2347` |
| 2 | **active-directive マーカー** | すべての `load-steering`/`run-stage` 発出時に `writeActiveDirectiveMarker` (`aidlc-lib.ts:2883`) 経由で書かれる。内容: `state_sha256`、attempt id、command digest、emitted result digest | `aidlc-orchestrate.ts:271-296, 310-355` |

**失敗時の扱い**: 両者はルーティングに対し advisory。失敗は throw せず `recordHookDrop` で記録。**例外は Copilot-commit アーム**で、work directive の発行自体を拒否する:

- `"This tracked \`next\` attempt is stale or superseded, so its prepared result was not issued. Run a fresh \`next\` in the current Copilot session."` (`:327-329`)
- `"The fresh Copilot directive could not be published, so no work directive was issued. Retry \`next\`; if coordination remains busy, run \`/aidlc --doctor\`."` (`:334-336`, `:347-349`)

**発出規律** (§3.2): `prepareEmission` (`:233-304`) → `validateDirective` → serialize → サイズ検査 → `writePrepared` が stdout に**ちょうど 1 行の JSON** を書く (`:306-308`)。ハード拒否 2 種 (いずれも `process.exit(1)` + stderr):
- `aidlc-orchestrate: refusing to emit a malformed directive: <errors joined by "; ">` (`:259-262`)
- `aidlc-orchestrate: refusing to emit a directive larger than ${DIRECTIVE_MAX_BYTES} bytes` (`:266-268`)

定数 (`:1140-1143`): `DIRECTIVE_MAX_BYTES = 28 * 1024` (28 672 bytes、"the common 28 KiB harness floor"), `STEERING_TEXT_TARGET_BYTES = 20 * 1024`, `CONTEXT_WARNINGS_MAX_BYTES = 6 * 1024`, `INLINE_CONTEXT_PATHS_MAX_BYTES = 8 * 1024`。

---

## 4. `load-steering` / `continue_token` 往復プロトコル (§4.4, §10)

### 4.1 原則 (§10)

ルールの**パス**はルーティングメタデータ、ルールの**テキスト**は必須 steering。`run-stage` 発出前に、エンジンは active-space のルールファイルを読み、1 つ以上の有界 `load-steering` directive でコンテンツを輸送する — "No rule is downgraded to a discretionary path read because it did not fit one tool result" (`:1131-1139`)。

`load-steering` の必須フィールド (§4.1): `stage`, `bundle`, `part`, `parts`, `rules_content[]{path,text}`, `continue_token`。conductor 契約 (`SKILL.md:44, 78`): `rules_content` を配列順に適用し、active bundle として保持し、`load-steering` を **report せず**、直ちに `continue <token>` を実行する。

### 4.2 チャンク分割 (§10)

| 工程 | 内容 | upstream 行 |
|---|---|---|
| `steeringPieces` | 各ルールを Markdown 見出し境界で分割し、過大セクションは実 JSON ワイヤサイズでコードポイント境界分割 | `:2170-2243` |
| `steeringChunks` | piece を 20 KiB ターゲット (`STEERING_TEXT_TARGET_BYTES`) までパック | `:2245-2260` |
| 分割不能 | `A rule section could not be split below the directive transport limit. Shorten the affected heading section, then run a fresh \`next\`.` | `:2544-2548` |
| 読み取り失敗 (blocking) | `readRuleBundle` (`aidlc-steering.ts:85-106`) → `Cannot load required stage rule "<rel>" (<error>). The stage has not started. Restore the file or fix its permissions/UTF-8 encoding, then run \`next\` again.` — run-stage の代わりに `error` directive になる (`:2487`) | — |

### 4.3 HMAC トークン構造 (§4.4)

`continue_token` は HMAC-SHA256 認証エンベロープ `{p: payload, m: mac}` の base64url エンコード (`:2358-2372`)、デコード時に `timingSafeEqual` で検証 (`:2395-2405`)。ペイロードフィールド (`:1156-1175`、投入 `:2438-2465`) — 完全列挙:

| キー | 意味 |
|---|---|
| `v` | =1 (バージョン) |
| `s` | stage |
| `c` | scope |
| `i` | next part index |
| `b` | bundle digest |
| `d` | directive digest |
| `r` | route hash |
| `a` | state-aware フラグ |
| `u` | unit |
| `k` | unit kind |
| `f` | force-persona |
| `g` | gate |
| `n` | next_stage |
| `x` | single |
| `p` | per-unit |
| `w` | wave |
| `z` | swarm-settled |
| `h` | state hash |

デコードは厳密型テーブル (`:2409-2431`) に反するペイロードを拒否する。

コンテキスト束縛の 4 ダイジェスト (§4.4):

| ダイジェスト | 計算 | 束縛対象 | upstream 行 |
|---|---|---|---|
| `bundle: "sha256:<hex>"` | `sha256(JSON.stringify(loaded.content))` | チャンク中のルールテキストバンドル | `:2492` |
| `directiveHash` | `sha256(JSON.stringify(directive))` | チャンク連鎖が届けようとしている run-stage | `:2493` |
| route hash `r` | `sha256(JSON.stringify({node, scopeStages: subgraphForScope(scope).map(s => s.slug)}))` | グラフノード**および** scope のステージメンバーシップ | `:2467-2474` |
| `state_sha256` / payload `h` | `sha256(stateContent)` | directive の計算元 state ファイル | `:2156`, `:5974` |

### 4.4 staleness / fail-closed 条件の完全列挙 (§10)

`transportRunStage` (`:2476-2550`) — 継続ペイロードを新規再構築したバンドルと比較:

| 条件 | 発出メッセージ (逐語) |
|---|---|
| `payload.s ≠ stage` または `payload.b ≠ bundle` または `payload.d ≠ directiveHash` | `This stage or its rules changed while they were being loaded, so what has arrived so far is stale. Run a fresh \`next\` to restart delivery from part 1.` |
| `payload.i > chunks.length` | `This request asks for a part of the stage rules that does not exist. Run a fresh \`next\` to restart delivery from part 1.` |
| `payload.i === chunks.length` | (終端) `run-stage` directive そのものを発出 |

`handleContinue` (`:5963-6094`) が追加する 4 条件 — **すべて fail-closed**:

| 条件 | メッセージ (逐語) |
|---|---|
| トークン欠落/デコード不能/MAC 不一致、または余分な argv | `Invalid steering continuation token: this stage's rules cannot be loaded from where they left off. Run a fresh \`next\` to restart delivery from part 1.` |
| state-aware トークンの `h` ≠ 現在の state digest | `The saved position moved on: the workflow state changed while this stage's rules were being loaded. Run a fresh \`next\` to restart delivery from part 1.` |
| stage slug がグラフに存在しない | `Stage "<slug>" no longer exists. Run a fresh \`next\` after recompiling the stage graph.` |
| route hash 不一致 | `Which stage runs next has changed: the stage route changed while its rules were being loaded. Run a fresh \`next\` to restart delivery from part 1.` |

**再構築原則** (`:5996-6037`): `continue` はキャッシュ済みオブジェクトを信用せず、現在のディスク状態から run-stage を再構築し、ペイロードのピン留めフィールド (`gate`, `unit`, `next_stage`, `single`, `swarm_settled`, `wave`) を再適用する。

**カーソル前進はトランザクショナル** (`:6090-6092`)。競合時: `Continuation coordination is busy. This call did not commit a cursor change. Retry the current token; if it is reported superseded, run a fresh \`next\`.`

検証規則 (§4.2, `aidlc-directive.ts:603-611`): `load-steering` のクロスフィールド規則は `part must be less than or equal to parts`。

---

## 5. per-unit 反復のカーソル規則 (§5.2, §6)

### 5.1 ルーティング (§5.2)

`emitForSlug` (`:4394-4416`): `for_each: unit-of-work` ノードを、state の `Construction Iteration` が正確に `unit-major` のとき `emitUnitMajorRunStage` へ、それ以外は `emitPerUnitRunStage` へルーティング。

対象は 5 つのコンパイル済み per-unit ステージ (§Measurement notes): `functional-design:inline`, `nfr-requirements:inline`, `nfr-design:inline`, `infrastructure-design:inline`, `code-generation:subagent`。

### 5.2 per-unit カーソル規則 (`:3616-3634`, `emitPerUnitRunStage` `:4013-4201`)

| 規則 | 内容 | upstream 行 |
|---|---|---|
| 未カバー判定 | カバレッジは per-unit の**ディスク上のアーティファクト** (+ unit ライフサイクル使用時は `UNIT_COMPLETED` レシート) | `:3672-3695` |
| 中間ユニット | 最初の未カバーユニットを `directive.gate = false` で発出 (`:4198`; 根拠コメント `:4190-4197`)。conductor は **report せずに** `next` を再実行 | `:4013-4201` |
| 最終ユニットへのゲート載せ | 未カバーユニットが尽きると、**最後のユニット**に対しステージの実計算ゲートを載せて再発出 — "This is the ONLY directive on which the gate fires" | `:4172-4186` |
| skeleton 未解決時の遅延 | ステージが skeleton-gate ステージで stance 未記録なら、per-unit 反復は**遅延**され、まず素の `{unit-name}` directive を `gate:"unresolved"` で発出 | `:4026-4044` |
| `unit` フィールド | 具体 Unit of Work に解決された per-unit Construction directive のみに存在。"a marker that this run-stage is ONE iteration of N" | `aidlc-directive.ts:225-236` |
| `wave` | 4 つの inline per-unit design ステージ向けの任意 stage-major 並列面 `{batch_index,entries[]}`; entry 検証に duplicate-unit と `required_produces ⊆ produces` チェックを含む | `aidlc-directive.ts:238-245`, `:1029-1199` |
| gate 抑止の全出現箇所 | `grep -n 'directive.gate = false'` → 4 箇所: `4139`, `4356`, `4486` および §5.2 が引用する `4198` | §Measurement notes |

### 5.3 swarm アーム (§5.2, 対比用)

`tryEmitSwarm` (`:3483-3589`) の発火条件 (すべて AND): Construction ステージ + `for_each: unit-of-work` + `mode: subagent` + skeleton-gate ステージ**でない** + `Construction Autonomy Mode` が正確に `autonomous` (`:3400-3410`)。`next` 1 回につき 1 Bolt バッチ前進。カーソルはディスクアーティファクトではなく **`SWARM_UNIT_CONVERGED` audit 行**にキーイング (`:3446-3463`)。全ユニット収束後、settle `run-stage` を `swarm_settled: true`・reviewer フィールド剥離・`protocol_modules: ["construction","swarm"]` で発出 (`:3435-3444`, `:3519-3532`)。`swarm_settled: true` の意味 (§4.3): gate 専用再入。"the conductor must not rerun the stage body or reviewer" (`aidlc-directive.ts:207-210`)。

### 5.4 gate モデルとの接続 (§6)

`computeGate` (`:1756-1771`; doc comment `:1734-1742`) の 3 帰結:
1. initialization phase → `false` ("bootstrap auto-proceed, no governance gate", `:1736`, `:1761`)
2. skeleton-gate ステージ (scope の最初の Construction EXECUTE ステージ) で stance 未記録 → `GATE_UNRESOLVED` (ワイヤ上はリテラル `"unresolved"`)
3. それ以外 → `true`

`isSkeletonGateStage` は**導出であってハードコードでない**: `firstInScopeStageOfPhase("construction", scope)` (`:1349-1361`)。gate 軸は `execution: ALWAYS|CONDITIONAL` 包含軸と明示的に直交 (`:1744-1746`)。validator の `checkGate` (`aidlc-directive.ts:783-799`) は boolean **または**リテラル `"unresolved"` のみ受理し、他は `<kind>: gate must be boolean or "unresolved", got <desc>` で拒否。

stance 往復: conductor が自由記述の `## Walking Skeleton` prose を分類し `report --skeleton-stance <on|off|scope-dependent>` で返す。`handleSkeletonStanceReport` (`:4943-5008`) は値検証・state ファイル必須・`Current Stage` が当該 scope の skeleton-gate ステージであることを要求 (`Current stage "<slug>" is not the skeleton-gate stage for scope "<scope>" — a skeleton stance is only reported for the first Construction Bolt's gate.`)、`aidlc-state.ts set-skeleton-stance` 経由で書き込み、`Recorded walking-skeleton stance "<stance>" for "<slug>". Re-run \`next\` to continue — the gate is now determined.` を print。`resolveSkeletonGate` は今日すべての stance に対し `true` を返すが、往復の存在理由は "the engine cannot EMIT a boolean it has not determined" (`:1371-1416`)。

skeleton-gate アンカーの scope 別解決 (01-workflow-model.md §6.4):

| アンカーステージ | scope |
|---|---|
| `functional-design` (3.1) | `enterprise`, `feature`, `classic`, `workshop`, `mvp`, `refactor` |
| `nfr-requirements` (3.2) | `infra`, `security-patch` |
| `code-generation` (3.5) | `express`, `poc`, `bugfix` |

---

## 出典ファイル
- docs/upstream/specs/02-orchestration-engine.md (§2.1, §3.1, §3.2, §4.1-4.5, §5, §5.1, §5.2, §6, §10, §11, Measurement notes)
- docs/upstream/specs/01-workflow-model.md (§5.5 scope 選択, §6.4 skeleton gate)