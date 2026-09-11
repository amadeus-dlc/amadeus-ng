# Stage-1追加採取の対象と本家引用

## 正本

以下の相対パスはすべて本家 `a277af218f0df7f325d3b8be7b6d90fce2c5bd40` の `dist/claude/.claude/` を基準とする。展開先 `/tmp/amadeus-u1-upstream/dist/claude/` の固定実バイトで調べた。配布設定 `settings.json` のmatcherと、実装の入力・副作用を照合した。引用は実装の該当行であり、本一覧は採取済みという宣言ではない。

## C3: 主要4本以外のフック

| 採取対象 | 固定元の引用・行 | 最短経路で必要なケースと観測 |
| --- | --- | --- |
| `session-start` | `hooks/aidlc-session-start.ts:202–212`: `source === "startup"`→`SESSION_STARTED`、resume→`SESSION_RESUMED` | 記録前のstartup、記録のあるstartup/resume。監査・session binding・出力を保存。現在の採取にあるsession/startはこの正常経路へ対応 |
| `session-end` | `hooks/aidlc-session-end.ts:67–77`: `Missing identity therefore fails closed`、`:85`: `SESSION_ENDED` | 記録へ結び付いたsessionの終了と、結び付かないsession。別intentへ監査を付け替えない。ワークフロー完了とは区別 |
| `log-subagent` | `hooks/aidlc-log-subagent.ts:49`: `completeSubagentInflight`、`:66`: `Status ... !== "Running"`、`:92`: `SUBAGENT_COMPLETED` | Runningの完了、記録なし/Completedの無視。agent_type/id/last_assistant_messageと監査の値を確認。REとコード生成の委譲で使う |
| `deliver-stage-rules` | `hooks/aidlc-deliver-stage-rules.ts:320–327`: `dispatchHookOutput` のerrorはexit2、`:356`: `process.stdout.write(output)` | Task/Agentへの規則付与と無関係入力の無変更。正常なupdatedInputの全文を採取。自律・background専用の台帳追加を必須実装へ広げない |
| `review-freeze` | `hooks/aidlc-review-freeze.ts:345`: `REVIEW_FREEZE_BLOCKED`、`:372`: `return 2` | 確定したレビュー後の対象資料編集の拒否、無関係な資料の許可。zero-Unitのstageレビューでも適用される |
| `reviewer-scope` | `hooks/aidlc-reviewer-scope.ts:753–754`: `0 allow, 2 block`、`:725`: `REVIEWER_SCOPE_BLOCKED` | 今回のbugfixはzero-Unitなので、per-unit dispatch記録を捏造しない。記録なしの通常経路の出力/heartbeatを採取。兄弟Unitへの越境拒否をbugfixの必須範囲として増やさない |
| `sync-workflow-state` | `hooks/aidlc-sync-workflow-state.ts:96–103`: TaskUpdateの`in_progress`と`[slug]`、`:115`: `set-status` | TaskUpdateで段階を示したときの状態変更、status/slugが該当しない無視。更新処理なので読取り専用TSとして残せない |
| `rebuild-stage-graph` | `hooks/aidlc-rebuild-stage-graph.ts:133–136`: `report`はpublic transition、`:194`: `transitionRegex`、`:240–242`: `aidlc-runtime.ts ... compile` | reportの後で監査に遷移あり→runtime-graph更新、非遷移のBash入力→無視。コンパイル済み配布グラフの書換えと混同しない |
| `validate-state` | `hooks/aidlc-validate-state.ts:43`: `invalidateActiveDirectiveContext`、`:65`: recovery breadcrumb、`:76`: `SESSION_COMPACTED` | PreCompactが発火した場合のみ。正常構造/必要セクション欠落で出力・復帰情報・監査を採取。強制的にコンパクションを追加する要求ではない |
| `fold-usage` | `hooks/aidlc-fold-usage.ts:71`: `usageTrackingDisabled()`、`:115–120`: session/transcript更新と`foldTranscriptIntoLedger` | 利用量集計が有効なら発火する。既存採取の無効化前提を明示し、無効時の無変更を確認する。有効状態の利用量を未実装なのに成功したと扱わない。OTel追加とは別問題 |

`run-sensors`は製品スコープ外。`plan-approval-guard`は担当の保護有効採取で正常・回答前・別session・対象/内容変更・古い受領を扱う。主要4本と合わせて「登録名だけ」でTS再利用可否を決めない。

## C6: 表示・計算と更新を分けるケース

| 操作 | 固定元の引用・行 | 採取すること |
| --- | --- | --- |
| `review-brief summary` | `tools/aidlc-review-brief.ts:1056–1070`: `Summary brief requires --questions-file <path>.` | questionsファイルの内容確認表示、引数不足の拒否。前後の状態/監査/承認が不変 |
| `review-brief review/context` | 同`:1030–1054`: `Review brief requires --why <first|revision|stale>.`、`hydrateReviewArtifactContexts` | READY/NOT-READY資料・指摘ありを表示し、contextが監査のdispositionを反映する。必要引数不正。呼出し前後を保存 |
| `testing-posture render/fingerprint` | `tools/aidlc-testing-posture.ts:1824–1866` | 担当のstage1採取を使用。計画と手順が揃う場合の出力および不足時の拒否。beginは更新操作なので不変の対象外 |
| `learnings surface/persist` | `tools/aidlc-learnings.ts:293`、`:684`、`:699–706`: surface時のspace/intentへ固定、`:849–861`: `RULE_LEARNED` | surface結果、空選択のpersist、規則1件のpersistと再実行で重複しないこと、slug不一致の拒否。sensor提案は対象外 |

これらは「表示候補」であって再利用の確定ではない。出力の2.7.1比較と副作用検査が揃ってからU4で判定する。DocumentKBはbugfixの必須呼出し表にないので採取ケースを新設しない。

## C7: doctor採取

親が `scripts/goldens/capture-doctor.ts` / `capture-doctor.test.ts` を担当する。関数 `captureDoctor(dist)` が観測配列を返す。固定元の全文stdout/stderrとファイル変化を保存し、C7に採用した行だけを後続で対応付ける。本家全文の結果をネイティブのサブセット結果へ偽装しない。

- 初回: `tools/aidlc-utility.ts:4669–4682` の `auditExists`、`:4737–4746` の条件付き`HEALTH_CHECKED`。coldは無書込み、initializedは診断記録あり。
- 構成欠落: `:2266–2278` Bun、`:2288–2349` hooks、`:2379–2447` disableAllHooks/managed-only、`:2713–2720` settings。
- heartbeat: `:3073–3218`。初回・進行後未発火・不読・300000ms境界と超過。
- 配布整合: `:4177–4377` cycle/orphan/schema/references/scope。2スコープ限定の数え方はC7で明示されたネイティブ側の選択であり、本家全文の数を改変しない。
- 状態版: `:3360–3409` のreadable/current/compatibleと版8。
- 表示/終了: `:4692–4736` と `:4794–4798`。通常のstderr、失敗数とexitを保存する。汎用履歴診断はadvisoryとして別枠に残す。
- C7のNative workflow identity、event store、projection等は本家に存在しない。U2/U3の内部検証へ引き継ぎ、本家の期待出力を作らない。

