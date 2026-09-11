//! コマンド引数を、被演算子・スイッチ・スイッチの値へ分けた読み。
use core_infrastructure::collections::Collection;
use std::collections::{BTreeMap, BTreeSet};

/// 引数列 1 本を POSIX 風の規約で読んだもの。**シェルを実行しない**。
///
/// upstream `hooks/review-freeze-command.ts` の `parseShellArgs` に対応する。
/// 値を取るスイッチは呼出側がコマンドごとに与える — 同じ `-t` が `cp` では宛先ディレクトリ、
/// 別のコマンドでは旗になるため、この型は既定の一覧を持たない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ShellOptions {
    operands: Collection<String>,
    options: BTreeSet<String>,
    values: BTreeMap<String, Vec<String>>,
}
impl ShellOptions {
    /// 分解済みの 3 つ組を固定する (**この型の唯一の構築経路**)。
    const fn new(
        operands: Collection<String>,
        options: BTreeSet<String>,
        values: BTreeMap<String, Vec<String>>,
    ) -> Self {
        Self {
            operands,
            options,
            values,
        }
    }
    /// 値を取る短い綴り・長い綴りを与えて引数列を読む。
    ///
    /// 短いスイッチは束ねられる (`-rf`)。値を取るスイッチは束の残り (`-t/tmp`) か
    /// 次の語 (`-t /tmp`) を値として取り、そこで束の読みを終える。
    pub(crate) fn parse(
        args: &Collection<String>,
        short_values: &[&str],
        long_values: &[&str],
    ) -> Self {
        let mut operands: Vec<String> = Vec::new();
        let mut options: BTreeSet<String> = BTreeSet::new();
        let mut values: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let mut options_ended = false;
        let mut index = 0;
        while let Some(arg) = args.at(index) {
            index += 1;
            if !options_ended && arg == "--" {
                options_ended = true;
                continue;
            }
            if options_ended || arg == "-" || !arg.starts_with('-') {
                operands.push(arg.clone());
                continue;
            }
            if arg.starts_with("--") {
                let name = arg.split_once('=').map_or(arg.as_str(), |(name, _)| name);
                options.insert(name.to_string());
                if !long_values.contains(&name) {
                    continue;
                }
                if let Some((_, value)) = arg.split_once('=') {
                    record(&mut values, name, value.to_string());
                } else if let Some(value) = args.at(index) {
                    record(&mut values, name, value.clone());
                    index += 1;
                }
                continue;
            }
            // 短いスイッチは束ねられる。値を取るものが現れたら、束の残り (`-t/tmp`) か
            // 次の語 (`-t /tmp`) をその値として読み、束の読みを終える。
            let letters: Vec<char> = arg.chars().skip(1).collect();
            for (offset, letter) in letters.iter().enumerate() {
                let name = format!("-{letter}");
                options.insert(name.clone());
                if !short_values.contains(&name.as_str()) {
                    continue;
                }
                let attached: String = letters.iter().skip(offset + 1).collect();
                if attached.is_empty() {
                    if let Some(value) = args.at(index) {
                        record(&mut values, &name, value.clone());
                        index += 1;
                    }
                } else {
                    record(&mut values, &name, attached);
                }
                break;
            }
        }
        Self::new(Collection::new(operands), options, values)
    }
    /// スイッチではない引数の列 (`--` の後ろを含む)。
    pub(crate) const fn operands(&self) -> &Collection<String> {
        &self.operands
    }
    /// 最後の被演算子 (無ければ `None`)。
    pub(crate) fn last_operand(&self) -> Option<&str> {
        self.operands
            .at(self.operands.len().checked_sub(1)?)
            .map(String::as_str)
    }
    /// 最後の 1 件を除いた被演算子。1 件以下なら空。
    pub(crate) fn operands_before_last(&self) -> Collection<String> {
        let keep = self.operands.len().saturating_sub(1);
        Collection::new(self.operand_list().into_iter().take(keep).collect())
    }
    /// 指定位置以後の被演算子。
    pub(crate) fn operands_from(&self, start: usize) -> Collection<String> {
        Collection::new(self.operand_list().into_iter().skip(start).collect())
    }
    /// 被演算子を挿入順の素の列として写す。
    fn operand_list(&self) -> Vec<String> {
        self.operands
            .fold_left(Vec::new(), |mut acc: Vec<String>, operand| {
                acc.push(operand.clone());
                acc
            })
    }
    /// そのスイッチが現れたか。
    pub(crate) fn has(&self, name: &str) -> bool {
        self.options.contains(name)
    }
    /// 挙げたスイッチのいずれかが値を取ったか。
    pub(crate) fn has_any_value(&self, names: &[&str]) -> bool {
        names.iter().any(|name| self.values.contains_key(*name))
    }
    /// 挙げたスイッチの値を掲載順につないだうちの最後 (無ければ `None`)。
    pub(crate) fn last_value(&self, names: &[&str]) -> Option<&str> {
        names
            .iter()
            .filter_map(|name| self.values.get(*name))
            .flatten()
            .next_back()
            .map(String::as_str)
    }
}
/// `--Destination=/x` `-Path:/y` のように綴りへ張り付いたパス値。
///
/// upstream `attachedPathOptionValues` に対応する。PowerShell 系の綴りを読むためのもので、
/// スイッチ名は大文字小文字を区別しない。
pub(crate) fn attached_path_option_values(
    args: &Collection<String>,
    path_options: &[&str],
) -> Vec<String> {
    args.fold_left(Vec::new(), |mut acc: Vec<String>, arg| {
        if let Some(value) = attached_path_value(arg, path_options) {
            acc.push(value);
        }
        acc
    })
}
/// 綴り 1 つから、指定名のパス値を読む。
fn attached_path_value(arg: &str, path_options: &[&str]) -> Option<String> {
    let rest = arg.strip_prefix("--").or_else(|| arg.strip_prefix('-'))?;
    let (name, value) = rest.split_once([':', '='])?;
    if name.is_empty() || value.is_empty() {
        return None;
    }
    path_options
        .contains(&name.to_lowercase().as_str())
        .then(|| value.to_string())
}
/// スイッチ 1 つの値を掲載順に積む。
fn record(values: &mut BTreeMap<String, Vec<String>>, name: &str, value: String) {
    values.entry(name.to_string()).or_default().push(value);
}

#[cfg(test)]
mod tests {
    use super::{Collection, ShellOptions, attached_path_option_values};

    fn args(spelling: &[&str]) -> Collection<String> {
        Collection::new(spelling.iter().map(|arg| (*arg).to_string()).collect())
    }

    fn operands(parsed: &ShellOptions) -> Vec<String> {
        parsed.operands().fold_left(Vec::new(), |mut acc, operand| {
            acc.push(operand.clone());
            acc
        })
    }

    #[test]
    fn plain_words_are_operands_and_dashes_are_switches() {
        let parsed = ShellOptions::parse(&args(&["-f", "a.md", "b.md"]), &[], &[]);
        assert!(parsed.has("-f"));
        assert_eq!(operands(&parsed), ["a.md", "b.md"]);
        assert_eq!(parsed.last_operand(), Some("b.md"));
    }

    #[test]
    fn a_short_cluster_names_every_letter_it_carries() {
        let parsed = ShellOptions::parse(&args(&["-rf"]), &[], &[]);
        assert!(parsed.has("-r") && parsed.has("-f"));
        assert!(operands(&parsed).is_empty());
    }

    #[test]
    fn a_value_switch_takes_the_cluster_remainder_or_the_next_word() {
        let attached = ShellOptions::parse(&args(&["-t/tmp", "a.md"]), &["-t"], &[]);
        assert_eq!(attached.last_value(&["-t"]), Some("/tmp"));
        assert_eq!(operands(&attached), ["a.md"]);
        let separate = ShellOptions::parse(&args(&["-t", "/tmp", "a.md"]), &["-t"], &[]);
        assert_eq!(separate.last_value(&["-t"]), Some("/tmp"));
        assert_eq!(operands(&separate), ["a.md"]);
    }

    #[test]
    fn a_long_value_switch_reads_both_spellings() {
        let joined = ShellOptions::parse(
            &args(&["--target-directory=/tmp", "a.md"]),
            &[],
            &["--target-directory"],
        );
        assert_eq!(joined.last_value(&["--target-directory"]), Some("/tmp"));
        assert_eq!(operands(&joined), ["a.md"]);
        let split = ShellOptions::parse(
            &args(&["--target-directory", "/tmp", "a.md"]),
            &[],
            &["--target-directory"],
        );
        assert_eq!(split.last_value(&["--target-directory"]), Some("/tmp"));
        assert_eq!(operands(&split), ["a.md"]);
    }

    #[test]
    fn the_last_value_wins_across_the_listed_spellings() {
        let parsed = ShellOptions::parse(
            &args(&["-t", "/one", "--target-directory", "/two"]),
            &["-t"],
            &["--target-directory"],
        );
        assert_eq!(
            parsed.last_value(&["-t", "--target-directory"]),
            Some("/two")
        );
        assert!(parsed.has_any_value(&["-t"]));
        assert!(!parsed.has_any_value(&["-e"]));
    }

    #[test]
    fn a_double_dash_ends_the_switches_and_a_bare_dash_is_an_operand() {
        let parsed = ShellOptions::parse(&args(&["--", "-r", "-"]), &[], &[]);
        assert!(!parsed.has("-r"));
        assert_eq!(operands(&parsed), ["-r", "-"]);
    }

    #[test]
    fn the_operand_views_drop_the_last_entry_or_skip_a_prefix() {
        let parsed = ShellOptions::parse(&args(&["a", "b", "c"]), &[], &[]);
        let before_last =
            parsed
                .operands_before_last()
                .fold_left(Vec::new(), |mut acc, operand| {
                    acc.push(operand.clone());
                    acc
                });
        assert_eq!(before_last, ["a", "b"]);
        let skipped = parsed
            .operands_from(1)
            .fold_left(Vec::new(), |mut acc, operand| {
                acc.push(operand.clone());
                acc
            });
        assert_eq!(skipped, ["b", "c"]);
        let single = ShellOptions::parse(&args(&["a"]), &[], &[]);
        assert!(single.operands_before_last().is_empty());
    }

    #[test]
    fn an_attached_path_option_is_read_case_insensitively() {
        let parsed = args(&["-Path:/x", "--Destination=/y", "-Force", "--other=/z"]);
        assert_eq!(
            attached_path_option_values(&parsed, &["path", "destination"]),
            ["/x", "/y"]
        );
        assert!(attached_path_option_values(&parsed, &["literalpath"]).is_empty());
    }
}
