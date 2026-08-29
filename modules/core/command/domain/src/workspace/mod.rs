//! workspace コンテキスト (11-workspace.md) — 永続化機構の Domain Primitive と純関数サービス。
//! upstream 契約の逐語根拠は docs/specs/research/workspace-*.md。
//!
//! **描画はここに無い** (11-workspace §2.3)。状態ファイル・監査ブロックを**描く**純関数
//! (`state_writers` / `render_audit_block`) は ES 化により投影の責務へ移った — 描くのは
//! ReadModelUpdater (`core-read-model-updater` の `workspace` 投影 API) であって、
//! ドメイン層ではない (ADR-003 / ADR-004)。ここに残るのは値オブジェクトの Always Valid 検証と、
//! 集約に置けない横断の判断 (`classify_state_version`) である。
//!
//! 型ファイルの mod は private。公開 API は以下の `pub use` が唯一の宣言であり、
//! 消費側のパスは `core_command_domain::workspace::<型>` で安定する
//! (aidlc/spaces/default/knowledge/aidlc-shared/coding-rules/module-visibility.md)。

mod audit_events;
mod audit_field;
mod audit_ordering;
mod bolt_refs;
mod checkbox;
mod clone_id;
mod intent_dir_name;
mod shard_name;
mod space_name;
mod state_field_value;
mod state_version;
mod store_path;

// Domain Primitive
pub use audit_events::{EventCategory, EventType};
pub use audit_field::{AuditFieldKey, AuditFieldValue, AuditFields};
pub use audit_ordering::{AuditEventRecord, OrderedAuditEvents};
pub use bolt_refs::BoltRefs;
pub use checkbox::{CheckboxEntry, CheckboxState};
pub use clone_id::CloneId;
pub use intent_dir_name::IntentDirName;
pub use shard_name::ShardName;
pub use space_name::SpaceName;
pub use state_field_value::StateFieldValue;
pub use state_version::{StateVersionClassification, StateVersionKind};
pub use store_path::StorePath;

// 純関数ドメインサービス
pub use audit_ordering::find_all_events;
pub use checkbox::{count_completed, parse_checkboxes, with_checkbox_marker, with_checkbox_suffix};
pub use state_field_value::unsafe_line_char;
pub use state_version::classify_state_version;

// エラー
pub use audit_field::AuditFieldKeyError;
pub use bolt_refs::BoltRefsError;
pub use checkbox::CheckboxUpdateError;
pub use clone_id::CloneIdError;
pub use intent_dir_name::IntentDirNameError;
pub use space_name::SpaceNameError;
pub use state_field_value::UnsafeLineChar;

// 逐語定数
pub use bolt_refs::EMPTY_LIST_LITERAL;
pub use state_version::CURRENT_STATE_VERSION;
