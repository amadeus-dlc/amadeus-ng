# U3 の実装計画の確認

## Plan Approval

[Question]: Approve this exact Code Generation plan?

[code-generation-plan.md](code-generation-plan.md) と、その中の Testing Contract、および [unit-test-instructions.md](unit-test-instructions.md) を承認対象とする。

`aidlc --doctor` を Orchestrate 面の新しい入口として受け、契約 C7 の D1–D5（本家対応 11 行 + 独自の必須診断 6 行）を読取り専用の Query として判定し、C7 の書式で描画して終了コードを返す。診断実施の記録だけを U2 の `RecordHealthCheckUseCase` 経由で `HEALTH_CHECKED` へ投影する。本家採取 14 観測の採用行とのバイト一致と、異常注入（DC1–DC10）を受入とする 7 ステップである。本リポジトリでの doctor 成功と切替は後続工程の達成条件として残す。

[Approval Fingerprint]: sha256:732060c84c640ffc1d3b1001f18452ea7ad5ffea25aed7af22efbad9e0f20617

- Approve Plan — 計画とテスト境界を承認して U3 を実装する。
- Request Changes — 計画またはテスト手順を修正する。

[Answer]: Approve Plan
