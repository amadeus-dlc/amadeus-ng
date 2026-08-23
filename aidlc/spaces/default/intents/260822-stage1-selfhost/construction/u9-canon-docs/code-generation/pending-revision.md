# pending-revision — U9 code-generation（ステージゲートの Request Changes で適用する改訂案）

> レビュー受領（READY）後に PR #28 の CodeRabbit 指摘で判明した記録の精度問題。code-summary / unit-test-instructions は凍結（受領・承認指紋）のため
> 本文は据え置き、ゲートで Request Changes を選んだ直後に適用。

1. code-summary §2 の行数: 表の値は統合前の委任報告ベースで、合計 `+148 / −94` は新規ファイル除外の作業ツリー diff。`git diff --numstat origin/main..HEAD` の実測に
   差し替える（レビュー所見反映コミットまで含む）:
   - `aidlc/spaces/default/intents/260822-stage1-selfhost/inception/domain-design/components.md`: +10 / −9
   - `aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/README.md`: +2 / −1
   - `aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/error-handling.md`: +24 / −0
   - `aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/gateway-taxonomy.md`: +13 / −8
   - `aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/use-case-rules.md`: +1 / −1
   - `docs/specs/01-domain-model.md`: +28 / −12
   - `docs/specs/10-orchestration.md`: +28 / −16
   - `docs/specs/11-workspace.md`: +38 / −21
   - `docs/specs/12-workflow-definition.md`: +30 / −29
   - `docs/specs/deviations.md`: +1 / −0
   - 合計 +175 / −97（新規 error-handling.md を含む、コミット時点の実測）
2. unit-test-instructions §1 表の検査コマンド（`grep -c '^\| \['` の Markdown エスケープ、`| 4 |` のセル）を fenced `bash` ブロックへ移し、そのまま実行できる形にする
   （実行形は `grep -c '^| \['` / `grep -c '^| 4 |' docs/specs/deviations.md`。developer-report-1/2 には実行形を記載済み）。
