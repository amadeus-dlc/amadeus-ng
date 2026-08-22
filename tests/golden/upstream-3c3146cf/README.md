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

---

## 採取ゴールデン（upstream ツールの実行出力）

上の 2 ファイルが「upstream が配布したバイト列そのもの」であるのに対し、以下の
サブディレクトリは **upstream ピン `3c3146cf` のコードを実行して採取した出力**である
（FR7.1 / FR7.2、BR2.1）。配布物ではなく採取物なので節を分けて扱う。

| サブディレクトリ | 族 | 内容 | 採取スクリプト |
| --- | --- | --- | --- |
| `hash-canonical/` | hash-canonical | 入力クラス別の受入表（`cases.json`）と来歴（`provenance.json`） | `scripts/goldens/recapture-hash-canonical.sh` |
| `cli/` | cli | CLI 主要遷移の stdout・状態差分・監査行 | `scripts/goldens/recapture-cli.sh`（後続 Bolt） |
| `hooks/` | hook | フック 4 本の代表ケース（exit code・stderr・監査行） | `scripts/goldens/recapture-cli.sh`（後続 Bolt） |
| `normalization.json` | 全族 | 非決定値の正規化規則（BR2.2） | 手書き（規則の正本） |

### 採取手順と来歴（BR2.1）

`hash-canonical/` は次の手順で採る。手順・来歴は `hash-canonical/provenance.json` に
機械可読な形で入っており、下表はその要約である。

1. upstream ピン `3c3146cfd7cef33020d48e8d48d4e80d0f8c2820` の
   `dist/claude/.claude/tools/aidlc-testing-posture.ts` を `raw.githubusercontent.com` から取得し、
   ファイル全体の sha256 を期待値と照合する。
2. `canonicalize` / `sha256` / `hashObject` の 3 関数を **104〜123 行目**から抽出し、
   抽出スニペットの sha256 を期待値と照合する（行番号がずれたら停止する）。
3. スニペットに `import { createHash } from "node:crypto";` を前置し、`function` を
   `export function` へ変えただけの一時モジュールを作る（**関数本体は 1 バイトも変えない**）。
4. bun からそのモジュールを import し、入力クラスごとに 5 つの観測を採る。

| 項目 | 値 |
| --- | --- |
| 取得 URL | `https://raw.githubusercontent.com/awslabs/aidlc-workflows/3c3146cfd7cef33020d48e8d48d4e80d0f8c2820/dist/claude/.claude/tools/aidlc-testing-posture.ts` |
| 取得ファイル sha256 | `99528925754da70e42106a35b52e5769001539042d07d0eecb5e0aa256196cb9` |
| 抽出スニペット | 104〜123 行（`canonicalize` / `sha256` / `hashObject`）、上流仕様 `docs/upstream/specs/09-cli-tools.md` §8.4 |
| 抽出スニペット sha256 | `c8894a433d620538e1701f178b8542528603f012b98680b6b79233f70704418f` |
| 採取日時 | 2026-08-22T12:40:55Z |
| 採取コマンド | `bash scripts/goldens/recapture-hash-canonical.sh` |
| bun | 1.3.13 |
| ケース数 | 30（欠落 0） |

各ケースが持つ 5 つの観測（`expected`）:

| フィールド | 採取式 | 対応するプロファイル / 族 |
| --- | --- | --- |
| `canonical_output` | `JSON.stringify(canonicalize(v))` | hash-canonical |
| `canonical_digest` | `hashObject(v)` | 正準族（`sha256:` 接頭辞） |
| `compact_output` | `JSON.stringify(v)` | contract-compact |
| `compact_digest_hex` | `sha256(JSON.stringify(v))` の生 hex | 非正準族 |
| `pretty_output` | `JSON.stringify(v, null, 2) + "\n"` | contract-pretty |

入力は原則 JSON テキスト（`input`）で表す。JSON で表せない NaN / ±Infinity のクラスだけ
`input_js`（採取時に評価した JS 式の記録）と `construct`（Rust 側も同じ木から組み立てる
宣言的な構築木）を持つ。

### 正規化規則（BR2.2）

`normalization.json` がプレースホルダ 4 種（`<TS>` / `<CLONE>` / `<ROOT>` / `<SESSION>`）の
正本である。比較器は期待値と実測値の**双方に同じ規則**を適用してからバイト比較する。
`hash-canonical` 族は純粋関数の出力しか含まず非決定値がないため、正規化を適用しない
（適用すると偽の一致を作る）。

### 更新方針（BR2.5）

**採取ゴールデンの更新は upstream ピン更新の intent でのみ行う。** ピンが変わらない限り、
再採取スクリプトの再実行は `captured_at` 以外に差分を出してはならない。ピンを動かすときは
このディレクトリごと新しいコミット ID のディレクトリを作り、差分を逸脱台帳と突き合わせて
レビューする。**実装とゴールデンが食い違ったら直すのは実装側であり、ゴールデンではない**
（BR2.3）。

### 採取で確定した upstream の実測値（ADR 0001 受入条件 (a)〜(e)）

ADR 0001 の「未確定事項」に挙がっていた (a)〜(e) は、本採取で次のとおり確定した
（ADR 本文の更新は U9 canon-docs の担当）。

| 条件 | 確定した実測値 | 根拠ケース |
| --- | --- | --- |
| (a) 再帰キーソートの照合順序 | **UTF-16 コード単位順**。非 BMP 文字（サロゲート `D83D…`）はキー `U+FB00` より**前**に並ぶ — コードポイント順／UTF-8 バイト順とは割れる | `non-ascii/utf16-vs-codepoint-key-order` |
| (a') integer-like キー | `canonicalize` の `Object.fromEntries` により、ソート後も integer-like キー（0〜2^32-2 の正準十進表記）が数値昇順で先頭に来る。`2^32-1` は integer-like ではない | `integer-like/boundary`, `integer-like/mixed` |
| (b) 数値表記 | 非指数表記の範囲は `1e-6 ≤ |x| < 1e21`。それ以外は `d.ddde±N` 形で、指数が正なら `e+` を付ける（`1e+21`, `1.5e+300`）。負指数は `e-`（`1e-7`） | `exponent/thresholds`, `exponent/fraction-boundary` |
| (b') 2^53 超の整数 | JS は f64 として丸めてから表記する（`9007199254740993` → `9007199254740992`、`-9223372036854775808` → `-9223372036854776000`） | `large-int/around-2p53`, `large-int/i64-min` |
| (c) 非有限数 | `NaN` / `±Infinity` はいずれも `null`（配列要素も詰められずその位置が `null` になる） | `non-finite/*` |
| (d) 負ゼロ | `-0` / `-0.0` はいずれも `0` | `negative-zero/*` |
| (e) 最小エスケープ集合 | `"` `\` と U+0000〜U+001F のみ。C0 のうち `\b \f \n \r \t` は短縮形、他は `\u00xx`（**小文字 hex** 4 桁）。`/`・U+007F(DEL)・非 ASCII・**U+2028 / U+2029** はエスケープせず生出力 | `escape/control-and-quotes`, `escape/line-separators` |
| (体裁) | pretty は 2 スペースインデント + `"key": value` + メンバごと改行 + ファイル末尾改行。空の配列/オブジェクトは pretty でも `[]` / `{}` と 1 行 | `empty/containers` |

### 既知の非対称（孤立サロゲート）

JS の `JSON.stringify` は ES2019 の well-formed 化により孤立サロゲート（対にならない
`U+D800`〜`U+DFFF`）を `\udXXX` としてエスケープ出力できる。Rust の `String` は UTF-8 の
不変条件により孤立サロゲートを**保持できない**ため、canon-json にはこの入力クラスが
存在しない（`serde_json` も `"\ud800"` を読取時に拒否し、`ParseError::Syntax` になる）。

この非対称は契約 JSON には現れない（契約キーは ASCII、値も整形式の UTF-8）ため実害はないが、
「upstream にできて canon-json にできないことがある」点として記録しておく。受入表にこの
クラスのケースが**無い**のは採取漏れではなく、Rust 側に対応する入力が構築できないためである。
