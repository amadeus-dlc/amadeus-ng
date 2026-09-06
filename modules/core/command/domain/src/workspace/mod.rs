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

mod audit_event_record;
mod audit_events;
mod audit_field_key;
mod audit_field_key_error;
mod audit_field_value;
mod audit_fields;
mod bolt_refs;
mod bolt_refs_error;
mod checkbox_entry;
mod checkbox_state;
mod checkbox_update_error;
mod checkboxes;
mod clone_id;
mod clone_id_error;
mod heading_not_found;
mod human_turns;
mod intent_dir_name;
mod intent_dir_name_error;
mod markdown_sections;
mod ordered_audit_events;
mod practices_promotion;
mod promoted_section;
mod promoted_sections;
mod promoted_sections_error;
mod promotion_plan_error;
mod rule_lines;
mod shard_name;
mod space_name;
mod space_name_error;
mod state_field_value;
mod state_version_classification;
mod state_version_kind;
mod store_path;
mod unsafe_line_char;

// Domain Primitive
pub use audit_event_record::AuditEventRecord;
pub use audit_events::{EventCategory, EventType};
pub use audit_field_key::AuditFieldKey;
pub use audit_field_value::AuditFieldValue;
pub use audit_fields::AuditFields;
pub use bolt_refs::BoltRefs;
pub use clone_id::CloneId;
pub use human_turns::HumanTurns;
pub use intent_dir_name::IntentDirName;
pub use ordered_audit_events::OrderedAuditEvents;
pub use practices_promotion::PracticesPromotion;
pub use promoted_section::PromotedSection;
pub use promoted_sections::PromotedSections;
pub use rule_lines::RuleLines;
pub use shard_name::ShardName;
pub use space_name::SpaceName;
pub use state_field_value::StateFieldValue;
pub use state_version_classification::StateVersionClassification;
pub use state_version_kind::StateVersionKind;
pub use store_path::StorePath;

// 純関数ドメインサービス
pub use checkbox_entry::CheckboxEntry;
pub use checkbox_state::CheckboxState;
pub use checkbox_update_error::CheckboxUpdateError;
pub use checkboxes::Checkboxes;
pub use markdown_sections::{append_under_heading, extract_section, replace_section};

// エラー
pub use audit_field_key_error::AuditFieldKeyError;
pub use bolt_refs_error::BoltRefsError;
pub use clone_id_error::CloneIdError;
pub use heading_not_found::HeadingNotFound;
pub use intent_dir_name_error::IntentDirNameError;
pub use promoted_sections_error::PromotedSectionsError;
pub use promotion_plan_error::PromotionPlanError;
pub use space_name_error::SpaceNameError;
pub use unsafe_line_char::UnsafeLineChar;

// 逐語定数
pub use bolt_refs::EMPTY_LIST_LITERAL;
pub use state_version_classification::CURRENT_STATE_VERSION;
