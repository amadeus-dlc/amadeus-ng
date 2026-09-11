//! Markdownのリスト/引用の所有範囲。固定本家のcontainer規則を表す。
#[derive(Debug, Clone, PartialEq, Eq)]
enum Segment {
    Quote,
    List(usize),
}
/// 入れ子のコンテナの一級コレクション。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Containers(Vec<Segment>);
impl Default for Containers {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}
impl Containers {
    const fn new(segments: Vec<Segment>) -> Self {
        Self(segments)
    }
    pub(super) fn from_line(mut line: &str) -> (&str, Self) {
        let mut segments = Vec::new();
        loop {
            if let Some(content) = quote_content(line) {
                line = content;
                segments.push(Segment::Quote);
                continue;
            }
            let trimmed = line.trim_start_matches(' ');
            if line.len() - trimmed.len() > 3 {
                break;
            }
            let marker = if trimmed.starts_with(['*', '+', '-']) {
                1
            } else {
                let digits = trimmed.bytes().take_while(u8::is_ascii_digit).count();
                if !(1..=9).contains(&digits)
                    || !trimmed
                        .get(digits..)
                        .is_some_and(|tail| tail.starts_with(['.', ')']))
                {
                    break;
                }
                digits + 1
            };
            let after = trimmed.split_at(marker).1;
            let spaces = after
                .bytes()
                .take_while(|byte| matches!(byte, b' ' | b'\t'))
                .count();
            if spaces == 0 {
                break;
            }
            let content = after.split_at(spaces).1;
            let prefix = line.split_at(line.len() - content.len()).0;
            let indent = prefix.chars().fold(0, |width, character| {
                if character == '\t' {
                    width + 4 - width % 4
                } else {
                    width + 1
                }
            });
            segments.push(Segment::List(indent));
            line = content;
        }
        (line, Self::new(segments))
    }
    pub(super) fn continuation<'a>(&self, mut line: &'a str) -> Option<&'a str> {
        for segment in &self.0 {
            match segment {
                Segment::Quote => line = quote_content(line)?,
                Segment::List(indent) => {
                    let mut width = 0;
                    let mut offset = 0;
                    for character in line.chars() {
                        if width >= *indent {
                            break;
                        }
                        match character {
                            ' ' => width += 1,
                            '\t' => width += 4 - width % 4,
                            _ => return None,
                        }
                        offset += character.len_utf8();
                    }
                    if width < *indent {
                        return None;
                    }
                    line = line.split_at(offset).1;
                }
            }
        }
        Some(line)
    }
    pub(super) const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    pub(super) fn has_quote(&self) -> bool {
        self.0
            .iter()
            .any(|segment| matches!(segment, Segment::Quote))
    }
    pub(super) fn appended(&self, other: &Self) -> Self {
        Self::new(self.0.iter().chain(&other.0).cloned().collect())
    }
}
fn quote_content(line: &str) -> Option<&str> {
    let trimmed = line.trim_start_matches(' ');
    if line.len() - trimmed.len() > 3 {
        return None;
    }
    let after = trimmed.strip_prefix('>')?;
    Some(
        after
            .strip_prefix(' ')
            .or_else(|| after.strip_prefix('\t'))
            .unwrap_or(after),
    )
}
