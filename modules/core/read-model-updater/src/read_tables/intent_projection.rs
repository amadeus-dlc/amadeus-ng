//! `read_intent` の行を組む投影 — intent 集約を [`IntentRow`] へ写す。

use chrono::SecondsFormat;
use core_command_domain::orchestration::Intent;

use crate::orchestration::IntentRow;

/// intent 集約を 1 行へ写す。
pub(super) fn row(intent: &Intent) -> IntentRow {
    let scan = intent.scan();
    let first = intent.first_post_initialization();
    IntentRow::new(
        intent.id().as_str().to_string(),
        intent.in_scope_count(),
        first.as_ref().map(|entry| entry.slug().to_string()),
        first
            .as_ref()
            .map(|entry| entry.phase().as_str().to_uppercase()),
        intent.definition_id().as_str().to_string(),
        intent.definition_revision().as_str().to_string(),
        intent.scope().to_string(),
        intent.request().to_string(),
        intent.depth().map(str::to_string),
        intent.test_strategy().map(str::to_string),
        intent.review().map(str::to_string),
        intent
            .created_at()
            .to_rfc3339_opts(SecondsFormat::Secs, true),
        scan.project_type().to_string(),
        scan.project_kind().as_str().to_string(),
        scan.languages().to_string(),
        scan.frameworks().to_string(),
        scan.build_system().to_string(),
    )
}
