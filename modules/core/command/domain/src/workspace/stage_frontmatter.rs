//! ステージ本体の frontmatter — 本家 `aidlc-lib.ts parseStageFrontmatter` の写しを値として持ち、
//! `aidlc-stage-schema.ts validateStageFrontmatter` の規則で自分を検査する。
//!
//! 手書きの YAML 部分集合 — スカラ・文字列リスト (ブロック / フロー)・`consumes[]` の
//! オブジェクトリスト・`produces_kinds` のリスト写像・`when` の 1 キー写像。本家と同じ
//! 正規表現を当て、値の型もそのまま (整数リテラルと `true`/`false` だけ強制変換する)。
//! 検査の失敗文言は本家の逐語 (Published Language) であり、診断の修復案にそのまま載る。
//!
//! 唯一の入口は [`StageFrontmatter::parse`] である (parse-don't-validate)。

use regex::Regex;

mod schema;

/// 解析済みの値 (JS のプレーンオブジェクトの写し)。
#[derive(Debug, Clone, PartialEq, Eq)]
enum FrontmatterValue {
    Text(String),
    Boolean(bool),
    Integer(i64),
    Sequence(Vec<FrontmatterValue>),
    Mapping(Vec<(String, FrontmatterValue)>),
}

/// 解析済みの frontmatter (キーは発見順)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageFrontmatter {
    entries: Vec<(String, FrontmatterValue)>,
}

const ARRAY_KEYS: [&str; 5] = [
    "support_agents",
    "produces",
    "requires_stage",
    "sensors",
    "scopes",
];

impl StageFrontmatter {
    const fn of_entries(entries: Vec<(String, FrontmatterValue)>) -> Self {
        Self { entries }
    }

    /// 本文から frontmatter を切り出して解析する (**唯一の入口**)。
    ///
    /// # Errors
    ///
    /// fence が無い、または `consumes[]` / `produces_kinds` / `when` の形が本家の受理範囲外
    /// (文言は本家 `parseStageFrontmatter` の逐語)。
    pub fn parse(raw: &str) -> Result<Self, String> {
        parse_entries(raw).map(Self::of_entries)
    }

    /// 本家 `validateStageFrontmatter` の規則 1〜9 を同じ順・同じ綴りで検査する
    /// (エラーが無ければ空)。`agents` を渡したときだけ規則 9 (役割の登録照合) を行う。
    #[must_use]
    pub fn schema_errors(&self, agents: Option<&[String]>) -> Vec<String> {
        schema::validate(&self.entries, agents)
    }
}

/// 発見順の項目から鍵で引く (schema 検査の読み方)。
fn get<'a>(entries: &'a [(String, FrontmatterValue)], key: &str) -> Option<&'a FrontmatterValue> {
    entries
        .iter()
        .find(|(name, _)| name == key)
        .map(|(_, value)| value)
}

/// 本文から frontmatter を切り出して発見順の項目にする。
fn parse_entries(raw: &str) -> Result<Vec<(String, FrontmatterValue)>, String> {
    let raw = raw.replace("\r\n", "\n");
    let fence = Regex::new(r"(?s)^---\n(.*?)\n---").map_err(|error| error.to_string())?;
    let Some(captures) = fence.captures(&raw) else {
        return Err("Stage file missing YAML frontmatter (---...---)".to_string());
    };
    let fm = captures.get(1).map_or("", |m| m.as_str());
    let mut object: Vec<(String, FrontmatterValue)> = Vec::new();

    let key_line = Regex::new(r"^([a-z_][a-z0-9_]*)\s*:").map_err(|error| error.to_string())?;
    let mut top_level_keys: Vec<String> = Vec::new();
    for line in fm.split('\n') {
        if let Some(captures) = key_line.captures(line) {
            let key = captures.get(1).map_or("", |m| m.as_str()).to_string();
            if !top_level_keys.contains(&key) {
                top_level_keys.push(key);
            }
        }
    }

    for key in &top_level_keys {
        if key == "consumes"
            || key == "when"
            || key == "produces_kinds"
            || key == "optional_produces"
            || key == "required_sections"
            || ARRAY_KEYS.contains(&key.as_str())
        {
            continue;
        }
        object.push((key.clone(), FrontmatterValue::Text(scalar_field(fm, key))));
    }
    for key in ARRAY_KEYS {
        object.push((key.to_string(), sequence(list_field(fm, key))));
    }
    object.push(("consumes".to_string(), object_list_field(fm, "consumes")?));
    if top_level_keys.iter().any(|key| key == "optional_produces") {
        object.push((
            "optional_produces".to_string(),
            sequence(list_field(fm, "optional_produces")),
        ));
    }
    if top_level_keys.iter().any(|key| key == "produces_kinds") {
        object.push((
            "produces_kinds".to_string(),
            map_of_lists_field(fm, "produces_kinds")?,
        ));
    }
    if top_level_keys.iter().any(|key| key == "required_sections") {
        object.push((
            "required_sections".to_string(),
            sequence(list_field(fm, "required_sections")),
        ));
    }
    coerce(&mut object, "reviewer_max_iterations", |raw| {
        let digits = raw.strip_prefix('-').unwrap_or(raw);
        (!digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()))
            .then(|| raw.parse::<i64>().ok().map(FrontmatterValue::Integer))
            .flatten()
    });
    coerce(&mut object, "workspace_requires", |raw| match raw {
        "true" => Some(FrontmatterValue::Boolean(true)),
        "false" => Some(FrontmatterValue::Boolean(false)),
        _ => None,
    });
    if top_level_keys.iter().any(|key| key == "when") {
        object.push(("when".to_string(), when_field(fm)?));
    }
    Ok(object)
}

fn sequence(items: Vec<String>) -> FrontmatterValue {
    FrontmatterValue::Sequence(items.into_iter().map(FrontmatterValue::Text).collect())
}

/// 文字列として捕まえた値を、条件を満たすときだけ型付きへ置き換える。
fn coerce(
    object: &mut [(String, FrontmatterValue)],
    key: &str,
    convert: impl Fn(&str) -> Option<FrontmatterValue>,
) {
    if let Some((_, value)) = object.iter_mut().find(|(name, _)| name == key)
        && let FrontmatterValue::Text(raw) = value
        && let Some(converted) = convert(raw)
    {
        *value = converted;
    }
}

/// `key: value` のスカラ (本家 `scalarField` — 折り畳み記号は空、引用符は剥がす)。
fn scalar_field(fm: &str, key: &str) -> String {
    let Ok(pattern) = Regex::new(&format!(r"(?m)^{}:\s*(.+?)\s*$", regex::escape(key))) else {
        return String::new();
    };
    let Some(captures) = pattern.captures(fm) else {
        return String::new();
    };
    let raw = captures.get(1).map_or("", |m| m.as_str()).trim();
    if matches!(raw, ">" | "|" | ">-" | "|-") {
        return String::new();
    }
    unquote_pair(raw).to_string()
}

/// 両端が同じ引用符ならその 1 文字ずつを剥がす。
fn unquote_pair(raw: &str) -> &str {
    if raw.len() >= 2
        && ((raw.starts_with('"') && raw.ends_with('"'))
            || (raw.starts_with('\'') && raw.ends_with('\'')))
    {
        raw.get(1..raw.len() - 1).unwrap_or(raw)
    } else {
        raw
    }
}

/// ブロック列または 1 行のフロー列 (本家 `listField`)。
fn list_field(fm: &str, key: &str) -> Vec<String> {
    let escaped = regex::escape(key);
    let block = Regex::new(&format!(
        r"(?m)^{escaped}:\s*\n((?:[ \t]+-[ \t]+[^\r\n]+\r?\n?)+)"
    ));
    if let Ok(block) = block
        && let Some(captures) = block.captures(fm)
    {
        let item = Regex::new(r"^\s*-[ \t]+(.+?)\s*$");
        return captures
            .get(1)
            .map_or("", |m| m.as_str())
            .split('\n')
            .filter_map(|line| {
                let captured = item.as_ref().ok()?.captures(line)?;
                let value = captured.get(1).map_or("", |m| m.as_str());
                Some(strip_edge_quotes(value))
            })
            .filter(|value| !value.is_empty())
            .collect();
    }
    let flow = Regex::new(&format!(r"(?m)^{escaped}:[ \t]*(\[[^\r\n]*)$"));
    if let Ok(flow) = flow
        && let Some(captures) = flow.captures(fm)
    {
        return parse_inline_list(captures.get(1).map_or("", |m| m.as_str()));
    }
    Vec::new()
}

/// `replace(/^["']|["']$/g, "")` — 先頭と末尾の引用符を独立に 1 文字ずつ落とす。
fn strip_edge_quotes(value: &str) -> String {
    let value = value
        .strip_prefix('"')
        .or_else(|| value.strip_prefix('\''))
        .unwrap_or(value);
    value
        .strip_suffix('"')
        .or_else(|| value.strip_suffix('\''))
        .unwrap_or(value)
        .to_string()
}

/// `[a, "b", 'c']` の 1 行リスト (本家 `parseInlineDepsList`)。
fn parse_inline_list(raw: &str) -> Vec<String> {
    let text = raw.trim();
    if text.is_empty() || text == "[]" {
        return Vec::new();
    }
    if !text.starts_with('[') {
        return vec![unquote_scalar(text)];
    }
    let chars: Vec<char> = text.chars().collect();
    let mut close = None;
    let mut quote: Option<char> = None;
    let mut index = 1;
    while index < chars.len() {
        let c = chars.get(index).copied().unwrap_or_default();
        match quote {
            Some('"') if c == '\\' => index += 1,
            Some(open) if c == open => quote = None,
            Some(_) => {}
            None if c == '"' || c == '\'' => quote = Some(c),
            None if c == ']' => {
                close = Some(index);
                break;
            }
            None => {}
        }
        index += 1;
    }
    let (Some(close), None) = (close, quote) else {
        return Vec::new();
    };
    let tail: String = chars.get(close + 1..).unwrap_or_default().iter().collect();
    let trailing = tail.trim_start_matches([' ', '\t']);
    if !(trailing.is_empty() || trailing.starts_with('#')) {
        return Vec::new();
    }
    let body: Vec<char> = chars.get(1..close).unwrap_or_default().to_vec();
    let mut items = Vec::new();
    let mut start = 0;
    let mut quote: Option<char> = None;
    let mut index = 0;
    while index < body.len() {
        let c = body.get(index).copied().unwrap_or_default();
        match quote {
            Some('"') if c == '\\' => index += 1,
            Some(open) if c == open => quote = None,
            Some(_) => {}
            None if c == '"' || c == '\'' => quote = Some(c),
            None if c == ',' => {
                items.push(
                    body.get(start..index)
                        .unwrap_or_default()
                        .iter()
                        .collect::<String>(),
                );
                start = index + 1;
            }
            None => {}
        }
        index += 1;
    }
    if quote.is_some() {
        return Vec::new();
    }
    items.push(
        body.get(start..)
            .unwrap_or_default()
            .iter()
            .collect::<String>(),
    );
    items
        .iter()
        .map(|item| unquote_scalar(item))
        .filter(|item| !item.is_empty())
        .collect()
}

/// trim してから両端の引用符を剥がす (本家 `unquoteScalar`)。
fn unquote_scalar(value: &str) -> String {
    unquote_pair(value.trim()).to_string()
}

/// `consumes[]` のオブジェクト列 (本家 `objectListField`)。
fn object_list_field(fm: &str, key: &str) -> Result<FrontmatterValue, String> {
    let escaped = regex::escape(key);
    let block = Regex::new(&format!(
        r"(?m)^{escaped}:\s*\n((?:[ \t]+-[ \t]+[^\n]+(?:\r?\n|$)(?:[ \t]+[^- \t\n][^\n]*(?:\r?\n|$))*)+)"
    ))
    .map_err(|error| error.to_string())?;
    let Some(captures) = block.captures(fm) else {
        return Ok(FrontmatterValue::Sequence(Vec::new()));
    };
    let whole = captures.get(0).map_or(0..0, |m| m.range());
    let rest = fm.get(whole.end..).unwrap_or_default();
    let indented_item = Regex::new(r"^[ \t]+-[ \t]").map_err(|error| error.to_string())?;
    for line in rest.split('\n') {
        if line.is_empty() || line.trim_matches([' ', '\t']).is_empty() {
            continue;
        }
        if indented_item.is_match(line) {
            return Err(format!(
                "Blank line not allowed inside {key}[] block — list items must be consecutive"
            ));
        }
        break;
    }
    let item = Regex::new(r"^\s*-\s+([a-z_]+):\s*(.+?)\s*$").map_err(|error| error.to_string())?;
    let sub = Regex::new(r"^\s+([a-z_]+):\s*(.+?)\s*$").map_err(|error| error.to_string())?;
    let mut items: Vec<FrontmatterValue> = Vec::new();
    let mut current: Option<Vec<(String, FrontmatterValue)>> = None;
    for line in captures
        .get(1)
        .map_or("", |m| m.as_str())
        .split('\n')
        .filter(|line| !line.trim().is_empty())
    {
        if let Some(captured) = item.captures(line) {
            if let Some(done) = current.take() {
                items.push(FrontmatterValue::Mapping(done));
            }
            current = Some(vec![(
                captured.get(1).map_or("", |m| m.as_str()).to_string(),
                coerce_scalar(captured.get(2).map_or("", |m| m.as_str())),
            )]);
        } else if let (Some(captured), Some(object)) = (sub.captures(line), current.as_mut()) {
            object.push((
                captured.get(1).map_or("", |m| m.as_str()).to_string(),
                coerce_scalar(captured.get(2).map_or("", |m| m.as_str())),
            ));
        } else {
            return Err(format!(
                "Malformed {key}[] entry in frontmatter: {}",
                line.trim()
            ));
        }
    }
    if let Some(done) = current {
        items.push(FrontmatterValue::Mapping(done));
    }
    Ok(FrontmatterValue::Sequence(items))
}

/// `true` / `false` は boolean、引用符付きは文字列 (本家 `coerceScalar`)。
fn coerce_scalar(raw: &str) -> FrontmatterValue {
    match raw {
        "true" => FrontmatterValue::Boolean(true),
        "false" => FrontmatterValue::Boolean(false),
        _ => FrontmatterValue::Text(unquote_pair(raw).to_string()),
    }
}

/// `produces_kinds` のリスト写像 (本家 `mapOfListsField`)。
fn map_of_lists_field(fm: &str, key: &str) -> Result<FrontmatterValue, String> {
    let escaped = regex::escape(key);
    let block = Regex::new(&format!(
        r"(?m)^{escaped}:\s*\n((?:[ \t]+[a-z][a-z0-9-]*\s*:\s*[^\n]*(?:\r?\n|$))+)"
    ))
    .map_err(|error| error.to_string())?;
    let Some(captures) = block.captures(fm) else {
        return Ok(FrontmatterValue::Mapping(Vec::new()));
    };
    let entry =
        Regex::new(r"^\s+([a-z][a-z0-9-]*)\s*:\s*(.+?)\s*$").map_err(|error| error.to_string())?;
    let mut out = Vec::new();
    for line in captures.get(1).map_or("", |m| m.as_str()).split('\n') {
        if line.trim().is_empty() {
            continue;
        }
        let Some(captured) = entry.captures(line) else {
            return Err(format!(
                "Malformed {key} entry in frontmatter: {}",
                line.trim()
            ));
        };
        let name = captured.get(1).map_or("", |m| m.as_str());
        let value = captured.get(2).map_or("", |m| m.as_str()).trim();
        if !(value.starts_with('[') && value.ends_with(']')) {
            return Err(format!(
                "{key}.{name} must be an inline list (e.g. [service, ui]), got: {value}"
            ));
        }
        out.push((name.to_string(), sequence(parse_inline_list(value))));
    }
    Ok(FrontmatterValue::Mapping(out))
}

/// `when:` の 1 キー写像 (ブロック形 / インライン形 / それ以外はスカラ)。
fn when_field(fm: &str) -> Result<FrontmatterValue, String> {
    let block = Regex::new(r"(?m)^when:\s*\n\s+([a-z][a-z0-9-]*)\s*:\s*(.+?)\s*$")
        .map_err(|error| error.to_string())?;
    if let Some(captures) = block.captures(fm) {
        return Ok(FrontmatterValue::Mapping(vec![(
            captures.get(1).map_or("", |m| m.as_str()).to_string(),
            FrontmatterValue::Text(captures.get(2).map_or("", |m| m.as_str()).to_string()),
        )]));
    }
    let inline = Regex::new(r"(?m)^when:\s*\{\s*([a-z][a-z0-9-]*)\s*:\s*([^}]+?)\s*\}\s*$")
        .map_err(|error| error.to_string())?;
    if let Some(captures) = inline.captures(fm) {
        return Ok(FrontmatterValue::Mapping(vec![(
            captures.get(1).map_or("", |m| m.as_str()).to_string(),
            FrontmatterValue::Text(
                captures
                    .get(2)
                    .map_or("", |m| m.as_str())
                    .trim()
                    .to_string(),
            ),
        )]));
    }
    Ok(FrontmatterValue::Text(scalar_field(fm, "when")))
}
