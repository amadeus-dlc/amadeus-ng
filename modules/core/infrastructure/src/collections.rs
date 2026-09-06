//! コレクションの操作契約と、空を許す列・非空列。内部のイテレータを公開せず操作で扱う。

mod collection;
mod first_class_collection;
mod non_empty_collection;

pub use collection::Collection;
pub use first_class_collection::FirstClassCollection;
pub use non_empty_collection::NonEmptyCollection;
