//! Testing Postureの明示フィールドと公開語句規則。
use crate::orchestration::TestingPostureError;
use core_infrastructure::ecmascript::trim;
use regex::Regex;
#[derive(Clone)]
pub(super) struct Classification {
    methodology: String,
    ordering: String,
    components: Vec<String>,
}
impl Classification {
    const fn of_values(methodology: String, ordering: String, components: Vec<String>) -> Self {
        Self {
            methodology,
            ordering,
            components,
        }
    }

    pub(super) fn fallback() -> Self {
        Self::of_values(
            "test-after".to_string(),
            default_ordering("test-after").to_string(),
            vec!["test-after".to_string()],
        )
    }
    pub(super) fn parse(body: &str) -> Result<Option<Self>, TestingPostureError> {
        if body.is_empty() {
            return Ok(None);
        }
        let method = structured_field(body, "Methodology");
        let ordering = structured_field(body, "Ordering");
        let structured = method
            .as_ref()
            .map(|method| trim(&method.to_lowercase().replace(['`', '*', '_'], "")).to_string());
        if let Some(value) = &structured
            && !matches!(
                value.as_str(),
                "tdd" | "bdd" | "atdd" | "test-after" | "custom"
            )
        {
            return Err(TestingPostureError::new(format!(
                "Invalid Testing Posture Methodology \"{}\". Expected one of: tdd, bdd, atdd, test-after, custom.",
                method.as_deref().unwrap_or_default()
            )));
        }
        let scan = format!(
            "{}\n{}",
            method.as_deref().unwrap_or_default(),
            ordering.as_deref().unwrap_or(body)
        )
        .to_lowercase();
        let mut components = Vec::new();
        for (name, pattern) in [
            ("tdd", r"\btdd\b|test[- ]driven"),
            ("bdd", r"\bbdd\b|behaviou?r[- ]driven"),
            ("atdd", r"\batdd\b|acceptance[- ]test[- ]driven"),
            (
                "test-after",
                r"test[- ]after|tests? after implementation|implementation[- ]first|classic",
            ),
        ] {
            if expression(pattern)?.is_match(&scan) {
                components.push(name.to_string());
            }
        }
        let order = matching_units(&ordering.as_deref().unwrap_or(body).to_ascii_lowercase());
        let first = expression(r"\b(?:tests?|scenarios?)\b[^.\n]{0,80}\bfirst\b(?:$|[^-])")?
            .is_match(&order);
        let before = expression(r"\b(?:tests?|scenarios?)\b[^.\n]{0,80}\bbefore\b[^.\n]{0,40}\bimplement(?:ation|ing)?\b")?.is_match(&order);
        let after = expression(r"\btests?\b[^.\n]{0,80}\bafter\b[^.\n]{0,40}\bimplement(?:ation|ing)?\b|\brefactor(?:ing)?\b[^.\n]{0,80}\bafter\b[^.\n]{0,40}\bgreen\b|\btests?\b[^.\n]{0,80}\bfollow\b[^.\n]{0,40}\bimplement(?:ation|ing)?\b")?.is_match(&order);
        let mixed = (first || before) && after;
        let custom = expression(r"\b(?:custom|mixed)[ -](?:ordering|cadence|posture|methodology)\b|\b(?:ordering|cadence|posture|methodology)[ -](?:custom|mixed)\b")?.is_match(&body.to_ascii_lowercase());
        if structured.is_none() && components.len() > 1 && !custom && !mixed {
            return Ok(None);
        }
        let methodology = structured.or_else(|| {
            if custom || mixed {
                Some("custom".to_string())
            } else {
                components.first().cloned()
            }
        });
        let Some(methodology) = methodology else {
            return Ok(None);
        };
        if methodology != "custom" && !components.contains(&methodology) {
            components.push(methodology.clone());
        }
        let ordering = ordering.unwrap_or_else(|| {
            if methodology == "custom" {
                core_infrastructure::ecmascript::trim(
                    &core_infrastructure::ecmascript::collapse_whitespace(body),
                )
                .to_string()
            } else {
                default_ordering(&methodology).to_string()
            }
        });
        Ok(Some(Self::of_values(methodology, ordering, components)))
    }
    pub(super) fn methodology(&self) -> &str {
        &self.methodology
    }
    pub(super) fn ordering(&self) -> &str {
        &self.ordering
    }
    pub(super) fn specializes(&self, broader: &Self) -> bool {
        self.methodology == broader.methodology
            || self.methodology == "custom" && self.components.contains(&broader.methodology)
    }
}
fn expression(pattern: &str) -> Result<Regex, TestingPostureError> {
    // 単語境界はJSのASCII定義。/i相当の入力は呼出側でASCIIだけ小文字化し、
    // Unicode foldingで長いsなどをASCIIの語句へ昇格させない。
    Regex::new(&pattern.replace(r"\b", r"(?-u:\b)")).map_err(|error| {
        TestingPostureError::new(format!("Invalid Testing Posture expression: {error}"))
    })
}
fn default_ordering(methodology: &str) -> &'static str {
    match methodology {
        "tdd" => "For each testable layer: Red, then Green, then Refactor.",
        "bdd" => {
            "Define executable behavior scenarios before implementing each observable feature slice."
        }
        "atdd" => {
            "Write executable acceptance tests before implementing the complete feature across its required layers."
        }
        "custom" => {
            "Preserve the explicitly affirmed custom ordering without converting it to another methodology."
        }
        _ => "Implement each testable layer, then write and run that layer's tests.",
    }
}
fn structured_field(body: &str, field: &str) -> Option<String> {
    for line in body.lines() {
        let line = line.trim_matches([' ', '\t']);
        for candidate in [
            Some(line),
            line.strip_prefix(['-', '*'])
                .map(|line| line.trim_start_matches([' ', '\t'])),
        ]
        .into_iter()
        .flatten()
        {
            if let Some(value) = field_value(candidate, field) {
                return Some(value);
            }
        }
    }
    None
}
fn field_value(line: &str, field: &str) -> Option<String> {
    let line = line.strip_prefix("**").unwrap_or(line);
    if !line.get(..field.len())?.eq_ignore_ascii_case(field) {
        return None;
    }
    let rest = line.get(field.len()..)?;
    let rest = rest.strip_prefix("**").unwrap_or(rest);
    let value = trim(rest.trim_start_matches([' ', '\t']).strip_prefix(':')?);
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

/// この語句規則の文字集合はASCIIのみ。量指定の照合ではUTF-16の各単位を1文字として
/// 運び、サロゲート単位は非ASCIIの置換文字で表す。契約の原文・注記・ハッシュには使わない。
fn matching_units(text: &str) -> String {
    text.encode_utf16()
        .map(|unit| char::from_u32(u32::from(unit)).unwrap_or(char::REPLACEMENT_CHARACTER))
        .collect()
}
