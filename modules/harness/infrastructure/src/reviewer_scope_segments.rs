//! reviewer-scope の字句規則で読んだ、シェルコマンド 1 本の実行位置の列。
use super::reviewer_scope_words::ReviewerScopeWords;
use core_infrastructure::collections::FirstClassCollection;

/// 語を切り、実行位置を分ける 5 文字 (upstream `";|&()".includes(ch)`)。
const SEPARATORS: &str = ";|&()";

/// 溜めた綴りを 1 語として積む。空の綴りは語にしない (upstream `pushWord`)。
fn push_word(current: &mut Vec<String>, word: &mut String) {
    if !word.is_empty() {
        current.push(std::mem::take(word));
    }
}

/// 溜めた語を 1 実行位置として積む。空の区間は落とす (upstream `shellSegments`)。
fn push_segment(segments: &mut Vec<ReviewerScopeWords>, current: &mut Vec<String>) {
    if !current.is_empty() {
        segments.push(ReviewerScopeWords::new(std::mem::take(current)));
    }
}

/// 引用を 1 つ読み切り、閉じ引用の位置 (無ければ終端) を返す。
///
/// `"` の中だけ `\` が次の 1 文字を逃がす (upstream の非対称をそのまま写す)。
fn read_quoted(characters: &[char], open: usize, quote: char, word: &mut String) -> usize {
    let mut index = open + 1;
    while let Some(&character) = characters.get(index) {
        if character == quote {
            return index;
        }
        if quote == '"' && character == '\\' && index + 1 < characters.len() {
            index += 1;
        }
        if let Some(&escaped) = characters.get(index) {
            word.push(escaped);
        }
        index += 1;
    }
    // 閉じない引用は終端まで食べる。呼出側が `index += 1` するので 1 手前を返す。
    characters.len().saturating_sub(1)
}

/// 区切り (`;` `|` `&` `(` `)`) で分けた実行位置の列。
///
/// upstream `aidlc-reviewer-scope.ts` の `shellTokens` + `shellSegments` に対応する。
/// **シェルを実行しない** — 語に切って区間へ分けるだけである。判定 (どの語が経路か、
/// どのコマンドがどう読むか) は持たない。それはドメインの読み取り範囲の規則であり、
/// `core_command_domain::orchestration::ReviewerScope` が持つ。
///
/// 空の区間は落とす (upstream `if (current.length > 0)`)。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ReviewerScopeSegments {
    items: Vec<ReviewerScopeWords>,
}
impl ReviewerScopeSegments {
    /// 実行位置の列を固定する (**この型の唯一の構築経路**)。
    const fn new(items: Vec<ReviewerScopeWords>) -> ReviewerScopeSegments {
        ReviewerScopeSegments { items }
    }
    /// コマンド文字列を語へ切り、区切りで実行位置へ分ける。
    #[must_use]
    pub fn parse(command: &str) -> ReviewerScopeSegments {
        let mut segments: Vec<ReviewerScopeWords> = Vec::new();
        let mut current: Vec<String> = Vec::new();
        let mut word = String::new();
        let characters: Vec<char> = command.chars().collect();
        let mut index = 0;
        while let Some(&character) = characters.get(index) {
            if character.is_whitespace() {
                push_word(&mut current, &mut word);
            } else if character == '\'' || character == '"' {
                index = read_quoted(&characters, index, character, &mut word);
            } else if character == '\\' {
                if let Some(&escaped) = characters.get(index + 1) {
                    index += 1;
                    word.push(escaped);
                }
            } else if SEPARATORS.contains(character) {
                push_word(&mut current, &mut word);
                // `&&` `||` は 1 つの区切り。`;;` は upstream も 2 つ数えるが、空の
                // 区間は落とされるので観測は変わらない。
                if (character == '|' || character == '&')
                    && characters.get(index + 1) == Some(&character)
                {
                    index += 1;
                }
                push_segment(&mut segments, &mut current);
            } else {
                word.push(character);
            }
            index += 1;
        }
        push_word(&mut current, &mut word);
        push_segment(&mut segments, &mut current);
        ReviewerScopeSegments::new(segments)
    }
    /// 実行位置の件数。
    #[must_use]
    pub const fn len(&self) -> usize {
        self.items.len()
    }
    /// 実行位置が 1 つも無いか。
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
    /// 出現順の添字参照。範囲外は `None`。
    #[must_use]
    pub fn at(&self, index: usize) -> Option<&ReviewerScopeWords> {
        self.items.get(index)
    }
}
impl FirstClassCollection for ReviewerScopeSegments {
    type Item<'a> = &'a ReviewerScopeWords;
    type Filtered = Self;
    fn len(&self) -> usize {
        ReviewerScopeSegments::len(self)
    }
    fn at(&self, index: usize) -> Option<&ReviewerScopeWords> {
        ReviewerScopeSegments::at(self, index)
    }
    fn fold_left<'a, A>(
        &'a self,
        initial: A,
        fold: impl FnMut(A, &'a ReviewerScopeWords) -> A,
    ) -> A {
        self.items.iter().fold(initial, fold)
    }
    fn filter(&self, mut predicate: impl FnMut(&ReviewerScopeWords) -> bool) -> Self {
        ReviewerScopeSegments::new(
            self.items
                .iter()
                .filter(|words| predicate(words))
                .cloned()
                .collect(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::ReviewerScopeSegments;

    /// 区間ごとの語を素朴な形へ落として突き合わせる。
    fn read(command: &str) -> Vec<Vec<String>> {
        let segments = ReviewerScopeSegments::parse(command);
        (0..segments.len())
            .filter_map(|index| segments.at(index))
            .map(|words| {
                (0..words.len())
                    .filter_map(|index| words.at(index))
                    .map(str::to_string)
                    .collect()
            })
            .collect()
    }

    fn one(command: &str) -> Vec<String> {
        read(command).into_iter().next().unwrap_or_default()
    }

    #[test]
    fn the_collection_exposes_count_index_fold_and_filter() {
        use core_infrastructure::collections::FirstClassCollection;
        let segments = ReviewerScopeSegments::parse("cat a.md && rm b.md; ls");
        assert_eq!(FirstClassCollection::len(&segments), 3);
        assert!(!segments.is_empty());
        assert_eq!(
            FirstClassCollection::at(&segments, 1).and_then(|words| words.at(0)),
            Some("rm")
        );
        assert_eq!(
            FirstClassCollection::at(&segments, 3).and_then(|words| words.at(0)),
            None
        );
        let names = segments.fold_left(Vec::new(), |mut acc: Vec<String>, words| {
            acc.extend(words.at(0).map(str::to_string));
            acc
        });
        assert_eq!(names, ["cat", "rm", "ls"]);
        let two_words = segments.filter(|words| words.len() == 2);
        assert_eq!(two_words.len(), 2);
        assert_eq!(two_words.at(1).and_then(|words| words.at(1)), Some("b.md"));
        let empty = ReviewerScopeSegments::parse("   ");
        assert!(empty.is_empty());
        assert_eq!(empty.len(), 0);
    }

    #[test]
    fn whitespace_separates_words_and_the_edges_are_dropped() {
        assert_eq!(one("  cat   /r/a.md  "), ["cat", "/r/a.md"]);
        assert_eq!(one("cat\t/r/a.md"), ["cat", "/r/a.md"]);
    }

    #[test]
    fn quotes_are_unwrapped_and_an_unterminated_quote_runs_to_the_end() {
        assert_eq!(one("cat '/r/a b.md'"), ["cat", "/r/a b.md"]);
        assert_eq!(one("cat \"/r/a b.md\""), ["cat", "/r/a b.md"]);
        assert_eq!(one("cat \"/r/a.md"), ["cat", "/r/a.md"]);
        assert_eq!(one("cat '/r/a.md"), ["cat", "/r/a.md"]);
    }

    #[test]
    fn only_a_double_quote_honours_the_backslash_escape() {
        // upstream: `if (quote === '"' && command[i] === "\\" ...) i++`
        assert_eq!(one(r#"cat "/r/desi\"gn.md""#), ["cat", "/r/desi\"gn.md"]);
        assert_eq!(one(r"cat '/r/desi\gn.md'"), ["cat", r"/r/desi\gn.md"]);
    }

    #[test]
    fn a_bare_backslash_escapes_the_next_character() {
        assert_eq!(one(r"cat \/r/a.md"), ["cat", "/r/a.md"]);
        assert_eq!(one(r"cat /r/a\ b.md"), ["cat", "/r/a b.md"]);
    }

    #[test]
    fn an_empty_quoted_word_never_becomes_a_word() {
        // upstream `pushWord` は `text.length > 0` のときだけ積む。
        assert_eq!(one("cat ''"), ["cat"]);
        assert_eq!(
            read("echo \"\" && cat /r/a.md"),
            [
                vec!["echo".to_string()],
                vec!["cat".to_string(), "/r/a.md".to_string()],
            ]
        );
    }

    #[test]
    fn the_five_separators_cut_a_segment_and_doubles_count_once() {
        assert_eq!(read("a; b"), [vec!["a".to_string()], vec!["b".to_string()]]);
        assert_eq!(
            read("a && b"),
            [vec!["a".to_string()], vec!["b".to_string()]]
        );
        assert_eq!(
            read("a | b"),
            [vec!["a".to_string()], vec!["b".to_string()]]
        );
        assert_eq!(
            read("a || b"),
            [vec!["a".to_string()], vec!["b".to_string()]]
        );
        assert_eq!(
            read("a & b"),
            [vec!["a".to_string()], vec!["b".to_string()]]
        );
        assert_eq!(read("(a)"), [vec!["a".to_string()]]);
    }

    #[test]
    fn redirection_characters_are_ordinary_text() {
        // `<` `>` は区切りでも演算子でもない — `cat>x` は 1 語になる。
        assert_eq!(one("cat>/r/a.md"), ["cat>/r/a.md"]);
        assert_eq!(
            one("cat /r/a.md > /r/b.md"),
            ["cat", "/r/a.md", ">", "/r/b.md"]
        );
        assert_eq!(
            one("xargs cat < /r/list.txt"),
            ["xargs", "cat", "<", "/r/list.txt"]
        );
    }

    #[test]
    fn a_descriptor_duplication_splits_on_its_ampersand() {
        // upstream の字句は `2>&1` の `&` を区切りとして読む (この移植は
        // review-freeze 側の是正パッチを**持ち込まない** — 別の上流関数だからである)。
        assert_eq!(
            read("rm x 2>&1; cat /r/a.md"),
            [
                vec!["rm".to_string(), "x".to_string(), "2>".to_string()],
                vec!["1".to_string()],
                vec!["cat".to_string(), "/r/a.md".to_string()],
            ]
        );
    }

    #[test]
    fn a_dollar_quote_is_a_dollar_followed_by_an_ordinary_quote() {
        assert_eq!(one("cat $'/r/a.md'"), ["cat", "$/r/a.md"]);
    }

    #[test]
    fn a_newline_is_whitespace_and_never_a_separator() {
        assert_eq!(
            read("cat /r/a.md\ncat /r/b.md"),
            [vec![
                "cat".to_string(),
                "/r/a.md".to_string(),
                "cat".to_string(),
                "/r/b.md".to_string(),
            ]]
        );
    }

    #[test]
    fn an_empty_or_separator_only_command_names_no_segment() {
        assert!(read("").is_empty());
        assert!(read("  ").is_empty());
        assert!(read(";;&&").is_empty());
    }
}
