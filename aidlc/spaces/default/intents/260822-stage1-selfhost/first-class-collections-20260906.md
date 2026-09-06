# ファーストクラスコレクションの操作と呼出側の是正

> 以下の初回3型の適用に続き、オーナーの横展開・共通trait・非空型の指示を追加反映した。最新の対象表は `collection-rollout-inventory.md`、追加結果は末尾を参照。

2026-09-06、オーナーのcombine/divide/filter/map/foldLeft/atの提案と継続指示に基づき、既存型へ適用した。Rustの名前はfold_leftとする。イテレータを最後の手段とする規則を具体的な操作へ反映する。

## 変更

| 型 | 実装した操作 | 保持する意味 |
|---|---|---|
| BoltRefs | empty・combine・divide・filter・map・fold_left・at | 辞書順の重複しない参照集合。combineは和集合、divideは差集合。mapは参照名から参照名へ写し、不正な変換結果はエラー |
| Checkboxes | filter・map・fold_left・at・find・has_completed | 行順と重複行を保持。従来の添字getは削除してatへ置換。findは最初の一致、has_completedは全行を考慮 |
| OrderedAuditEvents | filter・fold_left・at | 時刻順と同秒時の元位置を保持。時刻を自由に変更できるmapは追加しない |

投影処理の3箇所の検索をCheckboxes.findへ、完了行の存在判定をhas_completedへ移した。HumanTurnsの2本のイテレータ走査はfold_leftの1回の集計へまとめた。ユースケースへドメインgetterを移してはいない。

mapは現在の要素型の範囲で閉じた変換で、生のVecを返さない。異なる型への変換は変換先コレクションの意味が決まった時点で設計する。StageGraphなど順序・識別子衝突の制約が異なる型を、無理に集合Monoidへ合わせていない。

## 検証対象

- 和集合の結合法則、左右単位元、交換法則、冪等性。
- 同じ集合との差が空、空集合との差が元の集合になること。
- mapの重複排除、不正値・予約語拒否、元の値が変更されないこと。
- 空コレクションでmapの関数を呼ばず、fold_leftが初期値を返すこと。
- 添字0・末尾・範囲外・usize上限の参照。
- 重複slugの先頭検索と、後続の完了行を見落とさないこと。
- 監査イベントの時刻順と同秒内の元位置がfilter後も維持されること。

BoltRefsの新API不在、OrderedAuditEventsの新API不在、mapが空集合の予約語を受理する問題を失敗試験で確認してから実装・修正した。Checkboxesの最初の試験はテストモジュール属性の配置誤りでコンパイル失敗しており、これを振る舞いのRedとして計上しない。属性を修正した後、重複行の完了判定を公開操作の試験で確認した。

## 作業の境界

これはコレクション操作の適用であり、AI-DLCのU10要約確認や過去の機能設計所見を承認したことにはしない。以前のコーパス修正・承認制御修正は維持する。全型へ一律にメソッドを追加したり、不要な互換ラッパーを残したりはしない。

## 実行結果

- workspace領域の単体試験126件、投影の単体試験39件成功。
- ワークスペース全体の試験2,231件成功、失敗・無視0（/tmp/fcc-all-tests.log）。
- ワークスペースClippy（全target、警告をエラー扱い）、cargo lint、rustfmt検査成功。
- カバレッジ実測99.12081182748666%。90%床を満たす（/tmp/fcc-coverage.log）。
- origin/mainが引き続きf67268022aefa01c8a65cee11b0f459bae721d33であることを確認。同コミットの直前実測99.13072110635495%（/tmp/residual-coverage.log）との差は-0.009909278868292404ポイントで、許容0.01以内。基準側は同じコミットの測定済み結果を再利用し、今回は再計測していない。

差分では、findの「最初の一致」とhas_completedの「いずれかの一致」を区別したため、重複slugの既存動作を維持する。HumanTurnsは無効な時刻をNoneとして比較から除外し、DocumentKB行だけでは追跡を有効にしない既存動作を保つ。ユースケースへのgetter追加はない。

## 共通traitと非空型を含む追加の横展開

FirstClassCollectionはlen/is_empty/at/fold_left/filterと要素の借用形・絞込結果型を定める。既存7型へ適用し、Collection<T>とNonEmptyCollection<T>を追加した。非空型のmapは非空を保ち、filter/divideは空を許す型へ戻る。型付きの変換失敗や順序制約は各具体型に残す。

全体試験2,241件が成功し、追加した非空filterのcompile-fail doctestも成功した。Clippy・cargo lint・rustfmt検査が成功。最終カバレッジは99.14022164135578%で、90%床と実測済みmain基準99.13072110635495%を満たす。初回3型の測定値とは別の、横展開後の結果である。

先行する承認制御・コーパス修正は [PR #113](https://github.com/amadeus-dlc/amadeus-ng/pull/113) として分離した。元worktreeのGit管理領域が書込不可のため、書込可能な独立チェックアウトでコミットと公開を行う。元の作業ファイルは保持する。
