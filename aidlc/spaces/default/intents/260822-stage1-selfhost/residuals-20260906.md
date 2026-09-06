# 2026-09-06 残件の再確認と是正

ユーザーの「あるなら続けて」に基づき、古い所見を現物で確認して修正した。並行して提示されたファーストクラスコレクションの方針は、coding-rules/first-class-collections.mdへ記録した。

## 是正した問題

| 対象 | 実測と対応 |
|---|---|
| 欠落一覧が空だと失敗する検査 | 空のmissingを与えて失敗を再現。空を許可し、残る各項目の理由・証拠・後続対応と来歴の件数一致を検査するよう変更 |
| CLONEの広すぎる正規化 | 記録名example-1234abcdとupstream-3c3146cfまで消えることを再現。形による推測を削除し、実測して与えたcloneだけを置換。採取済み出力の固定点は維持 |
| フックの適用区分 | 14ケースを終了コード・stdout判定・監査イベントで検査。記録用フックには拒否区分がないことをREADMEへ記載 |
| 上書きケースの誤説明 | artifact-updated-by-overwriteの実出力はARTIFACT_CREATED。既存バイトは保存し、更新経路の証拠として使わないことを明記 |
| 未採取3経路 | 同じ固定ピンの262ファイルをマニフェスト照合後、合成前提で実行。7分割の継続配送・自律モード切替・会話だけのStop素通しを補完観測へ保存。自然生成状態でのset-autonomy失敗も対照として保持 |
| CI設定検査の古い許容差 | 現行coverageが0.01なのに検査器が0.05を要求する失敗を実測。検査器を承認済みの0.01へ是正し、正規表現でなく文字列の完全一致で確認 |

normalization.jsonは手書きの比較設定であり、upstream配布物や採取済み期待出力ではない。この区別をREADMEにも記載した。既存の入力・期待出力JSONは変更していない。補完観測はtests/golden/supplemental-3c3146cfへ分け、前提・採取コード・来歴を保存した。同じ入力での再採取はcases.jsonがバイト一致した。

## 検証結果

- ワークスペース試験: 2,225成功、失敗・無視0。
- tools/lint: fmt・Clippy成功、93試験成功。
- ワークスペースfmt・Clippy・cargo lint成功。
- ハーネス・同期・承認制御: 47試験成功、同期チェック成功。
- Quint: 全チェック成功。
- cargo audit: workspaceの125依存とtools/lintの5依存を、取得した1,239件のadvisoryで検査。両コマンド成功。
- CI設定・GitHub ruleset: 20項目成功。
- カバレッジ: headとorigin/main（f6726802）はともに99.13072110635495%。90%床と相対許容差0.01を通過。
- 独立レビュー: 対象差分に追加修正が必要な欠陥なし。

ログは/tmp/residual-workspace-tests.log、residual-clippy.log、residual-cargo-lint.log、residual-lint-tests.log、residual-harness.log、residual-quint.log、residual-audit-workspace.log、residual-audit-lint.log、residual-governance-green.log、residual-coverage.log。CIのリモート実行結果とは区別する。

## 記録上の限界と次の作業

過去のRedが独立コミットになっていないという履歴は後から捏造して補わない。今回の成功を過去の証明へ読み替えず、元のレビュー・要約を履歴として保持する。

補完観測は、明示した合成前提での経路到達と選択した出力の証拠である。本文全体・監査・状態差分を含めた全CLI互換性の検収を代替しない。既存の自然生成状態がset-autonomyの必須行を欠く事実も変更していない。

機能設計R-08の正式な所見処理と、U10 CI要件文書の更新は、AI-DLC上の確認・レビュー工程が残る。今回のコード修正や検証成功だけでその承認を代行していない。

コレクション操作の方針は規則化した。具体型へのcombine/divide/filter/map/fold_left/atの一括追加は今回実施していない。型ごとの順序・重複・空の制約、変換先のコレクション型を明確にして適用する。StageGraph.atは既存。イテレータをクロージャへ移すだけでユースケースのTell Don't Ask違反を許容しない。
