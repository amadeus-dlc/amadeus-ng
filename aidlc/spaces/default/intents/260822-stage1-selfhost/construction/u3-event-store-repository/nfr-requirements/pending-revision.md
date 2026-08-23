# pending-revision — U3 nfr-requirements（ステージゲートの Request Changes で適用する改訂案）

> レビュー（iteration 1、READY: Major 1）の所見。受領は終端のため本文は据え置き、ゲートで Request Changes を選んだ直後に適用。
> **B5 の code-generation 計画には取り込む**。

1. NFR2.3 に合格基準を追加: ロック退役完了後、`scripts/coverage.sh` の `TOLERANCE` を 0.05 → 0.01 へ引き締め、冒頭コメント（「U3 のロック退役でジッタ源が消えたら
   0.01 へ」）を更新する（team.md Testing Posture の確約「stage-1 スコープでシード固定により 0.01 へ引き締める」の実行主体が本 Unit）。
