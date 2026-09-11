//! 実行されるコマンド置換と、それを隠した外側の文字列。
use core_infrastructure::collections::Collection;
/// 本文は出現順。単一引用の中の置換は実行対象にしない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellSubstitutions {
    masked: String,
    bodies: Collection<String>,
}
impl ShellSubstitutions {
    /// バッククォートと$()の本文を読み取る。プロセス実行はしない。
    #[must_use]
    pub fn parse(command: &str) -> Self {
        let units: Vec<u16> = command.encode_utf16().collect();
        let mut chars: Vec<Option<char>> = command.chars().map(Some).collect();
        let mut bodies = Vec::new();
        let mut quote = None;
        let mut escaped = false;
        let mut i = 0;
        while let Some(ch) = unit(&units, i) {
            if escaped {
                escaped = false;
                i += 1;
                continue;
            }
            if ch == '\\' && quote != Some('\'') {
                escaped = true;
                i += 1;
                continue;
            }
            if quote == Some('\'') {
                if ch == '\'' {
                    quote = None;
                }
                i += 1;
                continue;
            }
            if ch == '\'' && quote.is_none() {
                quote = Some('\'');
                i += 1;
                continue;
            }
            if ch == '"' {
                quote = if quote == Some('"') { None } else { Some('"') };
                i += 1;
                continue;
            }
            if ch == '`' {
                let mut end = i + 1;
                let mut inner_escaped = false;
                while let Some(inner) = unit(&units, end) {
                    if inner_escaped {
                        inner_escaped = false;
                        end += 1;
                        continue;
                    }
                    if inner == '\\' {
                        inner_escaped = true;
                        end += 1;
                        continue;
                    }
                    if inner == '`' {
                        break;
                    }
                    end += 1;
                }
                if end >= units.len() {
                    i += 1;
                    continue;
                }
                bodies.push(substring(&units, i + 1, end));
                mask(&mut chars, i, end);
                if let Some(ch) = chars.get_mut(i) {
                    *ch = Some('$');
                }
                i = end + 1;
                continue;
            }
            if ch == '$'
                && unit(&units, i + 1) == Some('(')
                && let Some(end) = substitution_end(&units, i + 1)
            {
                bodies.push(substring(&units, i + 2, end));
                mask(&mut chars, i, end);
                if let Some(ch) = chars.get_mut(i) {
                    *ch = Some('$');
                }
                i = end + 1;
                continue;
            }
            i += 1;
        }
        Self {
            masked: chars.into_iter().flatten().collect(),
            bodies: Collection::new(bodies),
        }
    }
    /// 置換を隠した外側の文字列。
    #[must_use]
    pub fn masked(&self) -> &str {
        &self.masked
    }
    /// 出現順の置換本文。コレクションの操作で処理する。
    #[must_use]
    pub const fn bodies(&self) -> &Collection<String> {
        &self.bodies
    }
}
fn unit(units: &[u16], index: usize) -> Option<char> {
    units
        .get(index)
        .map(|unit| char::from_u32(u32::from(*unit)).unwrap_or('\u{fffd}'))
}
fn substring(units: &[u16], start: usize, end: usize) -> String {
    String::from_utf16_lossy(
        &units
            .iter()
            .skip(start)
            .take(end.saturating_sub(start))
            .copied()
            .collect::<Vec<_>>(),
    )
}
fn mask(chars: &mut Vec<Option<char>>, start: usize, end: usize) {
    // UTF-16の文字位置と文字配列の位置を分ける。範囲外の配列位置は空要素になる。
    if chars.len() <= end {
        chars.resize(end + 1, None);
    }
    for ch in chars.iter_mut().take(end + 1).skip(start) {
        if *ch != Some('\n') {
            *ch = Some(' ');
        }
    }
}
fn substitution_end(units: &[u16], open: usize) -> Option<usize> {
    let mut depth = 1;
    let mut quote = None;
    let mut escaped = false;
    let mut i = open + 1;
    while let Some(ch) = unit(units, i) {
        if escaped {
            escaped = false;
            i += 1;
            continue;
        }
        if ch == '\\' && quote != Some('\'') {
            escaped = true;
            i += 1;
            continue;
        }
        if let Some(active) = quote {
            if ch == active {
                quote = None;
            }
            i += 1;
            continue;
        }
        if matches!(ch, '\'' | '"' | '`') {
            quote = Some(ch);
            i += 1;
            continue;
        }
        if ch == '(' {
            depth += 1;
        }
        if ch == ')' {
            depth -= 1;
            if depth == 0 {
                return Some(i);
            }
        }
        i += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::ShellSubstitutions;

    fn bodies(command: &str) -> Vec<String> {
        ShellSubstitutions::parse(command).bodies().fold_left(
            Vec::new(),
            |mut acc: Vec<String>, body| {
                acc.push(body.clone());
                acc
            },
        )
    }

    #[test]
    fn a_dollar_paren_and_a_backquote_body_are_read_and_masked() {
        let parsed = ShellSubstitutions::parse("echo $(rm a) `rm b`");
        assert_eq!(bodies("echo $(rm a) `rm b`"), ["rm a", "rm b"]);
        assert_eq!(parsed.masked(), "echo $       $     ");
    }

    #[test]
    fn a_body_inside_single_quotes_is_not_executed() {
        assert!(bodies("echo '$(rm a)'").is_empty());
        assert!(bodies("echo '`rm a`'").is_empty());
        assert_eq!(bodies("echo \"$(rm a)\""), ["rm a"]);
    }

    #[test]
    fn a_backslash_escapes_the_substitution_opener_outside_single_quotes() {
        assert!(bodies("echo \\$(rm a)").is_empty());
        assert!(bodies("echo \\`rm a\\`").is_empty());
    }

    #[test]
    fn an_escaped_backquote_stays_inside_the_backquote_body() {
        assert_eq!(bodies("echo `a\\`b`"), ["a\\`b"]);
    }

    #[test]
    fn an_unterminated_substitution_is_left_untouched() {
        for command in ["echo `rm a", "echo $(rm a", "echo $(rm 'a)"] {
            let parsed = ShellSubstitutions::parse(command);
            assert!(parsed.bodies().is_empty(), "{command}");
            assert_eq!(parsed.masked(), command, "{command}");
        }
    }

    #[test]
    fn a_dollar_paren_body_honours_nesting_quotes_and_escapes() {
        assert_eq!(bodies("$( (rm a) )"), [" (rm a) "]);
        assert_eq!(bodies("$(echo ')' x)"), ["echo ')' x"]);
        assert_eq!(bodies("$(echo \")\" x)"), ["echo \")\" x"]);
        assert_eq!(bodies("$(echo `)` x)"), ["echo `)` x"]);
        assert_eq!(bodies("$(echo \\) x)"), ["echo \\) x"]);
        // 単一引用の中では `\\` は逃がしにならず、引用が閉じた後の `)` で終わる。
        assert_eq!(bodies("$(echo '\\') x)"), ["echo '\\'"]);
        assert_eq!(bodies("$(echo \"\\)\" x)"), ["echo \"\\)\" x"]);
    }

    #[test]
    fn a_body_after_a_surrogate_pair_is_still_read() {
        // UTF-16 の位置と文字配列の位置がずれても本文は落とさない。
        assert_eq!(bodies("echo 😀 $(rm a)"), ["rm a"]);
        let parsed = ShellSubstitutions::parse("😀$(rm a)");
        assert!(!parsed.masked().contains("rm a"), "{:?}", parsed.masked());
    }

    #[test]
    fn a_newline_inside_a_body_survives_masking() {
        let parsed = ShellSubstitutions::parse("a $(b\nc) d");
        assert_eq!(parsed.masked(), "a $  \n   d");
        assert_eq!(bodies("a $(b\nc) d"), ["b\nc"]);
    }
}
