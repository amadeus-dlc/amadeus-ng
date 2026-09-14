//! 自己診断の観測 DTO 群 — `DoctorObservationDao` が返す View の部品。
//!
//! 1 ファイル 1 公開型。mod は private で、公開は `port` → `orchestration` の
//! ファサード連鎖が唯一の宣言 (`coding-rules/module-visibility.md`)。

mod consume_view;
mod definition_assets_view;
mod doctor_check;
mod doctor_observation_view;
mod doctor_report;
mod doctor_summary_view;
mod execution_cursor_view;
mod graph_stage_view;
mod heartbeat_entry_view;
mod heartbeat_view;
mod hook_binding_declaration;
mod hook_binding_target;
mod hook_binding_view;
mod hook_wiring_view;
mod native_entry_points_view;
mod observation_failure;
mod projection_observation_view;
mod record_location_view;
mod record_observation_view;
mod scope_grid_entry_view;
mod stage_artifacts_view;
mod stage_file_view;
mod state_file_observation_view;
mod state_version_kind_view;
mod state_version_view;
mod store_observation_view;
mod store_schema_view;
mod timestamp_view;
mod wired_hook_view;
mod workspace_shell_view;

pub use consume_view::ConsumeView;
pub use definition_assets_view::DefinitionAssetsView;
pub use doctor_check::DoctorCheck;
pub use doctor_observation_view::DoctorObservationView;
pub use doctor_report::DoctorReport;
pub use doctor_summary_view::DoctorSummaryView;
pub use execution_cursor_view::ExecutionCursorView;
pub use graph_stage_view::GraphStageView;
pub use heartbeat_entry_view::HeartbeatEntryView;
pub use heartbeat_view::HeartbeatView;
pub use hook_binding_declaration::HookBindingDeclaration;
pub use hook_binding_target::HookBindingTarget;
pub use hook_binding_view::HookBindingView;
pub use hook_wiring_view::HookWiringView;
pub use native_entry_points_view::NativeEntryPointsView;
pub use observation_failure::ObservationFailure;
pub use projection_observation_view::ProjectionObservationView;
pub use record_location_view::RecordLocationView;
pub use record_observation_view::RecordObservationView;
pub use scope_grid_entry_view::ScopeGridEntryView;
pub use stage_artifacts_view::StageArtifactsView;
pub use stage_file_view::StageFileView;
pub use state_file_observation_view::StateFileObservationView;
pub use state_version_kind_view::StateVersionKindView;
pub use state_version_view::StateVersionView;
pub use store_observation_view::StoreObservationView;
pub use store_schema_view::StoreSchemaView;
pub use timestamp_view::TimestampView;
pub use wired_hook_view::WiredHookView;
pub use workspace_shell_view::WorkspaceShellView;
