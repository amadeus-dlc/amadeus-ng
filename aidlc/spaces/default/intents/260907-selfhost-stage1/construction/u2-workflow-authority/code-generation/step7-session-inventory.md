# Step 7 セッション補助フックの適用・差分

固定元は2.7.1 `a277af218f0df7f325d3b8be7b6d90fce2c5bd40`。277配布ファイルのmanifest `282b17c53cb82c24755b149f28e01508ce9eaebbdbe3020034a4dd4c8bda4459` を実物で再検証した。通常Claude、Brownfield bugfix 9段、Unitなしを対象とする。Testing Contractは承認済み `sha256:d904f82d20fc4ba2d0045d5697ecae08fac96371ae5ab4ab6ec4913ae045ab55` を維持。

## 担当する4入口

| フック | 固定元で必要な処理 | 開始時のnative差 |
| --- | --- | --- |
| session-start | coldでもcurrent session、PID ancestry、binding（intent:nullを含む）、任意transcript、active-space cursor、Claude includeを保持。activeではheartbeat、startup/clear/malformedのSESSION_STARTED、resumeのSESSION_RESUMED、逐語additionalContext。compactは重複監査しない。session binding/stampを保つrebind通知もある。 | hook入口なし。Stop向けのbinding読取りだけ存在。session producer・監査・contextは未接続。 |
| session-end | session stampの元intentへSESSION_ENDEDとheartbeat。UUID-backed workflowへ未stamp sessionを付け替えない。unknown stampはdrop。終了をworkflow完了にしない。 | hook入口・監査なし。 |
| log-subagent | 正しいJSON objectのとき、Runningの作業へSUBAGENT_COMPLETEDとheartbeat。agent type/id、UTF-16の先頭200単位のmessageを保存。無関係/不正JSON/作業なし/Completedは無視。 | hook入口・監査なし。通常Claudeのselected経路はbackground ledgerを生成しない。任意の自律background入力全般を対応済みにしない。 |
| validate-state（PreCompact） | heartbeat。stateがあれば必要2セクションを確認し、欠落warning・recovery breadcrumb。監査が既存ならSESSION_COMPACTED。同じsession/project/intent/stateに束縛されたactive directiveのcontextを失効し、許可runtimeをreset。 | hook入口・breadcrumb・監査なし。既存PlanApprovalRuntimeの対象付き失効経路へ結線が必要。 |

U1 `stage1/cases.json` のsession/start、hook/log-subagent、hook/validate-state、hook/session-endが正常入力の正本。追加22境界を `scripts/goldens/capture-session-hooks.ts` で同じ固定元から採り、`tests/golden/selfhost-stage1/session-hooks.json` へ前後ファイル・入出力全文を保存した。cold、startup/resume/clear/compact/unknown/malformed、終了の帰属あり/なし、subagentの長文・無視、PreCompact構造正常/欠落を含む。usageは無効化しており、有効transcript/ledgerの証明ではない。

session-start以前に作業がなく、その後intent-createする場合は、同じhost sessionへの新intent binding接続も必要。既存Stopの読取りと競合せず、metadataの公開元をRustの保存事実へそろえる。PreCompactのcontext失効は新しい監査だけで代替しない。

## Step 7 の残り（今回は棚卸しのみ）

| 項目 | 適用と開始時の差 |
| --- | --- |
| deliver-stage-rules | RE/コード生成のTask/Agentに適用。next/continueの規則描画はあるが、hookのupdatedInputと発火入力への接続はない。background専用producerの拡張は含めない。 |
| review-freeze | zero-Unitのstage reviewにも適用。確定済みレビュー後の資料変更を止める専用hook入口は未接続。 |
| reviewer-scope | 通常zero-Unitではdispatch記録なしの経路。入口/heartbeatの接続が必要。兄弟Unitの越境経路を今回の必須へ増やさない。 |
| TaskUpdate / runtime-graph | sync-workflow-stateとrebuild-stage-graphのhook入口は未接続。set-status相当と公開遷移後の再構築を、表示だけの処理と区別する。 |
| learnings | surfaceとpersistを分離する必要。現runtimeのhook入口には対応なし。空選択/追加/重複/不正のpersistをRust更新へ接続する作業が残る。 |
| health記録 | HEALTH_CHECKEDの監査語彙はあるが、U3が使う診断実施記録の更新入口は未接続。doctor表示・検査そのものはU3。 |
| fold-usage | 通常登録があり、無効時の無変更だけがU1採取済み。有効時のtranscript pointerとledgerの副作用分類・必要な接続は残る。 |

run-sensors、プラグイン製品、自律swarm、他ハーネスは今回追加しない。Step 7全体のチェックは未完のまま保持する。
