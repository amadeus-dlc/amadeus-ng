# 差分採取の手順と結果

corpus: differential-corpus.txt (101 行)
cwd: /r 固定

1) 本家 + 承認済み是正パッチ (.claude/hooks/review-freeze-command.ts)
   -> differential-upstream-patched.tsv
2) 本家そのまま (vendor/aidlc-workflows/dist/claude/.claude/hooks/review-freeze-command.ts, a277af21)
   -> differential-upstream-vendor-unpatched.tsv
3) 本 build (harness-infrastructure::ShellWriteTargets)
   -> differential-rust.tsv

diff (1) vs (3): 差分 0 行 / 101 件
diff (2) vs (3): 差分 3 行 (differential-vs-unpatched.diff) — いずれも 2>&1 / 2>&- で
  未是正の本家だけが /r/2 を書込み先として拾う既知不具合

## 再現手順

(1)(2) は bun で該当ファイルの `shellWriteTargets` を直接呼ぶ。
(3) は `harness-infrastructure` に一時の integration test を置いて同じ corpus を流し、
結果を TSV へ書き出した。**その一時テストは記録後に削除している**（CI には載っていない）。

```ts
// bun で走らせた採取スクリプト（(1) の例。(2) は import 先を vendor へ変える）
import { readFileSync } from "node:fs";
import { shellWriteTargets } from "<repo>/.claude/hooks/review-freeze-command.ts";
const corpus = readFileSync(process.argv[2], "utf-8").split("\n").filter((l) => l.length > 0);
for (const command of corpus) {
  process.stdout.write(`${JSON.stringify(command)}\t${JSON.stringify(shellWriteTargets(command, "/r"))}\n`);
}
```

```rust
// (3) 側の一時テスト（削除済み）。cwd は同じく "/r"。
let targets = ShellWriteTargets::parse(command, std::path::Path::new("/r"))
    .fold_left(Vec::new(), |mut acc: Vec<String>, target| { acc.push(target.to_string()); acc });
```

corpus が踏んでいない枝: `cp` / `install` / `mv` の宛先が実在ディレクトリのときの
`statSync` 判定（`/r` が実在しないため両側とも「ディレクトリでない」で一致してしまう）。

## 比較基準 (1) の来歴を検証した

`.claude/hooks/review-freeze-command.ts` が「固定コミット + 承認済みパッチ」であることを
実際に組み直して確かめた。vendor の同名ファイルへ `shell-redirection-tokens.patch` を当てた
結果と、リポジトリの `.claude/` 側は**バイト一致**する。

```
patch -p1 --forward --batch -i scripts/aidlc-sync/patches/shell-redirection-tokens.patch
diff <組み直したもの> .claude/hooks/review-freeze-command.ts   # 差分なし
```
