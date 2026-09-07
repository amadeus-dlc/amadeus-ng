# pending-revision — U9 nfr-design（ステージゲートの Request Changes で適用する改訂案）

> レビュー（iteration 1、READY: Minor 1）の所見。受領は終端のため本文は据え置き、ゲートで Request Changes を選んだ直後に適用。
> B4 の code-generation 計画には是正後の文言（下記）を採用する。

1. §3 受入 2 の `StageGraphReader` の一文を「`StageGraphReader` は履歴注記（gateway-taxonomy.md『適用の帰結』節の旧→新移行表、旧列）として対象外
   （`../functional-design/pending-revision.md` 所見 3）」に差し替える（禁止名テーブルに載るのは `StageGraphRepository` であり、`StageGraphReader` は
   旧→新移行表に出る）。

## 解消（2026-09-07、U9 再走）

上記 1 は前提ごと閉じた。再走版 `../functional-design/rules.md` BR5.1 (c) が `StageGraphReader` を sentinel から外す裁定を確定したため、
`security-design.md` に除外根拠の一文を置く必要がなくなった（再走版 §4 受入 (2) に BR5.1 (c) を出典として明記）。本ファイルは履歴として残す。
