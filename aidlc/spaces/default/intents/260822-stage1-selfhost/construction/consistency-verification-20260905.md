# 不整合是正と実測結果

2026-09-05。作業ツリー上の実装修正と設計記録の同期をまとめる。intent全体や各ステージの正式な承認完了を意味しない。

## 修正した内容

1. **公開中断の復旧**: ファイルの前後を持つ計画を先に保存し、確定前の停止から二重追記せず復旧する。出力先ごとの確定済み計画を保持し、欠落ファイルを復元する。共有リードモデルの巻戻り・同位置不一致・内容破損を拒否する。
2. **保存先の取り違え防止**: IntentExecutionとWorkflowDefinitionの両Repositoryで、イベントの集約IDと保存対象IDをI/O前に照合する。SQLite・メモリの双方で修正前の誤受入を再現した。Intentの保存には既存の全状態照合がある。
3. **設計記録の同期**: 正準JSONのキー順・丸め・変換失敗・文字列値域、集約の所有・現行イベント・開始状態・コマンド別ガード、Repository署名・版・破損検査・スナップショット間隔を是正した。

集約の再構成は、確定済みの「最新スナップショットと、その通番より後の差分イベント」を維持する。リードモデルの再計算とは区別する。

詳細:

- [正準JSONの是正](u1-canon-json-goldens/correction-report.md)
- [集約設計の是正](u2-domain-es-core/correction-report.md)
- [永続化設計の是正](u3-event-store-repository/correction-report.md)
- [保存先取り違えの実測と修正](u3-event-store-repository/implementation-report.md)
- [公開中断からの復旧の実測と修正](u4-read-model-updater/implementation-report.md)

## 最終検証

| 検証 | 結果 |
|---|---|
| cargo test --locked --workspace | 49スイート、2,140件成功、失敗0、無視0、終了コード0 |
| cargo clippy --locked --workspace --all-targets -- -D warnings | 成功 |
| cargo lint | 成功 |
| cargo fmt --all -- --check / git diff --check | 成功 |
| scripts/coverage.sh | 行カバレッジ98.64713774597496%、90%床を通過、終了コード0 |
| scripts/quint-gate.sh | 全項目成功、終了コード0。モデルは変更していない |
| 設計の構造検査 | YAML・JSON・規則と参照の検査に成功。過去のReview本文を保存 |

最後の全体検証には、CLIで出力を削除して復元する試験と、両RepositoryのID取り違えを両バックエンドで拒否する追加試験も含む。
実装の独立した読み取り専用再レビューでは、指摘された対象別復元の不足を是正し、保存前ID照合を含めて重大な残件は見つからなかった。

設計の技術確認でも、正準JSONのモジュール境界、U2の内容版の導出所有、U3の版所有裁定の出典に関する追加指摘を修正した。
U2の公開next_decisionには直接のID不一致Errがないが、現行RMUはID一致でIntentを取得し、不在なら失敗してから渡す。
そのため現行経路の取り違え不具合とは認定せず、公開APIと一般規律の差異として個別記録に残す。

ログは `/tmp/verify-this/consistency-all-tests.log`、`consistency-all-clippy.log`、`consistency-all-lint.log`、
`consistency-all-coverage.log`、`u4-quint.log`。一時ファイルに依存しないよう、試験名と再現内容は個別記録にも残した。

## 残る範囲

- プロセス強制終了は検証したが、電源断や記憶装置の故障は再現していない。
- 曖昧な利用者編集は競合として保持し、自動上書きしない。
- ID照合は同一IDの任意イベントと任意状態の意味的一致を証明しない。
- Quintの毎回スナップショット更新モデルを、既定の間欠更新全般の証明へ拡大解釈しない。
- 相対カバレッジとリモートCIは未実行。
- 正式な設計レビュー開始処理は、今回の試行に対する要約確認の記録不足を理由に拒否した。正準JSONの修正内容について確認を提示済みであり、以前のLooks correctを今回の回答として再記録していない。過去のReview節を今回の判定と読み替えたり、承認済みとして状態を進めたりしていない。
