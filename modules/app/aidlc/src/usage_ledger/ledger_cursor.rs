//! 1 つの会話履歴ファイルについて、どこまで畳んだかの覚え。
//!
//! 鍵はファイルパスである。台帳は session を跨いで積み上がるので、`"main"` のような
//! 定数を鍵にすると別 session の長いファイルへ他 session の位置を当ててしまう。
use super::pending_group::PendingGroup;
use core_infrastructure::canon_json::{JsonValue, Number, ObjectMembers};

/// 1 ファイル分の畳み込み位置。
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct LedgerCursor {
    last_uuid: String,
    last_timestamp: String,
    last_message_id: String,
    byte_offset: u64,
    pending: Option<PendingGroup>,
}

impl LedgerCursor {
    /// 5 欄から組む。
    pub(crate) const fn new(
        last_uuid: String,
        last_timestamp: String,
        last_message_id: String,
        byte_offset: u64,
        pending: Option<PendingGroup>,
    ) -> Self {
        Self {
            last_uuid,
            last_timestamp,
            last_message_id,
            byte_offset,
            pending,
        }
    }

    /// 既に畳んだバイト位置。
    pub(crate) const fn byte_offset(&self) -> u64 {
        self.byte_offset
    }

    /// 直近に畳んだ行の識別子。
    pub(crate) fn last_uuid(&self) -> &str {
        &self.last_uuid
    }

    /// 直近に畳んだ行の時刻。
    pub(crate) fn last_timestamp(&self) -> &str {
        &self.last_timestamp
    }

    /// 直近に畳んだ群の `message.id`（診断用）。
    pub(crate) fn last_message_id(&self) -> &str {
        &self.last_message_id
    }

    /// 保留中の群。
    pub(crate) const fn pending(&self) -> Option<&PendingGroup> {
        self.pending.as_ref()
    }

    /// JSON の 5 欄。`pending` は無ければ書かない（`undefined` と同じ扱い）。
    pub(crate) fn to_json(&self) -> JsonValue {
        let mut fields = ObjectMembers::new();
        fields.insert("lastUuid", JsonValue::String(self.last_uuid.clone()));
        fields.insert(
            "lastTimestamp",
            JsonValue::String(self.last_timestamp.clone()),
        );
        fields.insert(
            "lastMessageId",
            JsonValue::String(self.last_message_id.clone()),
        );
        fields.insert(
            "byteOffset",
            JsonValue::Number(Number::Float(self.byte_offset as f64)),
        );
        if let Some(pending) = &self.pending {
            fields.insert("pending", pending.to_json());
        }
        JsonValue::Object(fields)
    }

    /// JSON から読む。`byteOffset` が数値でない cursor は読めない（旧形の合図）。
    pub(crate) fn of_json(value: &JsonValue) -> Option<Self> {
        let JsonValue::Object(members) = value else {
            return None;
        };
        Some(Self::new(
            string(members.get("lastUuid")),
            string(members.get("lastTimestamp")),
            string(members.get("lastMessageId")),
            byte_offset_of(members.get("byteOffset"))?,
            PendingGroup::of_json(members.get("pending")),
        ))
    }
}

/// 数値の欄をバイト位置として読む。
pub(super) fn byte_offset_of(value: Option<&JsonValue>) -> Option<u64> {
    match value? {
        JsonValue::Number(Number::PosInt(number)) => Some(*number),
        JsonValue::Number(Number::NegInt(_)) => Some(0),
        JsonValue::Number(Number::Float(number)) if number.is_finite() => {
            Some(if *number <= 0.0 { 0 } else { *number as u64 })
        }
        _ => None,
    }
}

fn string(value: Option<&JsonValue>) -> String {
    match value {
        Some(JsonValue::String(text)) => text.clone(),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    use super::*;
    use core_infrastructure::canon_json::parse;

    /// `byteOffset` は数値でなければ cursor を読まない（旧形の合図）。負数は 0 へ丸め、
    /// 非有限の浮動小数は読まない。
    #[test]
    fn a_cursor_without_a_numeric_byte_offset_is_not_read() {
        assert!(LedgerCursor::of_json(&parse("[]").expect("JSON")).is_none());
        assert!(LedgerCursor::of_json(&parse(r#"{"byteOffset":"12"}"#).expect("JSON")).is_none());
        assert!(LedgerCursor::of_json(&parse(r#"{"lastUuid":"u"}"#).expect("JSON")).is_none());
        let negative =
            LedgerCursor::of_json(&parse(r#"{"byteOffset":-5}"#).expect("JSON")).expect("負数は 0");
        assert_eq!(negative.byte_offset(), 0);
        let fractional = LedgerCursor::of_json(&parse(r#"{"byteOffset":12.9}"#).expect("JSON"))
            .expect("小数は切り捨て");
        assert_eq!(fractional.byte_offset(), 12);
        let negative_float = LedgerCursor::of_json(&parse(r#"{"byteOffset":-0.5}"#).expect("JSON"))
            .expect("負の小数は 0");
        assert_eq!(negative_float.byte_offset(), 0);
        assert_eq!(byte_offset_of(Some(&JsonValue::Bool(true))), None);
        assert_eq!(byte_offset_of(None), None);
    }
}
