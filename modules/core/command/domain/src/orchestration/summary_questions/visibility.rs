//! 行位置を維持したまま、コメント・コード・HTML属性の可視性を判定する。
use super::containers::Containers;
use super::headings::{escaped_at, heading, html_heading_start, inline_code_end, underline_level};

fn block_boundary(line: &str) -> bool {
    let trimmed = line.trim_start_matches(' ');
    if line.len() - trimmed.len() > 3 {
        return false;
    }
    heading(line).is_some()
        || trimmed
            .bytes()
            .take_while(|byte| matches!(byte, b'`' | b'~'))
            .count()
            >= 3
        || [b'=', b'-'].iter().any(|marker| {
            !trimmed.trim_end_matches([' ', '\t']).is_empty()
                && trimmed
                    .trim_end_matches([' ', '\t'])
                    .bytes()
                    .all(|byte| byte == *marker)
        })
        || trimmed
            .bytes()
            .filter(|byte| !matches!(byte, b' ' | b'\t'))
            .count()
            >= 3
            && trimmed
                .bytes()
                .all(|byte| matches!(byte, b'*' | b'_' | b'-' | b' ' | b'\t'))
}
pub(super) fn visible_lines(lines: &[&str]) -> Vec<String> {
    let source: Vec<&str> = lines
        .iter()
        .enumerate()
        .map(|(index, line)| {
            if index == 0 {
                line.strip_prefix('\u{feff}').unwrap_or(line)
            } else {
                line
            }
        })
        .collect();
    let lines = source.as_slice();
    let mut inline = InlineVisibility::default();
    let mut fence: Option<(char, usize, Containers)> = None;
    let mut raw_html: Option<(&str, Containers)> = None;
    let mut active: Option<(Containers, bool)> = None;
    lines
        .iter()
        .enumerate()
        .map(|(line_number, line)| {
            let (explicit_content, explicit) = Containers::from_line(line);
            let mut content = explicit_content;
            let mut owners = explicit.clone();
            if let Some((prior, had_blank)) = active.clone() {
                let blank = line.trim().is_empty();
                let continuation = if blank {
                    Some("")
                } else {
                    prior.continuation(line)
                };
                let trimmed = line.trim_start_matches(' ');
                let lazy_block = prior.has_quote()
                    && line.len() - trimmed.len() <= 3
                    && (trimmed.starts_with("<!--")
                        || trimmed
                            .bytes()
                            .take_while(|byte| matches!(byte, b'`' | b'~'))
                            .count()
                            >= 3);
                if blank {
                    content = "";
                    owners = prior.clone();
                    active = Some((prior, true));
                } else if let Some(continuation) = continuation {
                    let (nested_content, nested) = Containers::from_line(continuation);
                    content = nested_content;
                    owners = prior.appended(&nested);
                    active = Some((owners.clone(), false));
                } else if !explicit.is_empty() {
                    active = None;
                } else if lazy_block || !had_blank && !block_boundary(line) {
                    content = line;
                    owners = prior.clone();
                    active = Some((prior, false));
                } else {
                    active = None;
                }
            }
            if !owners.is_empty() {
                active = Some((owners.clone(), line.trim().is_empty()));
            }
            if let Some((tag, container)) = raw_html.clone() {
                let continuation = if container.is_empty() {
                    Some(*line)
                } else if line.trim().is_empty() {
                    Some("")
                } else {
                    container.continuation(line)
                };
                if let Some(continuation) = continuation {
                    if continuation
                        .to_ascii_lowercase()
                        .contains(&format!("</{tag}>"))
                    {
                        raw_html = None;
                    }
                    return String::new();
                }
                raw_html = None;
            }
            if let Some((marker, length, container)) = fence.clone() {
                let continuation = if container.is_empty() {
                    Some(*line)
                } else if line.trim().is_empty() {
                    Some("")
                } else {
                    container.continuation(line)
                };
                if let Some(continuation) = continuation {
                    let closing = continuation.trim_matches([' ', '\t']);
                    if closing.len() >= length
                        && closing.chars().all(|character| character == marker)
                    {
                        fence = None;
                    }
                    return String::new();
                }
                fence = None;
            }
            if inline.in_comment
                && !inline.comment_container.is_empty()
                && !line.trim().is_empty()
                && inline.comment_container.continuation(line).is_none()
            {
                inline.in_comment = false;
                inline.comment_container = Containers::default();
            }
            if inline.html_tag_open
                && (explicit_content.trim().is_empty()
                    || heading(explicit_content).is_some()
                    || underline_level(explicit_content).is_some()
                    || html_heading_start(explicit_content).is_some()
                    || inline.html_quote.is_none() && explicit_content.starts_with("[Answer]:"))
            {
                inline.html_tag_open = false;
                inline.html_quote = None;
            }
            let continued_tag = inline.html_tag_open;
            let mut start_offset = 0;
            let mut continued_span = false;
            if let Some((end_line, end_offset)) = inline.code_span_end {
                if line_number < end_line {
                    return String::new();
                }
                start_offset = end_offset;
                inline.code_span_end = None;
                continued_span = true;
            }
            let candidate = content.trim_start_matches(' ');
            if !inline.in_comment
                && !inline.html_tag_open
                && !continued_span
                && content.len() - candidate.len() <= 3
                && let Some(marker @ ('`' | '~')) = candidate.chars().next()
            {
                let length = candidate
                    .chars()
                    .take_while(|character| *character == marker)
                    .count();
                if length >= 3
                    && (marker == '~'
                        || candidate
                            .get(length..)
                            .is_some_and(|tail| !tail.contains('`')))
                {
                    fence = Some((marker, length, owners));
                    return String::new();
                }
            }
            if !inline.in_comment
                && !inline.html_tag_open
                && !continued_span
                && (explicit_content.starts_with("    ") || explicit_content.starts_with('\t'))
            {
                return String::new();
            }
            if !inline.in_comment
                && !inline.html_tag_open
                && !continued_span
                && content.len() - candidate.len() <= 3
            {
                let lowercase = candidate.to_ascii_lowercase();
                for tag in ["script", "pre", "style", "textarea"] {
                    if lowercase
                        .strip_prefix(&format!("<{tag}"))
                        .is_some_and(|tail| tail.is_empty() || tail.starts_with([' ', '\t', '>']))
                    {
                        if !lowercase.contains(&format!("</{tag}>")) {
                            raw_html = Some((tag, owners));
                        }
                        return String::new();
                    }
                }
            }
            inline.scan(
                lines,
                (line_number, line),
                content,
                &owners,
                start_offset,
                continued_tag || continued_span,
            )
        })
        .collect()
}

#[derive(Default)]
struct InlineVisibility {
    in_comment: bool,
    comment_container: Containers,
    html_tag_open: bool,
    html_quote: Option<char>,
    code_span_end: Option<(usize, usize)>,
}
impl InlineVisibility {
    fn scan(
        &mut self,
        lines: &[&str],
        source: (usize, &str),
        content: &str,
        owners: &Containers,
        start: usize,
        continued: bool,
    ) -> String {
        let (number, raw) = source;
        let line = raw.replace('\0', "\u{2}");
        let mut result = if continued {
            "\u{1}".to_string()
        } else {
            String::new()
        };
        let mut cursor = start;
        while cursor < line.len() {
            let rest = line.split_at(cursor).1;
            if self.in_comment {
                let Some(end) = rest.find("-->") else { break };
                self.in_comment = false;
                self.comment_container = Containers::default();
                result.push('\0');
                cursor += end + 3;
                continue;
            }
            if rest.starts_with('`') && !self.html_tag_open && !escaped_at(&line, cursor) {
                let Some((end_line, end_offset)) = multiline_code_end(lines, number, cursor) else {
                    result.push_str(rest);
                    break;
                };
                if end_line == number {
                    result.push_str(rest.split_at(end_offset - cursor).0);
                    cursor = end_offset;
                    continue;
                }
                result.push('\u{1}');
                self.code_span_end = Some((end_line, end_offset));
                break;
            }
            if rest.starts_with("<!--") {
                let escaped = escaped_at(&line, cursor);
                let candidate = content.trim_start_matches(' ');
                let block_start = content.len() - candidate.len() <= 3
                    && candidate.starts_with("<!--")
                    && line.len() - candidate.len() == cursor;
                let closes = rest.split_at(4).1.contains("-->");
                cursor += 4;
                if escaped || !closes && (!block_start || self.html_tag_open) {
                    result.push_str("<!--");
                } else {
                    result.push('\0');
                    self.in_comment = true;
                    self.comment_container = owners.clone();
                }
                continue;
            }
            let Some(character) = rest.chars().next() else {
                break;
            };
            result.push(character);
            if self.html_tag_open {
                if let Some(quote) = self.html_quote {
                    if character == quote {
                        self.html_quote = None;
                    }
                } else if matches!(character, '\'' | '"') {
                    self.html_quote = Some(character);
                } else if character == '>' {
                    self.html_tag_open = false;
                }
            } else if character == '<'
                && rest
                    .chars()
                    .nth(1)
                    .is_some_and(|next| next.is_ascii_alphabetic() || matches!(next, '!' | '/'))
            {
                self.html_tag_open = true;
            }
            cursor += character.len_utf8();
        }
        result
    }
}
fn raw_html_start(line: &str) -> bool {
    let candidate = line.trim_start_matches(' ');
    if line.len() - candidate.len() > 3 {
        return false;
    }
    let lowercase = candidate.to_ascii_lowercase();
    ["script", "pre", "style", "textarea"].iter().any(|tag| {
        lowercase
            .strip_prefix(&format!("<{tag}"))
            .is_some_and(|tail| tail.is_empty() || tail.starts_with([' ', '\t', '>']))
    })
}
fn multiline_code_end(lines: &[&str], start_line: usize, start: usize) -> Option<(usize, usize)> {
    let line = lines.get(start_line)?;
    if let Some(end) = inline_code_end(line, start) {
        return Some((start_line, end));
    }
    if heading(Containers::from_line(line).0).is_some() {
        return None;
    }
    let length = line
        .split_at(start)
        .1
        .bytes()
        .take_while(|byte| *byte == b'`')
        .count();
    for (number, line) in lines.iter().enumerate().skip(start_line + 1) {
        let candidate = Containers::from_line(line).0;
        if candidate.trim().is_empty() || block_boundary(candidate) || raw_html_start(candidate) {
            return None;
        }
        let mut cursor = 0;
        while cursor < line.len() {
            let Some(start) = line.split_at(cursor).1.find('`') else {
                break;
            };
            let start = cursor + start;
            let closing = line
                .split_at(start)
                .1
                .bytes()
                .take_while(|byte| *byte == b'`')
                .count();
            if closing == length {
                return Some((number, start + closing));
            }
            cursor = start + closing;
        }
    }
    None
}
