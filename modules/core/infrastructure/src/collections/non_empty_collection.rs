//! 先頭要素を必ず保持する順序付きコレクション。
use super::{Collection, FirstClassCollection};

/// 少なくとも1要素を持つ列。空値を構築するAPIは提供しない。
///
/// 絞込みは空になり得るので、その結果を非空型へ代入できない。
/// ```compile_fail
/// use core_infrastructure::collections::NonEmptyCollection;
/// let items = NonEmptyCollection::new(1, vec![2]);
/// let _: NonEmptyCollection<i32> = items.filter(|_| false);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NonEmptyCollection<T> {
    first: T,
    rest: Vec<T>,
}

impl<T> NonEmptyCollection<T> {
    /// 必須の先頭要素と、任意の後続要素で構築する。
    #[must_use]
    pub const fn new(first: T, rest: Vec<T>) -> Self {
        Self { first, rest }
    }
    /// 必ず存在する先頭要素。
    #[must_use]
    pub const fn first(&self) -> &T {
        &self.first
    }
    /// 要素数。
    #[must_use]
    pub const fn len(&self) -> usize {
        self.rest.len() + 1
    }
    /// この型は常に非空。
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        false
    }
    /// 添字参照。範囲外はNone。
    #[must_use]
    pub fn at(&self, index: usize) -> Option<&T> {
        if index == 0 {
            Some(&self.first)
        } else {
            self.rest.get(index - 1)
        }
    }
    /// 要素数を保って変換し、非空の結果を返す。
    #[must_use]
    pub fn map<'a, U>(&'a self, mut transform: impl FnMut(&'a T) -> U) -> NonEmptyCollection<U> {
        let first = transform(&self.first);
        NonEmptyCollection::new(first, self.rest.iter().map(transform).collect())
    }
    /// 先頭から左へ畳み込む。
    pub fn fold_left<'a, A>(&'a self, initial: A, mut fold: impl FnMut(A, &'a T) -> A) -> A {
        let initial = fold(initial, &self.first);
        self.rest.iter().fold(initial, fold)
    }
    /// 空になる可能性があるため、通常のCollectionを返す。
    #[must_use]
    pub fn filter(&self, mut predicate: impl FnMut(&T) -> bool) -> Collection<T>
    where
        T: Clone,
    {
        Collection::new(self.fold_left(Vec::new(), |mut items, item| {
            if predicate(item) {
                items.push(item.clone());
            }
            items
        }))
    }
    /// 非空列同士の連結。先頭と順序を保持する。
    #[must_use]
    pub fn combine(&self, other: &Self) -> Self
    where
        T: Clone,
    {
        let mut rest = self.rest.clone();
        rest.push(other.first.clone());
        rest.extend(other.rest.iter().cloned());
        Self::new(self.first.clone(), rest)
    }
    /// 他方に含まれる要素を除き、空を許す型を返す。
    #[must_use]
    pub fn divide(&self, other: &Collection<T>) -> Collection<T>
    where
        T: Clone + PartialEq,
    {
        self.filter(|item| !other.fold_left(false, |found, candidate| found || candidate == item))
    }
    /// 要素を移して空を許す型へ変換する。
    #[must_use]
    pub fn into_collection(self) -> Collection<T> {
        Collection::new(std::iter::once(self.first).chain(self.rest).collect())
    }
}

impl<T> TryFrom<Collection<T>> for NonEmptyCollection<T> {
    type Error = Collection<T>;
    fn try_from(collection: Collection<T>) -> Result<Self, Self::Error> {
        collection.into_non_empty()
    }
}

impl<T: Clone> FirstClassCollection for NonEmptyCollection<T> {
    type Item<'a>
        = &'a T
    where
        Self: 'a;
    type Filtered = Collection<T>;
    fn len(&self) -> usize {
        Self::len(self)
    }
    fn at(&self, index: usize) -> Option<&T> {
        Self::at(self, index)
    }
    fn fold_left<'a, A>(&'a self, initial: A, fold: impl FnMut(A, &'a T) -> A) -> A {
        Self::fold_left(self, initial, fold)
    }
    fn filter(&self, predicate: impl FnMut(&T) -> bool) -> Collection<T> {
        Self::filter(self, predicate)
    }
}
