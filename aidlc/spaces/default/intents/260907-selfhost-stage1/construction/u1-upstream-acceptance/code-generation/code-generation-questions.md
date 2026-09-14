# U1の実装計画の確認

## Plan Approval

[Question]: Approve this exact Code Generation plan?

[code-generation-plan.md](code-generation-plan.md)と、その中のTesting Contract、および[unit-test-instructions.md](unit-test-instructions.md)を承認対象とする。採取元検証・観測保存・比較CLIの3つの公開境界、各境界7件を目安にしたTDD、本家2.7.1からの再採取と再現性検証を含む。

[Approval Fingerprint]: sha256:c7f5a78b8b2cca3f68fb59006eda0a53da2941775800a5a88a176250d37fcc0a

- Approve Plan — この計画とテスト境界を承認して実装する。
- Request Changes — 計画またはテスト手順を修正する。

[Answer]: Approve Plan

## 今回の再承認対象

2026-09-13 のセッション再開で工程の指示が再発行され、前回の承認（指紋 `sha256:81e0dd9c…`）は受領前に失効した。前回は native フック接続の切替後に人間の応答が監査へ記録されず、受領コマンドが拒否されたためである。計画本文・Testing Contract（`contract_sha256` は `sha256:d904f82d…` のまま）・テスト手順は無変更で、指紋だけを再計算した。要件や修正範囲の追加ではない。

R-01〜R-03 の修正と再検証は完了済みである。[修正証跡](review-repair-evidence.md)を参照する。前回の指摘は[正規ツールが返した引継ぎ](review-prior-findings.md)へ保持する。差し戻し（revision 3 of 3）で試行がリセットされたため、承認の受領後に Unit 完了の受領と独立レビューをこの試行で取り直す。
