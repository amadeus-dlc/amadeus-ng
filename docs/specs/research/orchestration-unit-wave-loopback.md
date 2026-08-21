> 抽出元: upstream as-built 仕様 (awslabs/aidlc-workflows v2 @ 3c3146cf, v2.6.40)。2026-08-22 の 3 エージェント精密抽出 (slice 2)。10-orchestration.md slice 2 の執筆材料。

# タスク 3 抽出結果: per-unit 実行機構 (wave / unit lifecycle / Build-and-Test loop-back / unit-major)

出典表記: `01` = 01-workflow-model.md, `02` = 02-orchestration-engine.md, `03` = 03-state-audit-runtime.md, `04` = 04-stage-protocol.md, `06` = 06-sensors.md, `07` = 07-hooks.md, `09` = 09-cli-tools.md (いずれも `docs/upstream/specs/`)。「L」は as-built 仕様ファイルの行番号、バッククォート内 `:n` は upstream ソースコードの行番号引用。

---

## 1. per-unit ステージの母集団と Unit of Work の位置づけ (01 §4.1)

| 項目 | 内容 | 出典 |
|---|---|---|
| per-unit ステージ (5) | `for_each: unit-of-work` を持つのは `functional-design`(3.1), `nfr-requirements`(3.2), `nfr-design`(3.3), `infrastructure-design`(3.4), `code-generation`(3.5)。ノード自身の `for_each` が真実の源で、防御的ハードコード集合 `KNOWN_PER_UNIT_STAGES` が同じ 5 つをクロスチェック (`core/tools/aidlc-lib.ts:77-93`) | 01 §4.1 L273-277 |
| 実測 | 5 per-unit の mode 内訳: `functional-design:inline, nfr-requirements:inline, nfr-design:inline, infrastructure-design:inline, code-generation:subagent` | 02 計測ノート L529 |
| build-and-test / ci-pipeline | per-unit 完了後に **1 回だけ** 実行。build-and-test の `condition` は "Always executes once after all per-unit stages are finished." | 01 §4.1 L278-280 |
| `workspace_requires: true` | `code-generation` のみ。「Markdown produces だけでなく実ソースを workspace root に書く」マーカー (`aidlc-lib.ts:60-65`) | 01 §4.1 L282-284 |
| コンパイル advisory | `for_each: unit-of-work` + `workspace_requires: true` かつ `mode !== "subagent"` の Construction ステージには**非致命的 advisory**(自律 swarm がこのフィールド一致で発火するため、黙って発火しなくなる) (`aidlc-graph.ts:1915-1929`) | 01 §4.1 L286-290; 04 §12 D5 L612 |
| Unit DAG の正本 | `inception/units-generation/unit-of-work-dependency.md` が Bolt/unit DAG のエッジブロック (`unitDependencyPath`, `aidlc-lib.ts:6165`) | 03 L371 |
| per-unit 成果物パス | `<record>/construction/<unit>/<slug>/` (通常ステージは `<record>/<phase>/<slug>/`) (`aidlc-orchestrate.ts:1512-1535`) | 01 L263-269 |
| per-unit memory | 通常は `memory_path` = `<recordPrefix>/construction/<unit>/<slug>/memory.md` (`aidlc-orchestrate.ts:1086-1098`); per-unit ステージは `unit_memory_path` (`aidlc-orchestrate.ts:3829`) | 02 §4.3 L158; 04 §11.1 L554 |
| `directive.unit` | 「具体的 Unit of Work に解決された per-unit Construction directive にのみ存在。『この run-stage は N 回中の 1 イテレーションである』ことのマーカーでもある」(`aidlc-directive.ts:225-236`) | 02 §4.3 L168 |
| ゼロ Unit ガード | construction プロトコル冒頭 (`stage-protocol-construction.md:5-11`): Bolt / walking-skeleton / ladder / autonomy / per-Unit 儀式は「エンジンが実在の非空 Unit DAG を解決したときのみ適用」。`directive.unit` または `directive.wave` が Unit 作業の識別子、`directive.swarm_settled` が自律実行の gate-only 終端の識別子。「A zero-Unit directive has none of those fields: run it once as an ordinary stage, with no Bolt, skeleton, ladder, or swarm ceremony.」 | 04 §6.1 L375 |

---

## 2. unit lifecycle 動詞 (`aidlc-state.ts unit start|pause|resume|complete`)

### 2.1 動詞とガード

| 動詞 | 呼び出し形 (逐語) | ガード / 意味論 | 出典 |
|---|---|---|---|
| `unit start` | `aidlc-state.ts unit start --stage <slug> --unit <name>` — ステージ本体の**前**に実行 | 同一ステージの別 Unit が open の間は**拒否**(単一アクティブユニット不変条件); 自律 swarm がステージを所有している場合は拒否 (`aidlc-state.ts:906-912`); unit が正本 DAG に存在することを要求 (`:921-925`) | 04 §6.4 L407; 03 §5.7 L620-622 |
| `unit pause` | `unit pause --reason "<why>" --next-action "<the exact next step>"` — Unit 途中停止用 | 理由と次アクションを必須で記録 | 04 §6.4 L407 |
| `unit resume` | `unit resume` | paused Unit の解除。明示的 `unit resume` まで他の作業は一切開始できない | 04 §6.4 L407 |
| `unit complete` | `unit complete` — 本体の**後**に実行 | レシートをコミットする**前**に「every required artifact が regular file としてディスクに実在することを検証し、ディレクトリや欠落パスを拒否」(`aidlc-state.ts:980-988`)。コード注釈いわく「claim-1 の逆転 — artifact walk が『遷移そのもの』から『遷移が検査するもの』へ移った」(`:976-979`) | 04 §6.4 L407; 03 §5.7 L622-625 |

- 4 動詞のディスパッチは `aidlc-state.ts:861`。`aidlc-state.ts` の 25 サブコマンド一覧に `unit`, `set-construction-iteration` を含む (09 §3 L84)。ソース順の case は `fork`(`:613`), `merge`(`:616`), `unit`(`:619`), `park`(`:622`), `unpark`(`:625`) で、**ツール自身の拒否文字列 (`:630`) は `unit` を列挙から漏らしている** (09 計測ノート L1093)。
- `unit` はエンジン専有 11 動詞 (`set, checkbox, advance, finalize, complete-workflow, gate-start, approve, reject, revise, skip, park`) には**含まれない**(03 §5.7 L582-588) — つまり conductor が直接叩く動詞。ただし**委譲サブエージェントからは遮断**: `DELEGATED_STATE_MUTATIONS` = 11 blocked + `set-skeleton-stance`, `set-construction-iteration`, `acknowledge-compaction`, `reuse-artifact`, `practices-event`, `practices-promote`, `fork`, `merge`, `unpark` (07 L167; 05 L241)。

### 2.2 state キャッシュフィールドと receipt mode

| 項目 | 内容 | 出典 |
|---|---|---|
| Runtime State フィールド | `Active Unit`, `Unit State`, `Unit Pause Reason`, `Unit Next Action` を `## Runtime State` に `setOrInsertField` で挿入 (`aidlc-state.ts:1046-1055`); 4 つとも `unit complete` で削除 (`:1041-1044`) | 03 §5.3 L521 |
| キャッシュ宣言 (逐語) | "audit stays the source of truth — these fields are a cache, exactly like Parked / Parked At Stage" (`aidlc-state.ts:1036-1038`) | 03 L524-525 |
| UNIT_* レシート (4) | 監査カテゴリ「Unit Lifecycle」: `UNIT_STARTED` `UNIT_PAUSED` `UNIT_RESUMED` `UNIT_COMPLETED` | 03 §6.5 L781 |
| receipt mode 固定 (逐語) | 「あるステージに 1 つでもレシートが存在したら、以後のすべての attempt は receipt mode に留まる: "Artifact files alone no longer settle a Unit."」 | 04 §6.4 L407 |
| receipt mode の粘着性 | 「**receipt-mode workflow** is sticky once any lifecycle row exists」— 再突入は per-unit directive を発行し、各該当 unit は `unit start` / `unit complete` を**再鋳造**する | 04 §6.3 L397 |
| paused の最優先 hard-stop (逐語) | paused Unit は「routes FIRST and hard-stops the loop」— エンジンは `unit_state: paused` を載せた `ask` を emit し、明示的 `unit resume` まで他の作業を開始できない | 04 §6.4 L407 |
| report 側の enforcement | `report` ガード 11: 完了証拠ガード `checkStageCompletionEvidence` (`aidlc-orchestrate.ts:5128-5230`) が未完了ステージに対し「pipeline link receipts, **per-unit coverage**, **paused-unit refusal**, ensemble contribution evidence」を検査 | 02 §7.2 L290 |

### 2.3 UNIT_* レシートの権威保護

| 保護 | 内容 | 出典 |
|---|---|---|
| `aidlc-audit.ts append` の拒否 | 権威レシートの直接 emit を拒否 (逐語列挙): `HUMAN_TURN, GATE_APPROVED, GATE_REJECTED, QUESTION_ANSWERED, REVIEW_REQUESTED, REVIEW_COMPLETED, PIPELINE_LINK_COMPLETED, ARTIFACT_REUSED, SWARM_STARTED, SWARM_UNIT_CONVERGED, AUTONOMY_MODE_SET, UNIT_STARTED, UNIT_PAUSED, UNIT_RESUMED, UNIT_COMPLETED` | 04 §4.4 L276 |
| `CLI_PROTECTED_EVENT_TYPES` (18) | 「4 つの `UNIT_*` レシート」を含む。拒否文言 (逐語): `Direct emission of <E> is blocked: it is an authority-bearing receipt owned by its emitting tool or hook (gate resolutions and approvals come from aidlc-orchestrate.ts report, interview answers and reviews from aidlc-log.ts, human presence from the prompt-submit hook). The audit CLI appends diagnostic events only.` (`aidlc-audit.ts:348`; env 回避 `AIDLC_ALLOW_DIRECT_AUDIT_EVENTS=1`) | 03 §6.6 L812, L815-821 |
| `MERGE_PROTECTED_EVENT_TYPES` (26) | **unit-lifecycle レシートは worktree delta で運搬禁止**(human authority・referee 帳簿・`DOCUMENT_*` prefix と並ぶ) (`aidlc-audit.ts:395`) | 03 §6.6 L813, L827-834 |

---

## 3. `set-construction-iteration` と Construction Iteration 切替

| 項目 | 内容 | 出典 |
|---|---|---|
| state フィールド | `Construction Iteration` (`unit-major`/`stage-major`) を `## Runtime State` に runtime 挿入。書き手は `aidlc-state.ts:764` (`set-construction-iteration` サブコマンド) | 03 §5.3 L519; 09 §3 L84 |
| unit-major の opt-in | 「**Unit-major iteration** (`stage-protocol-construction.md:267`) is opt-in via `Construction Iteration: unit-major` under `## Runtime State`.」 | 04 §6.4 L411 |
| 委譲遮断 | `set-construction-iteration` はサブエージェントから遮断 (`DELEGATED_STATE_MUTATIONS`) | 07 L167 |

---

## 4. ルーティング: emitUnitMajorRunStage vs emitPerUnitRunStage (02 §5.2)

### 4.1 分岐 (逐語)

> `emitForSlug` (`aidlc-orchestrate.ts:4394-4416`) routes a `for_each: unit-of-work` node to `emitUnitMajorRunStage` when `Construction Iteration` is exactly `unit-major`, else `emitPerUnitRunStage`. — 02 §5.2 L238

happy path (branch 10) では in-flight ステージに対しまず `tryEmitSwarm(...)`、だめなら `emitForSlug(...)` (`:3314-3328`) (02 §5.1 L231)。

### 4.2 stage-major (`emitPerUnitRunStage`) の意味論 — 02 §5.2 L240 (`:3616-3634`, `:4013-4201`)

| 規則 | 内容 | コード引用 |
|---|---|---|
| カバレッジ判定 | per-unit の**ディスク上の artifacts**(+ unit lifecycle 使用開始後は `UNIT_COMPLETED` レシート) | `:3672-3695` |
| 未カバー unit の emit | 最初の未カバー unit を `directive.gate = false` で emit; conductor は **report せずに** `next` を再実行 | `:4198` (根拠コメント `:4190-4197`) |
| ゲート発火 | 未カバー unit が尽きたら最後の unit を実ゲート値付きで再 emit — "This is the ONLY directive on which the gate fires" | `:4172-4186` |
| skeleton-gate 遅延 | ステージが skeleton-gate ステージでスタンス未記録なら per-unit 反復は保留され、素の `{unit-name}` directive を `gate:"unresolved"` で先に emit | `:4026-4044` |
| gate 抑制箇所 (実測) | `directive.gate = false` は engine 内 4 箇所: `4139`, `4198`, `4356`, `4486` (per-unit / per-batch のゲート抑制) | 02 計測ノート L532 |
| プロトコル側の記述 | 「エンジンは Bolt build 順に Unit ごとに ONE `run-stage` を emit し `directive.unit` を載せ、`next` ごとに次の未確定 Unit に差し替える。per-Unit gate は未確定 Unit すべてで `gate: false`、実ゲートは最後の Unit 確定後の再突入時に**ちょうど 1 回**発火 — "enforced deterministically: `report --result approved` on a not-yet-completed per-Unit stage is refused while any Unit is unsettled"」 | 04 §6.4 L405 (`stage-protocol-construction.md:257`) |

### 4.3 unit-major (`emitUnitMajorRunStage`) の意味論 — 04 §6.4 L411 (`stage-protocol-construction.md:267`)

| 規則 | 内容 |
|---|---|
| 歩行順 | Unit-outer / stage-inner。「最初の動くコードが ONE Unit の設計後に着地する」 |
| swarm 不発火 | 「The autonomous swarm never fires under unit-major.」 |
| ゲート | 「個数も機構も不変だが遅く発火し、ブロック末尾でカスケードする」 |
| conductor への標準規則 (逐語) | "Always act on the directive's own `directive.stage` + `directive.unit`, never on `Current Stage`."(全ハーネス小節で反復) |
| センサー側の含意 | run-sensors hook の Step 9 が marker-first なのは「unit-major 実行では `Current Stage` がまだ最初のブロックステージを指したまま後段ステージが走り得る」ため; マーカーのステージが stale/plugin-filtered graph に無ければ `Current Stage` にフォールバック (`:174-179`) | 06 L215-229 |
| Stop hook の例外 | 停止判定「Pending mid-stage question」は autonomous でも許可されるが、**unit-major の `code-generation` は例外で Plan Approval が必須** (`:527-537`) | 07 L297 |

### 4.4 swarm arm との排他 (参考、02 §5.2 L242)

`tryEmitSwarm` (`:3483-3589`) の発火条件: Construction ステージ + `for_each: unit-of-work` + `mode: subagent`、skeleton-gate ステージで**ない**、`Construction Autonomy Mode` が正確に `autonomous` (`:3400-3410`)。`next` ごとに 1 Bolt batch 前進、判定キーはディスク artifact ではなく `SWARM_UNIT_CONVERGED` 監査行 (`:3446-3463`)。全 unit 収束後は `swarm_settled: true`・reviewer フィールド剥奪・`protocol_modules: ["construction","swarm"]` の settle `run-stage` を emit (`:3435-3444`, `:3519-3532`)。

---

## 5. wave (`{batch_index, entries[]}`) — stage-major 並列面

### 5.1 directive フィールドと検証 (02 §4.3)

| 項目 | 内容 | 出典 |
|---|---|---|
| フィールド定義 | `wave`: `{batch_index,entries[]}`? — 「4 つの inline per-unit 設計ステージのための optional な stage-major 並列面」(`aidlc-directive.ts:238-245`) | 02 §4.3 L169 |
| 検証規則 | entry 形状は `aidlc-directive.ts:1029-1199` で検証、**duplicate-unit チェック**と **`required_produces ⊆ produces` チェック**を含む | 02 §4.3 L169 |
| 型検証の位置 | validateDirective のルール 4「Type/presence per field」の特殊検査群の一つ: 「positive integers, `{path,text}` arrays, `{path,expected}` arrays, the `pipeline` object, `protocol_modules` enum, and the nested `wave` structure」(`:829-1199`) | 02 §4.2 L139 |
| dispatch-subagent との差 | `DISPATCH_SUBAGENT_FIELDS` = `RUN_STAGE_FIELDS` から `single`, `wave`, `protocol_modules`, `swarm_settled` を除き `worker` を加えたもの (`:484-493`) — wave は run-stage 専用 | 02 §4.3 L150 |
| continue_token | payload に `w` = wave をピン留め (`p` = per-unit, `z` = swarm-settled と並ぶ)。`continue` は現在のディスク状態から run-stage を再構築し、payload の pinned フィールド (`gate`, `unit`, `next_stage`, `single`, `swarm_settled`, `wave`) を再適用 (`:5996-6037`) | 02 §4.4 L184; L399 |

### 5.2 プロトコル意味論 (04 §6.4 L409, `stage-protocol-construction.md:261-265`)

| 規則 | 内容 (逐語含む) |
|---|---|
| 適用範囲 | 「optional かつ **stage-major only**」。code-generation は wave 不適格 — "because it writes the shared workspace and hard-stops for Plan Approval" |
| lifecycle 動詞との関係 | 「wave builder は serial lifecycle 動詞を呼ばない — **wave directive がそのまま batch checkpoint**」 |
| entry を open に保つ方法 | 「blocking question は `entry.required_produces` からパスを 1 つ差し控えることで entry を open に保つ」 |
| entry ごとの review-state 語彙 (閉集合) | `outstanding`, `retry-required`, `repair-required`, `recovery-required`, `escalation-required` + 確定値 `READY` / 終端 `NOT-READY` / `not-required` |
| `escalation-required` (逐語) | recovery 消費済みの意味: "do not request another review or complete the Unit; halt and present the situation to the human" |

---

## 6. Build-and-Test loop-back (3.6 → 3.5) — 04 §6.3

### 6.1 発火条件・性格

| 項目 | 内容 | 出典 |
|---|---|---|
| 発火条件 | 「Build and Test が診断した failure の根本原因が生成コードまたは code-generation で選んだアプローチにあるとき、workflow は code-generation へ後方ジャンプしてよい」 | 04 §6.3 L389 |
| 例外としての地位 | NO EMERGENT BEHAVIOR RULE と checklist item 5 双方に対する公認例外。「a failed build-and-test run is deliberately left in-flight — its gate is NOT presented and its §13 learnings ritual DEFERS to the eventual passing run (the stage diary memory.md persists across the loop)」(`stage-protocol-construction.md:86-88`) | 04 §6.3 L389; §4.1 L226 |
| halt-and-ask との関係 | 自律モードが人間に伺いを立てる 2 ケースのうちの 1 つが「Build-and-Test loop-back の exhausted rung」(もう 1 つは Bolt の code-generation failure 時の halt-and-ask) | 04 §6.2 L385 |
| halt-and-ask 2 変種 | (`stage-protocol-construction.md:191-226`) impact-estimated 変種 = Retry with fix / Accept failure / Abort を effort・financial cost・risk 付きで提示; no-fix 変種は **"Retry with fix" を丸ごと省略**(候補 fix なしに提示することは「a fabricated fix to retry with」という禁止された逆方向の give-up option になるため)。テンプレのスロットを placeholder や捏造内容で埋めることは禁止 | 04 §6.3 L399 |

### 6.2 後方ジャンプの正確な手順

| # | 手順 | 逐語契約 | 出典 |
|---|---|---|---|
| 1 | `aidlc-orchestrate.ts next --stage code-generation` を実行 | ジャンプは「through the engine, never by hand」(`stage-protocol-construction.md:115-122`) | 04 §6.3 L395 |
| 2 | エンジンが `print` directive で正確なコマンドを返す | print 文言 (エンジン側逐語): ``Run `bun <harness>/tools/aidlc-jump.ts execute --target <slug> --direction <dir> --scope <scope>` to perform the jump, then re-run `next` to continue from the jump target.`` (`aidlc-orchestrate.ts:4577-4579`) | 02 §8 L327 |
| 3 | print されたコマンドを **verbatim** 実行 | construction モジュール側の指名形: `aidlc-jump.ts execute --target code-generation --direction backward --scope <scope>`。「Never compose the `execute` call by hand — the engine's print is the validated form.」 | 04 §6.3 L395 |
| 4 | `execute` の state 効果 (`backward`) | target と下流の EXECUTE ステージ (`completed/in-progress/awaiting-approval/revising/skipped`) → `pending` に戻し、target を `in-progress` に。`STAGE_JUMPED` (Direction/Source/Target/Scope/Details) + target の `STAGE_STARTED` を emit。監査 emit は `writeStateFile` **より前**で、emit 失敗は書き込みを中止 (`aidlc-jump.ts:221-479`, `:416-463`) | 02 §8 L331-339 |

### 6.3 回数の正本 (ledger)

| 項目 | 内容 (逐語含む) | 出典 |
|---|---|---|
| 正本 | 「カウンタは artefact ledger であり audit ではない」。`test-results.md` の `## Loop-Back Log` に存在し、「the count of `### Loop-back N` entries IS the bound (**max 3 per intent**)」(`stage-protocol-construction.md:90-91`) | 04 §6.3 L391 |
| 根拠 | ledger は後方ジャンプを生き延びる (ジャンプはチェックボックスをリセットするが artefact はリセットしない)、診断と同居、最終ゲートで読める。`STAGE_JUMPED` 行は決定論的な監査クロスチェックとして残る | 04 §6.3 L391 |
| append-only / 人間例外 | log は append-only。**人間指示の後方ジャンプは bound にカウントしない** | 04 §6.3 L391 |
| resume 時の数え方 | 「On any resume, the loop-back count is the ledger's entry count, never zero.」 | 04 §10.1 L528 |

### 6.4 Plan Approval の生存と再突入決済

| 項目 | 内容 (逐語含む) | 出典 |
|---|---|---|
| Plan Approval 生存 | (`stage-protocol-construction.md:100-108`) 記録済み Plan Approval の answer は権威のまま — 「the conductor MUST NOT blank its `[Answer]:` for the loop-back revision」。plan delta は Loop-Back Log エントリに記録。gated モードでは人間の "Retry with fix" **が** re-approval であり、replayed report の `--user-input` で運搬 | 04 §6.3 L393 |
| 再突入決済の分岐 | (`:139-153`) **artifact-only workflow**: 全カバー `gate: true` の fast path を取れる (fix は re-entry override 経由)。**receipt-mode workflow**: lifecycle 行が 1 つでもあれば sticky — per-unit directive を emit し各該当 unit が `unit start`/`unit complete` を再鋳造 | 04 §6.3 L397 |
| レビューレシート無効化 (逐語) | 両パスとも gate 前に unit ごとの fresh current-attempt review が必須。「The backward jump's `STAGE_JUMPED` invalidates every prior review receipt」(`stage-protocol-construction.md:158`)。エンジン側の実装: レシート走査は floored — 「ステージ最新の `STAGE_STARTED`、それ以後の `GATE_REJECTED`、最新の関連 `produces[]` write より後の行のみ有効; per-unit write はその unit のレシートだけを無効化」(`aidlc-state.ts:1763-1770`)。`for_each: unit-of-work` ステージでは**全 unit が各自の terminal receipt を要する** | 04 §6.3 L397; §5.5 L361 |
| Artifact Re-use 質問の抑制 | loop-back に紐づく 2 つの override が Keep/Modify/Redo 質問を抑制: **autonomous** 変種と **gated** 変種 (人間が既に "Retry with fix" を選択)。両者とも Loop-Back Log の planned fix から決定論的に決める — 対象 unit は Modify、残りは Keep、build-and-test 自身は Modify、**Redo は禁止**(Loop-Back Log を消すため)。いずれにせよ「fresh current-attempt reviews for every applicable unit are mandatory before the replayed gate is auto-approved」(`stage-protocol.md:1080-1081`) | 04 §4.7 L300 |

### 6.5 クラッシュ復旧 (recovery プロトコル §6)

| 状況 | 手当て (逐語含む) | 出典 |
|---|---|---|
| log 済み・jump 未実行 | `test-results.md` の `## Loop-Back Log` 最新エントリに planned fix があるのに、それ以後に一致する `STAGE_JUMPED` (Target: code-generation) が監査に無い場合 — 「the session died between logging and jumping — re-execute the jump… rather than re-diagnosing」(`stage-protocol-recovery.md:52-66`) | 04 §10.1 L528 |
| jump 済み | settlement-aware 再突入を resume: **receipt-mode** は最初の未確定 unit から / **artifact-only** は pre-gate override から / **swarm** は discard-and-reprepare パス。「None of the three paths may treat preserved artifacts or prior receipts as current-attempt evidence.」 | 04 §10.1 L528 |

---

## 7. produces_kinds による刈り込みと per-unit カバレッジ

| 項目 | 内容 (逐語含む) | 出典 |
|---|---|---|
| 宣言ステージ (4) | `functional-design`, `nfr-requirements`, `nfr-design`, `infrastructure-design` が `produces_kinds` を宣言 (計測 M14: `grep -l '^produces_kinds:'` → 4 ファイル) | 01 §4.1 L292-294; 04 §12/計測 L636, L156 |
| 語彙 (閉集合) | `UNIT_KINDS = ["service", "spec", "ui", "packaging", "library"]` (`core/tools/aidlc-lib.ts:10210`) — `produces_kinds` の値はここから引く | 04 §2.6 L158 |
| スキーマ検証 | map artifact → 非空 unit-kind リスト; 各キーは `produces`/`optional_produces` に存在必須 — エラー逐語: `` produces_kinds key "${name}" is not in produces `` (`:429-452`); パーサは `mapOfListsField` (`aidlc-lib.ts:9191`) | 01 §3.2 L173; 04 §2.2 L40 |
| 意味論 (逐語) | 「map に**載っていない** artefact は全 kind に適用; 載っている artefact は kind が不在の unit から刈り込まれる — "It prunes BOTH the directive produces paths and the coverage set - exempt from nothing" (`stage-definition.md:56-61`)」 | 04 §2.2 L75 |
| directive への反映 | `run-stage.produces` = 「解決済みパス、`produces_kinds` で kind フィルタ済み、`optional_produces` を含む」(`aidlc-orchestrate.ts:1705-1732`) | 02 §4.3 L161 |
| 例 | `infrastructure-design` は `infrastructure-specification: [service, ui, packaging]` を宣言 → `library` unit はこれを一切負わない。map 不在 artifact は全 kind 適用 (`aidlc-graph.ts:151-155`) | 01 §4.1 L294-297 |
| per-unit カバレッジ判定 | カバレッジ = per-unit の**ディスク上 artifacts**(kind 刈り込み後の集合) + lifecycle 使用後は `UNIT_COMPLETED` レシート (`aidlc-orchestrate.ts:3672-3695`)。§2 の receipt-mode 固定・§4.2 のゲート発火規則と接続 | 02 §5.2 L240 |
| continue_token の unit kind | payload に `u` = unit, `k` = unit kind をピン留め | 02 §4.4 L184 |

---

## 8. 補助事実 (仕様執筆時の注意)

| 事実 | 出典 |
|---|---|
| build-and-test の `produces` 一覧 (01 の inventory) は `build-instructions, integration-test-instructions, performance-test-instructions, security-test-instructions, build-and-test-summary, build-test-results, cross-unit-traceability` — 一方 loop-back ledger のファイル名はプロトコル逐語で `test-results.md`。両表記を原文どおり保存すること (as-built 間の名称差) | 01 L247 vs 04 §6.3 L391 |
| Within-Bolt question collection (`stage-protocol-construction.md:243-256`): 3.1–3.4 の質問を Bolt 開始時に全 Unit 分まとめて **stage 単位でグループ化** (Unit 名ラベル付き)、標準質問プロトコルは stage グループごとに 1 回。単一の Bolt-level answers gate で確認後、stage ファイルは ARTIFACT-ONLY モードで実行。code generation の per-Unit gate は「**suppressed by the orchestrator** — a single Bolt-level gate (or batch-level gate for parallel batches) replaces it」 | 04 §6.4 L403 |
| `park` は自律 Construction 下で拒否 (`aidlc-state.ts:796-801`)、`Status: Completed` でも拒否 (`:803-805`) — unit lifecycle と同じ節に記載 | 03 §5.7 L619-620 |
| §13 learnings ritual の 3 免除の 1 つが「unfinished per-unit iterations」(ステージ唯一の最終ゲートまで繰延)。「A `gate: false` iteration does not run it」(`stage-protocol.md:964`) | 04 §11.2 L566 |
| fork/merge・worktree パス導出・HOLD-MERGE は既存抽出 `docs/docs/specs/research/workspace-lock-fork-worktree.md` 側 (本抽出では対象外) | — |

対象ファイル: `docs/upstream/specs/01-workflow-model.md`, `02-orchestration-engine.md`, `03-state-audit-runtime.md`, `04-stage-protocol.md`, `06-sensors.md`, `07-hooks.md`, `09-cli-tools.md`