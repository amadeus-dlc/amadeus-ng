//! 引用・パラメータ展開・コメントを考慮したシェルコマンド区間。
use core_infrastructure::{
    collections::Collection,
    ecmascript::{is_whitespace, trim},
};
/// 実行位置の区間を文字列の順序付き列で返す。引用やパラメータの内側は分断しない。
#[must_use]
pub fn split_shell_segments(command: &str) -> Collection<String> {
    let chars: Vec<char> = command.chars().collect();
    let mut segments = Vec::new();
    let mut start = 0;
    let mut quote = None;
    let mut depth = 0;
    let mut escaped = false;
    let mut i = 0;
    let push = |segments: &mut Vec<String>, start: usize, end: usize| {
        let part: String = chars.iter().take(end).skip(start).collect();
        let part = trim(&part);
        if !part.is_empty() {
            segments.push(part.to_string());
        }
    };
    while let Some(ch) = chars.get(i).copied() {
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
        if ch == '$' && chars.get(i + 1) == Some(&'{') {
            depth += 1;
            i += 2;
            continue;
        }
        if depth > 0 {
            if ch == '}' {
                depth -= 1;
            }
            i += 1;
            continue;
        }
        if matches!(ch, '\'' | '"') {
            quote = Some(ch);
            i += 1;
            continue;
        }
        if ch == '#'
            && (i == 0
                || chars.get(i - 1).is_some_and(|previous| {
                    is_whitespace(*previous)
                        || matches!(*previous, ';' | '&' | '|' | '(' | ')' | '{' | '}')
                }))
        {
            push(&mut segments, start, i);
            if let Some(next) = chars
                .iter()
                .enumerate()
                .skip(i + 1)
                .find(|(_, ch)| **ch == '\n')
                .map(|(index, _)| index)
            {
                i = next + 1;
                start = i;
                continue;
            }
            start = chars.len();
            break;
        }
        if matches!(ch, ';' | '|' | '&' | '\n' | '(' | ')' | '{' | '}') {
            push(&mut segments, start, i);
            if matches!(ch, '|' | '&') && chars.get(i + 1) == Some(&ch) {
                i += 1;
            }
            start = i + 1;
        }
        i += 1;
    }
    push(&mut segments, start, chars.len());
    Collection::new(segments)
}

#[cfg(test)]
mod tests {
    use super::split_shell_segments;

    fn segments(command: &str) -> Vec<String> {
        split_shell_segments(command).fold_left(Vec::new(), |mut acc, segment| {
            acc.push(segment.to_string());
            acc
        })
    }

    #[test]
    fn separators_split_and_empty_segments_are_dropped() {
        assert_eq!(segments("rm a && rm b; (rm c)"), ["rm a", "rm b", "rm c"]);
        assert_eq!(segments("rm a | tee b\nrm c"), ["rm a", "tee b", "rm c"]);
        assert!(segments("  ;; ").is_empty());
    }

    #[test]
    fn a_separator_inside_a_parameter_expansion_or_a_quote_does_not_split() {
        assert_eq!(segments("echo ${A;B}; rm x"), ["echo ${A;B}", "rm x"]);
        assert_eq!(segments("echo ${A${B}}; rm x"), ["echo ${A${B}}", "rm x"]);
        assert_eq!(segments("echo 'a;b'; rm x"), ["echo 'a;b'", "rm x"]);
        assert_eq!(segments(r"echo a\;b; rm x"), [r"echo a\;b", "rm x"]);
    }

    #[test]
    fn a_comment_ends_the_segment_until_the_next_line() {
        assert_eq!(segments("rm x # note; rm y\nrm z"), ["rm x", "rm z"]);
        assert_eq!(segments("rm x;#note\nrm y"), ["rm x", "rm y"]);
        assert_eq!(segments("#only a comment"), Vec::<String>::new());
        assert_eq!(segments("rm x # trailing"), ["rm x"]);
        // 語の途中の `#` はコメントにならない。
        assert_eq!(segments("rm a#b; rm y"), ["rm a#b", "rm y"]);
    }
}
