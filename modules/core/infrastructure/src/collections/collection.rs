//! 空を許す順序付きコレクション。
use super::{FirstClassCollection, NonEmptyCollection};

/// 要素順と重複を保持する不変の列。mapは変換先のCollectionを返す。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Collection<T> {
    items: Vec<T>,
}

impl<T> Collection<T> {
    /// 順序付き要素から構築する。
    #[must_use]
    pub const fn new(items: Vec<T>) -> Self {
        Self { items }
    }
    /// 空列。結合の単位元。
    #[must_use]
    pub const fn empty() -> Self {
        Self::new(Vec::new())
    }
    /// 要素数。
    #[must_use]
    pub const fn len(&self) -> usize {
        self.items.len()
    }
    /// 空か。
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
    /// 挿入順の添字参照。範囲外はNone。
    #[must_use]
    pub fn at(&self, index: usize) -> Option<&T> {
        self.items.get(index)
    }
    /// 順序を保って異なる要素型へ写す。
    #[must_use]
    pub fn map<U>(&self, transform: impl FnMut(&T) -> U) -> Collection<U> {
        Collection::new(self.items.iter().map(transform).collect())
    }
    /// 左から畳み込む。空なら初期値。
    pub fn fold_left<'a, A>(&'a self, initial: A, fold: impl FnMut(A, &'a T) -> A) -> A {
        self.items.iter().fold(initial, fold)
    }
    /// 条件に一致する要素を元の順序で残す。
    #[must_use]
    pub fn filter(&self, mut predicate: impl FnMut(&T) -> bool) -> Self
    where
        T: Clone,
    {
        Self::new(
            self.items
                .iter()
                .filter(|item| predicate(item))
                .cloned()
                .collect(),
        )
    }
    /// 列を連結する。集合ではないため重複を消さない。
    #[must_use]
    pub fn combine(&self, other: &Self) -> Self
    where
        T: Clone,
    {
        Self::new(self.items.iter().chain(&other.items).cloned().collect())
    }
    /// 他方と等しい要素をすべて除く。残る順序は保つ。
    #[must_use]
    pub fn divide(&self, other: &Self) -> Self
    where
        T: Clone + PartialEq,
    {
        self.filter(|item| !other.items.contains(item))
    }
    /// 非空型へ移す。
    ///
    /// # Errors
    /// 空の場合は元の空コレクションを返す。
    pub fn into_non_empty(self) -> Result<NonEmptyCollection<T>, Self> {
        let mut items = self.items.into_iter();
        match items.next() {
            Some(first) => Ok(NonEmptyCollection::new(first, items.collect())),
            None => Err(Self::empty()),
        }
    }
}

impl<T> Default for Collection<T> {
    fn default() -> Self {
        Self::empty()
    }
}

impl<T: Clone> FirstClassCollection for Collection<T> {
    type Item<'a>
        = &'a T
    where
        Self: 'a;
    type Filtered = Self;
    fn len(&self) -> usize {
        Self::len(self)
    }
    fn at(&self, index: usize) -> Option<&T> {
        Self::at(self, index)
    }
    fn fold_left<'a, A>(&'a self, initial: A, fold: impl FnMut(A, &'a T) -> A) -> A {
        Self::fold_left(self, initial, fold)
    }
    fn filter(&self, predicate: impl FnMut(&T) -> bool) -> Self {
        Self::filter(self, predicate)
    }
}
