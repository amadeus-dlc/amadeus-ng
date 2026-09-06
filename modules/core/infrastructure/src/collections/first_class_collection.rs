//! コレクションごとの順序・要素表現を保持する共通操作。

/// 一級コレクションの読取と絞り込み。要素の借用形と、空を許す絞込結果を型ごとに定める。
pub trait FirstClassCollection {
    /// 借用した要素。キーと値の組なども表現できる。
    type Item<'a>
    where
        Self: 'a;
    /// 要素がなくなる可能性のある操作の結果型。
    type Filtered: FirstClassCollection;
    /// 要素数。
    fn len(&self) -> usize;
    /// 要素がないか。
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    /// コレクションの定める順序で添字参照する。範囲外はNone。
    fn at(&self, index: usize) -> Option<Self::Item<'_>>;
    /// 左から畳み込む。空なら初期値を返す。
    fn fold_left<'a, A>(&'a self, initial: A, fold: impl FnMut(A, Self::Item<'a>) -> A) -> A;
    /// 条件に一致する要素を元の順序で返す。非空の入力でも結果は空になり得る。
    fn filter(&self, predicate: impl FnMut(Self::Item<'_>) -> bool) -> Self::Filtered;
}
