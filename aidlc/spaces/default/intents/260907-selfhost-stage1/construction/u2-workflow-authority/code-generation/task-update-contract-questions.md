# TaskUpdate同期の契約確認

## 実測と衝突

[固定本家の調査記録](runtime-sync-contract.md)と[test-evidence/runtime-sync-observations.json](test-evidence/runtime-sync-observations.json)を根拠とする。

本家2.7.1の通常Claude TaskUpdateは、現在reverse-engineeringの状態で`status=in_progress, activeForm=Running [requirements-analysis]`を受けると、両方の工程を`[-]`とし現在位置をrequirements-analysisへ移す。`Running [market-research]`では、bugfixのSKIP工程も現在位置として受け入れる。いずれも終了0で成功監査は追加しない。同stageの正常同期はLast Updatedと指示の失効を更新する。

現在のIntentExecutionが持つ`at_most_one_active`と`cursor_in_scope`に衝突する。投影だけ別の現在位置を持たせて隠したり、TaskUpdateをjumpに読み替えたりしない。

## Q1: どちらの契約を優先するか

A. 本家を優先し、不変条件を更新する（推奨）。対象入力に対する複数実行中・範囲外の現在位置を含め、集約・再生・投影・Quint/ITFの契約を明示的に更新する。

B. 現在の不変条件を優先し、非互換を明示する。本家と異なる入力の拒否/無変更を契約差分として確定し、完全一致とは扱わない。

C. 保留する。TaskUpdateの製品実装を保留し、調査結果を保持する。

[Answer]: A. 本家を優先し不変条件を更新する

利用者の回答原文: 「推奨」。直前に提示した「3点すべて推奨案で再開する」を選択した回答として記録する。
