# pending-revision — U2 code-generation（ステージゲートの Request Changes で適用する改訂案）

> 終端の受領（iteration 1、READY）が凍結しているため、code-generation ステージゲートで Request Changes を選んだ直後に適用し、レビュアーを再実行する。

1. `code-summary.md` §1 の品質ゲート行: カバレッジを最終コミット `fa6bf64` での再計測値に統一 — `scripts/coverage.sh` 97.40%、
   `cargo llvm-cov -p core-domain --summary-only` lines 96.55%（regions 97.08% / functions 95.50%、`cargo llvm-cov clean --workspace` 後）。
   旧記載 97.38% / 96.53% は 1d035f5 時点の値（PR #27 CodeRabbit 指摘）。
