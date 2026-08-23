# pending-revision — U9 nfr-design（ステージゲートの Request Changes で適用する改訂案）

> レビュー（iteration 1、READY: Minor 1）の所見。受領は終端のため本文は据え置き、ゲートで Request Changes を選んだ直後に適用。
> B4 の code-generation 計画には是正後の文言（下記）を採用する。

1. §3 受入 2 の `StageGraphReader` の一文を「`StageGraphReader` は履歴注記（gateway-taxonomy.md『適用の帰結』節の旧→新移行表、旧列）として対象外
   （`../functional-design/pending-revision.md` 所見 3）」に差し替える（禁止名テーブルに載るのは `StageGraphRepository` であり、`StageGraphReader` は
   旧→新移行表に出る）。
