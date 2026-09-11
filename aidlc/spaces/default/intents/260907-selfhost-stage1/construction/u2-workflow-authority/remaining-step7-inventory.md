# Step 7 残作業の現物棚卸し

2026-09-09。製品コードを変更せず、現ディスクのRust入口と本家固定コミット `a277af218f0df7f325d3b8be7b6d90fce2c5bd40` のClaude配布を照合した。過去の「入口なし」メモを現在の状態として転記していない。

以下の本家参照はすべて同コミットの `dist/claude/.claude/` 以下である。`git show <完全SHA>:<path>` で取得した実バイトを確認した。scope-gridのbugfixは初期化3段＋reverse-engineering、requirements-analysis、code-generation、build-and-test、deployment-pipeline、deployment-executionの9段。reverse-engineeringはpipeline、code-generationはsubagent、requirements-analysisとcode-generationにレビューがある。スモークはゼロUnitであり、コード生成のfor_each宣言だけからUnitを増やさない。

## 分類

「未接続」は必要な入口がRustへ結ばれていないという意味で、フック内の全分岐を移植する許可ではない。「再利用候補」は読取りと副作用の実検証前であり、採用済みではない。

| 対象 | 現コード | Claude bugfixへの適用・残作業 | 根拠 |
| --- | --- | --- | --- |
| セッション開始/終了、subagent終了、PreCompact | 実装済み。セッション契約13件＋文脈失効4件が成功 | 直前のセッションsliceは完了。ただし、作業作成直後のstamp/handoffは下記の別経路で欠落 | `runtime.rs:980`以降、`runtime/session_start.rs`、`runtime/session_hooks.rs`、`session_hooks_contract.rs`、`session_compaction_contract.rs` |
| learnings surface | Rust入口なし。表示のみのTS再利用候補 | 各通常ゲート前に呼ばれる。状態・runtime-graphのstage行・memory_pathが必要なので、単に空候補を返して済ませられない。Open questionsは候補に昇格せず別配列 | 本家`aidlc-common/protocols/stage-protocol.md:1055–1110`、`tools/aidlc-learnings.ts:293–374` |
| learnings persist | 未接続。`RULE_LEARNED`の語彙だけがある。既存practices-promoteは別契約 | 空選択/追加/重複/不正選択、surface時点のspace/intent固定、規則行と監査の片側欠落復旧が必要。センサー生成・stage frontmatterのbind分岐は今回の製品範囲外 | 現`cli/face.rs:30–41`、`cli/request.rs:147–207`、`workspace/audit_events.rs:154`。本家`tools/aidlc-learnings.ts:684–948`、通常規則は`768–872`、センサーは`875`以降 |
| 診断実施記録 | 未接続。`HealthChecked`は監査イベントの列挙のみ。doctorは読取り要求へ分類されるが更新コマンドなし | C7がU2に割り当てた必須更新。起動時に監査ありの場合だけRequest/Detailsを持つ1件を記録。診断表示自体はU3。coldで空SQLite/監査を作らず、記録失敗を成功にしない | 現`cli/request.rs:250`付近、`workspace/audit_events.rs:127`。本家`tools/aidlc-utility.ts:4669–4674,4737–4746`。承認済み`contract-summary.md` C7 |
| 規則受渡し | `next/continue`の規則配送は実装済み。`deliver-stage-rules`のhook入口とupdatedInputなし | RE/CGの通常Task/Agentへ適用。通常foregroundは規則を読むだけで正本更新なしの再利用候補。stage明示パス→現在stage→唯一のslugの順、exact bundleの重複抑止、出力512KiB超の拒否を保持。run_in_backgroundのinflight更新は通常スモークに追加しない | 現`steering.rs`、RMU`read_tables/steering_plan_row.rs`、`runtime.rs:980`許可一覧。本家`hooks/aidlc-deliver-stage-rules.ts:76–165,193–221,266–358` |
| review-freeze | 未接続。レビュー受領の記録・鮮度判断は既存実装にあるが、書込前拒否hookも`REVIEW_FREEZE_BLOCKED`保存経路もない | ゼロUnitでもレビュー付きRA/CGに適用。terminal receiptからゲートまでのproduces/optional_produces書込を止める。成功した最短一周でblockが起きなくても、保護境界を無視してよい意味ではない。兄弟Unit分岐は不要 | 現`runtime.rs:980`、`review_closures.rs`、`review_attempt.rs`、`workspace/audit_events.rs:120`。本家`hooks/aidlc-review-freeze.ts:1–46,250–375` |
| reviewer-scope | 未接続。heartbeatも含めRust入口なし | 通常登録はある。ゼロUnitではdispatch記録を作らず、Unit間越境の禁止分岐は非適用。ただしheartbeat、およびreviewerがconstructionを読む際の記録不在助言/dropは発火し得るため、全hookを「無副作用・不要」とはしない | 本家`aidlc-common/protocols/stage-protocol-reviewer.md:96–98,155`、`hooks/aidlc-reviewer-scope.ts:756–860,863–932`。現`runtime.rs:980`、`workspace/audit_events.rs:119` |
| TaskUpdate同期 | 未接続。stateの通常report投影はあるが、TaskUpdate用入口やutility set-statusなし | Claudeのstage開始でstatus=in_progress＋activeForm末尾[slug]が規定されるため適用。同stageでもLast Updatedや指示の状態指紋更新がある。Kiro IDEのaudit-tail同期とUnit-major例外を今回必須へ増やさない | 本家`aidlc-common/protocols/stage-protocol.md:133,575–613`、`hooks/aidlc-sync-workflow-state.ts:92–120`、`tools/aidlc-utility.ts:7685–7740`。現`cli/request.rs:147–207` |
| runtime-graph再構築 | 未接続。静的stage-graphの読取りと別物であり、Rustにruntime-graph生成やMemoryEmptyの実処理はない | report後のBash＋監査末尾の対象遷移でcompile。graphはlearnings surfaceの前提。compileはgraphだけでなく、承認完了・日誌空のstageへ重複を抑えたMEMORY_EMPTYを記録するため、全体を無更新TS補助として残せない。センサー/swarm/DAG表示全般は今回追加しない | 本家`hooks/aidlc-rebuild-stage-graph.ts:110–259`、`tools/aidlc-runtime.ts:782–808`、`tools/aidlc-learnings.ts:322–325`。現`workspace/audit_events.rs:155`のみ |
| 成功したintent-createの会話帰属 | 一部実装済み。native作成成功でbindingを更新するが、stamp/handoff producerなし | 本家rebuild hookはcompileのフィルターより先に、成功応答＋正確なsession_idからbinding/stamp/handoffを更新する。Rustの既存作成処理だけではcold sessionにstampが残らず、UUID workflowのSessionEndが監査を記録できない経路がある。最小次slice候補 | 現`runtime.rs:2536`付近はwrite_bindingのみ。`session_navigation.rs:75`はstamp書込部品、`layout.rs:31–95`はhandoff読取/消費だけ。`runtime/session_hooks.rs:end_session`は未stampのUUID workflowをdrop。本家`hooks/aidlc-rebuild-stage-graph.ts:56–105,133–136` |
| fold-usage | 未接続。SessionStartのtranscript pointer保存はあるが、usage ledger producerと完了監査への集計はない | 通常Pre/PostToolUseに登録。有効時はsession/transcript pointerとusage-ledgerを更新し、後に完了監査のtoken/cost項目へ使われる。表示だけの処理とは分類しない。U1の無効時観測だけで有効経路を成功/非適用とせず、固定元で有効transcript・重複・stage境界を追加採取して必要境界を確定する。費用表示の一般化は追加しない | 現`session_navigation.rs:write_transcript`、`runtime.rs:980`。本家`hooks/aidlc-fold-usage.ts:77–125`、`tools/aidlc-usage.ts:141–149,1244–1264,1604–1688`、`tools/aidlc-state.ts:219–245` |

## 正本更新の有無を混同しないための整理

- learnings surfaceと通常foregroundの規則受渡しは、依存する読取りがRustの出力と整合するか確認したうえで再利用候補になる。フック名や同じTSファイルの別動詞を根拠に一律Rust化/一律再利用しない。
- runtime compileはMEMORY_EMPTY、review-freezeは拒否監査、reviewer-scopeは稼働記録/dropを持つ。終了0や見た目が表示だけでも更新を隠していない。
- fold-usageはローカルledgerを書き、`stageUsageAuditFields` / `workflowUsageAuditFields`から完了監査へ影響する。無効化フラグは現物で存在するが、この棚卸しは本番スモークを無断で無効化する決定をしていない。
- Nativeの公開入口一覧から、上記の未接続hook名は現在`Unknown hook`になる。既存の監査イベント列挙、レビュー判定、静的グラフ読取りを対応するhook実装完了とは数えない。

## 最小の次実装slice

推奨は**成功した作業作成を、そのCLIを起動したセッションのstampへ結ぶ部分**。既存のbinding書込み直後に、作成したintentのUUIDを対象として本家と同じ帰属を成立させる。cold SessionStart→作成成功→SessionEndで正しい監査へ1件、作成失敗では既存帰属を変更しない、別の作業作成では既存UUIDから新UUIDへのhandoffを保持する、という公開境界を1件ずつTDDで固定する。新しい作業やホストを実環境で作成せず、一時fixture内で検証する。

この小さいsliceの後は、実行経路の依存からTaskUpdate同期・runtime compile→learnings surface読取り検証/persistへ進める。診断実施記録はU3向けの独立した小さいsliceとして実装できる。review-freezeと通常reviewer-scopeは各保護境界を分ける。fold-usage有効時は追加採取の結果を先に置く。全項目を単一の巨大変更にまとめない。

本記録は実装の完了表でも、Step 7を飛ばす承認でもない。scopeの非適用はゼロUnit・他ハーネス・センサー製品・自律backgroundなど、既存要求が明示した境界に限る。

## 作成帰属の実測と着手

棚卸し後、一時ワークスペースで既存debugバイナリのSessionStart→intent-create→SessionEndを実行した。開始・作成・終了はいずれも終了0、bindingは新しい作業を指したが、stampは存在せずSESSION_ENDEDは0件だった。実環境の作業記録やホストは変更していない。

親からこの最小sliceの修正依頼を受け、公開契約テストに終了監査1件とstampのUUID一致を追加した。`session-create-stamp-red.log` は期待1件に対し実測0件の失敗を記録している。上表はその修正前の棚卸しであり、後続の実装結果は`code-generation/session-create-attribution-verification.md`で追記する。

## 限定修正後の更新

作成帰属sliceは完了した。CLIが起動セッションを解決できる場合は、作成成功後のstamp/handoffを保存し、cold→作成→SessionEndも成功した。拒否・別会話・session不在/不正を含むセッション契約20件、lint、担当fmtは成功。詳細は[作成帰属の証跡](code-generation/session-create-attribution-verification.md)。上表の未接続扱いのうち、この直接CLI成功経路だけが解消した。PostToolUse封筒による補完・runtime compile・ほかの行は引き続き残る。

診断実施記録のU2 APIも追加した。既存IntentExecutionのHealthCheckedイベント、RecordHealthCheckUseCase、両側DTO、通常RMUのC7監査投影とSQLite結合7件を検証済み。[診断記録の証跡](code-generation/diagnostic-record-verification.md)。doctorの起動時監査観測・coldで呼ばない条件・実際の呼出しはU3の未接続として残る。公開CLI動詞は追加していない。
