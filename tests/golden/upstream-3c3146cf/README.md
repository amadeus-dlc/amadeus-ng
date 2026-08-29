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
| `harness.json` | `dist/claude/.claude/tools/data/harness.json` | 76 | md5 `4108544495aeb5260fad0fcba21b664d`<br>sha256 `85bfdec8f1449f17f164599dbccdb79ffda9af76cdc18588e60dde75e589ace9` |

`harness.json` は 2026-08-23 に上と同じ方法（`curl -fsSL .../3c3146cfd7cef33020d48e8d48d4e80d0f8c2820/dist/claude/.claude/tools/data/harness.json`、
HTTP 200）で追加取得した。内容は `{ "name": "claude", "harnessDir": ".claude", "rulesSubdir": "rules" }` で、
本リポジトリの `.claude/tools/data/harness.json` と実バイトが一致する（同 sha256）。定義の系譜 ID
`WorkflowDefinitionId` の供給元（ADR-008）であり、ゴールデンパリティテストが
`find_by_id(WorkflowDefinitionId::parse("claude"))` で実グラフを引く鍵になる。既存 2 行のバイトは不変。

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
| `cli/` | cli | CLI 主要遷移の stdout・状態差分・監査行 | `scripts/goldens/recapture-cli.sh` |
| `hooks/` | hook | フック 4 本の代表ケース（exit code・stderr・監査行） | `scripts/goldens/recapture-cli.sh` |
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
| 採取日時 | 2026-08-22T13:07:22Z |
| 採取コマンド | `bash scripts/goldens/recapture-hash-canonical.sh` |
| bun | 1.3.13 |
| ケース数 | 32（欠落 0） |

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

さらに、実装時の棚卸し（I2 / I4）で `.claude/tools/data/*.json` に **integer-like キー**
（`ars-priors.json` の `evThresholds` の `"1"`〜`"5"`）と **浮動小数フィールド**（22 種）が
実在することが判明したため、その実データを入力クラス `contract-observed` として受入表に
追加してある。「契約 JSON に integer-like キー・浮動小数は現れない」という当初の想定は
実測により否定されており、BR1.2 の先頭寄せと BR1.3 の数値ライタはどちらも机上ではなく
実データに効く規則である。

### 既知の非対称（孤立サロゲート）

JS の `JSON.stringify` は ES2019 の well-formed 化により孤立サロゲート（対にならない
`U+D800`〜`U+DFFF`）を `\udXXX` としてエスケープ出力できる。Rust の `String` は UTF-8 の
不変条件により孤立サロゲートを**保持できない**ため、canon-json にはこの入力クラスが
存在しない（`serde_json` も `"\ud800"` を読取時に拒否し、`ParseError::Syntax` になる）。

この非対称は契約 JSON には現れない（契約キーは ASCII、値も整形式の UTF-8）ため実害はないが、
「upstream にできて canon-json にできないことがある」点として記録しておく。受入表にこの
クラスのケースが**無い**のは採取漏れではなく、Rust 側に対応する入力が構築できないためである。

### cli / hooks 族の採取手順と来歴（BR2.1 / BR2.4）

`cli/` と `hooks/` は次の手順で採る。来歴は各族の `provenance.json` に機械可読な形で
入っており、下表はその要約である。

1. upstream ピン `3c3146cfd7cef33020d48e8d48d4e80d0f8c2820` の `dist/claude/` を
   **SHA 指定の shallow fetch**（`git init` → `git fetch --depth 1 origin <sha>` →
   `git checkout FETCH_HEAD`）で使い捨てディレクトリに取得する。失敗したら
   `codeload.github.com` の tarball 取得へフォールバックする。
2. 取得ツリーの sha256 マニフェスト（`<sha256>  <dist/claude からの相対パス>` を
   `LC_ALL=C` でパス順ソートしたテキストの sha256）とファイル数を期待値と照合する
   （ずれたら upstream が動いたということなので停止する）。
3. `dist/claude/` の `.claude/` と `aidlc/` を `mktemp -d` の使い捨てワークスペースへ
   置き、**そのピンのツールを bun で実行して**遷移を順に進める。インストール済みの
   別バージョンのシェルは使わない。
4. 各ステップで stdout・stderr・終了コード・`aidlc-state.md` の差分・監査シャードの
   追記分を採り、`normalization.json` の規則で正規化して書く。
5. 書き終えたコーパスをホスト名・ユーザ名・ホームディレクトリ・一時ディレクトリの
   残留について機械検査する（NFR4.4）。1 件でも見つかれば採取は失敗として止まる。

| 項目 | 値 |
| --- | --- |
| 取得元 | `https://github.com/awslabs/aidlc-workflows` の `dist/claude`（262 ファイル） |
| 取得方法 | SHA 指定の shallow fetch（フォールバック: codeload tarball） |
| ツリーマニフェスト sha256 | `ea223c423bebf32cd240d45b645fcd9649efc0d19592de75fd48565a6ded0b9f` |
| 採取コマンド | `bash scripts/goldens/recapture-cli.sh` |
| bun | 1.3.13 |
| ケース数 | cli 25（欠落 2）/ hooks 14（欠落 1） |

#### 非対話化に使った環境変数

upstream のツールは人間の在席・合議の寄稿・質問フローの受領証を要求するゲートを持つ。
非対話の採取ではそのどれも本物を用意できないため、次の環境変数で無効化して採った。
**採取の主題は遷移そのものの出力**であり、ゲートの内容ではない。値は各族の
`provenance.json` の `non_interactive_env` にも機械可読な形で入っている。

| 環境変数 | 何を止めるか |
| --- | --- |
| `AIDLC_SKIP_HUMAN_PRESENCE_GUARD` | 直近のゲート以降に人間のターンがあったかの確認 |
| `AIDLC_DISABLE_ENSEMBLE_EVIDENCE` | 合議ステージの寄稿ファイル検査 |
| `AIDLC_SKIP_SUMMARY_CONFIRMATION_GUARD` | 質問フローと人間承認の受領証の検査 |
| `AIDLC_SKIP_ARTIFACT_GUARD` | 成果物の存在・鮮度の検査 |
| `AIDLC_DISABLE_USAGE_TRACKING` | 採取ごとに変わる利用量の記録 |

#### ケースのレイアウト（C7）

```
cli/<verb>/<case>/     argv  stdin  exit  stdout.json|stdout.txt  stderr  state.diff  audit.md  case.json
                       state-full.md                                    骨格が生まれる遷移だけ
hooks/<hook>/<case>/   stdin.json  exit  stdout  stderr  audit.md  case.json
cli/provenance.json    hooks/provenance.json        族単位の来歴（BR2.1）
cli/cases-missing.json hooks/cases-missing.json     採取できなかったケース（W4）
```

C7 の必須ファイルに加えて `exit` / `stderr` / `case.json`（および hooks 側の `stdout`）を
足してある。終了コードは C1 が 0 / 1 / 2 を契約として定めており、フックの拒否は
stderr の逐語文言が契約そのものだからである。**削除はしていない**（BR2.4「不足は追加、
削除はしない」）。

- `argv` は JSON 配列。非決定値はプレースホルダに置いてある（`--project-dir <ROOT>`、
  継続トークンは `<SESSION>`）。採取時に実際に渡した値ではなく、**記録として読める形**である。
- `stdout.json` は stdout が JSON として読めたケース、`stdout.txt` はそれ以外。
  どちらも upstream が出したバイト列を正規化しただけで、再整形はしていない。
- `state.diff` は `aidlc-state.md` の遷移前後の unified diff（文脈 3 行、行単位 LCS）。
  ハンクヘッダは `@@ -<開始>,<行数> +<開始>,<行数> @@` の 1 始まり。差分が無いケースは空。
- `audit.md` は監査シャードへの**追記分だけ**（既存行は含まない）。追記が無いケースは空。
- `state-full.md` は遷移**後**の `aidlc-state.md` の全文（正規化済み）。差分は「前」があって
  はじめて読める観測なので、状態ファイルを**ゼロから起こす**側（genesis）の検収には全文が
  要る。全ケースに付けると同じ本文が 25 通並ぶだけなので、骨格が生まれる遷移
  （`intent-create/classic-scope`）にだけ付けてある。upstream 側の正本は
  `aidlc-utility.ts` の状態ファイル template literal である
  （`knowledge/aidlc-shared/state-template.md` は LLM 向けの契約文書で、ツールは読まない
  — 両者は食い違っており、下の `set-autonomy` の項がその実害である）。

#### CLI ゴールデンの範囲（BR2.4）

`cli/` は 1 本の作業を頭から進めながら採る。順序に意味があり（ゲートは開いてからでないと
差し戻せない、recompose は保留ステージにしか効かない、park の次は unpark）、
`case.json` の説明がそれぞれの遷移の意味を書いている。

| verb | ケース | upstream のツール |
| --- | --- | --- |
| `next` | `no-active-intent` / `start` / `after-approval` / `stage-jump-print` | `aidlc-orchestrate.ts next` |
| `intent-create` | `classic-scope` | `aidlc-utility.ts intent-create` |
| `continue` | `load-steering` / `invalid-token` | `aidlc-orchestrate.ts continue` |
| `report` | `awaiting-approval` / `awaiting-approval-repeat` / `rejected` / `revised` / `approved` | `aidlc-orchestrate.ts report --result` |
| `practices-promote` | `affirm` | `aidlc-state.ts practices-promote` |
| `skip` | `skipped` | `aidlc-orchestrate.ts report --result skipped` |
| `jump` | `resolve-forward` / `execute-forward` / `execute-forward-to-conditional` | `aidlc-jump.ts resolve` / `execute` |
| `recompose` | `skip-one` / `rejected-starved-input` | `aidlc-utility.ts recompose` |
| `park` | `park` | `aidlc-orchestrate.ts park` |
| `unpark` | `unpark` | `aidlc-state.ts unpark` |
| `set-autonomy` | `state-field-absent` | `aidlc-bolt.ts set-autonomy` |

`jump/execute-backward`・`jump/execute-forward-across-phases`・`report/completed-ungated`
は 2026-08-29 に追加採取した
（上の表の `jump` / `report` 行に含まれる）。既存 22 ケースのバイトは 1 バイトも動いていない
— 新ケースは列の**末尾に足した**ので、先行ケースの観測は採り直しても同一である。

#### フックの写像（C2）

C2 が名指すフック 4 本と upstream の実装ファイルの対応。`settings.json` の登録内容から
確定した（`hooks/provenance.json` の `hook_files` が正本）。

| C2 の名前 | upstream の実装ファイル | 登録イベント |
| --- | --- | --- |
| `stop-forwarding-loop` | `.claude/hooks/aidlc-continue-workflow.ts` | `Stop` |
| `record-human-turn` | `.claude/hooks/aidlc-record-human-turn.ts` | `UserPromptSubmit`、`PostToolUse(AskUserQuestion)` |
| `state-transition-guard` | `.claude/hooks/aidlc-state-transition-guard.ts` | `PreToolUse` |
| `write-audit-log` | `.claude/hooks/aidlc-write-audit-log.ts` | `PostToolUse(Write\|Edit)` |

各フックについて許可（終了コード 0）・拒否（終了コード 2 + stderr に理由）・無視
（終了コード 0 で副作用なし）を 2〜3 件ずつ採ってある。`stop-forwarding-loop` の拒否は
終了コード 2 ではなく **stdout の `{"decision":"block", "reason": …}` で表現される**
（Claude Code の Stop フック契約）点が他の 3 本と違う。

#### 採取で分かった upstream の実測挙動

| 観測 | 根拠ケース |
| --- | --- |
| `intent-create` が書く状態ファイルに `- **Construction Autonomy Mode**:` 行が無いため、`set-autonomy` は終了コード 1 で拒否される。`knowledge/aidlc-shared/state-template.md` は当該行を規定しており、テンプレートと実装が食い違っている | `cli/set-autonomy/state-field-absent` |
| `report --result skipped` は `--stage` の明示が必須で、かつ `execution: CONDITIONAL` の**現ステージ**しか受け付けない | `cli/skip/skipped` |
| `next --stage <slug>` は自分でジャンプせず、実行すべき `aidlc-jump.ts execute` を print directive で名指す | `cli/next/stage-jump-print` |
| `recompose` は下流の必須入力を枯らす flip を strict validator が拒否する | `cli/recompose/rejected-starved-input` |
| `write-audit-log` フックはツール名で絞らない。`Write` / `Edit` 以外の `tool_name` でも記録ディレクトリ配下なら監査行を残す（絞り込みは `settings.json` の matcher の責務） | `hooks/write-audit-log/trusts-the-settings-matcher` |
| `write-audit-log` は `Edit` を常に UPDATE、`Write` は mtime と birthtime の差が 10 ms 未満なら CREATE として扱う | `hooks/write-audit-log/artifact-created`, `artifact-updated-by-edit`, `artifact-updated-by-overwrite` |
| 非ゲートの initialization ステージを `report --result completed` で報告すると、ゲートを開かずに `advance` だけが走る。`STAGE_COMPLETED` の `**Details**:` は `Stage <表示名> completed`（ゲート経由の `Stage <表示名> approved by gate` とは別文言） | `cli/report/completed-ungated` |
| 後方ジャンプは対象と下流の `[x]/[-]/[?]/[R]/[S]` を `[ ]` へ戻し、対象を `[-]` にする。フェーズ境界をまたぐと `PHASE_COMPLETED`（`**Details**: Phase boundary crossed via <方向> jump`）・`PHASE_VERIFIED`（`**Details**: Traceability verification on jump`）・`PHASE_STARTED` の 3 本が `STAGE_JUMPED` の**前**に並ぶ。この 3 本はゲート経由の境界 3 本と**同型ではない** — ジャンプ側だけが `**Details**:` を持ち、`**Stages completed**:` は計画上のフェーズ内件数ではなく**チェックボックスの数え直し**である（後方 0 / 前方 1） | `cli/jump/execute-backward`, `cli/jump/execute-forward-across-phases` |
| 前方ジャンプの `STAGE_SKIPPED` は「間のステージを文書順」→「**最後に出発点そのもの**」の順で並ぶ（出発点は間のループの外で後から足されるため） | `cli/jump/execute-forward-across-phases` |
| ジャンプ後の `- **Last Completed Stage**:` は到達点より手前を逆順に辿った最初の `[x]`。1 つも無ければ upstream の既定値 `state-init` になる | `cli/jump/execute-backward` |
| 状態ファイルの `- **Next Action**:` は書き手で綴りが割れる。genesis（`intent-create`）は **slug**（`Execute practices-discovery`）、`advance` は**表示名**（`Execute Workspace Detection`） | `cli/intent-create/classic-scope`, `cli/report/completed-ungated` |

#### 採取できなかったケース（W4）

非対話で再現できなかった遷移は**値を捏造せず** `cases-missing.json` に理由・根拠・
引き取り先つきで記録してある（cli 2 件、hooks 1 件）。

| ケース | 理由 | 引き取り先 |
| --- | --- | --- |
| `cli/set-autonomy/gated` | ピン `3c3146cf` の配布シェルには `- **Construction Autonomy Mode**:` 行を**書き込む経路が 1 つも無く**、正常系そのものが到達不能（下記） | 逸脱台帳 |
| `cli/continue/multi-part` | 規則束が 28 KiB 上限を超えないため `parts > 1` の分割配送を再現できない | U6 |
| `hooks/stop-forwarding-loop/transcript-carve-out` | 会話ターンの切り出し判定に本物のトランスクリプト JSONL が要る | U7 |

##### `set-autonomy` の正常系が到達不能である根拠（2026-08-29 全数走査）

`dist/claude/` の 262 ファイルを走査し、`- **Construction Autonomy Mode**:` **行を状態
ファイルへ書き込む経路が 1 つも無い**ことを確かめた。行を手で足せば `set-autonomy` は通るが、
それは upstream の挙動ではなく採取者の捏造なので採らない。

1. 状態ファイルを起こす唯一のテンプレート（`aidlc-utility.ts` の template literal）に当該行が
   無い。`state-full.md` の全文がその実測である。
2. `setField` は行が無ければ**黙って no-op**、`setFieldStrict` は **throw** する。どちらも
   挿入はしない（`set-autonomy` が踏むのは後者で、これが終了コード 1 の出どころ）。
3. 行を挿入できる唯一の関数 `setOrInsertField` の呼出先は `Merge-Held` / `Skeleton Stance` /
   `Construction Iteration` / `Practices Affirmed Timestamp` / `Parked` / `Parked At Stage` /
   `Active Unit` / `Unit State` / `Unit Pause Reason` / `Unit Next Action` の 10 種のみで、
   autonomy は含まれない。
4. 汎用の `aidlc-state.ts set <field>=<value>` も `setField` 経由なので挿入しない。

当該行を規定しているのは LLM 向けの契約文書 `knowledge/aidlc-shared/state-template.md`
だけで、ツールはこれを読まない。**テンプレートと契約文書が食い違っている upstream 側の
欠落**であり、逸脱台帳の対象である。帰結として `AUTONOMY_MODE_SET` の監査行のフィールドキー
（`**Mode**:`）はピンの**ソース**（`aidlc-bolt.ts` の `emitAudit` 呼出）からしか読めず、
実行出力としては採れていない。ピン更新で当該行がテンプレートへ入ったら採り直す。

### 正規化規則の追加（cli / hooks 族、BR2.2）

`normalization.json` にはプレースホルダ 4 種（NFR1.3）を保ったまま規則を 4 本足した。
**規則は配列順に適用する** — 実行時 literal 置換（`runtime-path` / `runtime-clone`）を
先に当てて環境固有値を丸ごと潰し、次に形の決まったパターンを当て、取りこぼしを拾う
広めのパターン（`<CLONE>` のシャード名形）を最後に置く。

| 足した規則 | プレースホルダ | 何のため |
| --- | --- | --- |
| `runtime-clone` | `<CLONE>` | 監査シャード名とホスト名を実行時の実値で literal 置換する（形から拾う規則では取り切れない） |
| `intents/\d{6}-` | `<TS>` | 記録ディレクトリ名の先頭に付く YYMMDD（UTC）。採取日で変わる |
| `\d{6}-golden` | `<TS>` | パス以外の逐語文言に裸で現れる同じ日付スタンプ。`golden` は採取フィクスチャが `intent-create --label` に渡すラベルで、`provenance.json` の `fixture_intent_label` が正本 |
| `[A-Za-z0-9_-]{200,}` | `<SESSION>` | load-steering の継続トークン。記録ごとのランダム鍵で MAC を張るため同じ入力でも毎回変わる |

置換文字列は**リテラル扱い**である（`$1` のような後方参照は展開しない）。採取側は
`$` を退避し、比較器側は `regex::NoExpand` を使って解釈を合わせてある。

#### 再現性の実測（BR2.5）

再採取スクリプトを 2 回続けて実行し、`captured_at` を除く全ファイルがバイト一致すること
を確認済み（2026-08-22）。ピンが変わらない限り再実行は `captured_at` 以外に差分を出さない。

### 比較器（W5）

コーパスを読んで正規化し行ごとの差分を出す比較器は
`modules/shared/canon-json/tests/support/mod.rs` にある（**テスト支援であり、canon-json の
ライブラリ本体には入らない** — `nfr-design/logical-components.md` §4）。読取・正規化・
差分の 3 機能だけを持ち、`normalization.json` を規則の唯一の正本として読む。

本 Unit（U1）が固定するのは cli / hooks 族について「読めて、範囲を満たしていて、正規化が
固定点になっている」ところまでである（`modules/shared/canon-json/tests/golden_corpus_read.rs`）。
**正規化が固定点**とは、採取時に正規化済みのコーパスを比較器に通しても 1 バイトも動かない
という性質で、採取側（TypeScript）と比較側（Rust）で規則の解釈がずれていないことの機械的な
証拠になる。実装出力との突合せは U6（next / continue）と U7（CLI・フック）が同じ比較器で行う。
