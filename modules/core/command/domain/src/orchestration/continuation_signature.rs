//! ステージ・安定状態・指示の3面を束ねた進捗署名。
use super::ContinuationError;
#[derive(Debug, Clone, PartialEq, Eq)]
/// ステージ・安定状態・指示の3面を束ねた進捗署名。
pub struct ContinuationSignature(String);
impl ContinuationSignature {
    /// 公開された状態と指示の意味のある部分を本家と同じ順序でハッシュする。
    /// # Errors
    /// 指示がオブジェクトでない、またはkindが無い場合。
    pub fn from_observation(
        state: &str,
        directive: &core_infrastructure::canon_json::JsonValue,
    ) -> Result<Self, ContinuationError> {
        use core_infrastructure::canon_json::{
            JsonValue, ObjectMembers, SerializationProfile, serialize,
        };
        let JsonValue::Object(source) = directive else {
            return Err(ContinuationError::InvalidSignature);
        };
        let text = |key: &str| match source.get(key) {
            Some(JsonValue::String(value)) => core_infrastructure::ecmascript::trim(value),
            _ => "",
        };
        let hash = |value: &str| core_infrastructure::hash::sha256_hex(value.as_bytes());
        let compact = |value: &JsonValue| serialize(value, SerializationProfile::ContractCompact);
        if text("kind").is_empty() {
            return Err(ContinuationError::InvalidSignature);
        }
        let stage_pattern = regex::Regex::new(r"Current Stage\*{0,2}:?\s*`?([^\n`]*)`?")
            .map_err(|_| ContinuationError::InvalidSignature)?;
        let stage_match = stage_pattern.captures(state);
        let stage = stage_match
            .as_ref()
            .and_then(|capture| capture.get(1))
            .map_or("", |part| {
                core_infrastructure::ecmascript::trim(part.as_str())
            });
        let stable = state
            .split_inclusive('\n')
            .filter(|line| !line.starts_with("- **Last Updated**:"))
            .collect::<String>();
        let mut fields = ObjectMembers::new();
        for key in ["kind", "stage", "unit"] {
            fields.insert(key, JsonValue::String(text(key).to_string()));
        }
        for key in ["part", "parts"] {
            fields.insert(key, source.get(key).cloned().unwrap_or(JsonValue::Null));
        }
        fields.insert(
            "continue_token_sha256",
            JsonValue::String(if text("continue_token").is_empty() {
                String::new()
            } else {
                hash(text("continue_token"))
            }),
        );
        fields.insert(
            "rules_content_sha256",
            JsonValue::String(
                source
                    .get("rules_content")
                    .map_or_else(String::new, |value| hash(&compact(value))),
            ),
        );
        fields.insert(
            "units",
            source
                .get("units")
                .cloned()
                .unwrap_or_else(|| JsonValue::Array(Vec::new())),
        );
        for key in ["worker", "repo"] {
            fields.insert(key, JsonValue::String(text(key).to_string()));
        }
        fields.insert(
            "wave_sha256",
            JsonValue::String(
                source
                    .get("wave")
                    .map_or_else(String::new, |value| hash(&compact(value))),
            ),
        );
        Self::parse(&format!(
            "{stage}::{}::{}",
            hash(&stable),
            hash(&compact(&JsonValue::Object(fields)))
        ))
    }
    /// 本家のstage::state-hash::directive-hashを検査する。
    /// # Errors
    /// 要素数または2つのSHA-256の表記が不正。
    pub fn parse(value: &str) -> Result<Self, ContinuationError> {
        let mut parts = value.split("::");
        let stage = parts.next().ok_or(ContinuationError::InvalidSignature)?;
        let digest = |part: Option<&str>| {
            part.is_some_and(|s| {
                s.len() == 64
                    && s.bytes()
                        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            })
        };
        if stage.contains(['\r', '\n'])
            || !digest(parts.next())
            || !digest(parts.next())
            || parts.next().is_some()
        {
            return Err(ContinuationError::InvalidSignature);
        }
        Ok(Self(value.to_string()))
    }
    /// 公開互換markerへ描く署名。
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
