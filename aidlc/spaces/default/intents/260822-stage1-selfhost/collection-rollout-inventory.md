# コレクションの横展開確認

2026-09-06、modules配下の公開型とVec/BTreeSet/BTreeMapを保持する型、既存規則がファーストクラスコレクションと名指す型を照合した。

## 対象と判定

| 対象 | 共通trait | 変換・絞込み | 注意点 |
|---|---|---|---|
| BoltRefs | 適用 | 空・combine・divide・filter・map・fold_left・at | 辞書順の集合。mapは不正なslugを拒否 |
| Checkboxes | 適用 | filter・map・fold_left・at・find・has_completed | 重複slugの行を保持。getという添字APIは削除 |
| OrderedAuditEvents | 適用 | filter・fold_left・at | 時刻順・元位置が不変条件なので任意mapは適用しない |
| AuditFields | 適用 | combine・divide・filter・map・fold_left・at | Timestamp破棄と行偽造防止を同じ構築処理で維持 |
| StageGraph | 適用 | filter・map・fold_left・at | 文書順索引とslug一意性。mapは衝突を拒否。汎用的な和集合の衝突優先順位は定めない |
| ScopeGrid | 適用 | セルのfilter・map・fold_left・at | 宣言済み空列と不存在を区別。セル数0でもスコープ名の情報は保持する |
| ObjectMembers | 適用 | combine・divide・filter・map・fold_left・at | キー一意、最初の位置と最後の値 |
| Collection<T> | 新設 | 異型map、filter、連結combine、要素除去divide、fold_left、at | 空を許す順序付き列。集合ではなく重複を保持 |
| NonEmptyCollection<T> | 新設 | mapとcombineは非空、filter/divideはCollection | 先頭を独立して持ち、空を構築できない。filter結果の非空型への代入はコンパイル失敗試験で拒否 |

## 一律適用しない型

- Intent・IntentExecution・CompiledDefinition・WorkflowDefinitionは集約であり、複数の値や状態遷移を所有する。内部に配列があってもコレクションtraitを直接適用しない。
- Started・Created・Recomposed等のイベント、各DTO、StageNode/Builder、ScopeMetadataは記録・構築材料。コレクション操作だけで定義される型ではない。
- ReviewAttemptは承認試行の状態を所有し、単なる試行集合ではない。
- PracticesPromotionは複数の節・規則と検証を伴う昇格計画。ResolvedPlanはステージ列に加えてscope・request・scanを持つ投影材料。汎用mapで関連情報を失わせない。
- JsonValueはJSON値の直和型であり、すべての値がコレクションではない。ObjectMembers側を対象とする。

## 呼出側の変更

投影の検索・完了判定をCheckboxesへ、人間の発話の集計をOrderedAuditEvents.fold_leftへ移した。監査描画はAuditFields.fold_leftで走査する。JSONライタはObjectMembers.atを使い、全メンバの参照配列を先に作る処理を除去した。グラフのダイジェストと書込側/RMU側のDTO変換はStageGraph.fold_leftを使う。

残るイテレータは、具体的な順序付きデータの書出し、索引付き走査、既存DTOへの変換などを個別に扱う。全イテレータを機械置換したとは主張しない。新たなファーストクラスコレクションを追加するときは、この対象表と共通契約試験を同時に更新する。
