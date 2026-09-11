//! リダイレクトと標準実行ラッパーを除いた、観測できる引数列。
use super::ShellWords;
/// 動的な実行ラッパーと、静的に読める語列を区別する。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShellInvocation {
    /// 静的な引数列。実行を伴わないラッパーは空列になる。
    Known(ShellWords),
    /// ラッパーの構文から実行対象を決められない。
    Uninspectable,
}
impl ShellInvocation {
    /// 文字列を実行せず、標準ラッパーの対象を読む。
    #[must_use]
    pub fn parse(segment: &str) -> Self {
        let mut words = ShellWords::parse(segment);
        let mut cursor = 0;
        let mut allow_prefix = true;
        while cursor < words.len() {
            if allow_prefix {
                skip_prefixes(&words, &mut cursor);
            } else {
                skip_redirections(&words, &mut cursor);
            }
            allow_prefix = false;
            let wrapper = basename(words.at(cursor).unwrap_or(""));
            match wrapper.as_str() {
                "time" => {
                    cursor += 1;
                    while words.at(cursor).is_some_and(|word| word.starts_with('-')) {
                        let value = matches!(
                            words.at(cursor),
                            Some("-f" | "--format" | "-o" | "--output")
                        );
                        cursor += 1;
                        if value {
                            skip_redirections(&words, &mut cursor);
                            if cursor >= words.len() {
                                return empty();
                            }
                            cursor += 1;
                        }
                    }
                    allow_prefix = true;
                }
                "command" | "exec" => {
                    cursor += 1;
                    while cursor < words.len() {
                        skip_redirections(&words, &mut cursor);
                        let option = words.at(cursor).unwrap_or("");
                        if option == "--" {
                            cursor += 1;
                            break;
                        }
                        if !option.starts_with('-') {
                            break;
                        }
                        if wrapper == "command" {
                            if option.trim_start_matches('-').contains(['v', 'V'])
                                || !option.strip_prefix('-').is_some_and(|rest| {
                                    !rest.is_empty() && rest.chars().all(|ch| ch == 'p')
                                })
                            {
                                return empty();
                            }
                            cursor += 1;
                            continue;
                        }
                        if option == "-a" {
                            cursor += 1;
                            skip_redirections(&words, &mut cursor);
                            if cursor >= words.len() {
                                return empty();
                            }
                            cursor += 1;
                            continue;
                        }
                        if !option.strip_prefix('-').is_some_and(|rest| {
                            !rest.is_empty() && rest.chars().all(|ch| matches!(ch, 'c' | 'l'))
                        }) {
                            return empty();
                        }
                        cursor += 1;
                    }
                    allow_prefix = true;
                }
                "env" => {
                    cursor += 1;
                    while cursor < words.len() {
                        skip_redirections(&words, &mut cursor);
                        let Some(word) = words.at(cursor) else {
                            break;
                        };
                        if assignment(word) {
                            cursor += 1;
                            continue;
                        }
                        if word == "--" {
                            cursor += 1;
                            break;
                        }
                        let original = cursor;
                        let (split, rest) = if let Some(split) =
                            word.strip_prefix("-S").filter(|tail| !tail.is_empty())
                        {
                            (Some(split.to_string()), cursor + 1)
                        } else if let Some(split) = word.strip_prefix("--split-string=") {
                            (Some(split.to_string()), cursor + 1)
                        } else if matches!(word, "-S" | "--split-string") {
                            cursor += 1;
                            skip_redirections(&words, &mut cursor);
                            (Some(words.at(cursor).unwrap_or("").to_string()), cursor + 1)
                        } else {
                            (None, cursor + 1)
                        };
                        if let Some(split) = split {
                            if split.contains(['\\', '$', '`', '#']) {
                                return Self::Uninspectable;
                            }
                            words = words.replace_range(original, rest, &ShellWords::parse(&split));
                            cursor = original;
                            continue;
                        }
                        let word = words.at(cursor).unwrap_or("");
                        if matches!(word, "-u" | "--unset" | "-C" | "--chdir" | "-P") {
                            cursor += 1;
                            skip_redirections(&words, &mut cursor);
                            if cursor >= words.len() {
                                return empty();
                            }
                            cursor += 1;
                            continue;
                        }
                        let attached = ["-u", "-C", "--unset=", "--chdir="].iter().any(|prefix| {
                            word.strip_prefix(prefix)
                                .is_some_and(|value| !value.is_empty())
                        });
                        let flags = word.strip_prefix('-').is_some_and(|rest| {
                            !rest.is_empty() && rest.chars().all(|ch| matches!(ch, 'i' | 'v'))
                        });
                        let signal = ["--block-signal", "--default-signal", "--ignore-signal"]
                            .iter()
                            .any(|prefix| {
                                word == *prefix
                                    || word
                                        .strip_prefix(prefix)
                                        .is_some_and(|rest| rest.starts_with('='))
                            });
                        if attached
                            || flags
                            || signal
                            || matches!(
                                word,
                                "-" | "--ignore-environment" | "--debug" | "--list-signal-handling"
                            )
                        {
                            cursor += 1;
                            continue;
                        }
                        if matches!(word, "-0" | "--null" | "--help" | "--version") {
                            return empty();
                        }
                        if word.starts_with('-') {
                            return Self::Uninspectable;
                        }
                        break;
                    }
                }
                "nice" => {
                    cursor += 1;
                    while cursor < words.len() {
                        skip_redirections(&words, &mut cursor);
                        let Some(word) = words.at(cursor) else {
                            break;
                        };
                        if word == "--" {
                            cursor += 1;
                            break;
                        }
                        if matches!(word, "--help" | "--version") {
                            return empty();
                        }
                        if matches!(word, "-n" | "--adjustment") {
                            cursor += 1;
                            skip_redirections(&words, &mut cursor);
                            if !signed_number(words.at(cursor).unwrap_or("")) {
                                return empty();
                            }
                            cursor += 1;
                            continue;
                        }
                        let attached = word
                            .strip_prefix("-n")
                            .or_else(|| word.strip_prefix("--adjustment="))
                            .is_some_and(signed_number)
                            || word
                                .strip_prefix('-')
                                .and_then(|rest| rest.strip_prefix('-').or(Some(rest)))
                                .is_some_and(unsigned_number);
                        if attached {
                            cursor += 1;
                            continue;
                        }
                        if word.starts_with('-') {
                            return empty();
                        }
                        break;
                    }
                }
                "nohup" => {
                    cursor += 1;
                    let word = words.at(cursor).unwrap_or("");
                    if matches!(word, "--help" | "--version") {
                        return empty();
                    }
                    if word == "--" {
                        cursor += 1;
                    } else if word.starts_with('-') {
                        return empty();
                    }
                }
                _ => break,
            }
        }
        Self::Known(words.suffix(cursor))
    }
}
const fn empty() -> ShellInvocation {
    ShellInvocation::Known(ShellWords::new(Vec::new()))
}
fn unsigned_number(word: &str) -> bool {
    !word.is_empty() && word.bytes().all(|byte| byte.is_ascii_digit())
}
fn signed_number(word: &str) -> bool {
    unsigned_number(word.strip_prefix(['+', '-']).unwrap_or(word))
}
fn basename(word: &str) -> String {
    word.replace('\\', "/")
        .rsplit('/')
        .next()
        .unwrap_or("")
        .to_string()
}
fn assignment(word: &str) -> bool {
    word.split_once('=').is_some_and(|(name, _)| {
        let mut chars = name.chars();
        chars
            .next()
            .is_some_and(|ch| ch.is_ascii_alphabetic() || ch == '_')
            && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
    })
}
fn skip_redirections(words: &ShellWords, cursor: &mut usize) {
    while let Some(word) = words.at(*cursor) {
        let suffix = word.trim_start_matches(|ch: char| ch.is_ascii_digit());
        let operators = ["<<<", "<<-", "<<", "<>", ">>", ">|", "<&", ">&", ">", "<"];
        if !operators.iter().any(|op| suffix.starts_with(op)) {
            break;
        }
        *cursor += 1;
        if operators.contains(&suffix) {
            *cursor += 1;
        }
    }
}
fn skip_prefixes(words: &ShellWords, cursor: &mut usize) {
    loop {
        let before = *cursor;
        while matches!(
            words.at(*cursor),
            Some("if" | "then" | "while" | "until" | "do" | "else" | "elif" | "!")
        ) {
            *cursor += 1;
        }
        skip_redirections(words, cursor);
        while words.at(*cursor).is_some_and(assignment) {
            *cursor += 1;
        }
        if *cursor == before {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ShellInvocation;

    /// 静的に読めた引数列。読めない呼出しは `None`。
    fn argv(segment: &str) -> Option<Vec<String>> {
        match ShellInvocation::parse(segment) {
            ShellInvocation::Known(words) => Some(
                (0..words.len())
                    .filter_map(|index| words.at(index))
                    .map(str::to_string)
                    .collect(),
            ),
            ShellInvocation::Uninspectable => None,
        }
    }

    fn known(segment: &str) -> Vec<String> {
        argv(segment).expect("静的に読める呼出し")
    }

    #[test]
    fn a_plain_command_and_its_prefixes_are_read_verbatim() {
        assert_eq!(known("rm x"), ["rm", "x"]);
        assert_eq!(known("FOO=1 BAR=2 rm x"), ["rm", "x"]);
        assert_eq!(known("if rm x"), ["rm", "x"]);
        assert_eq!(known("> /dev/null rm x"), ["rm", "x"]);
        assert_eq!(known("2>&1 rm x"), ["rm", "x"]);
        assert!(known("").is_empty());
    }

    #[test]
    fn a_time_wrapper_skips_its_format_and_output_values() {
        assert_eq!(known("time -f %e rm x"), ["rm", "x"]);
        assert_eq!(known("time -p rm x"), ["rm", "x"]);
        assert_eq!(known("time -o > log out rm x"), ["rm", "x"]);
        // 値が無ければ実行対象も無い。
        assert!(known("time -f").is_empty());
        assert!(known("time -o >out").is_empty());
    }

    #[test]
    fn command_and_exec_wrappers_read_their_switch_tables_verbatim() {
        assert_eq!(known("command -- rm x"), ["rm", "x"]);
        assert_eq!(known("command -p rm x"), ["rm", "x"]);
        assert!(known("command -v rm").is_empty());
        assert!(known("command -x rm").is_empty());
        assert_eq!(known("exec -- rm x"), ["rm", "x"]);
        assert_eq!(known("exec -cl rm x"), ["rm", "x"]);
        assert_eq!(known("exec -a name rm x"), ["rm", "x"]);
        assert!(known("exec -a").is_empty());
        assert!(known("exec -a >out").is_empty());
        assert!(known("exec -x rm x").is_empty());
    }

    #[test]
    fn an_env_wrapper_reads_its_switch_table_verbatim() {
        assert_eq!(known("env FOO=1 rm x"), ["rm", "x"]);
        assert_eq!(known("env -- rm x"), ["rm", "x"]);
        assert_eq!(known("env -u FOO rm x"), ["rm", "x"]);
        assert_eq!(known("env -uFOO --chdir=/tmp rm x"), ["rm", "x"]);
        assert_eq!(known("env -i - --debug rm x"), ["rm", "x"]);
        assert_eq!(known("env --default-signal=INT rm x"), ["rm", "x"]);
        assert!(known("env -u").is_empty());
        assert!(known("env -u >out").is_empty());
        assert!(known("env --null rm x").is_empty());
        assert!(known("env > /dev/null").is_empty());
        assert_eq!(argv("env --unknown rm x"), None);
    }

    #[test]
    fn a_split_string_env_is_re_read_unless_it_needs_the_shell() {
        assert_eq!(known("env -S 'rm x' y"), ["rm", "x", "y"]);
        assert_eq!(known("env -Srm x"), ["rm", "x"]);
        assert_eq!(known("env '--split-string=rm x' y"), ["rm", "x", "y"]);
        assert_eq!(known("env -S > out 'rm x'"), ["rm", "x"]);
        assert_eq!(argv("env -S 'rm $x'"), None);
        assert_eq!(argv("env '--split-string=rm `x`'"), None);
    }

    #[test]
    fn a_nice_wrapper_reads_its_adjustment_spellings_verbatim() {
        assert_eq!(known("nice rm x"), ["rm", "x"]);
        assert_eq!(known("nice -n 5 rm x"), ["rm", "x"]);
        assert_eq!(known("nice -n -5 rm x"), ["rm", "x"]);
        assert_eq!(known("nice -n5 --adjustment=+3 -7 rm x"), ["rm", "x"]);
        assert_eq!(known("nice -- rm x"), ["rm", "x"]);
        assert!(known("nice --help").is_empty());
        assert!(known("nice -n x rm").is_empty());
        assert!(known("nice -x rm").is_empty());
        assert!(known("nice > /dev/null").is_empty());
    }

    #[test]
    fn a_nohup_wrapper_accepts_a_double_dash_but_no_other_switch() {
        assert_eq!(known("nohup rm x"), ["rm", "x"]);
        assert_eq!(known("nohup -- rm x"), ["rm", "x"]);
        assert!(known("nohup --help").is_empty());
        assert!(known("nohup -x rm x").is_empty());
    }

    #[test]
    fn wrappers_nest_and_the_executable_is_reduced_to_its_basename() {
        assert_eq!(known("sudo rm x"), ["sudo", "rm", "x"]);
        assert_eq!(known("/usr/bin/env nice /usr/bin/time rm x"), ["rm", "x"]);
        // 引用した Windows パスは `\` を区切りとして葉名を取る。
        assert_eq!(known(r"'C:\bin\nohup' rm x"), ["rm", "x"]);
    }
}
