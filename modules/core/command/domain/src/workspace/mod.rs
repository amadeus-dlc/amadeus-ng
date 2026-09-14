//! workspace コンテキスト — 永続化機構の Domain Primitive と純関数サービス。
//! 現行受入の基準は固定本家2.7.1（`tests/golden/upstream-a277af21/`）。
//! 旧ピン3c3146cfを引用する個別コメントは、当時の実装根拠として区別する。
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
pub use markdown_sections::{
    append_under_heading, ensure_heading, extract_section, replace_section,
};

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

mod hook_health_target;
pub use hook_health_target::HookHealthTarget;

mod hook_name;
pub use hook_name::HookName;
mod hook_health_error;
pub use hook_health_error::HookHealthError;

mod hook_health_id;
pub use hook_health_id::HookHealthId;

mod hook_health;
pub use hook_health::HookHealth;
mod hook_health_event;
pub use hook_health_event::{
    HookAuditDropped, HookHealthEvent, HookHealthStarted, HookHeartbeatObserved,
};
mod hook_health_event_id;
pub use hook_health_event_id::HookHealthEventId;

mod hook_drop_reason;
pub use hook_drop_reason::HookDropReason;

mod hook_drop_summary;
pub use hook_drop_summary::HookDropSummary;

mod artifact_write_observation;
pub use artifact_write_observation::ArtifactWriteObservation;
mod artifact_audit_id;
pub use artifact_audit_id::ArtifactAuditId;
mod artifact_audit_event_id;
pub use artifact_audit_event_id::ArtifactAuditEventId;
mod artifact_audit_event;
pub use artifact_audit_event::ArtifactSaved;
mod artifact_audit_event_family;
pub use artifact_audit_event_family::ArtifactAuditEvent;
mod artifact_audit;
pub use artifact_audit::ArtifactAudit;

mod artifact_audit_record;
pub use artifact_audit_record::ArtifactAuditRecord;

mod session_audit;
pub use session_audit::SessionAudit;

mod session_audit_error;
pub use session_audit_error::SessionAuditError;

mod session_audit_id;
pub use session_audit_id::SessionAuditId;

mod session_audit_event_id;
pub use session_audit_event_id::SessionAuditEventId;

mod session_audit_event;
pub use session_audit_event::SessionAuditEvent;

mod session_audit_record;
pub use session_audit_record::SessionAuditRecord;

mod session_audit_observation;
pub use session_audit_observation::SessionAuditObservation;

mod session_audit_observation_id;
pub use session_audit_observation_id::SessionAuditObservationId;

pub use hook_health_event::HookFirstDropObserved;

// 自己診断 (`aidlc --doctor`) — 観測の値オブジェクト群と、それを評価する集約 (U3 / C5・C7)。
// 観測はファイル・環境・ストアの事実の写しであり、判断はすべて集約 `WorkspaceDoctor` の側にある
// (オーナー裁定 2026-09-12)。
mod consumed_artifact;
mod definition_assets;
mod doctor_check;
mod doctor_check_id;
mod doctor_checks;
mod doctor_observation;
mod execution_cursor_observation;
mod graph_stage;
mod heartbeat_entry;
mod heartbeat_observation;
mod hook_binding;
mod hook_binding_declaration;
mod hook_binding_target;
mod hook_wiring;
mod native_entry_points;
mod observation_failure;
mod observed_timestamp;
mod projection_observation;
mod record_location;
mod record_observation;
mod scope_grid_entry;
mod stage_artifacts;
mod stage_file;
mod stage_frontmatter;
mod state_file_observation;
mod state_version_observation;
mod store_observation;
mod store_schema;
mod wired_hook;
mod workspace_doctor;
mod workspace_doctor_error;
mod workspace_doctor_event;
mod workspace_doctor_event_id;
mod workspace_doctor_id;
mod workspace_shell;

pub use consumed_artifact::ConsumedArtifact;
pub use definition_assets::DefinitionAssets;
pub use doctor_check::DoctorCheck;
pub use doctor_check_id::DoctorCheckId;
pub use doctor_checks::DoctorChecks;
pub use doctor_observation::DoctorObservation;
pub use execution_cursor_observation::ExecutionCursorObservation;
pub use graph_stage::GraphStage;
pub use heartbeat_entry::HeartbeatEntry;
pub use heartbeat_observation::HeartbeatObservation;
pub use hook_binding::HookBinding;
pub use hook_binding_declaration::HookBindingDeclaration;
pub use hook_binding_target::HookBindingTarget;
pub use hook_wiring::HookWiring;
pub use native_entry_points::NativeEntryPoints;
pub use observation_failure::ObservationFailure;
pub use observed_timestamp::ObservedTimestamp;
pub use projection_observation::ProjectionObservation;
pub use record_location::RecordLocation;
pub use record_observation::RecordObservation;
pub use scope_grid_entry::ScopeGridEntry;
pub use stage_artifacts::StageArtifacts;
pub use stage_file::StageFile;
pub use stage_frontmatter::StageFrontmatter;
pub use state_file_observation::StateFileObservation;
pub use state_version_observation::StateVersionObservation;
pub use store_observation::StoreObservation;
pub use store_schema::StoreSchema;
pub use wired_hook::WiredHook;
pub use workspace_doctor::WorkspaceDoctor;
pub use workspace_doctor_error::WorkspaceDoctorError;
pub use workspace_doctor_event::WorkspaceDoctorEvent;
pub use workspace_doctor_event_id::WorkspaceDoctorEventId;
pub use workspace_doctor_id::WorkspaceDoctorId;
pub use workspace_shell::WorkspaceShell;

// codekb (リポジトリごとの durable な知識ストア) — 群 D の compare-and-swap が突き合わせる値。
// 世代と源の指紋は「呼び手が渡した合言葉」と「実測した値」の両方を同じ型で運ぶ (upstream は
// 合言葉の綴りを検査せず突き合わせるだけなので、型が形を検査すると観測差が出る)。
mod codekb_generation;
mod codekb_repo_id;
mod codekb_repo_id_error;
mod codekb_source_fingerprint;
mod codekb_token_error;
pub use codekb_generation::CodekbGeneration;
pub use codekb_repo_id::CodekbRepoId;
pub use codekb_repo_id_error::CodekbRepoIdError;
pub use codekb_source_fingerprint::CodekbSourceFingerprint;
pub use codekb_token_error::CodekbTokenError;

// 公開候補の材料 — 9 成果物ちょうどの集合と、走査範囲の主張・被覆の判断。
mod codekb_artifact;
mod codekb_artifact_name;
mod codekb_artifact_name_error;
mod codekb_artifacts;
mod codekb_artifacts_error;
mod codekb_candidate;
mod codekb_scope_path;
mod codekb_scope_path_error;
mod codekb_scope_paths;
pub use codekb_artifact::CodekbArtifact;
pub use codekb_artifact_name::CodekbArtifactName;
pub use codekb_artifact_name_error::CodekbArtifactNameError;
pub use codekb_artifacts::CodekbArtifacts;
pub use codekb_artifacts_error::CodekbArtifactsError;
pub use codekb_candidate::CodekbCandidate;
pub use codekb_scope_path::CodekbScopePath;
pub use codekb_scope_path_error::CodekbScopePathError;
pub use codekb_scope_paths::CodekbScopePaths;

// 集約 Codekb — 公開の compare-and-swap を判断し、公開の事実を 1 件返す。
mod codekb;
mod codekb_event;
mod codekb_event_id;
mod codekb_event_id_error;
mod codekb_publish_refusal;
mod codekb_snapshot;
pub use codekb::Codekb;
pub use codekb_event::{CodekbEvent, CodekbInterruptedPublicationSettled, CodekbPublished};
pub use codekb_event_id::CodekbEventId;
pub use codekb_event_id_error::CodekbEventIdError;
pub use codekb_publish_refusal::CodekbPublishRefusal;
pub use codekb_snapshot::CodekbSnapshot;
