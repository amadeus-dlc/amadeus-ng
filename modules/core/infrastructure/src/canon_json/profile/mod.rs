//! 直列化プロファイル 3 値の閉集合 (ADR 0001 決定 2)。
//!
//! 型ファイルの mod は private。公開 API は下の `pub use` が唯一の宣言である
//! (`coding-rules/module-visibility.md`)。
//!
//! `canon_json` の兄弟ファイル（`parse.rs` / `writer.rs` / `canonical.rs`）は
//! `crate::canon_json::profile::{SerializationProfile, Indent, KeyOrder}` を直接参照する。
//! ここを 1 型 1 ファイルへ分割してもその参照が壊れないよう、本ディレクトリはファサードとして
//! 従来どおり `canon_json::profile` という 1 モジュールに見える形を保つ。

mod indent;
mod key_order;
mod serialization_profile;

pub use indent::Indent;
pub use key_order::KeyOrder;
pub use serialization_profile::SerializationProfile;
