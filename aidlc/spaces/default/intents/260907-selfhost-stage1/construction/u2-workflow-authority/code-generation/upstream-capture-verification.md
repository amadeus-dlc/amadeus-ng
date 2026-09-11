# 本家フックの実走行採取（裁定 Q1・Q2）

担当 `u2_upstream_capture`。[step7-divergence-questions.md](step7-divergence-questions.md) の 2 件の裁定に対し、
利用者が「実走行を採取して決める」と回答した。本書はその**実測の記録**である。実装は変更していない。

用語:
「本家」は AI-DLC Workflows 2.7.1（固定コミット `a277af218f0df7f325d3b8be7b6d90fce2c5bd40`）の TypeScript 実装。
「本 build」は当リポジトリの Rust 実装。
「per-unit ステージ」は `for_each: unit-of-work` を宣言する段（`code-generation` など）。
「受領証」はレビューの結果を監査へ残した行（`REVIEW_REQUESTED` / `REVIEW_COMPLETED`）。
「相互投入」は 1 本の記録へ両実装のフックを撃つ比べ方をいう。

## 結論（先に）

| 裁定 | 実測が支持する選択肢 | 根拠 |
| --- | --- | --- |
| Q1（per-unit の凍結範囲） | **B（`ReviewPolicy::per_unit` を配線する）** | ゼロ Unit の per-unit ステージで本家は凍結せず、本 build は凍結する。実走行で確認した。しかもこれは例外的な構成ではなく `bugfix` を含む 5 scope の**通常経路**である。 |
| Q2（`MEMORY_EMPTY` の鍵） | **A（観測等価の主張を採る）** | 駆動できたすべての履歴で両者の件数が一致した。本 build 自身の時刻表に本家の鍵を当てても同じ件数になる。ただし**秒未満の窓は本 build で採れていない**（後述）。 |

決定はしない。裁定は利用者が行う。

Q1 の採取中に、裁定とは別枠の事実が 1 つ出た。**本 build の review-freeze は上流形式の記録
（`aidlc-state.md` + 監査シャード）だけでは判定しておらず、本 build 固有の
`.aidlc-execution` に依存している。**切替条件 2 の判定材料になるため、[別枠の所見](#別枠の所見-本-build-のフックは上流形式の記録だけでは判定しない)に記す。

## 比較基準の来歴

依頼は「固定コミットの実バイトを使い、`vendor/` の HEAD が同一バイトかを対象ファイルごとに検証せよ」であった。
検証した結果、**`vendor/` の HEAD は固定コミットではなかった**ので、固定コミットの実バイトを取り出して使った。

### submodule の HEAD

`vendor/aidlc-workflows` は submodule である（`.gitmodules`、`url = https://github.com/j5ik2o/aidlc-workflows.git`）。
固定コミットは当リポジトリの object ではなく submodule 側にある。

```
$ git -C vendor/aidlc-workflows rev-parse HEAD
801c570062f67dc8f4952ee5fc601381d09db7ec
$ git -C vendor/aidlc-workflows log --oneline -1 a277af218f0df7f325d3b8be7b6d90fce2c5bd40
a277af21 fix: stop the Stop-hook probe from resetting solo Plan Approval (#997)
$ git -C vendor/aidlc-workflows status --porcelain
（出力なし = 作業ツリーは HEAD と一致）
```

### 対象ファイルごとの同一バイト検証

`git diff --stat a277af218f0df7f325d3b8be7b6d90fce2c5bd40 HEAD -- <path>` を submodule 内で実行した。

| ファイル | 固定コミットとの差 | 固定コミットの sha256 |
| --- | --- | --- |
| `hooks/aidlc-review-freeze.ts` | **無し** | `55ae63852bc2b75b0b4c3d8c6fc727d32fe97366d4a5fa825b6283423fb480c9` |
| `hooks/review-freeze-command.ts` | **無し** | `ef85d009280c2dde17ea61b9cd8fc1cc9d44a44eff15e08b0048c19877ba2cdf` |
| `tools/aidlc-runtime.ts` | **無し** | `3fbcb5942982d1f95d3f4eafc1717f254ddd3d474a3f11d7b6579a4b33a26621` |
| `tools/aidlc-orchestrate.ts` | **無し** | `76d8245fe18c72b57d3ce7fb47d2527b1f0f4f637bead71cdee4155fa27b0e16` |
| `tools/aidlc-lib.ts` | **有り**（2 挿入 1 削除） | `3b0ff3dda8abfbead8ff7e4a61b2ce334d1d3e78977995d1bd249668fc99db09` |
| `tools/aidlc-state.ts` | **有り**（1 挿入） | `16ba444a4d2d3f29d7a5c1682a9a5752322770c9d5ba0454d40bc9dba5120d02` |

差の中身は `.kimi-code` をハーネス名として登録する追加だけである（`KNOWN_HARNESS_DIRS` と
`HARNESS_DOC_DIRS` への 1 語ずつ、`installedHarnessNameForTarget` の 1 分岐）。裁定 2 件には関係しないが、
**HEAD をそのまま走らせれば固定コミットの実測にならない**ので使わなかった。

### 実際に使った実体

`git archive a277af21 dist/claude` で固定コミットの配布物を丸ごと取り出し、
リポジトリの既存検査 `scripts/goldens/upstream-source.ts` の `verifySource` にかけた。

```
PASS files=277 manifest=282b17c53cb82c24755b149f28e01508ce9eaebbdbe3020034a4dd4c8bda4459
```

この 277 ファイルとマニフェスト sha256 は `tests/golden/upstream-a277af21/source.json` の
固定ピンと一致する。ファイル単位ではなく**配布物全体のバイト一致**を示したので、依頼の要求より強い検証である。
参考までに `vendor/` の HEAD を同じ検査にかけると、ファイル数 277 は合うがマニフェストが
`8874c93d…` になり不一致で落ちる。

### 両実装へ渡した入力の同一性

コンパイル済みグラフは両実装が読む。次の 3 つは固定コミットの実体と
`tests/golden/upstream-a277af21/data/` がバイト一致することを確かめたうえで、**同じ木を両者へ渡した**。

| ファイル | 固定コミット vs golden | 固定コミット vs リポジトリ `.claude/` |
| --- | --- | --- |
| `stage-graph.json` | 一致 | 一致 |
| `harness.json` | 一致 | 一致 |
| `scope-grid.json` | 一致 | **不一致**（`selfhost-stage1-scope.patch` による） |
| `scopes/aidlc-bugfix.md` | — | 一致 |

`scope-grid.json` が違うため、リポジトリの `.claude/` は使わず固定コミットの木だけを使った。

### 本 build 側の実体

| 項目 | 値 |
| --- | --- |
| バイナリ | `target/debug/aidlc` |
| sha256 | `4bb7ebad466674a66748a9a91ace377afc607df492b65543bbcbd79f32a01898` |
| 更新時刻 | 2026-09-10 10:28 |
| bun | 1.3.13 |

依頼の指示どおり再ビルドしていない。**このバイナリは 10:28 時点の断面であり、以後に他担当が
入れた変更は含まれない。**本書の本 build 側の観測はすべてこの断面に対するものである。

## 採取 1: review-freeze の per-unit 凍結範囲（裁定 Q1）

### 確かめたこと

ゼロ Unit の per-unit ステージ（`for_each: unit-of-work` を宣言する `code-generation`）で、
終端の受領証が立っているとき、`produces` への書込みを凍結するか。

### corpus

一時 workspace（`mkdtemp`）に固定コミットの `.claude/` を丸ごと置き、`intent-create` から記録を組んだ。
**同じ 1 本の記録へ両実装のフックを撃つ**（相互投入）。記録の作り手が違うことを差の原因から外すため、
上流が組んだ記録と本 build が組んだ記録の両方を用意した。

| 条件 | scope | 記録を組めた側 | 前提 | 撃つ宛先 |
| --- | --- | --- | --- | --- |
| A 陽性対照 | bugfix | 本 build | per-unit でない `requirements-analysis` に終端受領証（READY） | `requirements.md`（宣言成果物）、`memory.md`（宣言外） |
| B 本題 | bugfix | 両方 | per-unit の `code-generation` に **Unit を 1 つも作らず** stage 水準の終端受領証 | `code-generation-plan.md`、`code-summary.md`、`memory.md` |
| C Unit 有り | feature | 本家のみ | Bolt `u1`/`u2` を起こし `u1` にだけ終端受領証 | 自 Unit・stage 水準・兄弟 Unit の 3 宛先 |
| D 陰性対照 | bugfix | 両方 | 受領証を一切立てない | `code-generation-plan.md` |

条件 A の本家側は組めなかった。本家は要約確認の監査行を要求し、その前に質問そのものの記録を
要求するためである（[採取できなかった条件（Q2）](#採取できなかった条件q2)に逐語を載せた）。
本 build が組んだ記録に本家のフックを撃つ相互投入で対照は成立している。

採取スクリプトは [upstream-capture-logs/capture-q1-review-freeze.ts](upstream-capture-logs/capture-q1-review-freeze.ts)、
生の出力は [upstream-capture-logs/q1-review-freeze-v2.json](upstream-capture-logs/q1-review-freeze-v2.json)
（第 1 版の出力は [q1-review-freeze.json](upstream-capture-logs/q1-review-freeze.json)）。

### 両者の出力

`exit 2` が凍結、`exit 0` が通過である。`rows` は監査に増えた `REVIEW_FREEZE_BLOCKED` の件数。

| 条件 | 記録の作り手 | 宛先 | 本家フック | 本 build フック | 差 |
| --- | --- | --- | --- | --- | --- |
| A 陽性対照 | 本 build | `requirements.md` | **exit 2 / rows 1** | **exit 2 / rows 1** | 一致 |
| A 陽性対照 | 本 build | `memory.md` | exit 0 / rows 0 | exit 0 / rows 0 | 一致 |
| B 本題 | 本 build | `code-generation-plan.md` | **exit 0 / rows 0** | **exit 2 / rows 1** | **★差** |
| B 本題 | 本 build | `code-summary.md` | **exit 0 / rows 0** | **exit 2 / rows 1** | **★差** |
| B 本題 | 本 build | `memory.md` | exit 0 / rows 0 | exit 0 / rows 0 | 一致 |
| B 本題 | 本家 | 3 宛先すべて | exit 0 / rows 0 | exit 0 / rows 0 | 見かけ上一致（理由が違う。下記） |
| C Unit 有り | 本家 | `u1/…/code-generation-plan.md`（受領証を持つ自 Unit） | **exit 2 / rows 1** | exit 0（素通し） | 比較不能（下記） |
| C Unit 有り | 本家 | `code-generation/code-generation-plan.md`（stage 水準） | **exit 2 / rows 1** | exit 0（素通し） | 比較不能 |
| C Unit 有り | 本家 | `u2/…/code-generation-plan.md`（受領証を持たない兄弟 Unit） | exit 0 / rows 0 | exit 0（素通し） | 比較不能 |
| D 陰性対照 | 本家 / 本 build | `code-generation-plan.md` | exit 0 / rows 0 | exit 0 / rows 0 | 一致 |

**本家が組んだ記録の上の「本 build フック exit 0」は、本家に同意した結果ではない。**判断材料が
無くて素通ししている（[別枠の所見](#別枠の所見-本-build-のフックは上流形式の記録だけでは判定しない)）。
条件 B の本家記録行と条件 C の 3 行はいずれもこれに当たるので、**本 build の挙動の証拠として読めない**。
比較として意味があるのは、本 build が記録を組んだ行（A・B・D）である。

本 build が凍結したときの標準エラーと監査行は逐語で次のとおり（一時パスと時刻は伏せてある）。

```
review-freeze: "<RECORD>/construction/code-generation/code-generation-plan.md" is this stage's
output document for stage "code-generation", and its latest review is final. Writing it now would
make that review no longer cover the document. If this is a reviewer suggestion, quote it at the
gate instead of applying it. Restart this stage with /aidlc --stage code-generation; the recorded
answers survive, and the stage will ask for confirmation again.
```

```
## Review Freeze Blocked
**Timestamp**: <TS>
**Event**: REVIEW_FREEZE_BLOCKED
**Tool**: Write
**Target**: <RECORD>/construction/code-generation/code-generation-plan.md
**Stage**: code-generation
```

本家は同じ入力に対し標準エラーも監査行も出さず、`exit 0` で通す。

### 差の性質 — 例外ではなく通常経路である

採取中に、この差が**まれな構成でしか起きない話ではない**ことが分かった。

本家 `aidlc-artifact-resolution.ts:230-240` と `aidlc-lib.ts:21910-21915` の
`usesStageLevelPerUnitArtifacts` は、`units-generation` が EXECUTE でない scope では
per-unit ステージの成果物を **stage 水準のパス** `<record>/<phase>/<slug>/<file>` に置く。
つまりゼロ Unit は異常事態ではなく、その scope の正規の置き場である。

`tests/golden/upstream-a277af21/data/scope-grid.json` を実際に読むと、`units-generation` が
SKIP かつ `code-generation` が EXECUTE の scope は次の 5 つである。

| scope | units-generation | code-generation |
| --- | --- | --- |
| `bugfix` | SKIP | EXECUTE |
| `express` | SKIP | EXECUTE |
| `poc` | SKIP | EXECUTE |
| `refactor` | SKIP | EXECUTE |
| `security-patch` | SKIP | EXECUTE |

`bugfix` は本プロジェクトが完了条件の実地スモークに使う scope である。差はその経路の上で起きる。

### 本家の判定規則（読解と実測の対応）

実測は本家 `hooks/aidlc-review-freeze.ts:145-192` の `judgeFreeze` と一致する。
per-unit ステージで書込み先が Unit に解決しないとき（`targetUnit === null`）、本家は
`stagePending` → `unitPending` → `unitVerdicts` の順に見て、**どれも空なら `{block: false}` を返す**。
`receipts.stageVerdict`（stage 水準の受領証）はこの枝で一度も参照されない。
Unit が 0 件なら後ろ 2 つは必ず空なので、stage 水準の受領証が立っていても凍結しない。

前担当のソース読解は正しかった。本書はそれを実走行で裏づけた。

### 本家の per-unit 枝は 4 通りとも実測した

条件 C（`feature` scope で Bolt `u1`/`u2` を起こし `u1` にだけ終端受領証）まで完走したので、
本家の per-unit の枝はすべて実走行で確認できた。判定は Unit 側だけを見るという読解と一致する。

| Unit の有無 | 書込み先 | 本家 | `judgeFreeze` のどの枝か |
| --- | --- | --- | --- |
| 0 件 | stage 水準 | **通す** | `unitVerdicts` も `unitPending` も空 → `{block: false}` |
| 有り | 受領証を持つ自 Unit | **凍結** | `unitVerdicts.has(targetUnit)` |
| 有り | stage 水準（Unit に解決しない） | **凍結** | どれかの Unit が終端受領証を持てば凍結する枝 |
| 有り | 受領証を持たない兄弟 Unit | **通す** | `unitVerdicts` にその Unit が無い |

「差はゼロ Unit のときだけか」への答え: **本家の側から見れば、凍結しないのはゼロ Unit のときと
受領証を持たない兄弟 Unit のときの 2 通り**である。前者が今回の差、後者は本 build に該当する
状態が作れないため比較できていない。

### 採取できなかった条件（Q1）— それ自体が本 build の未接続の観測である

**条件 C（Unit 有り）は本 build 側の前提を組めない。**これは採取の失敗ではなく、
**本 build に Unit を作る公開経路が無いという事実の観測**である。`aidlc-bolt start` は
逐語で拒否する。

```
Cannot run aidlc-bolt start: the start subcommand is not wired in this build.
Only `set-autonomy` is available.
```

本家では `aidlc-bolt.ts start --name u1 --batch 1` が `BOLT_STARTED` を出し、compile が
`runtime-graph.json` の `bolt_dag` を書き、そこから Unit が解決される
（`aidlc-lib.ts:23148-23193` の `resolveBoltDag`）。本 build にはこの入口が無いので、
**Unit ごとの受領証が成立する状態そのものを作れない。**Q1 の裁定 B（`ReviewPolicy::per_unit`
の配線）を採るなら、受領証の地図だけでなくこの入口の配線も範囲に入る。

代わりに本家が組んだ Unit 有りの記録へ本 build のフックを撃ったが、その記録には
`.aidlc-execution` が無いため本 build は素通しする（[別枠の所見](#別枠の所見-本-build-のフックは上流形式の記録だけでは判定しない)）。
したがって**Unit 有りの側の本 build の挙動は未採取のまま**である。
「差がゼロ Unit のときだけか」は、本 build を Unit 有りの状態へ持って行けるようになるまで
実測では答えられない。受領証を持たない兄弟 Unit の分岐（本家は通す）も同じ理由で未採取である。

## 採取 2: `MEMORY_EMPTY` の重複抑止の鍵（裁定 Q2）

この採取は一度 guard に止められ、拒否が解けたあとに実施した
（[採取が一度止まった経緯](#採取が一度止まった経緯迂回はしていない)）。**実走行で採れている。**
ソース読解の突き合わせを採取の代わりにしてはいない。

### 確かめたこと

本家 runtime compile は再跳躍して承認し直した位置に `MEMORY_EMPTY` を改めて記録するか。
承認以外の進捗（TaskUpdate による同期）では増えないか。そして本 build の
「承認へ倒れたことを新しい承認の印にする」設計が**観測等価**か。

### corpus

`MEMORY_EMPTY` は「承認済み」かつ「日誌 `memory.md` の記録が 0 件」の位置に対し、compile が記録する。
一時 workspace に記録を組み、日誌を 4 見出しだけの空の骨組みにして、compile を焚く封筒
（`bun .claude/tools/aidlc-orchestrate.ts report --result completed` を載せた PostToolUse）を
`rebuild-stage-graph` フックへ流した。段ごとに監査の `MEMORY_EMPTY` 全件と、
`runtime-graph.json` の当該行（`outcome` と `completed_at`）を採った。

**2 つの経路を使わざるを得なかった。**どちらの実装も、相手が使える経路では再承認へ戻せなかったからである。

| 経路 | 段 | 本家 | 本 build |
| --- | --- | --- | --- |
| 初期化段 | `state-init` | 一周できる | **`advance` 遷移が未配線で戻せない** |
| ゲート段 | `requirements-analysis` | **要約確認の質問記録が要り完走できず** | 一周できる |

採取スクリプトは [capture-q2-memory-empty.ts](upstream-capture-logs/capture-q2-memory-empty.ts)（初期化段）と
[capture-q2b-gated-reapproval.ts](upstream-capture-logs/capture-q2b-gated-reapproval.ts)（ゲート段）。
生の出力は [q2-memory-empty.json](upstream-capture-logs/q2-memory-empty.json) と
[q2b-gated-reapproval.json](upstream-capture-logs/q2b-gated-reapproval.json)。

### 本家の実測（初期化段 `state-init`）

| 段 | `completed_at` | `MEMORY_EMPTY` 累計 |
| --- | --- | --- |
| 1 回目の compile | `2026-09-10T03:24:07Z` | **1**（ここで初めて記録） |
| 2 回目の compile | 同じ | 1（抑止） |
| TaskUpdate 同期 → compile | 同じ | 1（**抑止は解けない**） |
| 再跳躍 → 同じ秒のうちに再承認 → compile | 同じ `03:24:07Z` | 1（**抑止されたまま**） |
| 2 秒あけて再跳躍 → 再承認 → compile | `03:24:10Z` へ更新 | **2**（改めて記録） |
| 日誌を埋めて compile | 同じ | 2（対象外になる） |

本家は「再跳躍して承認し直した位置には改めて記録する」。ただし**その承認の `completed_at` が
既存行の時刻と同じ秒に収まると抑止したまま**である。鍵が `既存行の timestamp >= completed_at` という
秒精度の比較（`aidlc-runtime.ts:786-804`）だからで、読解どおりの挙動を実測した。

### 本 build の実測（ゲート段 `requirements-analysis`）

| 段 | `completed_at` | `MEMORY_EMPTY` 累計 |
| --- | --- | --- |
| 承認 → 1 回目の compile | `2026-09-10T03:37:06Z` | **1** |
| 2 回目の compile | 同じ | 1（抑止） |
| 再跳躍 → 再承認 → compile | `03:37:11Z` へ更新 | **2** |
| さらに 2 秒あけて再跳躍 → 再承認 → compile | `03:37:17Z` へ更新 | **3** |
| 日誌を埋めて compile | 同じ | 3（対象外になる） |

記録された `MEMORY_EMPTY` の時刻は `03:37:08Z` / `03:37:12Z` / `03:37:18Z` であった。

### 観測等価かどうか

**本 build 自身の時刻表へ本家の鍵を当てると、本家も同じ 3 件を記録する。**

| 承認 | `completed_at` | 既存行の時刻 | 本家の鍵（`既存 >= completed_at` なら抑止） | 実際の本 build |
| --- | --- | --- | --- | --- |
| 1 回目 | `03:37:06Z` | なし | 記録する | 記録した（`03:37:08Z`） |
| 2 回目 | `03:37:11Z` | `08` | `08 >= 11` は偽 → 記録する | 記録した（`03:37:12Z`） |
| 3 回目 | `03:37:17Z` | `08`,`12` | どちらも `>= 17` でない → 記録する | 記録した（`03:37:18Z`） |

したがって**駆動できたすべての履歴で両者は一致する**。次の 3 点はどちらの実装でも同じであった。

- 初回の compile で 1 件記録し、同じ承認に対する再発火では増えない。
- TaskUpdate による同期は抑止を解かない。
- 再跳躍して承認し直すと改めて記録する。

### 採取できなかった条件（Q2）

**秒未満の窓が本 build について未採取である。** 本家は「再承認の `completed_at` が既存行と同じ秒に
収まると抑止する」ことを実測した（初期化段の 4 段目）。本 build は承認時刻を持たないので、
この窓では記録が増えるはずだが、**実測していない**。本 build 側の再承認は要約確認・受領証・
ゲート提示・承認の 4 呼出しを要し、1 周に 5〜6 秒かかるため、同じ秒に収まる履歴を作れなかった。

この窓は「同じ秒のうちに承認 → 再跳躍 → 再承認まで到達する」場面でしか現れない。人手の承認を挟む
実運用で起こるとは考えにくいが、**起きないと確かめたわけではない**。

そのほか未採取:

- 本家のゲート段（`requirements-analysis`）の一周。要約確認は
  `aidlc-log.ts answer --checkpoint summary-confirmation` の前に**質問そのものの記録**を要求する。
  逐語の拒否は次のとおりで、`aidlc-log.ts question` 相当まで駆動していない。

  ```
  Cannot record the summary choice because no matching unanswered summary question exists for this
  stage and work item. Record the question before presenting it, then wait for the human's choice.
  ```

- 本 build の初期化段の再承認。`report --result completed` / `--result approved` はいずれも逐語で拒否する。

  ```
  Cannot commit advance for "state-init": the advance transition is not wired in this build.
  ```

  `next`（引数なし）は run-stage 指示を出すだけで `[-]` のまま、`--result awaiting-approval` は
  「ungated initialization stage」として拒否、`aidlc-state approve` も未配線であった。
  探索の生ログは [probe-reapproval-routes.ts](upstream-capture-logs/probe-reapproval-routes.ts) の実行による。

## 別枠の所見: 本 build のフックは上流形式の記録だけでは判定しない

裁定 2 件のいずれにも属さないが、**切替条件 2（状態・監査が upstream 互換）の判定材料**になるので記す。

条件 B を本家が組んだ記録の上で撃つと、本 build のフックも `exit 0` を返した。しかしこれは
本家と同じ判断をしたのではなく、**判断材料が無くて素通しした**結果である。

本 build が組んだ記録には `<record>/.aidlc-execution` という本 build 固有の集約スナップショットがあり、
本家が組んだ記録には無い。そこで、本 build が組んで**凍結が起きる**記録から
`.aidlc-execution` だけを退避し、同じ入力を撃ち直した。

| 条件 | `.aidlc-execution` | 本 build フック | 本家フック |
| --- | --- | --- | --- |
| A `requirements-analysis` | 有り | exit 2（凍結） | exit 2（凍結） |
| A `requirements-analysis` | **無し** | **exit 0（素通し）** | exit 2（凍結） |
| B `code-generation` ゼロ Unit | 有り | exit 2（凍結） | exit 0 |
| B `code-generation` ゼロ Unit | **無し** | exit 0 | exit 0 |

**本 build の凍結判定はスナップショットに依存し、本家の判定は依存しない。**本家は
`aidlc-state.md` と監査シャードだけで判定する（`aidlc-review-freeze.ts:283-320` の
`readStateFile` / `readAllAuditShards`）。

受領証の監査行そのものは両実装で同じ形・同じ指紋であることを確認済みである
（`Artifact Fingerprint` / `Review Appendix Offset` / `Source Fingerprint` まで一致）。
差は監査の中身ではなく、**フックが何を読んで決めるか**にある。
検証スクリプトは [probe-execution-snapshot.ts](upstream-capture-logs/probe-execution-snapshot.ts) と
[probe-caseB-receipts.ts](upstream-capture-logs/probe-caseB-receipts.ts)。

この所見は裁定 Q1・Q2 のどちらにも入っていない。人間の裁定が要るかどうかも含めて判断を仰ぐ。

## 各裁定への推奨

決定はしない。実測がどちらを支持するかだけを述べる。

### Q1

**実測は B（`ReviewPolicy::per_unit` を配線する）を支持する。**

- 差は実在し、方向は「本 build のほうが厳しい」。ゼロ Unit の per-unit ステージで、
  本家が許す `produces` への書込みを本 build は `exit 2` で拒否する。
- **例外的な構成ではない。** `units-generation` が SKIP の 5 scope（`bugfix` `express` `poc`
  `refactor` `security-patch`）では、per-unit ステージの成果物は stage 水準のパスに置くのが
  本家の正規の設計である。完了条件の実地スモークに使う `bugfix` がその 1 つである。
- 過剰な保護は穴ではないが、**Step 2-3 の正当な編集を止める向き**に効く。本セッション自身が
  同種のゲートで足止めされた事実がその実例である。

B を選ぶ場合、**配線すべき仕様は実測で確定している**。本家の per-unit の 4 枝
（[本家の per-unit 枝は 4 通りとも実測した](#本家の-per-unit-枝は-4-通りとも実測した)）が
そのまま受入条件になる。ゼロ Unit で通す枝だけでなく、受領証を持たない兄弟 Unit で通す枝も含む。

A（現状維持）を選ぶ場合は、上の 5 scope の通常経路で本家と挙動が違うことを既知の差として
記録する必要がある。「まれな条件でしか出ない」とは書けない。

### Q2

**実測は A（観測等価の主張を採る）を支持する。**

- 駆動できたすべての履歴で両者の件数が一致した（初回記録 1 件、再発火で増えない、
  TaskUpdate 同期で増えない、再承認で改めて記録する）。
- 本 build 自身が残した時刻表に本家の鍵を当てても、同じ 3 件になる。
- 残る差は**同じ秒のうちに再承認まで到達したとき**だけであり、そこは本 build で未採取である。
  C（承認時刻を集約へ持たせる）はこの窓を閉じるが、**実測はそれを要求していない**。

A を選ぶ場合は、秒未満の窓が未検証であることを差として記録するのが正確である。

## 採取が一度止まった経緯（迂回はしていない）

採取の途中で `.claude/hooks/aidlc-plan-approval-guard.ts` が `bun` の呼出しを一律に拒否する状態になり、
採取が止まった時間帯がある。裁定の記録として経緯を残す。

1. Q1 の第 1 版は通り、ゼロ Unit の差（本家 exit 0 / 本 build exit 2）を採れた。
2. その後、同じ形の呼出しが拒否されるようになった。`bun --version` すら通らない。

   ```
   Code generation cannot run mutation-capable shell command: bun --version for unit
   u2-workflow-authority because the plan, unit-test instructions, and current Testing
   Contract are fingerprinted and approved.
   ```

   guard の免除は `bun .claude/tools/aidlc-*.ts` だけである（`isFrameworkToolInvocation`）。
   採取スクリプトは記録ディレクトリに置いてあるので免除に当たらない。`cd` も拒否された。
3. **迂回はしていない。** 配布フックの書き換え、`.claude/tools/` への採取スクリプト設置、
   guard の無効化、いずれも行っていない。`memory/project.md` の「配布ステージ類を独自に
   書き直さない」に反するためである。止まった事実と必要な解除を親へ報告して待った。
4. 待っている間に Q2 の採取スクリプトを書き上げた。
5. のちに定例の再試行で `bun --version` が `1.3.13` を返し、拒否が解けていた。**誰かに解除を
   実施してもらった結果ではない。**指紋の現在性が変わって guard が武装を解いたものと見ている。
   解けたことを確認したうえで Q1 の第 2 版と Q2 を走らせ、両方とも採り切った。

親からは「A（ゲートの一時解除）と B（guard の対象外化の合意取り付け）は採らない」との指示を
受けている。**その 2 つはいずれも実施していない。**採取が成立したのは 3〜5 の経緯による。

## 採取スクリプトの置き場と再走

後続の担当が拾えるよう、採取に使ったものはすべて [upstream-capture-logs/](upstream-capture-logs/)
に残してある。手順は同ディレクトリの [README.md](upstream-capture-logs/README.md) にある。
リポジトリの既存の採取一式（`scripts/goldens/capture-*.ts`）と同じ作りで、来歴検査
`verifySource` を共有しているので、そのまま走る。

| 種別 | ファイル | 用途 |
| --- | --- | --- |
| 採取 | `capture-q1-review-freeze.ts` | Q1。条件 A〜D を相互投入で比べる。 |
| 採取 | `capture-q2-memory-empty.ts` | Q2。初期化段 `state-init` の経路。 |
| 採取 | `capture-q2b-gated-reapproval.ts` | Q2。ゲート段 `requirements-analysis` の経路。 |
| 探索 | `probe-execution-snapshot.ts` | `.aidlc-execution` への依存の切り分け。 |
| 探索 | `probe-caseB-receipts.ts` | 記録の作り手ごとの受領証の突き合わせ。 |
| 探索 | `probe-reapproval-routes.ts` | 再承認へ戻せる経路の洗い出し。 |
| 探索 | `probe-gated-approval.ts` | ゲート段を一周できるかの確認。 |
| 探索 | `probe-unit-bearing.ts` | Unit 有りの前提の組み方。 |

**この採取は一度きりで、CI に載る検査にはなっていない。** golden として固定するなら
`tests/golden/` の来歴規則に沿って `scripts/goldens/` 側へ移す担当が要る。

## 採取の作法（本リポジトリへの影響）

- すべての採取は `mkdtemp` の一時 workspace で行った。本リポジトリの記録・監査シャード・
  状態ファイル・`memory/` の規則・受領証は読んだだけで、書き換えていない。
- 実装コードは 1 行も変更していない。コミット・push・マージ・GitHub Issue の起票も行っていない。
- 本書と [upstream-capture-logs/](upstream-capture-logs/) 以外に新規ファイルを作っていない。
- 配布フック・配布ツールを書き換えていない。`.claude/tools/` へ採取スクリプトを置いていない。
  guard を無効化していない（[採取が一度止まった経緯](#採取が一度止まった経緯迂回はしていない)）。

## Sources

- 固定コミット `a277af218f0df7f325d3b8be7b6d90fce2c5bd40` の配布物
  （`git archive` で取り出し、`scripts/goldens/upstream-source.ts` の `verifySource` で
  277 ファイル・マニフェスト `282b17c5…` の一致を確認）。
- 本家 `hooks/aidlc-review-freeze.ts:145-192,283-320`、
  `tools/aidlc-runtime.ts:358-401,786-806`、
  `tools/aidlc-artifact-resolution.ts:125-290`、
  `tools/aidlc-lib.ts:7961-7998,13815-13852,21910-21915`、
  `hooks/aidlc-sync-workflow-state.ts:92-119`。
- 本 build `target/debug/aidlc`（sha256 `4bb7ebad…`、2026-09-10 10:28）と、
  `modules/core/command/domain/src/orchestration/{stage_slot.rs,empty_memory_stages.rs}`、
  `modules/app/aidlc/src/cli/face.rs`、`modules/app/aidlc/tests/{review_guards_contract.rs,runtime_graph_contract.rs}`。
- 先行記録 [review-guards-verification.md](review-guards-verification.md)、
  [runtime-graph-verification.md](runtime-graph-verification.md)、
  [shell-write-targets-verification.md](shell-write-targets-verification.md)、
  裁定 [step7-divergence-questions.md](step7-divergence-questions.md)。
- 本採取の corpus・スクリプト・生出力は [upstream-capture-logs/](upstream-capture-logs/)。

## Assumptions & Open Questions

- 本 build 側の観測はすべて `target/debug/aidlc`（2026-09-10 10:28、sha256 `4bb7ebad…`）に対するものである。
  以後に他担当が入れた変更は含まない。裁定の直前に採り直す価値がある。
- Q1 条件 C（Unit 有り）は本家側だけ実測できた。本 build は `aidlc-bolt start` が未配線で
  同じ状態を作れず、上流が組んだ記録では素通しするため、**Unit 有りの側の本 build の挙動は未知**である。
- Q2 の秒未満の窓は本 build で未採取である。閉じる必要があるかは利用者の裁定に属する。
- [別枠の所見](#別枠の所見-本-build-のフックは上流形式の記録だけでは判定しない)（`.aidlc-execution` 依存）は
  裁定 2 件のどちらにも含まれていない。切替条件 2 の判定に入れるかどうかの指示を求める。
