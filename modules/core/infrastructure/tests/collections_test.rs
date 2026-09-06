//! 空を許す列・非空列・共通操作の契約検査。
use core_infrastructure::canon_json::{JsonValue, ObjectMembers};
use core_infrastructure::collections::{Collection, FirstClassCollection, NonEmptyCollection};
use proptest::prelude::*;

#[test]
fn empty_and_non_empty_results_are_distinct() {
    let empty = Collection::<i32>::new(Vec::new());
    assert!(NonEmptyCollection::try_from(empty).is_err());
    let nonempty = NonEmptyCollection::new(1, vec![2, 3]);
    assert_eq!(nonempty.first(), &1);
    assert_eq!(nonempty.map(|x| x.to_string()).first(), "1");
    assert!(nonempty.filter(|_| false).is_empty());
    assert_eq!(nonempty.at(2), Some(&3));
    assert_eq!(nonempty.at(3), None);
    assert_eq!(nonempty.at(usize::MAX), None);
    assert_eq!(
        nonempty.fold_left(String::new(), |acc, x| acc + &x.to_string()),
        "123"
    );
}

#[test]
fn operations_compose_without_exposing_iterators() {
    let numbers = Collection::new(vec![1, 2, 3]);
    let result = numbers.filter(|x| *x != 2).map(|x| x * 10);
    assert_eq!(result, Collection::new(vec![10, 30]));
    assert_eq!(
        numbers.combine(&Collection::new(vec![3, 4])),
        Collection::new(vec![1, 2, 3, 3, 4])
    );
    assert_eq!(
        numbers.divide(&Collection::new(vec![2])),
        Collection::new(vec![1, 3])
    );
    assert_eq!(numbers.at(1), Some(&2));
    assert_eq!(numbers.at(usize::MAX), None);
    assert_eq!(Collection::<i32>::empty().fold_left(9, |a, x| a + x), 9);
    assert!(
        NonEmptyCollection::new(1, vec![])
            .divide(&numbers)
            .is_empty()
    );
    assert_eq!(
        NonEmptyCollection::try_from(numbers.clone())
            .unwrap()
            .into_collection(),
        numbers
    );
}

fn count<C: FirstClassCollection>(collection: &C) -> usize {
    collection.fold_left(0, |count, _| count + 1)
}

#[test]
fn trait_covers_both_cardinalities() {
    let empty = Collection::<i32>::empty();
    assert_eq!(count(&empty), 0);
    assert_eq!(FirstClassCollection::len(&empty), 0);
    assert!(FirstClassCollection::is_empty(&empty));
    let nonempty = NonEmptyCollection::new(4, vec![5]);
    assert_eq!(nonempty.clone().into_collection().len(), 2);
    assert_eq!(count(&nonempty), 2);
    assert_eq!(
        FirstClassCollection::filter(&nonempty, |x| *x == 5),
        Collection::new(vec![5])
    );
    assert_eq!(FirstClassCollection::at(&nonempty, 0), Some(&4));
    assert!(!FirstClassCollection::is_empty(&nonempty));
    assert_eq!(FirstClassCollection::len(&nonempty), 2);
    assert_eq!(FirstClassCollection::at(&empty, 0), None);
    assert!(FirstClassCollection::filter(&empty, |_| true).is_empty());
    let object: ObjectMembers = [("a".to_string(), JsonValue::Null)].into_iter().collect();
    assert_eq!(count(&object), 1);
    assert_eq!(FirstClassCollection::len(&object), 1);
    assert_eq!(
        FirstClassCollection::at(&object, 0),
        Some(("a", &JsonValue::Null))
    );
    assert_eq!(FirstClassCollection::filter(&object, |_| true), object);
    assert!(FirstClassCollection::filter(&object, |_| false).is_empty());
}

proptest! {
    #[test]
    fn concatenation_has_an_identity_and_is_associative(a in proptest::collection::vec(0u8..10, 0..10), b in proptest::collection::vec(0u8..10, 0..10), c in proptest::collection::vec(0u8..10, 0..10)) {
        let (a,b,c) = (Collection::new(a),Collection::new(b),Collection::new(c));
        prop_assert_eq!(a.combine(&b).combine(&c), a.combine(&b.combine(&c)));
        prop_assert_eq!(&a.combine(&Collection::empty()), &a);
        prop_assert_eq!(&Collection::empty().combine(&a), &a);
        prop_assert!(a.divide(&a).is_empty());
        prop_assert_eq!(a.map(|x| *x), a);
    }
}

#[test]
fn non_empty_concatenation_preserves_the_first_element() {
    let a = NonEmptyCollection::new(1, vec![]);
    let b = NonEmptyCollection::new(2, vec![3]);
    assert!(!a.is_empty());
    assert_eq!(a.len(), 1);
    assert_eq!(a.combine(&b), NonEmptyCollection::new(1, vec![2, 3]));
    assert_eq!(a.combine(&b).combine(&a), a.combine(&b.combine(&a)));
    assert_eq!(a.filter(|_| true), Collection::new(vec![1]));
    assert_eq!(a.divide(&Collection::empty()), Collection::new(vec![1]));
    assert_eq!(Collection::<i32>::default(), Collection::empty());
}
