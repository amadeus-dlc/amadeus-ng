# ゴールデンフィクスチャ — upstream 配布物 `3c3146cf` (v2.6.40)

このディレクトリのファイルは **upstream の配布物そのもの**であり、amadeus-ng の実装が
Published Language（`12-workflow-definition.md` §3）を本家と同一に読めることを検証する
パリティテスト（`modules/core/interface-adapter/tests/golden_parity_test.rs`）の入力である。

## バイトを変更してはならない

**このディレクトリの `*.json` は 1 バイトたりとも変更してはならない。**
整形・キー並べ替え・末尾改行の追加削除・BOM 付与のいずれも禁止する。
これらのファイルは「本家が実際に配布しているバイト列」であることに価値があり、
編集した瞬間にゴールデンとしての意味を失う。

更新が必要になるのは upstream のピン留めコミットを動かすときだけで、その場合は
このディレクトリごと新しいコミット ID のディレクトリを作り、`docs/specs/00-policy.md` の
ピン留め方針に従って差分を審査する。フォーマッタ・リンタ・エディタの自動整形が
このディレクトリに掛からないよう注意すること。

## 出典

| 項目 | 値 |
| --- | --- |
| 上流リポジトリ | <https://github.com/awslabs/aidlc-workflows> |
| ピン留めコミット | `3c3146cfd7cef33020d48e8d48d4e80d0f8c2820`（`3c3146cf`、v2.6.40、branch `v2`） |
| 取得日 | 2026-08-22 |
| 取得方法 | `curl -fsSL https://raw.githubusercontent.com/awslabs/aidlc-workflows/3c3146cf/<path>` |

| ファイル | upstream パス | bytes | ハッシュ |
| --- | --- | ---: | --- |
| `stage-graph.json` | `dist/claude/.claude/tools/data/stage-graph.json` | 81,850 | md5 `3ee59d7a177bd55d2e8392fb9028561d`<br>sha256 `c7afda6e0c57a7a248cb6322878d3ed3c58b14d7b483269e03add20d436bab8c` |
| `scope-grid.json` | `dist/claude/.claude/tools/data/scope-grid.json` | 13,509 | sha1 `60fb4547307a925456bafbcfabf2ffd408552f1d`<br>sha256 `326deb8be9e027f832adf21f37e89c3fa86e531840233852d7be5d9bc5ff67aa` |

`stage-graph.json` の md5 は as-built 仕様 `docs/upstream/specs/00-overview.md:445`（測定 M18）の実測値と一致する。
`scope-grid.json` については as-built 仕様 `docs/upstream/specs/01-workflow-model.md:1133` が `60fb4547…` を挙げるが、
**これは md5 ではなく `shasum`（SHA-1）である**（測定コマンドが `shasum` であることと整合）。参考までに
`scope-grid.json` の md5 は `ef5c35ef6e6a31ffb636383d673dd31f`。

## ライセンス

upstream `awslabs/aidlc-workflows` は **MIT-0（MIT No Attribution）** で公開されている
（Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.）。
MIT-0 は帰属表示を要求しないが、トレーサビリティと礼儀のためリポジトリルートの
[`NOTICE`](../../../NOTICE) に Amazon の著作権表示とピン留めコミットを記載済みである。
本ディレクトリのファイルはその NOTICE の適用範囲に入る。

> 補足: 一部の作業指示に「upstream は Apache-2.0」という記述があるが、
> ピン留め `3c3146cf` の `LICENSE` を実取得して確認した結果は **MIT-0** であり、
> リポジトリの `NOTICE` の記載（MIT-0）が正しい。

## 採取レポート

このフィクスチャの中身に対する全数実測（33 ノード・11 スコープ列・363 セル・
`FIELD_ORDER` 28 の部分列検証・`enabled` 0/33 など）は
[`docs/specs/research/golden-3c3146cf-graph-dist.md`](../../../docs/specs/research/golden-3c3146cf-graph-dist.md)
に記録してある。パリティテストのアサート値はすべてこのレポートの実測に由来する。
