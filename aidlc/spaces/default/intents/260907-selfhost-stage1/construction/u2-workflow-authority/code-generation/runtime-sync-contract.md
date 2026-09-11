# TaskUpdate同期とruntime compileの固定契約

2026-09-09。調査・設計記録のみ。製品コード、ワークフロー状態、承認記録、ホストを変更していない。

固定元は `a277af218f0df7f325d3b8be7b6d90fce2c5bd40` のClaude配布。277ファイル、manifest `282b17c53cb82c24755b149f28e01508ce9eaebbdbe3020034a4dd4c8bda4459` を検証してから、一時ワークスペースで本家の実CLIとhookを呼んだ。以降の本家パスはすべて `dist/claude/.claude/` 以下である。

生の入力・出力・前後ファイルは [runtime-sync-observations.json](test-evidence/runtime-sync-observations.json) に保存した。TaskUpdate 7ケースとcompile/読取り6ケースの計13観測。初期状態は本家のintent-createで作り、指示が必要なケースは本家のnextを実行した。監査の捏造や保護の無効化はしていない。空日誌ケースだけ、固定配布の `knowledge/aidlc-shared/memory-template.md` を、実際に初期化済みのstate-initの日誌へ入力fixtureとして置いた。このfixtureはMEMORY_EMPTYの発火を観測するためであり、更新するTS処理を読取り専用に見せかけるための入力ではない。

## TaskUpdateの通常Claude契約

登録はPostToolUseのTaskUpdate。`hooks/aidlc-sync-workflow-state.ts:35–124`が入口、`tools/aidlc-utility.ts:7685–7740`が実際のstate更新を所有する。

- JSON objectでない入力、不正JSON、TTY、状態ファイル不在は終了0。workflowの更新はしない。
- 通常Claudeでは `tool_input.status === "in_progress"` と、非空activeFormの末尾 `/\[([a-z][a-z0-9-]*)\]$/` が発火条件。末尾に余分な空白があれば一致しない。
- source=`ide-audit-sync`はKiro IDE専用の別分岐であり、今回実装する通常Claude経路に持ち込まない。
- 発火すると `.aidlc-hooks-health/sync-workflow-state.last` を書き、hook自身が与えるstatusline所有印付きの子プロセスで `aidlc-utility.ts set-status --stage <slug> --project-dir <root>` を呼ぶ。直接のset-status呼出しは拒否される。
- 子のstdout/stderrは抑止され、終了コードもhookの終了値に反映されない。hookは終了0を返す。ただし子の拒否がERROR_LOGGEDを作る場合は監査変化がある。
- set-statusは、static graphにstageがあれば、Lifecycle Phase、Active Agent、Status=Running、Last Updated、Current Stage、In Progress、対象stageのcheckboxを更新する。通常ゼロUnitではCurrent Stageを保つ特例はない。旧stageのcheckboxは戻さず、EXECUTE/SKIPの接尾辞も変更しない。
- この同期は成功時のSTAGE_STARTED、GATE_APPROVED等を新たに監査へ書かない。reportやjumpへ置き換えると外部観測が変わる。

### 実観測

初期の現在位置はreverse-engineering、checkboxは同stageだけ`[-]`。

| 入力 | 終了・出力 | stateと監査の変化 |
| --- | --- | --- |
| coldでin_progress＋`Running [reverse-engineering]` | 終了0、stdout/stderr空 | workflow更新なし。隔離HOMEにBun自身のcacheファイルができた点は生観測に残している |
| in_progress＋`Running [reverse-engineering]` | 終了0、stdout/stderr空 | 同stageのままLast Updatedが更新。監査追記なし。発行済み指示の変更は後述 |
| completed＋`Running [reverse-engineering]` | 終了0、stdout/stderr空 | 全ファイル無変更 |
| in_progress＋`Running reverse-engineering` | 終了0、stdout/stderr空 | 全ファイル無変更 |
| in_progress＋`Running [unknown-stage]` | 終了0、stdout/stderr空 | stateは不変、heartbeatとERROR_LOGGEDを更新。Errorは`Unknown stage: unknown-stage` |
| in_progress＋`Running [requirements-analysis]` | 終了0、stdout/stderr空 | Current Stage/In Progressがrequirements-analysis、Agentがproductへ変化。REとRAが両方`[-]`。成功監査なし |
| in_progress＋`Running [market-research]` | 終了0、stdout/stderr空 | bugfixでSKIPのstageも現在位置として受理。Phase=IDEATION、Agent=product。REとmarket-researchが両方`[-]`。成功監査なし |

unknown-stageで追記された監査は以下の形だった。hookの終了0を「何もしなかった」と解釈しない。

```text
## Error Logged
**Event**: ERROR_LOGGED
**Tool**: aidlc-utility
**Command**: aidlc-utility set-status --stage unknown-stage --project-dir <project-dir>
**Error**: Unknown stage: unknown-stage
```

### 指示と承認への副作用

`tools/aidlc-lib.ts:5238–5260`の `refreshActiveDirectiveMarker` は、marker.stageと同期stage、変更前state指紋が一致した場合だけ刷新する。v2の場合は `crossActiveDirectiveBoundary` を通り、state指紋を書き替え、revisionを1増加、kind=error、delivery=superseded、needs_rehydrate=trueとし、継続トークンを除去する。共有承認runtimeもresetする。

実測した通常同stageケースはload-steering/revision1からerror/revision2へ変化した。owner_sessionはsessionlessのまま、context_epochは0のまま。**PreCompactのcontext_epoch+1失効と同じ操作ではない。** 既存PreCompact APIをそのまま呼ぶ設計は避ける。

別stageを指定した2ケースはmarker.stage不一致のため、元のmarkerを刷新しなかった。state指紋との不一致が残る。単なる表示の更新とは扱えない。

## 実装前に人間の裁定が必要な差

現 `modules/core/command/domain/src/orchestration/intent_execution.rs:check_invariants` は次を要求する。

- `at_most_one_active`: activeなcheckboxは最大1個。
- `cursor_in_scope`: Runningな実行のcursorは実効計画のEXECUTE内。
- 併せてno_gate_bypassとparked_positionも集約内で維持する。

本家の別stage/範囲外stage指定を逐語的に再現すると、上の最初の2条件に違反する。これは本家ソースの推測だけでなく、上表の実プロセス観測で確認した。後からQueryや投影だけに別のCurrent Stageを持たせて隠すこと、TaskUpdateをJumpedへ読み替えること、承認済み互換契約を黙って変更することは行わない。

親が人間に提示する選択肢は以下の3つ。ここでは選択を記録・代行しない。

1. **本家を優先し、不変条件を更新する** — 別stageとSKIP stageも本家どおり同期する。複数active/範囲外cursorを含む状態を、集約・再生・投影・Quint/ITFでどう表すかを設計し直してから実装する。
2. **既存規則を優先し、非互換を明示する** — 同stage等の整合する入力だけ同期し、別stage/範囲外stageの扱いを拒否または無変更のどちらにするか明示した契約差分として定める。本家完全一致とは主張しない。
3. **保留する** — TaskUpdateの製品実装を進めず、入力・両側の規則・生観測を保持する。他の独立作業の継続とは分ける。

## runtime compileの入出力

### 読取り計算

`tools/aidlc-runtime.ts:319–780`は、状態、全監査shard、static stage-graph、段階別memory.md、存在する依存関係資料を読み、runtime graphを組み立てる。

- 状態なしではstderrへpre-initのskip案内を出し、終了0。graphは作らない。
- 状態あり・監査なし、またはWORKFLOW_STARTEDなしでは、scopeを状態から取った空graphを保存する。これは無変更ではない。
- headerは最後のWORKFLOW_STARTEDのtimestampをworkflow_id/started_atにし、scopeは状態優先。
- STAGE_STARTED/STAGE_COMPLETEDをworkflow開始以降で対応付ける。single-stageのWorkflow行は除く。各slugの最新STARTEDを使い、最後に始まった順で並べる。
- 同秒の並びはソースどおり扱う。現在の実装はSTARTED側のindexとCOMPLETED側のindex+100000でtieを解くので、「監査原文の全行順と常に同じ」と単純化しない。
- 既知stageだけがgraphへ載る。未開始の9段を機械的に全てpending行として追加しない。実測した作成直後は初期化3段approved＋RE pendingの4行だった。
- memoryなしは件数と内訳がnull、存在する空日誌は0。Open questionsも件数内訳に含むが、learnings surfaceの学習候補とは別。
- approved stageだけlearnings_capturedを持つ。RULE_LEARNED/SENSOR_PROPOSEDのSourceを期間内で数える。Sensor/Boltの既存監査があれば対応する集計があり、producerの製品実装とは区別する。
- ゼロUnitスモークではinstances/bolt_dagは不要だが、必須キーやnull/0/空配列の意味を省略しない。未知・未検証の材料を空配列で成功に見せかけない。

### 書込みの全区分

| 書込み | 発火条件・順序 | 所有すべき処理 |
| --- | --- | --- |
| runtime-graph.json | 通常compileと空graph分岐。JSONの2-space整形＋末尾LFをatomicに保存。同内容でもmtimeは更新し得る | Rustの投影。単なるread操作ではない |
| MEMORY_EMPTY監査 | approvedかつmemory_entries=0。lock内で再読した監査に、同slugの今回completed_at以降の既存MEMORY_EMPTYがなければ追記 | Rustの更新事実→SQLite→RMU。状態・承認を進めない |
| 監査shardの作成/append | 上のMEMORY_EMPTYが必要なとき。監査directoryやclone-idの初回準備が派生する場合もある | 既存の監査保存・投影境界 |
| 一時ファイルとlock | atomic保存の一時ファイル、system tempの`.aidlc-audit-<hash>.lock`と所有印、解放時の除去 | 汎用I/O機構。read-only主張のために監査lockを無効化しない |
| hookの稼働/drop | rebuild-stage-graphから起動する場合、対象監査を得た後のheartbeat。子compile失敗はdropを記録し、親Bashを止めない | Rustの既存HookHealth経路 |
| セッション帰属 | rebuild-stage-graph hookはcompile判定より先に、成功したintent-create応答の正確なsessionへbinding/stamp/handoffを書く | 作成帰属の独立境界。今回の調査では変更しない |

compile自体はstate/規則/日誌/承認受領を修復・変更しない。fragment-fork/fragment-mergeは別の書込み動詞であり、compileやread-onlyの代替にしない。上記の他にBun自身のcacheが隔離HOMEへ作られた実行があり、生観測で区別した。

MEMORY_EMPTYの実測では、固定の空日誌templateを実際の初期化済みstate-initに置くと1件追記された。直後の再compileでは監査追記なし、graph本文は同一だがmtimeが更新された。**繰返しで監査が増えないことと、処理が読取り専用であることは別である。**

## 副作用なしで使える既存公開面

| 公開面 | 再利用候補としてできること | できないこと・確認範囲 |
| --- | --- | --- |
| `aidlc-runtime.ts read <slug>` | 既存graphのstage行をそのまま読む | graphの再計算はしない。実測でファイル本文/mtimeとも無変更 |
| `aidlc-runtime.ts summary --json` | 既存graphと状態/static graphから集約値を読む | 監査からgraphを再生成しない。実測で無変更 |
| `aidlc-learnings.ts surface --slug <slug>` | active stageのgraph行と実日誌を読み、候補とparked_open_questionsを表示 | graph/行欠落は失敗。実測で無変更 |
| `aidlc-lib.ts`の既存export `findAllEvents`、`parseMemoryHeadings`、`relativeMemoryPath` | 文字列/引数に対する限定した純粋処理。ソース上の再利用候補 | full graph builderではない。wrapperを作る場合も既存exportだけを呼び、入出力契約を別途検証する |

`aidlc-runtime.ts`の公開exportはmainで、privateなcompile、pairStartedCompleted、buildWorkflowHeader、readMemory等を外部から計算だけとして呼ぶ面はない。CompileOptionsにもdry-run/write=falseの選択肢はない。mainのcompile動詞は常に書込み分岐へ進む。**既存の公開read-only full compilerは見つからなかった。** 裏口export、private関数のソース抜出し、監査を空にする入力、書込み関数の差替え、監査無効化で作る疑似read-onlyは再利用案に含めない。

## learnings surfaceが読む最小契約

`tools/aidlc-learnings.ts:247–325`は、stateのCurrent Stageが要求slugに一致することを確認し、JSON objectの`stages[]`から同じ`stage_slug`の行を探す。その行の文字列`memory_path`をproject rootから解決して日誌を読む。

この読取りだけに必要なのは`stages[]`、対象行の`stage_slug`/`memory_path`である。しかしC1のruntime graphや既存read/summaryまで、その3要素に縮めてよい意味ではない。Rust側は本家のgraph schemaを保ち、まず通常ゼロUnitで観測された行と必須キーを生成する。

日誌がない場合は0候補で成功するが、graphがない、対象stageがない、memory_pathがない場合を同じ0候補へ潰してはならない。実測のREはmissing diaryで0候補、space/default・intentの帰属値を正確に返した。

## 最小実装案と停止線

1. **TaskUpdateは上の人間裁定後。** 入力封筒の解析、適用外、heartbeat、unknown stageのERROR_LOGGEDとhook終了0を分ける。適用可能な同期は専用の集約コマンド/イベントにし、RMUがstatusline項目と指示の刷新を投影する。PreCompactやreportの別意味を流用しない。
2. **runtimeの読取り計算と更新を分離する。** 状態/監査/日誌を一度の観測入力として受け、graphと空日誌の観測候補を導出する。MEMORY_EMPTYの今回完了境界に対する重複判断は集約が所有し、保存した事実を通常RMUが監査へ投影する。graph公開は同じ通常の保存・公開機構に合わせる。既存TS full compileの更新を残さない。
3. **先に通常ゼロUnitの形を固定する。** 作成直後の4行、日誌不在/nullと空/0、MEMORY_EMPTYの一回だけの追記、再compile時の同一バイト、graph不在時のread/surface失敗をTDD対象にする。sensor/Bolt producerの実装を追加する段階ではない。
4. **保存したgraphに対して既存read/summary/surfaceを照合する。** 生のRust出力をそのまま既存公開動詞へ渡し、読取り整合と正本無変更を検証してからU4の再利用へ渡す。

本調査でこれらの製品実装には着手していない。特にTaskUpdateの非互換選択を、既存集約を守るための単なる実装都合として決めない。
