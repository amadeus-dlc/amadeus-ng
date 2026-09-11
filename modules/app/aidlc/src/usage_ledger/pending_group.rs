//! 保留した群と、その帰属。
//!
//! 締めない畳み込みは最後の群を保留する。保留した時点の所属（ステージ・session・作業）を
//! ここで捕まえておき、後の畳み込みで工程が進んでいても当時の所属へ算入する。
use super::fold_attribution::FoldAttribution;
use core_infrastructure::canon_json::{JsonValue, Number, ObjectMembers};

/// 保留中の群。
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PendingGroup {
    byte_offset: u64,
    message_id: String,
    attribution: FoldAttribution,
}

impl PendingGroup {
    /// 保留位置と当時の所属から組む。
    pub(crate) const fn new(
        byte_offset: u64,
        message_id: String,
        attribution: FoldAttribution,
    ) -> Self {
        Self {
            byte_offset,
            message_id,
            attribution,
        }
    }

    /// 群の先頭バイト位置。
    pub(crate) const fn byte_offset(&self) -> u64 {
        self.byte_offset
    }

    /// 保留時の帰属。
    pub(crate) const fn attribution(&self) -> &FoldAttribution {
        &self.attribution
    }

    /// JSON の 5 欄。
    pub(crate) fn to_json(&self) -> JsonValue {
        let mut fields = ObjectMembers::new();
        fields.insert(
            "byteOffset",
            JsonValue::Number(Number::Float(self.byte_offset as f64)),
        );
        fields.insert("messageId", JsonValue::String(self.message_id.clone()));
        fields.insert(
            "stageSlug",
            self.attribution
                .stage_slug()
                .map_or(JsonValue::Null, |slug| JsonValue::String(slug.to_string())),
        );
        fields.insert(
            "sessionKey",
            JsonValue::String(self.attribution.session_key().to_string()),
        );
        fields.insert(
            "workflowKey",
            JsonValue::String(self.attribution.workflow_key().to_string()),
        );
        JsonValue::Object(fields)
    }

    /// JSON から読む。形が違えば保留なしとする。
    pub(crate) fn of_json(value: Option<&JsonValue>) -> Option<Self> {
        let JsonValue::Object(members) = value? else {
            return None;
        };
        Some(Self::new(
            super::ledger_cursor::byte_offset_of(members.get("byteOffset"))?,
            string(members.get("messageId")),
            FoldAttribution::new(
                members.get("stageSlug").and_then(|value| match value {
                    JsonValue::String(text) => Some(text.clone()),
                    _ => None,
                }),
                string(members.get("sessionKey")),
                string(members.get("workflowKey")),
            ),
        ))
    }
}

fn string(value: Option<&JsonValue>) -> String {
    match value {
        Some(JsonValue::String(text)) => text.clone(),
        _ => String::new(),
    }
}
