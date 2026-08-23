//! workspace コンテキスト (11-workspace.md) — 永続化機構の Domain Primitive と純関数サービス。
//! upstream 契約の逐語根拠は docs/specs/research/workspace-*.md。
//!
//! 型ファイルの mod は private。公開 API は以下の `pub use` が唯一の宣言であり、
//! 消費側のパスは `core_domain::workspace::<型>` で安定する
//! (aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/module-visibility.md)。

mod bolt_refs;
mod checkbox;
mod clone_id;
mod intent_dir_name;
mod shard_name;
mod space_name;
mod state_field_value;
mod state_version;
mod state_writers;

// Domain Primitive
pub use bolt_refs::BoltRefs;
pub use checkbox::{CheckboxEntry, CheckboxState};
pub use clone_id::CloneId;
pub use intent_dir_name::IntentDirName;
pub use shard_name::ShardName;
pub use space_name::SpaceName;
pub use state_field_value::StateFieldValue;
pub use state_version::{StateVersionClassification, StateVersionKind};

// 純関数ドメインサービス
pub use checkbox::{count_completed, parse_checkboxes, set_checkbox};
pub use state_field_value::unsafe_line_char;
pub use state_version::classify_state_version;
pub use state_writers::{
    get_field, remove_field, set_field, set_field_strict, set_or_insert_field,
};

// エラー
pub use bolt_refs::BoltRefsError;
pub use checkbox::CheckboxWriteError;
pub use clone_id::CloneIdError;
pub use intent_dir_name::IntentDirNameError;
pub use space_name::SpaceNameError;
pub use state_field_value::UnsafeLineChar;
pub use state_writers::{FieldNotFound, HeadingNotFound};

// 逐語定数
pub use bolt_refs::EMPTY_LIST_LITERAL;
pub use state_version::CURRENT_STATE_VERSION;
