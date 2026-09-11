//! 再開の選択を待つ共有markerの読取り。

use crate::engine_tool_call::js_string;
use core_infrastructure::{ecmascript::trim, hash::sha256_hex};
use serde_json::{Map, Value};

/// 構造が正しい共有の再開待ち記録。ファイルや実行方針は扱わない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StopResumeWait {
    state_hash: Option<String>,
}

impl StopResumeWait {
    const fn new(state_hash: Option<String>) -> Self {
        Self { state_hash }
    }

    /// markerの構造と共有の回答待ち条件を検査する。
    #[must_use]
    pub fn parse(marker: &str) -> Self {
        Self::new(waiting_state_hash(marker))
    }

    /// 現在の状態本文から得たSHA-256と一致する回答待ちか。
    #[must_use]
    pub fn matches_state_hash(&self, state_hash: &str) -> bool {
        self.state_hash.as_deref() == Some(state_hash)
    }
}

fn waiting_state_hash(marker: &str) -> Option<String> {
    let value: Value = serde_json::from_str(marker).ok()?;
    let object = value.as_object()?;
    if object
        .get("version")
        .and_then(Value::as_f64)
        .map(f64::to_bits)
        != Some(2.0_f64.to_bits())
        || text(object, "kind") != Some("ask")
        || !text(object, "owner_session").is_some_and(|owner| owner.starts_with("sessionless:"))
    {
        return None;
    }
    let stage = trim(text(object, "stage")?);
    let state_hash = text(object, "state_sha256")?;
    if !stage.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
        || !stage
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        || !hex(state_hash, 64)
        || !optional(object, "unit", |unit| {
            unit.as_str().is_some_and(|name| !trim(name).is_empty())
        })
        || !hash_value(object.get("project_sha256"))
        || !nullable_string(object.get("intent_uuid"))
        || !object.get("state_present").is_some_and(Value::is_boolean)
        || !object.get("needs_rehydrate").is_some_and(Value::is_boolean)
        || ![
            "revision",
            "owner_epoch",
            "context_epoch",
            "event_sequence",
            "human_sequence",
            "engine_sequence",
            "conversation_sequence",
            "stop_count",
        ]
        .iter()
        .all(|field| integer(object.get(*field)))
        || !["issued", "delivered", "consumed", "superseded"]
            .contains(&js_string(object.get("delivery")).as_str())
        || !optional(object, "code_generation_source_sha256", |source| {
            let source = js_string(Some(source));
            source == "unbindable" || hex(&source, 40) || hex(&source, 64)
        })
        || !optional(object, "code_generation_authority_revision", |value| {
            integer(Some(value))
        })
        || !optional(object, "units", |units| {
            units.as_array().is_some_and(|units| {
                !units.is_empty()
                    && units.iter().all(|unit| {
                        unit.as_str().is_some_and(|name| {
                            let name = trim(name);
                            name.len() <= 64 && identifier(name)
                        })
                    })
            })
        })
        || !optional(object, "cursor_harness", |value| {
            value.as_str().is_some_and(identifier)
        })
    {
        return None;
    }
    let attempt = object.get("active_attempt")?.as_object()?;
    if !valid_attempt(attempt) {
        return None;
    }
    let resume = object.get("resume")?.as_object()?;
    if text(resume, "status") != Some("waiting")
        || text(resume, "issuing_stage").is_none()
        || !hash_value(resume.get("issuing_state_sha256"))
        || text(resume, "issuing_session").is_none()
        || !nullable_string(resume.get("issuing_intent_uuid"))
    {
        return None;
    }
    if let Some(token) = object.get("continue_token") {
        let token = token.as_str()?;
        if token.len() > 16 * 1024
            || text(object, "continue_token_sha256") != Some(sha256_hex(token.as_bytes()).as_str())
        {
            return None;
        }
    }
    Some(state_hash.to_string())
}

fn valid_attempt(attempt: &Map<String, Value>) -> bool {
    optional(attempt, "id", Value::is_string)
        && ["next", "continue", "report", "park"]
            .contains(&js_string(attempt.get("command_kind")).as_str())
        && hash_value(attempt.get("command_sha256"))
        && hash_value(attempt.get("issued_state_sha256"))
        && text(attempt, "session_id").is_some()
        && integer(attempt.get("owner_epoch"))
        && integer(attempt.get("context_epoch"))
        && ["pending", "settled", "failed"].contains(&js_string(attempt.get("status")).as_str())
        && ["claim_revision", "result_revision", "resume_gate_revision"]
            .iter()
            .all(|field| optional(attempt, field, |value| integer(Some(value))))
        && optional(attempt, "shared_attempt", Value::is_boolean)
        && ["cursor_input_sha256", "result_sha256"]
            .iter()
            .all(|field| optional(attempt, field, |value| hash_value(Some(value))))
}

fn text<'a>(object: &'a Map<String, Value>, field: &str) -> Option<&'a str> {
    object.get(field).and_then(Value::as_str)
}

fn optional(object: &Map<String, Value>, field: &str, valid: impl FnOnce(&Value) -> bool) -> bool {
    object.get(field).is_none_or(valid)
}

const fn nullable_string(value: Option<&Value>) -> bool {
    matches!(value, Some(Value::Null | Value::String(_)))
}

fn integer(value: Option<&Value>) -> bool {
    value
        .and_then(Value::as_f64)
        .is_some_and(|number| number.is_finite() && number >= 0.0 && number.fract() == 0.0)
}

fn hex(text: &str, length: usize) -> bool {
    text.len() == length
        && text
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn hash_value(value: Option<&Value>) -> bool {
    hex(&js_string(value), 64)
}

fn identifier(text: &str) -> bool {
    text.as_bytes()
        .first()
        .is_some_and(u8::is_ascii_alphanumeric)
        && text
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
}
