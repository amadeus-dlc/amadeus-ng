# 本家フックの実走行採取 — corpus と生ログ

[upstream-capture-verification.md](../upstream-capture-verification.md) の証跡である。
結論と読み方は本体を参照。ここには**再現手順と生の出力**だけを置く。

## 前提の用意（固定コミットの実バイト）

`vendor/aidlc-workflows` の HEAD は固定コミット `a277af218f0df7f325d3b8be7b6d90fce2c5bd40` では
ないので、固定コミットの配布物を取り出して使う。

```
mkdir -p /tmp/aidlc-upstream
git -C vendor/aidlc-workflows archive a277af218f0df7f325d3b8be7b6d90fce2c5bd40 dist/claude \
  | tar -x -C /tmp/aidlc-upstream
```

取り出した木が固定ピンと一致することを、リポジトリ既存の検査で確かめる（277 ファイル、
マニフェスト `282b17c53cb82c24755b149f28e01508ce9eaebbdbe3020034a4dd4c8bda4459`）。

```
bun -e 'import {verifySource} from "./scripts/goldens/upstream-source.ts"; \
  console.log(verifySource("/tmp/aidlc-upstream/dist/claude").manifest)'
```

来歴検証の生ログ: [provenance.log](provenance.log)

## 採取（2 件の裁定）

いずれも `mkdtemp` の一時 workspace だけを触る。本リポジトリの記録・監査・状態・`memory/`・
受領証は書き換えない。第 2 引数は本 build のバイナリである。

```
bun capture-q1-review-freeze.ts   /tmp/aidlc-upstream/dist/claude target/debug/aidlc q1-review-freeze-v2.json
bun capture-q2-memory-empty.ts    /tmp/aidlc-upstream/dist/claude target/debug/aidlc q2-memory-empty.json
bun capture-q2b-gated-reapproval.ts /tmp/aidlc-upstream/dist/claude target/debug/aidlc q2b-gated-reapproval.json
```

| ファイル | 中身 |
| --- | --- |
| [capture-q1-review-freeze.ts](capture-q1-review-freeze.ts) | 裁定 Q1。per-unit ステージの凍結範囲。条件 A〜D を、1 本の記録へ両実装のフックを撃って比べる。 |
| [q1-review-freeze-v2.json](q1-review-freeze-v2.json) | Q1 の生出力（第 2 版。対照群と Unit 有りの条件を通したもの）。 |
| [q1-review-freeze.json](q1-review-freeze.json) | Q1 の生出力（第 1 版。対照群が本家側で成立しておらず、Unit 有りも未完走）。 |
| [capture-q2-memory-empty.ts](capture-q2-memory-empty.ts) | 裁定 Q2。初期化段 `state-init` の経路。本家だけが一周できる。 |
| [q2-memory-empty.json](q2-memory-empty.json) | 同上の生出力。段ごとの `MEMORY_EMPTY` 全件と `runtime-graph.json` の行。 |
| [capture-q2b-gated-reapproval.ts](capture-q2b-gated-reapproval.ts) | 裁定 Q2。ゲート段 `requirements-analysis` の経路。本 build だけが一周できる。 |
| [q2b-gated-reapproval.json](q2b-gated-reapproval.json) | 同上の生出力。正規化していない生の時刻も入れてある。 |

## 探索（前提を組む経路を探した記録）

裁定の答えそのものではないが、「なぜその条件が採れた/採れなかったか」の根拠である。

| ファイル | 何を確かめたか | 生ログ |
| --- | --- | --- |
| [probe-execution-snapshot.ts](probe-execution-snapshot.ts) | 本 build の凍結判定が `.aidlc-execution` に依存するか。**依存する。**本家は依存しない。 | [probe-execution-snapshot.log](probe-execution-snapshot.log) |
| [probe-caseB-receipts.ts](probe-caseB-receipts.ts) | 条件 B で記録の作り手を変えると結論が変わる理由。受領証の監査行は両者で同形・同指紋。 | [probe-caseB-receipts.log](probe-caseB-receipts.log) |
| [probe-reapproval-routes.ts](probe-reapproval-routes.ts) | 再跳躍のあと承認済みへ戻せる経路。本 build は初期化段の `advance` が未配線。 | [probe-reapproval-routes.log](probe-reapproval-routes.log) |
| [probe-gated-approval.ts](probe-gated-approval.ts) | ゲート段で一周できるか。本 build は通る。本家は要約確認の質問記録を要求する。 | [probe-gated-approval.log](probe-gated-approval.log) |
| [probe-unit-bearing.ts](probe-unit-bearing.ts) | Unit 有りの前提の組み方（Bolt の起こし方、成果物の置き場）。 | — |

## 正規化について

比較のため、出力から次だけを伏せている。判定に効く値は伏せていない。

- 一時 workspace の絶対パス（`<ROOT>` / `<RECORD>` / `<PARENT>`）
- ISO 8601 の時刻（`<TS>`）— ただし `q2b-gated-reapproval.json` は
  `raw_memory_empty_timestamps` と `raw_final_completed_at` に生の時刻を残してある。
  秒精度の鍵を検算するのに要るからである。
- sha256（`<HASH>`）— [probe-caseB-receipts.log](probe-caseB-receipts.log) では
  指紋の一致を示すため生のまま残してある。
- intent の記録名（`<INTENT>`）

## 固定化していない

この採取は一度きりで、CI に載る検査にはなっていない。golden として固定するなら、
`tests/golden/` の来歴規則に沿って `scripts/goldens/` 側へ移す担当が要る。
