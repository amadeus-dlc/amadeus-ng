//! 確認対象に見える見出しと、見出しを装う文字列の区別。
use super::containers::Containers;

pub(super) fn heading(line: &str) -> Option<(usize, String)> {
    let trimmed = line.trim_start_matches(' ');
    if line.len() - trimmed.len() > 3 {
        return None;
    }
    let level = trimmed.bytes().take_while(|byte| *byte == b'#').count();
    if !(1..=6).contains(&level) {
        return None;
    }
    let rest = trimmed.get(level..)?;
    if !rest.is_empty() && !rest.starts_with([' ', '\t']) {
        return None;
    }
    let title = rest.trim_start_matches([' ', '\t']).replace('\0', "");
    let ending = title.trim_end_matches([' ', '\t']);
    let without_hashes = ending.trim_end_matches('#');
    let title = if without_hashes.len() < ending.len() && without_hashes.ends_with([' ', '\t']) {
        without_hashes.trim_end_matches([' ', '\t'])
    } else {
        &title
    };
    Some((level, title.trim().to_string()))
}
pub(super) fn question_id(title: &str) -> Option<&str> {
    let digits = title.strip_prefix('Q')?;
    let count = digits.bytes().take_while(u8::is_ascii_digit).count();
    if count == 0 || digits.starts_with('0') {
        return None;
    }
    let tail = digits.get(count..)?;
    if !tail.is_empty() {
        let suffix = tail.strip_prefix('.').or_else(|| tail.strip_prefix(':'))?;
        if !suffix.is_empty() && !suffix.starts_with([' ', '\t']) {
            return None;
        }
    }
    title.get(..count + 1)
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum HeadingStyle {
    Atx,
    NestedAtx,
    Setext,
    Html,
}
pub(super) fn hash_heading(
    lines: &[String],
    index: usize,
) -> Option<(HeadingStyle, usize, String)> {
    let current = lines.get(index)?;
    let (candidate, containers) = Containers::from_line(current);
    if let Some((level, title)) = heading(candidate) {
        return Some((
            if containers.is_empty() {
                HeadingStyle::Atx
            } else {
                HeadingStyle::NestedAtx
            },
            level,
            title,
        ));
    }
    if let Some((level, title)) = setext_heading(lines, index) {
        return Some((HeadingStyle::Setext, level, title));
    }
    html_heading(current).map(|level| (HeadingStyle::Html, level, format!("<h{level}>")))
}
pub(super) fn underline_level(current: &str) -> Option<usize> {
    let candidate = current.trim_start_matches(' ');
    if current.len() - candidate.len() > 3 {
        return None;
    }
    let underline = candidate.trim_end_matches([' ', '\t']);
    let marker = underline.chars().next()?;
    if !matches!(marker, '=' | '-') || !underline.chars().all(|character| character == marker) {
        return None;
    }
    Some(if marker == '=' { 1 } else { 2 })
}
fn setext_heading(lines: &[String], index: usize) -> Option<(usize, String)> {
    let current = lines.get(index)?;
    let level = underline_level(Containers::from_line(current).0)?;
    let previous = lines.get(index.checked_sub(1)?)?.replace('\0', "");
    let (previous, _) = Containers::from_line(&previous);
    if previous.trim().is_empty() || heading(previous).is_some() {
        return None;
    }
    Some((level, previous.trim().to_string()))
}
pub(super) fn html_heading_start(line: &str) -> Option<usize> {
    let candidate = line.trim_start_matches(' ');
    if line.len() - candidate.len() > 3 {
        return None;
    }
    let after = candidate.strip_prefix('<')?;
    if !matches!(after.as_bytes().first(), Some(b'h' | b'H')) {
        return None;
    }
    let digit = *after.as_bytes().get(1)?;
    if !(b'1'..=b'6').contains(&digit)
        || after
            .split_at(2)
            .1
            .chars()
            .next()
            .is_some_and(|character| character.is_ascii_alphanumeric() || character == '_')
    {
        return None;
    }
    Some(usize::from(digit - b'0'))
}

pub(super) fn escaped_at(line: &str, offset: usize) -> bool {
    line.split_at(offset)
        .0
        .chars()
        .rev()
        .take_while(|character| *character == '\\')
        .count()
        % 2
        == 1
}
pub(super) fn inline_code_end(line: &str, start: usize) -> Option<usize> {
    let length = line
        .split_at(start)
        .1
        .bytes()
        .take_while(|byte| *byte == b'`')
        .count();
    let mut cursor = start + length;
    while cursor < line.len() {
        let position = cursor + line.split_at(cursor).1.find('`')?;
        let closing = line
            .split_at(position)
            .1
            .bytes()
            .take_while(|byte| *byte == b'`')
            .count();
        if closing == length {
            return Some(position + closing);
        }
        cursor = position + closing;
    }
    None
}
fn without_inline_code(line: &str) -> String {
    let mut out = String::new();
    let mut cursor = 0;
    while cursor < line.len() {
        let rest = line.split_at(cursor).1;
        let Some(start) = rest.find('`') else {
            out.push_str(rest);
            break;
        };
        out.push_str(rest.split_at(start).0);
        let Some(end) = inline_code_end(line, cursor + start) else {
            break;
        };
        cursor = end;
    }
    out
}
fn angle_link_destination(line: &str, tag: usize) -> bool {
    let before = line.split_at(tag).0;
    let Some(destination) = before.rfind("](") else {
        return false;
    };
    if !before
        .split_at(destination + 2)
        .1
        .chars()
        .all(|character| matches!(character, ' ' | '\t'))
        || escaped_at(before, destination)
    {
        return false;
    }
    let Some(label) = before.split_at(destination).0.rfind('[') else {
        return false;
    };
    if escaped_at(before, label) {
        return false;
    }
    let Some(closing) = line.split_at(tag + 1).1.find('>') else {
        return false;
    };
    line.split_at(tag + closing + 2)
        .1
        .trim_start_matches([' ', '\t'])
        .starts_with(')')
}
fn html_heading(line: &str) -> Option<usize> {
    let line = line.replace('\0', "");
    let (line, _) = Containers::from_line(&line);
    if line.starts_with("    ") || line.starts_with('\t') {
        return None;
    }
    let line = without_inline_code(line);
    let mut cursor = 0;
    while cursor < line.len() {
        let start = cursor + line.split_at(cursor).1.find('<')?;
        cursor = start + 1;
        if escaped_at(&line, start) || angle_link_destination(&line, start) {
            continue;
        }
        let after = line.split_at(cursor).1;
        if let Some(level) = html_heading_start(line.split_at(start).1) {
            return Some(level);
        }
        let mut quote = None;
        for (offset, character) in after.char_indices() {
            if let Some(active) = quote {
                if character == active {
                    quote = None;
                }
            } else if matches!(character, '\'' | '"') {
                quote = Some(character);
            } else if character == '>' {
                cursor += offset + 1;
                break;
            }
        }
    }
    None
}
