//! リダイレクトを分断しないコマンド区間の分割。
use core_infrastructure::collections::Collection;

/// 実行位置ごとの区間を、原文の綴りのまま順序付きで返す。
///
/// upstream `hooks/review-freeze-command.ts` の `shellCommandSegments` に対応する。
/// [`split_shell_segments`](crate::split_shell_segments) とは別の分割規則である —
/// あちらは `(` `)` `{` `}` とコメントでも切り、こちらは `;` 改行 `|` `&` だけで切る。
///
/// `&` は常に区切りではない。`2>&1` `<&0` の `&` は記述子の複製であり、`&>log` の `&` は
/// 両方の流れをファイルへ送る綴りの一部である。どちらもコマンドを分けない
/// (`scripts/aidlc-sync/patches/shell-redirection-tokens.patch` の是正)。
#[must_use]
pub(crate) fn split_command_segments(command: &str) -> Collection<String> {
    let chars: Vec<char> = command.chars().collect();
    let mut segments: Vec<String> = Vec::new();
    let mut start = 0;
    let mut quote: Option<char> = None;
    let mut escaped = false;
    // 直前の、引用にもエスケープにも属さない 1 文字。`>` `<` の直後の `&` は記述子の複製で、
    // `>` の直前の `&` は両方の流れの送り先である。どちらもコマンドを分けない。
    let mut previous: Option<char> = None;
    let mut i = 0;
    while let Some(ch) = chars.get(i).copied() {
        if escaped {
            escaped = false;
            previous = None;
            i += 1;
            continue;
        }
        if ch == '\\'
            && quote == Some('"')
            && !matches!(
                chars.get(i + 1).copied(),
                Some('$' | '`' | '"' | '\\' | '\n')
            )
        {
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
        if matches!(ch, '\'' | '"') {
            quote = Some(ch);
            previous = None;
            i += 1;
            continue;
        }
        let redirection = ch == '&'
            && (previous == Some('>')
                || previous == Some('<')
                || chars.get(i + 1).copied() == Some('>'));
        previous = Some(ch);
        if redirection || !matches!(ch, ';' | '\n' | '|' | '&') {
            i += 1;
            continue;
        }
        segments.push(chars.iter().skip(start).take(i - start).collect());
        if matches!(ch, '|' | '&') && chars.get(i + 1).copied() == Some(ch) {
            i += 1;
        }
        start = i + 1;
        i += 1;
    }
    segments.push(chars.iter().skip(start).collect());
    Collection::new(segments)
}

#[cfg(test)]
mod tests {
    use super::split_command_segments;

    fn segments(command: &str) -> Vec<String> {
        split_command_segments(command).fold_left(Vec::new(), |mut acc, segment| {
            acc.push(segment.to_string());
            acc
        })
    }

    #[test]
    fn a_chain_is_split_at_each_operator() {
        assert_eq!(segments("rm a && rm b"), ["rm a ", " rm b"]);
        assert_eq!(segments("rm a; rm b"), ["rm a", " rm b"]);
        assert_eq!(segments("rm a\nrm b"), ["rm a", "rm b"]);
        assert_eq!(segments("rm a | tee b"), ["rm a ", " tee b"]);
        assert_eq!(segments("rm a & rm b"), ["rm a ", " rm b"]);
    }

    #[test]
    fn a_descriptor_duplication_never_splits_a_command() {
        // 是正前の本家はここで幽霊コマンド `1` を作っていた。
        assert_eq!(segments("rm x 2>&1"), ["rm x 2>&1"]);
        assert_eq!(segments("cat f <&0"), ["cat f <&0"]);
    }

    #[test]
    fn a_both_streams_redirection_never_splits_a_command() {
        assert_eq!(segments("rm x &> log"), ["rm x &> log"]);
        assert_eq!(segments("rm x &>> log"), ["rm x &>> log"]);
    }

    #[test]
    fn a_backslash_inside_double_quotes_is_literal_before_an_ordinary_character() {
        // bash: `"` の中の `\` は `$` `` ` `` `"` `\` 改行 の前でだけ逃がしになる。
        // `\n` の `\` は文字として残り、閉じ引用は引用のまま読まれるので `;` で切れる。
        assert_eq!(
            segments("echo \"a\\nb\"; rm x"),
            ["echo \"a\\nb\"", " rm x"]
        );
        assert_eq!(segments("echo \"a\\\"; b\""), ["echo \"a\\\"; b\""]);
    }

    #[test]
    fn operators_inside_quotes_and_after_an_escape_are_literal() {
        assert_eq!(segments(r#"echo "a && b""#), [r#"echo "a && b""#]);
        assert_eq!(segments(r"echo a\&\&b"), [r"echo a\&\&b"]);
        assert_eq!(segments("echo 'a | b'"), ["echo 'a | b'"]);
    }

    #[test]
    fn the_whole_command_is_one_segment_when_nothing_separates_it() {
        assert_eq!(segments("rm -rf /x"), ["rm -rf /x"]);
        assert_eq!(segments(""), [""]);
    }

    #[test]
    fn an_escaped_newline_does_not_split_because_the_escape_consumes_it() {
        assert_eq!(segments("rm a\\\nrm b"), ["rm a\\\nrm b"]);
    }

    #[test]
    fn a_subshell_is_not_split_because_parentheses_are_not_separators() {
        assert_eq!(segments("(rm a)"), ["(rm a)"]);
    }
}
