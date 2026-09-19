//! Claude Code ハーネス — マニフェストデータとフックアダプタシム。エンジンはハーネス中立 (core) であり、ここにはデータと薄いシムのみを置く。

#![forbid(unsafe_code)]
mod task_update_envelope;
pub use task_update_envelope::TaskUpdateEnvelope;

mod human_turn_envelope;
pub use human_turn_envelope::HumanTurnEnvelope;

mod state_transition_guard;
pub use state_transition_guard::StateTransitionGuard;

mod write_tool_envelope;
pub use write_tool_envelope::WriteToolEnvelope;

mod rule_file;
pub use rule_file::RuleFile;

mod stage_rule_bundle;
pub use stage_rule_bundle::StageRuleBundle;

mod dispatch_rules_envelope;
pub use dispatch_rules_envelope::DispatchRulesEnvelope;

mod reviewer_dispatch_record;
mod reviewer_scope_envelope;
pub use reviewer_dispatch_record::parse_reviewer_dispatch;
pub use reviewer_scope_envelope::ReviewerScopeEnvelope;

mod delegated_lifecycle;

mod stop_transcript;
pub use stop_transcript::StopTranscript;
mod engine_tool_call;

mod stop_resume_wait;
pub use stop_resume_wait::StopResumeWait;

mod subagent_stop_envelope;
pub use subagent_stop_envelope::SubagentStopEnvelope;
mod subagent_envelope_error;
pub use subagent_envelope_error::SubagentEnvelopeError;

mod session_workflow_fields;
pub use session_workflow_fields::SessionWorkflowFields;
mod runtime_compile_envelope;
mod session_context_notices;
pub use runtime_compile_envelope::RuntimeCompileEnvelope;
pub use session_context_notices::SessionContextNotices;
mod session_command_spellings;
pub use session_command_spellings::SessionCommandSpellings;
mod session_start_context;
pub use session_start_context::SessionStartContext;

mod session_end_envelope;
pub use session_end_envelope::SessionEndEnvelope;

mod session_start_envelope;
pub use session_start_envelope::SessionStartEnvelope;

mod fold_mode;
pub use fold_mode::FoldMode;
mod fold_usage_envelope;
mod lifecycle_boundary_command;
pub use fold_usage_envelope::FoldUsageEnvelope;
mod transcript_token_counts;
pub use transcript_token_counts::TranscriptTokenCounts;
mod transcript_usage_row;
pub use transcript_usage_row::TranscriptUsageRow;
