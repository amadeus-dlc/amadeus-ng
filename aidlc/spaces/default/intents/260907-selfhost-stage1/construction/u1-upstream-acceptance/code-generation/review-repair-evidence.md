# 第1回レビュー指摘の是正証跡

## R-01: ファイル面とJSONのUTF-8

`initial_files`、`changed_files`、`fixture_changes`へそれぞれFF/FEの異なるバイトを入れ、両辺をsealした。修正前は3件とも比較CLIが誤って終了0となった。修正後は、不正UTF-8を文字列へ変換せず、元の数値バイト配列として比較する。正常なUTF-8だけが文字列正規化の対象になる。

外側のobservations.jsonの文字列中へ不正UTF-8を直接入れた場合も、修正前はseal後の単体検査が終了0だった。すべてのJSONコンテナを厳密なUTF-8として検証し、不正な場合は終了1で拒否するようにした。

```bash
bun test ./scripts/goldens/compare-corpus.test.ts
```

ファイル面のRedは10成功/3失敗、Greenは13成功/失敗0。コンテナのRed確認後の最終Greenは14成功/失敗0・27 assertions。[ファイル面Red](test-evidence/r01-red.log)、[コンテナRed](test-evidence/r01-container-red.log)、[最終Green](test-evidence/r01-container-green.log)を保存した。

## R-02: 浅いsubmodule取得

fork tip `801c570062f67dc8f4952ee5fc601381d09db7ec` だけをdepth1で取得した隔離repoで、本家 `a277af21` の `git archive` が終了128となることを確認した。CIにあった取得条件をそのまま再現し、修正前は取得準備後のarchiveも128で失敗した。

CIのBun検査直前へ次の処理を追加した。チェックアウト済みforkのHEADや配布作業ツリーは変更しない。

```bash
git -C vendor/aidlc-workflows fetch --depth=1 --no-tags https://github.com/awslabs/aidlc-workflows a277af218f0df7f325d3b8be7b6d90fce2c5bd40
```

テストはCIの該当コマンドを読み、通信先だけを固定コミットを持つローカルGitへ置き換えて、同じ浅いrepoで実行する。archiveが成功し、配布のaidlc-version.tsを含むことを確認した。採取元テストは修正前8成功/1失敗、修正後9成功/失敗0・36 assertions。[Red](test-evidence/r02-red.log)、[Green](test-evidence/r02-green.log)を保存した。

加えて、空の一時repoから上記本家HTTPSへ実際にfetchし、終了0と固定コミットの存在を確認した。remoteからの応答は `a277af218f0df7f325d3b8be7b6d90fce2c5bd40 -> FETCH_HEAD` だった。CI全ジョブが実行済みという主張とは区別する。

## R-03: Unitに割り当てられた要求範囲

期待するFRは、検査対象のupstream_idsやcoverageから選ばない。Unit DAGで存在を確認した対象と単位定義のID対応、承認済み割当表の主担当・Directory・支援列から選ぶ。説明欄のUnit名は割当として扱わない。共通NFRの従来の検査は維持する。

- U1が支援するFR1を受理する正常系は、修正前に3配布先すべてで失敗した。
- 未割当FR2を検査対象JSONへ追加するケースも、修正前の誤受理を確認して是正した。担当FRを宣言とcoverageから両方消しても、権威ある入力から欠落を検出する。
- zero-Unitは要求全体を検査する。Unitがある場合はDAGの宣言を必須とし、ディレクトリ名だけからUnitを捏造しない。
- 他Unit、割当表欠落、説明欄だけの言及、既存US→ACの経路を確認した。要求・割当表と実際のU1 traceability.jsonは書き換えていない。

変更は `scripts/aidlc-sync/patches/traceability-unit-requirements.patch` に保存し、既存のFRマッピングパッチの後へ適用した。各Greenで `bun scripts/aidlc-sync.ts --apply` を使用し、配布3本とinstalled.jsonを正規更新した。

```bash
bun test ./scripts/aidlc-traceability.test.ts
bun .codex/tools/aidlc-sensor-traceability.ts --output-path aidlc/spaces/default/intents/260907-selfhost-stage1/construction/u1-upstream-acceptance/code-generation/traceability.json
bun scripts/aidlc-sync.ts --check
```

最終回帰は30成功/失敗0・129 assertions。実際のU1は `pass: true`、`findings_count: 0`。同期は差分0。[Red 1](test-evidence/r03-red1.log)、[Red 2](test-evidence/r03-red2.log)、[zero-Unit Red](test-evidence/r03-red3.log)、[宣言欠落Red](test-evidence/r03-red4.log)、[最終Green](test-evidence/r03-green4.log)を保存した。

## 再採取と関連検証

```bash
bash scripts/goldens/recapture-cli.sh /tmp/amadeus-u1-review2-a /tmp/amadeus-u1-upstream/dist/claude
bash scripts/goldens/recapture-cli.sh /tmp/amadeus-u1-review2-b /tmp/amadeus-u1-upstream/dist/claude
bun scripts/goldens/verify-corpus.ts /tmp/amadeus-u1-review2-a /tmp/amadeus-u1-review2-b
bun scripts/goldens/verify-corpus.ts tests/golden/upstream-a277af21 /tmp/amadeus-u1-review2-a
```

両採取・相互比較・保存済みコーパスとの比較はすべて終了0。新しい比較器でも保存済みの本家期待バイトを変更する必要はなかった。[採取A](test-evidence/review2-a.log)、[採取B](test-evidence/review2-b.log)、[相互比較](test-evidence/review2-comparison.log)、[保存物との比較](test-evidence/review2-stored-comparison.log)を保存した。

U1検査と追跡検査の6ファイルは74成功/失敗0・352 assertions。Unit宣言欠落の追加修正は上記30テストで再検証した。配布・承認を含む既存6ファイルも92成功/失敗0・636 assertions。[U1関連](test-evidence/review2-tests.log)、[既存関連](test-evidence/review2-common.log)を保存した。

第1回Reviewは原文のまま保持している。これらは修正側の検証結果であり、第2回レビューがREADYと判断した記録ではない。
