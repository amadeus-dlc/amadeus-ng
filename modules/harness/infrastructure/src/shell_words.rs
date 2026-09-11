//! 引用とエスケープをほどいた、順序付きシェル語列。
use core_infrastructure::{collections::FirstClassCollection, ecmascript::is_whitespace};
/// 空を許し、語順と空の引用語を保持する。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellWords {
    words: Vec<String>,
}
impl ShellWords {
    /// 語の順序を固定する。
    #[must_use]
    pub const fn new(words: Vec<String>) -> Self {
        Self { words }
    }
    /// 引用された文字列を一語として解析する。シェルを実行しない。
    #[must_use]
    pub fn parse(input: &str) -> Self {
        let chars: Vec<char> = input.chars().collect();
        let mut words = Vec::new();
        let mut word = String::new();
        let mut quote = None;
        let mut escaped = false;
        let mut started = false;
        let mut i = 0;
        while let Some(ch) = chars.get(i).copied() {
            if ch == '\\' && chars.get(i + 1) == Some(&'\n') {
                i += 2;
                continue;
            }
            if escaped {
                word.push(ch);
                escaped = false;
                started = true;
                i += 1;
                continue;
            }
            if ch == '\\' && quote != Some('\'') {
                escaped = true;
                started = true;
                i += 1;
                continue;
            }
            if let Some(active) = quote {
                if ch == active {
                    quote = None;
                } else {
                    word.push(ch);
                }
                started = true;
                i += 1;
                continue;
            }
            if ch == '$' && matches!(chars.get(i + 1), Some('\'' | '"')) {
                quote = chars.get(i + 1).copied();
                started = true;
                i += 2;
                continue;
            }
            if matches!(ch, '\'' | '"') {
                quote = Some(ch);
                started = true;
            } else if is_whitespace(ch) {
                if started {
                    words.push(std::mem::take(&mut word));
                    started = false;
                }
            } else {
                word.push(ch);
                started = true;
            }
            i += 1;
        }
        if escaped {
            word.push('\\');
        }
        if started {
            words.push(word);
        }
        Self::new(words)
    }
    /// 挿入順の位置。範囲外はNone。
    #[must_use]
    pub fn at(&self, index: usize) -> Option<&str> {
        self.words.get(index).map(String::as_str)
    }
    /// 語数。
    #[must_use]
    pub const fn len(&self) -> usize {
        self.words.len()
    }
    /// 空か。
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.words.is_empty()
    }
    /// 指定位置以後を同じ語順で取り出す。
    #[must_use]
    pub fn suffix(&self, start: usize) -> Self {
        Self::new(self.words.iter().skip(start).cloned().collect())
    }
    /// 指定範囲を語列で置換する。範囲外の位置は末尾へ丸める。
    #[must_use]
    pub fn replace_range(&self, start: usize, end: usize, replacement: &Self) -> Self {
        let start = start.min(self.words.len());
        let end = end.max(start).min(self.words.len());
        Self::new(
            self.words
                .iter()
                .take(start)
                .chain(&replacement.words)
                .chain(self.words.iter().skip(end))
                .cloned()
                .collect(),
        )
    }
    /// 語を指定区切りで再構成する。
    #[must_use]
    pub fn join(&self, separator: &str) -> String {
        self.words.join(separator)
    }
}
impl FirstClassCollection for ShellWords {
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
    use super::ShellWords;
    use core_infrastructure::collections::FirstClassCollection;

    fn words(input: &str) -> Vec<String> {
        ShellWords::parse(input).fold_left(Vec::new(), |mut acc: Vec<String>, word| {
            acc.push(word.to_string());
            acc
        })
    }

    #[test]
    fn an_ansi_c_or_locale_quote_opens_a_quoted_word() {
        assert_eq!(words("echo $'a b' $\"c d\""), ["echo", "a b", "c d"]);
        // `$` の後ろが引用でなければ普通の文字である。
        assert_eq!(words("echo $HOME"), ["echo", "$HOME"]);
    }

    #[test]
    fn a_trailing_backslash_is_kept_as_a_literal_backslash() {
        assert_eq!(words("echo a\\"), ["echo", "a\\"]);
        assert_eq!(words("echo a\\ b"), ["echo", "a b"]);
        assert_eq!(words("\\"), ["\\"]);
    }

    #[test]
    fn a_line_continuation_is_folded_and_an_empty_quote_stays_a_word() {
        assert_eq!(words("echo a \\\nb"), ["echo", "a", "b"]);
        assert_eq!(words("echo \"\" x"), ["echo", "", "x"]);
    }

    #[test]
    fn the_collection_exposes_count_index_and_filter() {
        let parsed = ShellWords::parse("rm -rf a.md b.txt");
        assert_eq!(FirstClassCollection::len(&parsed), 4);
        assert_eq!(FirstClassCollection::at(&parsed, 1), Some("-rf"));
        assert_eq!(FirstClassCollection::at(&parsed, 4), None);
        let markdown = parsed.filter(|word| word.ends_with(".md"));
        assert_eq!(markdown.join(","), "a.md");
        assert!(parsed.filter(|_| false).is_empty());
    }

    #[test]
    fn a_range_replacement_clamps_to_the_word_count() {
        let parsed = ShellWords::parse("a b c");
        let replaced = parsed.replace_range(1, 2, &ShellWords::parse("x y"));
        assert_eq!(replaced.join(" "), "a x y c");
        let clamped = parsed.replace_range(5, 1, &ShellWords::parse("z"));
        assert_eq!(clamped.join(" "), "a b c z");
        assert_eq!(parsed.suffix(2).join(" "), "c");
        assert!(parsed.suffix(9).is_empty());
    }
}
