//! テスト方針の注記と、方法論の判定対象を分けるMarkdown境界。
use core_infrastructure::ecmascript::{trim, trim_end};
#[derive(Clone, Copy)]
struct Fence {
    marker: u8,
    length: usize,
}
impl Fence {
    fn opening(line: &str) -> Option<Self> {
        let content = line.trim_start_matches(' ');
        if line.len() - content.len() > 3 {
            return None;
        }
        let marker = *content.as_bytes().first()?;
        if !matches!(marker, b'`' | b'~') {
            return None;
        }
        let length = content.bytes().take_while(|byte| *byte == marker).count();
        if length < 3 || marker == b'`' && content.get(length..)?.contains('`') {
            return None;
        }
        Some(Self { marker, length })
    }
    fn closes(self, line: &str) -> bool {
        let content = line.trim_start_matches(' ');
        if line.len() - content.len() > 3 {
            return false;
        }
        let content = content.trim_end_matches([' ', '\t']);
        content.len() >= self.length && content.bytes().all(|byte| byte == self.marker)
    }
}
#[derive(Default)]
struct Comments {
    in_comment: bool,
    inline_ticks: usize,
}
impl Comments {
    fn strip(&mut self, raw: &str) -> String {
        let mut line = String::new();
        let mut cursor = 0;
        while let Some(rest) = raw.get(cursor..).filter(|rest| !rest.is_empty()) {
            if self.in_comment {
                let Some(end) = rest.find("-->") else {
                    break;
                };
                self.in_comment = false;
                cursor += end + 3;
                continue;
            }
            if rest.starts_with('`') && (self.inline_ticks > 0 || !escaped(raw, cursor)) {
                let ticks = rest.bytes().take_while(|byte| *byte == b'`').count();
                if self.inline_ticks == 0 && matching_ticks(raw, cursor + ticks, ticks) {
                    self.inline_ticks = ticks;
                } else if self.inline_ticks == ticks {
                    self.inline_ticks = 0;
                }
                line.extend(std::iter::repeat_n('`', ticks));
                cursor += ticks;
                continue;
            }
            if self.inline_ticks == 0 && !escaped(raw, cursor) && rest.starts_with("<!--") {
                self.in_comment = true;
                cursor += 4;
                continue;
            }
            let Some(character) = rest.chars().next() else {
                break;
            };
            line.push(character);
            cursor += character.len_utf8();
        }
        line
    }
}
fn escaped(line: &str, offset: usize) -> bool {
    line.get(..offset).is_some_and(|prefix| {
        prefix
            .bytes()
            .rev()
            .take_while(|byte| *byte == b'\\')
            .count()
            % 2
            == 1
    })
}
fn matching_ticks(line: &str, mut cursor: usize, expected: usize) -> bool {
    while let Some(rest) = line.get(cursor..).filter(|rest| !rest.is_empty()) {
        let Some(offset) = rest.find('`') else {
            return false;
        };
        cursor += offset;
        if escaped(line, cursor) {
            cursor += 1;
            continue;
        }
        let count = line
            .get(cursor..)
            .unwrap_or_default()
            .bytes()
            .take_while(|byte| *byte == b'`')
            .count();
        if count == expected {
            return true;
        }
        cursor += count;
    }
    false
}
/// 注記にはコード例を残し、分類対象からはコード例を除く。
pub(super) fn projections(content: &str) -> (String, String) {
    let normalized = content
        .strip_prefix('\u{feff}')
        .unwrap_or(content)
        .replace("\r\n", "\n");
    let raw: Vec<&str> = normalized.split('\n').collect();
    let visible = without_comments(&raw);
    let mut fence: Option<Fence> = None;
    let classified: Vec<&str> = raw
        .iter()
        .zip(&visible)
        .map(|(raw, line)| {
            let structural = structural_line(raw, line);
            if let Some(open) = fence {
                if open.closes(structural) {
                    fence = None;
                }
                return "";
            }
            fence = Fence::opening(structural);
            if fence.is_some() { "" } else { line }
        })
        .collect();
    (
        trim(&visible.join("\n")).to_string(),
        trim(&classified.join("\n")).to_string(),
    )
}

fn without_comments(raw: &[&str]) -> Vec<String> {
    let mut comments = Comments::default();
    let mut fence: Option<Fence> = None;
    raw.iter()
        .map(|raw| {
            if let Some(open) = fence {
                if open.closes(raw) {
                    fence = None;
                }
                return (*raw).to_string();
            }
            let started_in_comment = comments.in_comment;
            let line = comments.strip(raw);
            let start = raw.find("<!--");
            let prefix = if started_in_comment || line != *raw && start.is_none() {
                ""
            } else {
                start.and_then(|start| raw.get(..start)).unwrap_or(raw)
            };
            fence = Fence::opening(prefix);
            if fence.is_some() {
                comments = Comments::default();
                (*raw).to_string()
            } else {
                line
            }
        })
        .collect()
}
fn structural_line<'a>(raw: &'a str, line: &'a str) -> &'a str {
    if raw == line {
        line
    } else {
        match (raw.find("<!--"), raw.find("-->")) {
            (Some(start), end) if end.is_none_or(|end| start < end) => {
                raw.get(..start).unwrap_or_default()
            }
            _ => "",
        }
    }
}
pub(super) fn extract_section(content: &str) -> String {
    let normalized = content
        .strip_prefix('\u{feff}')
        .unwrap_or(content)
        .replace("\r\n", "\n");
    let raw: Vec<&str> = normalized.split('\n').collect();
    let visible = without_comments(&raw);
    let mut fence: Option<Fence> = None;
    let mut start = None;
    let mut end = raw.len();
    for (index, (raw, visible)) in raw.iter().zip(&visible).enumerate() {
        let line = structural_line(raw, visible);
        if let Some(open) = fence {
            if open.closes(line) {
                fence = None;
            }
            continue;
        }
        fence = Fence::opening(line);
        if fence.is_some() {
            continue;
        }
        if start.is_none() {
            if trim_end(line) == "## Testing Posture" {
                start = Some(index + 1);
            }
        } else if line.starts_with("## ") {
            end = index;
            break;
        }
    }
    start
        .and_then(|start| raw.get(start..end))
        .unwrap_or_default()
        .join("\n")
}
