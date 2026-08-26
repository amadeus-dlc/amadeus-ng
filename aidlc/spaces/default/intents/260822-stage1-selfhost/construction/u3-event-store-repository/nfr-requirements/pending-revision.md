# pending-revision — U3 nfr-requirements（ステージゲートの Request Changes で適用する改訂案）

> レビュー（iteration 1、READY: Major 1）の所見。受領は終端のため本文は据え置き、ゲートで Request Changes を選んだ直後に適用。
> **B5 の code-generation 計画には取り込む**。

1. NFR2.3 に合格基準を追加: ロック退役完了後、`scripts/coverage.sh` の `TOLERANCE` を 0.05 → 0.01 へ引き締め、冒頭コメント（「U3 のロック退役でジッタ源が消えたら
   0.01 へ」）を更新する（team.md Testing Posture の確約「stage-1 スコープでシード固定により 0.01 へ引き締める」の実行主体が本 Unit）。
2. NFR4.3 の合格基準を実態に合わせる: `unwrap_used` / `expect_used` は clippy deny 済みだが、`indexing_slicing` / `panic` は workspace lints に無く人力レビュー任せ
   （レビュー所見 2）。`clippy::indexing_slicing` / `clippy::panic` の workspace lint 昇格はオーナー裁定事項として B5 計画承認時に確認する（昇格すれば B5 で追加、
   既存コードの是正込み。見送るなら NFR4.3 を「deny（unwrap / expect）+ レビュー（索引 / panic!）」に書き改める）。
3. NFR4.1 の「`cargo audit` が CI で緑」は、`audit` ジョブが advisory（`CI Success` の必須チェック外）である現状運用に合わせて注記する（レビュー所見 3）。
