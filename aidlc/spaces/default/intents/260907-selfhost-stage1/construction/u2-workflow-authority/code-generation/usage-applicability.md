# fold-usageの適用と副作用

2026-09-09。Step 7の必要性調査。固定本家 `a277af218f0df7f325d3b8be7b6d90fce2c5bd40` のClaude配布を読んだ。現時点で有効時のRust実装・互換検証は未完了である。

## 通常実行での適用

`aidlc-fold-usage.ts` はPreToolUse・PostToolUseに登録されている。`usageTrackingDisabled()` は環境変数が文字列 `1` のときだけ停止する。本プロジェクトのClaude登録にも存在し、設定ファイル内に常時無効化する指定はない。U1の無効化した採取だけを根拠に、通常のClaude実行でも不要とは判断できない。

| 発火点 | 本家のfold方式 |
| --- | --- |
| 一般のPreToolUse | seal-main。mainの直近assistant群を確定する |
| 工程更新を伴うPreToolUse | flush-all。subagent側の完了群も確定する |
| PostToolUse | holdback。最後の未確定message-id群を保留する |
| Stop | flush-all。全ファイルの完了群を確定する |

## 書込みと読取り

- 現在session ID、session別/共通transcript pointerを更新する。
- `aidlc/.aidlc-sessions/usage-ledger.json` を原子的に更新する。ファイルごとの読取り位置・保留群、session/workflow/stage/model/agent別の集計を持つ。
- fold hook自身はstate・audit・approval authorityを直接更新しない。stdoutは空、終了0、失敗も元のフローへ影響させない契約である。
- `aidlc-state.ts` は完了時に `stageUsageAuditFields` / `workflowUsageAuditFields` を読み、トークン数・キャッシュ数・Cost USD・モデル/agent別内訳をSTAGE_COMPLETED / WORKFLOW_COMPLETEDへ入れる。
- 利用データなし、利用データはあるが価格不明、価格計算可能の3状態を区別する。未取得を0ドルとして表示する契約ではない。

## U2・U4へ残す接続

fold-usageを表示専用の補助処理として完了扱いにはできない。通常有効なproducerの更新と、完了時の利用量観測を扱う必要がある。Step 7の有効経路とStep 4の完了監査の残件へ含める。U1で未採取だった有効経路を追加採取し、分割行・message-id重複・保留・flush・工程の切替・再実行で検証する。

表示・集計読取り関数についてはC6の再利用候補としてU4へ渡せるが、操作単位で副作用とRust投影の読取り互換を確認する。状態・監査・承認の更新をTSへ戻す経路や、無効化を暗黙の既定とする変更は追加していない。

## 根拠

- `.claude/hooks/aidlc-fold-usage.ts:71–128`
- `.claude/tools/aidlc-usage.ts:148–150`、`:767–769`、`:1200–1264`、`:1299–1327`、`:1610–1694`
- `.claude/hooks/aidlc-continue-workflow.ts:1319–1345`
- `.claude/tools/aidlc-state.ts:230–243`、`:4107`、`:4364`、`:4378`、`:5107`
- 要求FR4、契約C6、承認済み計画Step 7
