# hook-differential — 本家 2.7.1 フックの実走行と本 build の突き合わせ

固定コミット `a277af218f0df7f325d3b8be7b6d90fce2c5bd40` の `dist/claude` を
`git -C vendor/aidlc-workflows archive <commit> dist/claude | tar -x` で一時ディレクトリへ展開し
（`hooks/aidlc-reviewer-scope.ts` の sha256 は `git show` の実バイトと一致
`b4ef0550761ce6c946941153b98d50c825ef6a4640820db2d69dcf5b25b548af`）、
`AIDLC_PROJECT_DIR=<一時 project> bun <dist>/.claude/hooks/aidlc-reviewer-scope.ts < 入力` で
本家フックを丸ごと走らせた。本 build は `target/debug/aidlc hook reviewer-scope` を、契約テストと同じ
手順（`setup_rs.sh` — `intent-create` で一時ワークスペースを作る）で走らせた。

- `gen_cases.py` — 32 ケースを、その側の絶対パスで生成する（両側同一の意味）。
- `run_side.py` — 1 側を走らせ、exit / stdout / stderr / 監査シャードの差分 / drops の差分 /
  稼働記録 `.last` / 記録不在の目印 / 差し向け記録の残存 を `*.result.json` に残す。
- `compare.py` — project dir・記録名・時刻を `<P>` `<REC>` `<TS>` に正規化して突き合わせる。
- `probe.ts` — 本家の記録・監査・稼働記録の解決先を印字する（治具の組立て確認用）。
- `hook-cases-32.json` — 32 ケースの標準入力と前状態（本家側の絶対パス）。
- `hook-results-upstream.json` — 本家の観測。
- `hook-results-this-build-before-cwd-fix.json` / `differential-hook-32-before-cwd-fix.tsv` — 修正前（12 件差）。
- `hook-results-this-build-after-cwd-fix.json` / `differential-hook-32-after-cwd-fix.tsv` — `cwd` 欠落の基点修正後（11 件差）。

正規化で消しているのは project dir・記録名・ISO 時刻だけである。本家の監査値に現れる
`<project-dir>` は本家自身の置換結果であり、正規化ではない。
