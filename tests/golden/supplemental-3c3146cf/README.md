# 固定ピンの補完観測

2026-09-06、既存コーパスで未採取だった3経路を、前提を明示した合成入力で実測した。
upstreamのコードは変更せず、262ファイルの配布物マニフェストを照合してから実行する。
`cases.json`は決定的な観測値、`provenance.json`は採取日時・コマンド・版と入力区分を記録する。

| ケース | 入力と観測 |
|---|---|
| continue/multi-part | project.mdへ240個の合成ルールを与え、7分割の配送からrun-stageまで継続。番号順と全ルールの一度だけの到達を確認 |
| set-autonomy/gated | ツール単独生成の状態は行不足でexit 1。その対照として、契約テンプレートが定めるAutonomy Mode行を合成入力へ追加すると、gatedへの切替とstate_updated=trueを観測 |
| transcript-carve-out | 合成した会話だけのJSONLではStopは素通し。同じターンにエンジン呼出を追加した対照ではdecision:block |

set-autonomyの正常系は「ツールが自然に生成した状態で到達できた」とは主張しない。
初回採取の行不足という観測は有効なままであり、今回のケースは前提を満たしたときの動作を補う。
既存 `upstream-3c3146cf/` の採取済み入力・期待出力・未採取一覧は履歴として変更していない。

再現は `scripts/goldens/capture-supplemental.ts` を使う。検証済みの `dist/claude` と出力先を引数で渡す。
配布物はvendorリポジトリの固定コミット `3c3146cfd7cef33020d48e8d48d4e80d0f8c2820` から取得する。
当該Gitオブジェクトがある場合は `git archive` で `dist/claude` を使い捨てディレクトリへ取り出せる。

```sh
bun scripts/goldens/capture-supplemental.ts /path/to/verified/dist/claude /tmp/supplemental-check
```

採取スクリプト自身が全経路と対照の結果を検査してから保存する。Rustのコーパス試験でも
補完ケースと元の未採取一覧、分割配送・自律モード・Stop判定の整合を検査する。
