//! リダイレクト演算子を取り除いたシェル語列。
use core_infrastructure::{collections::FirstClassCollection, ecmascript::is_whitespace};

/// コマンド区間 1 つを語へ分けた列。**リダイレクトは語に残さない**。
///
/// upstream `hooks/review-freeze-command.ts` の `shellWords` に対応する。同名の
/// [`ShellWords`](crate::ShellWords) とは別の字句規則であり、混ぜて使えない —
/// あちらは `;` `|` `&` `<` `>` を語の一部として残し、こちらは語の区切りとして扱う。
///
/// リダイレクト (`>`, `>>`, `>|`, `<`, `<<`, `2>`, `2>&1`, `&>`) は演算子ごと読み飛ばし、
/// 記述子番号 (`2>` の `2`) も語にしない。これはコマンドの操作対象だけを見るためであり、
/// リダイレクト**先**は [`ShellWriteTargets`](crate::ShellWriteTargets) が原文の走査で別に拾う。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RedirectionFreeWords {
    words: Vec<String>,
}
impl RedirectionFreeWords {
    /// 語の順序を固定する (**この型の唯一の構築経路**)。
    const fn new(words: Vec<String>) -> Self {
        Self { words }
    }
    /// 引用・エスケープをほどき、リダイレクトを落として語へ分ける。シェルを実行しない。
    pub(crate) fn parse(command: &str) -> Self {
        let chars: Vec<char> = command.chars().collect();
        let mut words: Vec<String> = Vec::new();
        let mut word = String::new();
        let mut quote: Option<char> = None;
        let mut escaped = false;
        let mut i = 0;
        while let Some(ch) = chars.get(i).copied() {
            if escaped {
                word.push(ch);
                escaped = false;
                i += 1;
                continue;
            }
            // `"` の中の `\` は `$` `` ` `` `"` `\` 改行 の前でだけ逃がしになる (bash)。
            if ch == '\\'
                && quote == Some('"')
                && !matches!(
                    chars.get(i + 1).copied(),
                    Some('$' | '`' | '"' | '\\' | '\n')
                )
            {
                word.push(ch);
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
                } else {
                    word.push(ch);
                }
                i += 1;
                continue;
            }
            if matches!(ch, '\'' | '"') {
                quote = Some(ch);
                i += 1;
                continue;
            }
            if matches!(ch, '>' | '<') {
                // `[n]>word` / `[n]<word` の裸の記述子番号は演算子側に属し、
                // `>&n` `>&-` `<&n` `<&-` は記述子の複製・閉鎖でファイルを名指さない。
                if !word.is_empty() && word.chars().all(|digit| digit.is_ascii_digit()) {
                    word.clear();
                }
                push(&mut words, &mut word);
                let mut end = i + 1;
                if chars.get(end).copied() == Some(ch) {
                    end += 1;
                }
                if ch == '>' && chars.get(end).copied() == Some('|') {
                    end += 1;
                }
                if chars.get(end).copied() == Some('&') {
                    end += 1 + descriptor_length(&chars, end + 1);
                }
                i = end;
                continue;
            }
            if ch == '&' && chars.get(i + 1).copied() == Some('>') {
                // `&>word` / `&>>word` (bash): 両方の流れをファイルへ。`>` は次の周回で読む。
                push(&mut words, &mut word);
                i += 1;
                continue;
            }
            if is_whitespace(ch) || matches!(ch, ';' | '|' | '&' | '(' | ')') {
                push(&mut words, &mut word);
                i += 1;
                continue;
            }
            word.push(ch);
            i += 1;
        }
        push(&mut words, &mut word);
        Self::new(words)
    }
    /// 挿入順の位置。範囲外は `None`。
    pub(crate) fn at(&self, index: usize) -> Option<&str> {
        self.words.get(index).map(String::as_str)
    }
    /// 語数。
    pub(crate) const fn len(&self) -> usize {
        self.words.len()
    }
    /// 指定位置以後を同じ語順で取り出す。
    pub(crate) fn suffix(&self, start: usize) -> Vec<String> {
        self.words.iter().skip(start).cloned().collect()
    }
    /// 先頭に別の語列を継ぎ、指定位置以後を残した列。
    pub(crate) fn spliced(&self, prefix: &[String], start: usize) -> Self {
        Self::new(
            prefix
                .iter()
                .cloned()
                .chain(self.words.iter().skip(start).cloned())
                .collect(),
        )
    }
}
/// 空でない語だけを列へ移す。
fn push(words: &mut Vec<String>, word: &mut String) {
    if !word.is_empty() {
        words.push(std::mem::take(word));
    }
}

/// `>&` `<&` の直後に続く記述子綴り (`1` / `-`) の長さ。名指しがファイルなら `0`。
///
/// 記述子は区切り (空白・`;` `|` `&` `(` `)` `<` `>`) か文字列末尾で終わらなければならない。
/// upstream の `/^(?:\d+|-)(?![^\s;|&()<>])/` に対応する。
fn descriptor_length(chars: &[char], start: usize) -> usize {
    let length = if chars.get(start).copied() == Some('-') {
        1
    } else {
        chars
            .iter()
            .skip(start)
            .take_while(|ch| ch.is_ascii_digit())
            .count()
    };
    if length == 0 {
        return 0;
    }
    match chars.get(start + length).copied() {
        None => length,
        Some(next)
            if is_whitespace(next) || matches!(next, ';' | '|' | '&' | '(' | ')' | '<' | '>') =>
        {
            length
        }
        Some(_) => 0,
    }
}

impl FirstClassCollection for RedirectionFreeWords {
    type Item<'a> = &'a str;
    type Filtered = Self;
    fn len(&self) -> usize {
        Self::len(self)
    }
    fn at(&self, index: usize) -> Option<&str> {
        Self::at(self, index)
    }
    fn fold_left<'a, A>(&'a self, initial: A, mut fold: impl FnMut(A, &'a str) -> A) -> A {
        self.words.iter().fold(initial, |acc, word| fold(acc, word))
    }
    fn filter(&self, mut predicate: impl FnMut(&str) -> bool) -> Self {
        Self::new(
            self.words
                .iter()
                .filter(|word| predicate(word))
                .cloned()
                .collect(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::RedirectionFreeWords;

    fn words(command: &str) -> Vec<String> {
        RedirectionFreeWords::parse(command).suffix(0)
    }

    #[test]
    fn quotes_and_escapes_are_unwrapped_into_single_words() {
        assert_eq!(
            words(r#"echo "a b" 'c d' e\ f"#),
            ["echo", "a b", "c d", "e f"]
        );
    }

    #[test]
    fn a_backslash_inside_double_quotes_survives_unless_it_escapes_a_special() {
        // bash: `"` の中の `\` は $ ` " \ 改行 の前でだけ逃がしになる。
        assert_eq!(words(r#"printf "a\nb""#), ["printf", r"a\nb"]);
        assert_eq!(words(r#"printf "a\"b""#), ["printf", "a\"b"]);
    }

    #[test]
    fn a_compact_append_redirection_is_not_a_word() {
        assert_eq!(words("printf x>>file"), ["printf", "x", "file"]);
        assert_eq!(words("printf x > file"), ["printf", "x", "file"]);
    }

    #[test]
    fn a_descriptor_number_belongs_to_the_operator_and_is_dropped() {
        assert_eq!(words("cmd 2>/dev/null"), ["cmd", "/dev/null"]);
        assert_eq!(words("cmd 2>>log"), ["cmd", "log"]);
    }

    #[test]
    fn a_descriptor_duplication_names_no_word_at_all() {
        // 本家の既知不具合の是正 (`scripts/aidlc-sync/patches/shell-redirection-tokens.patch`) —
        // `2>&1` の `&` を区切りと誤認して幽霊コマンド `1` を作らない。
        assert_eq!(words("rm x 2>&1"), ["rm", "x"]);
        assert_eq!(words("rm x 2>&-"), ["rm", "x"]);
        assert_eq!(words("rm x <&0"), ["rm", "x"]);
    }

    #[test]
    fn a_descriptor_duplication_ends_at_a_separator_or_the_end_of_the_string() {
        // `2>&1` の `1` は区切り (`;` `|` 空白) か終端で終わるときだけ記述子である。
        assert_eq!(words("cmd 2>&1;rm b"), ["cmd", "rm", "b"]);
        assert_eq!(words("cmd 2>&1|tee b"), ["cmd", "tee", "b"]);
        assert_eq!(words("cmd 2>&1 rm b"), ["cmd", "rm", "b"]);
        assert_eq!(words("cmd >&-"), ["cmd"]);
        // 数字の後ろに区切り以外が続けば記述子ではなくファイル名である。
        assert_eq!(words("cmd >&1x"), ["cmd", "1x"]);
        assert_eq!(words("cmd >&-x"), ["cmd", "-x"]);
    }

    #[test]
    fn the_collection_exposes_count_index_fold_and_filter() {
        use core_infrastructure::collections::FirstClassCollection;
        let words = RedirectionFreeWords::parse("rm -rf a.md b.txt");
        assert_eq!(FirstClassCollection::len(&words), 4);
        assert_eq!(FirstClassCollection::at(&words, 1), Some("-rf"));
        assert_eq!(FirstClassCollection::at(&words, 4), None);
        let joined = words.fold_left(String::new(), |acc, word| format!("{acc}{word} "));
        assert_eq!(joined, "rm -rf a.md b.txt ");
        let markdown = words.filter(|word| word.ends_with(".md"));
        assert_eq!(markdown.suffix(0), ["a.md"]);
        assert_eq!(RedirectionFreeWords::parse("").len(), 0);
    }

    #[test]
    fn a_descriptor_position_that_names_a_file_keeps_the_file_as_a_word() {
        assert_eq!(words("cmd >&file"), ["cmd", "file"]);
        assert_eq!(words("cmd &>log"), ["cmd", "log"]);
        assert_eq!(words("cmd &>>log"), ["cmd", "log"]);
        assert_eq!(words("cmd >|clobber"), ["cmd", "clobber"]);
    }

    #[test]
    fn separators_end_a_word_without_becoming_one() {
        assert_eq!(words("rm a;rm b"), ["rm", "a", "rm", "b"]);
        assert_eq!(words("(rm a)"), ["rm", "a"]);
        assert_eq!(words("rm a|tee b"), ["rm", "a", "tee", "b"]);
    }

    #[test]
    fn an_empty_spelling_never_becomes_a_word() {
        assert_eq!(words(""), Vec::<String>::new());
        assert_eq!(words(r#"rm "" x"#), ["rm", "x"]);
    }

    #[test]
    fn a_line_continuation_is_a_plain_escape_rather_than_a_join() {
        // 本家の `shellWords` は `\` を単なる逃がしとして扱う。改行は語の一部になり、
        // 実際の bash のように行が繋がるわけではない。既存の `ShellWords` とはここが違う。
        assert_eq!(words("rm a\\\nb"), ["rm", "a\nb"]);
    }

    #[test]
    fn an_ansi_c_quote_is_read_as_a_dollar_then_a_quote() {
        // 本家は `$'...'` を特別扱いしない。`$` は普通の文字で、`'` が引用を開く。
        assert_eq!(words("rm $'a b'"), ["rm", "$a b"]);
    }

    #[test]
    fn a_heredoc_operator_is_dropped_and_its_tag_stays_a_word() {
        // 本家は heredoc 本文を隠さない。ここも同じで、演算子だけを落とす。
        assert_eq!(words("cat <<EOF"), ["cat", "EOF"]);
    }
}
