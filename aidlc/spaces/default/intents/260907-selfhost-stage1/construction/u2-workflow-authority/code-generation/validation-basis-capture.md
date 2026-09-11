# 完了時検証根拠の追加採取

2026-09-09。`capture-validation-basis.ts` で固定本家2.7.1の `stageValidationAuditFields` を実行し、22観測を `tests/golden/selfhost-stage1/validation-basis.json` に保存した。検証根拠をnativeの保存・監査へ接続したという報告ではない。

## 採取条件

固定元は `a277af218f0df7f325d3b8be7b6d90fce2c5bd40`。配布277ファイルのmanifestを照合後、一時配置へコピーして実行した。明示的なrecordPath、codekbRepos、state、stage、stages、実ファイル・ディレクトリを渡している。入力はJSONにすべて保存し、出力を正規化していない。

合成fixtureの期待ハッシュは本家関数が計算した。最後の6ケースは固定配布グラフそのものを入力にする。

## 観測

- 必須成果物の不在・存在、CRLFを含む原バイト。
- 任意入出力の不在・存在、任意入力の生成元なし。
- 必須入力の生成元なし・複数生成元。この2件はValidation Warning。
- Brownfield / Greenfield / 種別不明の条件付き入力。
- directoryを通常fileと扱わないこと。
- test-results.md / traceability.jsonの既知ファイル名。
- requires_stageの変更だけではgraphContractを変えないこと、execution/conditionの変更は契約へ入ること。
- spaceのcodekbにある入力。
- bugfixの非初期化6工程（reverse-engineering、requirements-analysis、code-generation、build-and-test、deployment-pipeline、deployment-execution）。Unitを作らない工程直下の解決を含む。

結果はValidation Basisが20件、Validation Warningが2件。別の一時rootと出力先 `/tmp/amadeus-u2-validation-basis-repeat.json` で再採取し、`cmp` 終了0でファイル全文の一致を確認した。最初の採取スクリプトに存在しない工程slugを記した失敗は、製品のRedには数えない。

## 残る範囲

この素材はnativeの指紋計算・ファイル解決・保存DTO・通常RMU投影を検証する入力である。読取り不能時のOS診断ハッシュ、複数repoやUnit DAG、権限・レビュー・センサーの完了前検査は、この採取だけでは検証していない。公開CLI全体の状態・監査比較も別に必要である。
