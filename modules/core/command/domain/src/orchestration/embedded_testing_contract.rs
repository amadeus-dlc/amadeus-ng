//! 計画へ埋め込まれたテスト契約の自己整合性。
use core_infrastructure::canon_json::{JsonValue, Number, hash_canonical, parse};
/// 自己ハッシュを検証した契約。現在の規則との一致は別に検査する。
#[derive(Debug, Clone, PartialEq)]
pub struct EmbeddedTestingContract {
    value: JsonValue,
    hash: String,
}
impl EmbeddedTestingContract {
    /// Testing Contract節のJSONと自己ハッシュを検証する。
    #[must_use]
    pub fn parse(plan: &str) -> Option<Self> {
        let section = contract_section(plan);
        // ASCIIの大小だけを畳むので、元のJSONを取り出すバイト位置は変わらない。
        let normalized = section.to_ascii_lowercase();
        let expression = regex::Regex::new(r"(?s)```json[ \t]*\r?\n(.*?)\r?\n```").ok()?;
        let matched = expression.captures(&normalized)?.get(1)?;
        let value = parse(section.get(matched.range())?).ok()?;
        let JsonValue::Object(fields) = &value else {
            return None;
        };
        let valid_version = match fields.get("version")? {
            JsonValue::Number(Number::PosInt(1) | Number::NegInt(1)) => true,
            JsonValue::Number(Number::Float(value)) => value.to_bits() == 1.0f64.to_bits(),
            _ => false,
        };
        if !valid_version {
            return None;
        }
        let JsonValue::String(hash) = fields.get("contract_sha256")? else {
            return None;
        };
        let digits = hash.strip_prefix("sha256:")?;
        if digits.len() != 64
            || !digits
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return None;
        }
        let body = fields.filter(|key, _| key != "contract_sha256");
        if hash_canonical(&JsonValue::Object(body)).rendered() != *hash {
            return None;
        }
        let hash = hash.clone();
        Some(Self { value, hash })
    }
    /// 現在の規則から解決した契約と同じか。
    #[must_use]
    pub fn is_current(&self, posture: &super::TestingPosture) -> bool {
        matches!(posture.value(), JsonValue::Object(fields) if matches!(fields.get("contract_sha256"), Some(JsonValue::String(hash)) if hash == self.hash()))
    }

    /// 検証済みの契約値。
    #[must_use]
    pub const fn value(&self) -> &JsonValue {
        &self.value
    }
    /// 自己ハッシュ。
    #[must_use]
    pub fn hash(&self) -> &str {
        &self.hash
    }
}

fn contract_section(plan: &str) -> String {
    let normalized = plan.replace("\r\n", "\n");
    let mut found = false;
    let mut fenced = false;
    let mut body = Vec::new();
    for line in normalized.split('\n') {
        if line.starts_with("```") {
            if found {
                body.push(line);
            }
            fenced = !fenced;
            continue;
        }
        if !fenced && line.trim_end() == "## Testing Contract" {
            found = true;
            continue;
        }
        if found && !fenced && line.starts_with("## ") {
            break;
        }
        if found {
            body.push(line);
        }
    }
    body.join("\n")
}
