//! メモリ上の JSON 値 (挿入順保持・不変) と型付き struct からの変換点。
//!
//! 型ファイルの mod は private。公開 API は下の `pub use` が唯一の宣言である
//! (`coding-rules/module-visibility.md`)。
//!
//! `canon_json` の兄弟ファイル（`digest.rs` / `parse.rs` / `writer.rs` / `canonical.rs`）は
//! `crate::canon_json::value::{JsonValue, Number, ObjectMembers, arbitrary, ..}` を直接参照
//! する。ここを 1 型 1 ファイルへ分割してもその参照が壊れないよう、本ディレクトリはファサード
//! として従来どおり `canon_json::value` という 1 モジュールに見える形を保つ。

mod json_value;
mod number;
mod object_members;
mod to_value_error;

pub use json_value::{JsonValue, to_value};
pub use number::Number;
pub use object_members::ObjectMembers;
pub use to_value_error::ToValueError;

// PBT 生成器 (テスト専用) — 兄弟ファイルのプロパティテストから
// `crate::canon_json::value::arbitrary::..` で参照される。
#[cfg(test)]
pub(crate) mod arbitrary;
