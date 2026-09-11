# セッション開始文面の描画

2026-09-09。Step 7担当から分担した、Claude SessionStartのadditionalContext描画部品である。Sessionの帰属・ファイル更新・監査・権限の判断は担当しない。

## 公開API

`harness_claude::SessionWorkflowFields` が7個の表示値を、`SessionContextNotices` が判断済みのrebind案内・Unit行・復旧記録の有無・未コンパイル工程一覧を持つ。どちらも完全コンストラクタと読取りアクセサだけを持つ。

`SessionStartContext::workflow`、`runtime`、`rebind_probe` がそれぞれの文面を作る。`to_json_line()` はadditionalContextのJSONを返し、末尾改行は呼出側が付ける。`format_rebind_offer` は渡された旧slug・現slug・切替命令を文面へ写し、再選択の可否や切替命令を判断しない。

欠落フィールドの既定値、Unit行の有無、rebindが必要か、ファイルやグラフの走査はcallerの責務である。未コンパイル工程の案内は今回のClaude配布の `.claude` を使う。

## 本家採取と結果

`capture-session-start-context.ts` が固定本家 `a277af218f0df7f325d3b8be7b6d90fce2c5bd40` の277ファイルを照合し、SessionStartの文面式を原文のまま実行する。context、cold、probe、recovery、drift、rebindの式を使い、`session-start-context.json` に原出力を保存した。

11文面のJSON全文と、再選択案内2文面の本文が一致した。通常、Sessionなし、空の表示値、Unicode・改行・引用符、復旧案内、再選択案内、複数drift、補足の順序を含む。冒頭ラベルしか描いていない実装でRedを取得し、全情報と本家の定型文を描いてGreenへ進めた。採取スクリプトの式抽出エラーは製品のRedに数えていない。

harness-claude全体の単体・契約・docテストも成功した。定期cargo lintは24回目まで成功。最終Clippyは並行作業の確認後に追記する。

実フックのIO・イベント・セッション帰属を含む検証は、この純粋描画比較では代用しない。Step 7担当が既存 `session-hooks.json` の実観測と合わせて接続を検証する。

## 変更ファイル

- `modules/harness/claude/src/session_start_context.rs`
- `modules/harness/claude/src/session_workflow_fields.rs`
- `modules/harness/claude/src/session_context_notices.rs`
- 同crateのlib.rs、`tests/session_start_context_contract.rs`
- `scripts/goldens/capture-session-start-context.ts` と追加採取JSON

Red/Green・全回帰ログは `test-evidence/session-context-*.log` に保存した。
