# 保存判定の拒否 — 再現と修復

## 判定

**VERIFIED** — 成果物の内容を変えずに、監査付きの編集経路で保存すると、回答確認後の保存判定が通る。

2026-09-05 に実測した。原因は、担当エージェントが Python で保存したことで、必要な成果物更新イベントが記録されなかったことだった。ファイルの更新時刻だけでは確認を満たさない。検証ロジックや監査内容の手修正、検証を無効化する設定は必要なかった。

## 実測条件と結果

実プロジェクトの aidlc と .codex を一時ディレクトリへ複製し、実際の `checkSummaryConfirmationEvidence` を同じ Unit・回答・成果物に対して実行した。

| 条件 | 結果 |
| --- | --- |
| 複製直後 | `ok: false`。entities.md が回答確認後に保存されていないという拒否を再現 |
| 同じ内容を通常のファイル書込で再保存 | 同じ拒否 |
| 同じ保存内容を、Codex の成果物更新フックを通す条件で検証 | `ok: true, required: true` |
| 前後の成果物 4 ファイルの SHA-256 比較 | すべて同一 |
| 実プロジェクトで native apply_patch による再保存後 | `ok: true, required: true` |

隔離環境では、保存後の PostToolUse 通知をアダプタへ入力して経路の差を比較した。実プロジェクトではフック入力や監査行を捏造せず、実際に native apply_patch で保存した。entities / rules / traceability の内容は保持した。

## 根拠

- `.codex/tools/aidlc-lib.ts` の `checkSummaryConfirmationEvidence` は、確認記録より後の `ARTIFACT_CREATED` / `ARTIFACT_UPDATED` を照合する。
- `.codex/hooks/aidlc-codex-adapter.ts` の `audit-and-sensors` は apply_patch の対象ファイルを成果物更新フックへ渡す。
- 実際の監査には、Python での再保存に対応する更新がなく、native apply_patch での保存後に更新が記録された。
- 再現用コードと比較生データは `/tmp/verify-this/aidlc-save-guard/` に保存した。一時ディレクトリは永続的な配布物ではない。

## レビュー再開

保存判定の修復後、既存のレビュー回数上限が別の理由で再レビューを拒否した。オーナーが指示済みの「最新スナップショットと差分イベントによる再生」への変更要求を記録したところ、新しいレビュー要求が受理された。

保存判定の修復とレビューの受理は別々に確認した。レビュー要求の受理だけを、レビュー完了や成果物の承認とは扱わない。
