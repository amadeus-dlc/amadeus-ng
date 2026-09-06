# ファーストクラスコレクションの操作

**例外**: 理由付きで可。既存の基本操作・ドメイン固有操作で表せない境界処理に限り、イテレータ公開の理由を記載する。

裁定: オーナー、2026-09-06。配列やイテレータを外へ取り出して処理する前に、コレクション自身の操作で表現する。

## 基本操作

共通の読取・絞込み契約は `core_infrastructure::collections::FirstClassCollection` とする。要素の借用形は関連型、filterの結果は空を許すコレクション型で表す。mapは、異型変換や重複拒否等の結果型が型ごとに異なるため、各具体型の契約として定める。

一般の順序付き列には `Collection<T>`（空を許す）と `NonEmptyCollection<T>`（先頭要素を必ず持つ）を使える。非空型はmap・非空列同士のcombineで非空を保ち、filter・divideではCollectionへ戻る。空列からの非空変換は失敗し得る。列のcombineは連結であり、集合の和集合と混同しない。

必要な操作を `filter`・`map`・`fold_left`・`at` としてコレクション側に持たせる。RustではfoldLeftを慣用のsnake_caseで表記する。

- `filter`: 元の順序と不変条件を保つコレクションを返す。空を許さない型では、空結果の扱いを明示する。
- `map`: 変換先の要素に対応するコレクションを返す。生の配列を返すための抜け道にせず、要素型や一意性・順序の制約を維持する。変換で制約に違反し得る型は、その失敗を明示する。
- `fold_left`: 初期値から、コレクションが定める順序で左から集約する。空なら初期値を返す。
- `at`: インデックスで要素を参照する。範囲外はOptionで表現し、panicしない。集合型で提供する場合も、添字が指す順序を定義する。

イテレータ公開は、これらとドメイン固有の操作で表せない境界処理の最後の手段とする。内部実装でイテレータを使うことは問題ない。既存のgetterをfilterやmapのクロージャへ移しただけでTell Don't Ask違反を解消したとは扱わない。ユースケースからの業務判断は、ドメインオブジェクトやコレクションの命令・名前付き操作に閉じ込める。

## 結合と差集合

集合を表す型では `combine` で和集合、`divide` で差集合を構成できるようにする。元のコレクションを変更せず、不変条件を満たす新しい値を返す。

空集合を許し、和集合を全域的に定義できる型は、空集合を単位元としたMonoidになる。結合法則と左右の単位元を試験する。集合なら冪等性・交換法則も確認する。divideは結合の逆演算ではなく、差集合の操作として `A \ A = empty`、`A \ empty = A` 等を確認する。

すべてのファーストクラスコレクションを無条件に集合やMonoidとみなさない。順序付き列の結合、重複排除、同じ識別子の異なる内容の扱いは型の意味で決める。現行StageGraphは文書順とslug一意性を持つため、結合時の衝突を黙って捨ててMonoidに見せかけない。

## 検証と適用

型の不変条件、操作の境界値、順序・重複・空の扱いを単体試験と性質試験で固定する。業務上意味のある操作を優先し、使われない共通メソッド群を機械的に追加しない。

現在は設計・レビュー基準。コレクション操作の一律なリンター強制は未実装であり、実装済みと主張しない。ユースケースのドメインgetter禁止は既存のTell Don't Ask規則を引き続き適用する。

## 適用例（2026-09-06）

| 型 | 操作と契約 |
|---|---|
| BoltRefs | empty・combine（和集合）・divide（差集合）・filter・map・fold_left・at。辞書順、重複なし。mapは参照名から参照名への変換で、不正な変換結果はResultで拒否する。atの走査時間は位置に比例する |
| Checkboxes | filter・map・fold_left・at・find・has_completed。元の行順と重複行を維持する。従来の添字getはatへ置換。findは最初の一致、has_completedは同じslugの全行を考慮する |
| OrderedAuditEvents | filter・fold_left・at。既存の時刻順と同秒の元位置を保持する。任意mapで時刻や位置を変更できる入口は追加しない |
| AuditFields | combine・divide・filter・map・fold_left・at。挿入順・同名キーの後勝ち・Timestampの破棄・値の改行無害化を保持する |
| ObjectMembers | combine・divide・filter・map・fold_left・at。挿入順・キー一意・同名キーの後勝ちを保持する |
| StageGraph | filter・map・fold_left・at。filter後は文書順索引を再構築し、mapでslugが衝突すればResultで拒否する。combine/divideの衝突規則を推測で追加しない |
| ScopeGrid | セル単位のfilter・map・fold_left・at。scope/slugの辞書順、欠落とSKIPの区別、宣言済み空列を保持する。列を消す集合演算とセル除去を混同しない |

CheckboxesのmapはCheckboxEntryからCheckboxEntryへの変換であり、別の要素型へ写す場合の汎用コンテナではない。必要な変換先の型が定まった時点で、その型の不変条件を守る操作を設計する。順序付き列を便宜的な集合として扱わず、任意の重複排除・並べ替えは行わない。

投影側のチェックボックス検索・完了判定はfind/has_completedを使い、人間の発話記録の集計はOrderedAuditEvents.fold_leftを使う。コレクション外での配列・イテレータ操作を減らす実例とする。各基本操作の内部でイテレータを使うことは禁止しない。

既存7型のtrait適合は `modules/core/command/domain/tests/collection_contract_test.rs` と `modules/core/infrastructure/tests/collections_test.rs` で検査する。集合型以外のcombineは順序や後勝ちの規則を持つため交換法則を仮定しない。非空型に空の単位元はないので、非空型自身をMonoidとはみなさない。
