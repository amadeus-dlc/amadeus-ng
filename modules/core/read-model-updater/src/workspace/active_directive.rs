//! Claude用の指示発行記録を、本家2.7.1の公開表現へ写す。
use core_command_domain::orchestration::{ActiveDirective, PublishedDirective};
use core_infrastructure::canon_json::{
    JsonValue, Number, ObjectMembers, SerializationProfile, serialize,
};
fn text(value: &str) -> JsonValue {
    JsonValue::String(value.to_string())
}
const fn number(value: u64) -> JsonValue {
    JsonValue::Number(Number::PosInt(value))
}
pub(crate) fn render(directive: &ActiveDirective) -> String {
    let owner = directive.owner_session();
    let mut marker = ObjectMembers::new();
    marker.insert("version", number(2));
    marker.insert("revision", number(directive.revision()));
    marker.insert("project_sha256", text(directive.project_sha256()));
    marker.insert("intent_uuid", text(directive.intent_id().as_str()));
    marker.insert("state_present", JsonValue::Bool(true));
    marker.insert("state_sha256", text(directive.state_sha256()));
    marker.insert("cursor_harness", text("claude"));
    marker.insert("owner_session", text(owner));
    marker.insert("owner_epoch", number(directive.owner_epoch()));
    marker.insert("context_epoch", number(directive.context_epoch()));
    marker.insert(
        "kind",
        text(match directive.directive() {
            PublishedDirective::Error { .. } => "error",
            PublishedDirective::RunStage { .. } => "run-stage",
            PublishedDirective::LoadSteering { .. } => "load-steering",
        }),
    );
    marker.insert("stage", text(directive.directive().stage().as_str()));
    if let Some(floor) = directive.source_floor() {
        marker.insert("code_generation_source_sha256", text(floor));
        marker.insert(
            "code_generation_authority_revision",
            number(directive.issuance_revision()),
        );
    }
    match directive.directive() {
        PublishedDirective::Error { .. } => {}
        PublishedDirective::RunStage { unit, .. } => {
            if let Some(unit) = unit {
                marker.insert("unit", text(unit));
            }
        }
        PublishedDirective::LoadSteering {
            part, parts, token, ..
        } => {
            marker.insert("part", number(u64::from(*part)));
            marker.insert("parts", number(u64::from(*parts)));
            marker.insert("continue_token", text(token));
            marker.insert(
                "continue_token_sha256",
                text(&core_infrastructure::hash::sha256_hex(token.as_bytes())),
            );
        }
    }
    let invalidated = matches!(directive.directive(), PublishedDirective::Error { .. });
    marker.insert(
        "delivery",
        text(if invalidated { "superseded" } else { "issued" }),
    );
    marker.insert("needs_rehydrate", JsonValue::Bool(invalidated));
    let mut attempt = ObjectMembers::new();
    attempt.insert("id", text("sessionless"));
    attempt.insert("command_kind", text("next"));
    attempt.insert("command_sha256", text(directive.initial_state_sha256()));
    attempt.insert(
        "issued_state_sha256",
        text(directive.initial_state_sha256()),
    );
    attempt.insert("session_id", text(owner));
    attempt.insert("owner_epoch", number(0));
    attempt.insert("context_epoch", number(0));
    attempt.insert("status", text("settled"));
    marker.insert("active_attempt", JsonValue::Object(attempt));
    for name in [
        "event_sequence",
        "human_sequence",
        "engine_sequence",
        "conversation_sequence",
        "stop_count",
    ] {
        marker.insert(name, number(0));
    }
    serialize(
        &JsonValue::Object(marker),
        SerializationProfile::ContractPretty,
    )
}
