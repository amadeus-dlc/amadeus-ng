//! 実行ラッパーを剥がした、コマンド 1 本の呼出し。
use super::{
    command_segments::split_command_segments, redirection_free_words::RedirectionFreeWords,
    shell_options::ShellOptions,
};
use core_infrastructure::collections::Collection;

/// 実行ラッパー (`sudo` `env` `xargs` `timeout` など) を剥がしたコマンド 1 本。
///
/// upstream `hooks/review-freeze-command.ts` の `shellInvocation` /
/// `shellCommandInvocationDetails` のうち、**書込み先の抽出に要る 4 項目だけ**を持つ。
/// upstream の `executable` / `launchers` / `dataDrivenMutation` /
/// `executableResolutionChanged` は別の利用者 (`aidlc-plan-approval-guard.ts` の
/// `shellCommandInvocations`) のもので、凍結判定は読まないので移していない。
///
/// ラッパーの綴りを静的に読み切れないときは [`ShellMutation::is_ambiguous`] が真になる。
/// そのときコマンド名も引数も空であり、「無害」ではなく「読めない」を意味する。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ShellMutation {
    name: String,
    args: Collection<String>,
    ambiguous: bool,
    data_driven: bool,
}
impl ShellMutation {
    /// 4 項目を同時に固定する (**この型の唯一の構築経路**)。
    const fn new(
        name: String,
        args: Collection<String>,
        ambiguous: bool,
        data_driven: bool,
    ) -> Self {
        Self {
            name,
            args,
            ambiguous,
            data_driven,
        }
    }
    /// コマンド文字列を実行位置ごとに読む。読めなかった区間は列に現れない。
    pub(crate) fn read_all(command: &str) -> Collection<Self> {
        Collection::new(split_command_segments(command).fold_left(
            Vec::new(),
            |mut acc: Vec<Self>, segment| {
                if let Some(mutation) =
                    read_segment(&RedirectionFreeWords::parse(segment), 0, false)
                {
                    acc.push(mutation);
                }
                acc
            },
        ))
    }
    /// 実行ファイルの葉名 (小文字・拡張子なし)。読めない呼出しでは空。
    pub(crate) fn name(&self) -> &str {
        &self.name
    }
    /// コマンド名より後ろの引数列。
    pub(crate) const fn args(&self) -> &Collection<String> {
        &self.args
    }
    /// ラッパーの綴りから実行対象を決められなかったか。
    pub(crate) const fn is_ambiguous(&self) -> bool {
        self.ambiguous
    }
    /// 標準入力の内容が引数になる呼出し (`xargs`) か。
    pub(crate) const fn is_data_driven(&self) -> bool {
        self.data_driven
    }
    /// このコマンドがファイルを変更しうるか。
    ///
    /// upstream `invocationMayMutate` に対応する。**`mkdir` は upstream の一覧に無い**ので
    /// ここにも無い (ディレクトリの新設は宣言成果物の綴りを名指さないため)。
    pub(crate) fn may_mutate(&self) -> bool {
        if DIRECT_MUTATION_COMMANDS.contains(&self.name.as_str())
            || self.is_static_remove()
            || self.is_static_move()
            || self.is_static_content()
        {
            return true;
        }
        if self.name == "sed" {
            return self.in_place_options(&SED_SHORT_VALUES, &SED_LONG_VALUES);
        }
        if self.name == "perl" {
            return self.in_place_options(&PERL_SHORT_VALUES, &[]);
        }
        self.name == "find"
            && self.args.fold_left(false, |found, arg| {
                found || FIND_WRITE_PREDICATES.contains(&arg.as_str())
            })
    }
    /// PowerShell 系の削除コマンドか。
    pub(crate) fn is_static_remove(&self) -> bool {
        STATIC_REMOVE_COMMANDS.contains(&self.name.as_str())
    }
    /// PowerShell 系の移動・改名コマンドか。
    pub(crate) fn is_static_move(&self) -> bool {
        STATIC_MOVE_COMMANDS.contains(&self.name.as_str())
    }
    /// PowerShell 系の内容書込みコマンドか。
    pub(crate) fn is_static_content(&self) -> bool {
        STATIC_CONTENT_COMMANDS.contains(&self.name.as_str())
    }
    /// `-i` / `--in-place` が立っているか。
    fn in_place_options(&self, short_values: &[&str], long_values: &[&str]) -> bool {
        let parsed = ShellOptions::parse(&self.args, short_values, long_values);
        parsed.has("-i") || parsed.has("--in-place")
    }
}

/// 名前だけで書込みと分かるコマンド (upstream `invocationMayMutate` の先頭一覧、逐語)。
const DIRECT_MUTATION_COMMANDS: [&str; 11] = [
    "cp",
    "dd",
    "install",
    "mv",
    "rm",
    "rsync",
    "tee",
    "touch",
    "truncate",
    "unlink",
    "copy-item",
];
/// upstream `STATIC_REMOVE_COMMANDS` (逐語)。
const STATIC_REMOVE_COMMANDS: [&str; 9] = [
    "rmdir",
    "rd",
    "del",
    "erase",
    "shred",
    "remove-item",
    "clear-item",
    "ri",
    "cli",
];
/// upstream `STATIC_MOVE_COMMANDS` (逐語)。
const STATIC_MOVE_COMMANDS: [&str; 7] = [
    "move",
    "rename",
    "move-item",
    "rename-item",
    "mi",
    "ren",
    "rni",
];
/// upstream `STATIC_CONTENT_COMMANDS` (逐語)。
const STATIC_CONTENT_COMMANDS: [&str; 6] = [
    "set-item",
    "new-item",
    "set-content",
    "add-content",
    "clear-content",
    "out-file",
];
/// `find` が書込みを行う述語 (逐語)。
const FIND_WRITE_PREDICATES: [&str; 5] = ["-delete", "-fprint", "-fprint0", "-fprintf", "-fls"];
/// `sed` の値を取るスイッチ。
pub(crate) const SED_SHORT_VALUES: [&str; 3] = ["-e", "-f", "-l"];
/// `sed` の値を取る長いスイッチ。
pub(crate) const SED_LONG_VALUES: [&str; 3] = ["--expression", "--file", "--line-length"];
/// `perl` の値を取るスイッチ。
pub(crate) const PERL_SHORT_VALUES: [&str; 6] = ["-E", "-F", "-I", "-M", "-e", "-m"];

/// ラッパーの綴りから実行対象を決められなかった呼出し。
const fn unreadable() -> ShellMutation {
    ShellMutation::new(String::new(), Collection::new(Vec::new()), true, false)
}

/// 区間 1 つを読む。upstream `shellInvocation` に対応する。
fn read_segment(
    segment: &RedirectionFreeWords,
    depth: usize,
    data_driven: bool,
) -> Option<ShellMutation> {
    if depth > 8 {
        return None;
    }
    let words = segment;
    let mut index = 0;
    let mut data_driven = data_driven;
    skip_assignments(words, &mut index);
    while let Some(current) = words.at(index) {
        let wrapper = executable_leaf(current);
        match wrapper.as_str() {
            "}" | "fi" | "done" | "esac" | "for" | "select" | "case" => return None,
            "{" | "then" | "else" | "do" | "!" | "if" | "elif" | "while" | "until" => {
                index += 1;
                skip_assignments(words, &mut index);
            }
            "command" => {
                index += 1;
                while let Some(option) = words
                    .at(index)
                    .filter(|option| option.starts_with('-'))
                    .map(str::to_string)
                {
                    index += 1;
                    if option == "--" {
                        break;
                    }
                    // `command -v/-V` は名前を問い合わせるだけで、続く語を実行しない。
                    if option.contains(['v', 'V']) {
                        return None;
                    }
                    if !option.chars().skip(1).all(|flag| flag == 'p') {
                        return Some(unreadable());
                    }
                }
                skip_assignments(words, &mut index);
            }
            "builtin" => {
                index += 1;
                match words.at(index) {
                    Some("--") => index += 1,
                    Some(option) if option.starts_with('-') => return Some(unreadable()),
                    _ => {}
                }
                skip_assignments(words, &mut index);
            }
            "env" => {
                index += 1;
                let split = match read_env_options(words, &mut index) {
                    EnvPrefix::Read(split) => split,
                    EnvPrefix::Missing => return None,
                    EnvPrefix::Unreadable => return Some(unreadable()),
                };
                skip_assignments(words, &mut index);
                if !split.is_empty() {
                    return read_segment(&words.spliced(&split, index), depth + 1, data_driven);
                }
            }
            "busybox" | "toybox" => {
                index += 1;
                if words.at(index).is_none_or(|applet| applet.starts_with('-')) {
                    return Some(unreadable());
                }
            }
            "timeout" => {
                let (next, ambiguous) = consume_wrapper_options(words, index + 1, &TIMEOUT);
                if ambiguous {
                    return Some(unreadable());
                }
                index = next;
                if index < words.len() {
                    index += 1; // 持続時間
                }
                skip_assignments(words, &mut index);
            }
            _ => {
                let Some(spec) = simple_wrapper(&wrapper) else {
                    break;
                };
                let (next, ambiguous) = consume_wrapper_options(words, index + 1, spec);
                if ambiguous {
                    return Some(unreadable());
                }
                index = next;
                data_driven = data_driven || wrapper == "xargs";
                skip_assignments(words, &mut index);
            }
        }
    }
    let executable = words.at(index)?;
    Some(ShellMutation::new(
        executable_leaf(executable),
        Collection::new(words.suffix(index + 1)),
        false,
        data_driven,
    ))
}

/// `env` のスイッチを読んだ結末。
///
/// upstream は 2 つの失敗を区別する — 知らないスイッチは `{ambiguous: true}`
/// (実行対象が**読めない**) で、`-S` の値欠落は `null` (実行対象が**無い**) である。
/// 前者は作業ディレクトリ自身が書込み先になり、後者は何も生まない。
enum EnvPrefix {
    /// 読み切れた。`-S` / `--split-string` が運ぶ語列 (無ければ空)。
    Read(Vec<String>),
    /// `-S` が値を伴わない。
    Missing,
    /// 知らないスイッチがある。
    Unreadable,
}

/// `env` のスイッチを読み、`-S` / `--split-string` が運ぶ語列を返す。
fn read_env_options(words: &RedirectionFreeWords, index: &mut usize) -> EnvPrefix {
    let mut split: Vec<String> = Vec::new();
    while let Some(option) = words.at(*index).map(str::to_string) {
        if option == "--" {
            *index += 1;
            break;
        }
        if option == "-S" || option == "--split-string" {
            let Some(value) = words.at(*index + 1) else {
                return EnvPrefix::Missing;
            };
            split = RedirectionFreeWords::parse(value).suffix(0);
            *index += 2;
            continue;
        }
        if let Some(value) = option
            .strip_prefix("--split-string=")
            .or_else(|| option.strip_prefix("-S").filter(|rest| !rest.is_empty()))
        {
            split = RedirectionFreeWords::parse(value).suffix(0);
            *index += 1;
            continue;
        }
        if ENV_VALUE_OPTIONS.contains(&option.as_str()) {
            *index += 2;
            continue;
        }
        if env_attached_value(&option) || env_flag(&option) {
            *index += 1;
            continue;
        }
        if option.starts_with('-') {
            return EnvPrefix::Unreadable;
        }
        break;
    }
    EnvPrefix::Read(split)
}

/// 次の語を値として取る `env` のスイッチ (逐語)。
const ENV_VALUE_OPTIONS: [&str; 6] = ["-u", "--unset", "-C", "--chdir", "-a", "--argv0"];

/// `-uNAME` `-C/dir` `-aNAME` `--unset=NAME` のように値が張り付いた綴りか。
fn env_attached_value(option: &str) -> bool {
    ["-u", "-C", "-a"].iter().any(|prefix| {
        option
            .strip_prefix(prefix)
            .is_some_and(|rest| !rest.is_empty())
    }) || option.starts_with("--unset=")
        || option.starts_with("--chdir=")
        || option.starts_with("--argv0=")
}

/// 値を取らない `env` の旗か (逐語)。
fn env_flag(option: &str) -> bool {
    if matches!(
        option,
        "-" | "-i"
            | "--ignore-environment"
            | "-0"
            | "--null"
            | "-v"
            | "--debug"
            | "--list-signal-handling"
            | "--help"
            | "--version"
    ) {
        return true;
    }
    if option.strip_prefix('-').is_some_and(|rest| {
        !rest.is_empty() && rest.chars().all(|ch| matches!(ch, 'i' | '0' | 'v'))
    }) {
        return true;
    }
    ["--default-signal", "--ignore-signal", "--block-signal"]
        .iter()
        .any(|name| {
            option == *name
                || option
                    .strip_prefix(name)
                    .is_some_and(|rest| rest.starts_with('='))
        })
}

/// `NAME=value` の代入語を読み飛ばす。
fn skip_assignments(words: &RedirectionFreeWords, index: &mut usize) {
    while words.at(*index).is_some_and(is_assignment) {
        *index += 1;
    }
}

/// `NAME=` で始まる代入語か (upstream `/^([A-Za-z_][A-Za-z0-9_]*)=/`)。
fn is_assignment(word: &str) -> bool {
    let Some((name, _)) = word.split_once('=') else {
        return false;
    };
    let mut letters = name.chars();
    letters
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic() || first == '_')
        && letters.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

/// 実行ファイルの葉名 (小文字・Windows 拡張子なし)。upstream `shellExecutableName`。
fn executable_leaf(value: &str) -> String {
    let normalized = value.replace('\\', "/");
    let leaf = normalized.rsplit('/').next().unwrap_or("").to_lowercase();
    [".exe", ".com", ".cmd", ".bat"]
        .iter()
        .find_map(|extension| leaf.strip_suffix(extension).map(str::to_string))
        .unwrap_or(leaf)
}

/// ラッパー 1 つのスイッチ仕様。
struct WrapperOptions {
    short_values: &'static [&'static str],
    long_values: &'static [&'static str],
    short_optional_values: &'static [&'static str],
    long_optional_values: &'static [&'static str],
    short_flags: &'static [&'static str],
    long_flags: &'static [&'static str],
    numeric_short_value: bool,
}
impl WrapperOptions {
    /// 6 つの綴り集合と数値短縮の可否を固定する (**この型の唯一の構築経路**)。
    const fn new(
        short_values: &'static [&'static str],
        long_values: &'static [&'static str],
        short_optional_values: &'static [&'static str],
        long_optional_values: &'static [&'static str],
        short_flags: &'static [&'static str],
        long_flags: &'static [&'static str],
        numeric_short_value: bool,
    ) -> Self {
        Self {
            short_values,
            long_values,
            short_optional_values,
            long_optional_values,
            short_flags,
            long_flags,
            numeric_short_value,
        }
    }
}

/// upstream `simpleWrappers` の各仕様 (逐語)。
const EXEC: WrapperOptions = WrapperOptions::new(&["-a"], &[], &[], &[], &["-c", "-l"], &[], false);
const NOHUP: WrapperOptions =
    WrapperOptions::new(&[], &[], &[], &[], &[], &["--help", "--version"], false);
const NICE: WrapperOptions = WrapperOptions::new(
    &["-n"],
    &["--adjustment"],
    &[],
    &[],
    &[],
    &["--help", "--version"],
    true,
);
const IONICE: WrapperOptions = WrapperOptions::new(
    &["-c", "-n", "-p", "-P", "-u"],
    &["--class", "--classdata", "--pid", "--pgid", "--uid"],
    &[],
    &[],
    &["-t"],
    &["--ignore", "--help", "--version"],
    false,
);
const STDBUF: WrapperOptions = WrapperOptions::new(
    &["-i", "-o", "-e"],
    &["--input", "--output", "--error"],
    &[],
    &[],
    &[],
    &["--help", "--version"],
    false,
);
const SETSID: WrapperOptions = WrapperOptions::new(
    &[],
    &[],
    &[],
    &[],
    &["-c", "-f", "-w"],
    &["--ctty", "--fork", "--wait", "--help", "--version"],
    false,
);
const SUDO: WrapperOptions = WrapperOptions::new(
    &["-C", "-D", "-g", "-h", "-p", "-r", "-t", "-T", "-u"],
    &[
        "--chdir",
        "--close-from",
        "--group",
        "--host",
        "--prompt",
        "--role",
        "--type",
        "--user",
    ],
    &["-E"],
    &["--preserve-env"],
    &[
        "-A", "-b", "-e", "-H", "-K", "-k", "-l", "-n", "-P", "-S", "-s", "-V", "-v",
    ],
    &[
        "--askpass",
        "--background",
        "--edit",
        "--help",
        "--login",
        "--non-interactive",
        "--remove-timestamp",
        "--reset-timestamp",
        "--set-home",
        "--shell",
        "--stdin",
        "--validate",
        "--version",
    ],
    false,
);
const DOAS: WrapperOptions = WrapperOptions::new(
    &["-C", "-u"],
    &[],
    &[],
    &[],
    &["-L", "-n", "-s"],
    &[],
    false,
);
const XARGS: WrapperOptions = WrapperOptions::new(
    &["-a", "-d", "-E", "-I", "-J", "-L", "-n", "-P", "-s"],
    &[
        "--arg-file",
        "--delimiter",
        "--max-args",
        "--max-procs",
        "--max-chars",
        "--process-slot-var",
    ],
    &["-e", "-i", "-l"],
    &["--eof", "--replace", "--max-lines"],
    &["-0", "-o", "-p", "-r", "-t", "-x"],
    &[
        "--null",
        "--open-tty",
        "--interactive",
        "--no-run-if-empty",
        "--show-limits",
        "--verbose",
        "--exit",
        "--help",
        "--version",
    ],
    false,
);
const TIME: WrapperOptions = WrapperOptions::new(
    &["-f", "-o"],
    &["--format", "--output"],
    &[],
    &[],
    &["-a", "-p", "-v"],
    &[
        "--append",
        "--portability",
        "--verbose",
        "--help",
        "--version",
    ],
    false,
);
const UNBUFFER: WrapperOptions = WrapperOptions::new(&[], &[], &[], &[], &["-p"], &[], false);
const TIMEOUT: WrapperOptions = WrapperOptions::new(
    &["-k", "-s"],
    &["--kill-after", "--signal"],
    &[],
    &[],
    &[],
    &[
        "--foreground",
        "--preserve-status",
        "--verbose",
        "--help",
        "--version",
    ],
    false,
);

/// 名前からスイッチ仕様を引く。ラッパーでなければ `None`。
fn simple_wrapper(name: &str) -> Option<&'static WrapperOptions> {
    match name {
        "exec" => Some(&EXEC),
        "nohup" => Some(&NOHUP),
        "nice" => Some(&NICE),
        "ionice" => Some(&IONICE),
        "stdbuf" => Some(&STDBUF),
        "setsid" => Some(&SETSID),
        "sudo" => Some(&SUDO),
        "doas" => Some(&DOAS),
        "xargs" => Some(&XARGS),
        "time" => Some(&TIME),
        "unbuffer" => Some(&UNBUFFER),
        _ => None,
    }
}

/// ラッパーのスイッチを読み進める。読めない綴りに当たると真を返す。
fn consume_wrapper_options(
    words: &RedirectionFreeWords,
    start: usize,
    spec: &WrapperOptions,
) -> (usize, bool) {
    let mut index = start;
    while let Some(option) = words.at(index) {
        if option == "--" {
            return (index + 1, false);
        }
        if option == "-" || !option.starts_with('-') {
            return (index, false);
        }
        if option.starts_with("--") {
            let name = option.split_once('=').map_or(option, |(name, _)| name);
            if spec.long_values.contains(&name) {
                if option.contains('=') {
                    index += 1;
                } else if words.at(index + 1).is_some() {
                    index += 2;
                } else {
                    return (index, true);
                }
                continue;
            }
            if spec.long_optional_values.contains(&name) || spec.long_flags.contains(&name) {
                index += 1;
                continue;
            }
            return (index, true);
        }
        if spec.numeric_short_value
            && option
                .strip_prefix('-')
                .is_some_and(|rest| !rest.is_empty() && rest.bytes().all(|b| b.is_ascii_digit()))
        {
            index += 1;
            continue;
        }
        let name: String = option.chars().take(2).collect();
        if spec.short_values.contains(&name.as_str()) {
            if option.chars().count() > 2 {
                index += 1;
            } else if words.at(index + 1).is_some() {
                index += 2;
            } else {
                return (index, true);
            }
            continue;
        }
        if spec.short_optional_values.contains(&name.as_str()) {
            index += 1;
            continue;
        }
        if option.chars().count() > 1
            && option
                .chars()
                .skip(1)
                .all(|flag| spec.short_flags.contains(&format!("-{flag}").as_str()))
        {
            index += 1;
            continue;
        }
        return (index, true);
    }
    (index, false)
}

#[cfg(test)]
mod tests {
    use super::ShellMutation;

    fn read(command: &str) -> Vec<(String, Vec<String>, bool, bool)> {
        ShellMutation::read_all(command).fold_left(Vec::new(), |mut acc, mutation| {
            acc.push((
                mutation.name().to_string(),
                mutation
                    .args()
                    .fold_left(Vec::new(), |mut args: Vec<String>, arg| {
                        args.push(arg.clone());
                        args
                    }),
                mutation.is_ambiguous(),
                mutation.is_data_driven(),
            ));
            acc
        })
    }

    fn names(command: &str) -> Vec<String> {
        read(command)
            .into_iter()
            .map(|(name, _, _, _)| name)
            .collect()
    }

    fn ambiguous(command: &str) -> bool {
        let read = read(command);
        assert_eq!(read.len(), 1, "{command}: {read:?}");
        read.first().is_some_and(|entry| entry.2)
    }

    /// `env -S` の値として 1 段包む。空白と `\\` を逃がし、値をほどくと内側の綴りが現れる。
    fn wrapped_in_split_string(inner: &str) -> String {
        let escaped = inner.replace('\\', "\\\\").replace(' ', "\\ ");
        format!("env -S {escaped}")
    }

    #[test]
    fn a_split_string_chain_deeper_than_eight_levels_is_not_followed() {
        // 値をほどくたびに 1 段深くなる。8 段までは辿り、9 段目で追跡を止めて実行対象なしとする。
        let eight = (0..8).fold("rm x".to_string(), |acc, _| wrapped_in_split_string(&acc));
        assert_eq!(names(&eight), ["rm"]);
        let nine = wrapped_in_split_string(&eight);
        assert!(names(&nine).is_empty());
    }

    #[test]
    fn the_command_builtin_reads_only_the_posix_path_switch() {
        assert_eq!(names("command -p rm x"), ["rm"]);
        assert_eq!(names("command -pp rm x"), ["rm"]);
        assert_eq!(names("command -- rm x"), ["rm"]);
        assert!(
            names("command -V rm").is_empty(),
            "名前の問合せは実行しない"
        );
        assert!(ambiguous("command -x rm x"), "知らないスイッチは読めない");
    }

    #[test]
    fn the_builtin_builtin_accepts_a_double_dash_but_no_other_switch() {
        assert_eq!(names("builtin -- rm x"), ["rm"]);
        assert!(ambiguous("builtin -x rm x"));
    }

    #[test]
    fn a_busybox_without_a_readable_applet_is_ambiguous() {
        assert!(ambiguous("busybox"));
        assert!(ambiguous("busybox --list"));
        assert!(ambiguous("toybox -h"));
        assert_eq!(names("toybox rm x"), ["rm"]);
    }

    #[test]
    fn a_timeout_with_an_unreadable_switch_is_ambiguous() {
        assert!(ambiguous("timeout --unknown 5 rm x"));
        assert_eq!(names("timeout --foreground -k 2 5 rm x"), ["rm"]);
        assert!(names("timeout").is_empty(), "持続時間もコマンドも無い");
    }

    #[test]
    fn the_env_switches_are_read_verbatim_from_the_upstream_list() {
        assert_eq!(names("env -- rm x"), ["rm"]);
        assert_eq!(names("env -u FOO rm x"), ["rm"]);
        assert_eq!(names("env --chdir /tmp rm x"), ["rm"]);
        assert_eq!(names("env -uFOO rm x"), ["rm"]);
        assert_eq!(names("env --unset=FOO rm x"), ["rm"]);
        assert_eq!(names("env -i rm x"), ["rm"]);
        assert_eq!(names("env -i0v rm x"), ["rm"]);
        assert_eq!(names("env --ignore-environment rm x"), ["rm"]);
        assert_eq!(names("env --default-signal rm x"), ["rm"]);
        assert_eq!(names("env --ignore-signal=INT rm x"), ["rm"]);
        assert_eq!(names("env --split-string='rm x' y"), ["rm"]);
        assert!(ambiguous("env --default-signalx rm x"));
        assert!(ambiguous("env -x rm x"));
        assert!(names("env").is_empty(), "コマンドが無ければ実行対象も無い");
    }

    #[test]
    fn the_wrapper_switch_tables_are_walked_verbatim() {
        // `--` はスイッチの終わり。
        assert_eq!(names("sudo -- rm x"), ["rm"]);
        // 値を取る長いスイッチは `=` 付きでも次の語でも読む。値が無ければ読めない。
        assert_eq!(names("sudo --user=root rm x"), ["rm"]);
        assert_eq!(names("sudo --user root rm x"), ["rm"]);
        assert!(ambiguous("sudo --user"));
        // 任意値・旗の長いスイッチは 1 語で終わる。
        assert_eq!(names("sudo --preserve-env rm x"), ["rm"]);
        assert_eq!(names("sudo --non-interactive rm x"), ["rm"]);
        // `nice` は `-5` のような数値短縮を取る。
        assert_eq!(names("nice -5 rm x"), ["rm"]);
        // 値を取る短いスイッチは束の残りか次の語。値が無ければ読めない。
        assert_eq!(names("sudo -uroot rm x"), ["rm"]);
        assert!(ambiguous("sudo -u"));
        // 任意値の短いスイッチと、旗の束。
        assert_eq!(names("sudo -E rm x"), ["rm"]);
        assert_eq!(names("sudo -nH rm x"), ["rm"]);
        assert!(ambiguous("sudo -Z rm x"), "知らない短いスイッチ");
        assert!(ambiguous("sudo -nZ rm x"), "知らない旗を含む束");
        // `-` 単独は被演算子であり、ラッパーの引数列を終える。
        assert_eq!(names("sudo - x"), ["-"]);
        // スイッチだけで語が尽きれば実行対象は無い。
        assert!(names("sudo -n").is_empty());
        assert!(names("sudo").is_empty());
    }

    #[test]
    fn a_plain_command_is_read_with_its_arguments() {
        assert_eq!(
            read("rm -rf /x"),
            [(
                "rm".to_string(),
                vec!["-rf".to_string(), "/x".to_string()],
                false,
                false
            )]
        );
    }

    #[test]
    fn each_segment_of_a_chain_is_read_separately() {
        assert_eq!(names("rm a && cp b c ; echo done"), ["rm", "cp", "echo"]);
    }

    #[test]
    fn a_launcher_is_peeled_off_before_the_command() {
        for command in [
            "sudo rm x",
            "sudo -u root rm x",
            "nohup rm x",
            "nice -n 5 rm x",
            "timeout 5 rm x",
            "timeout --signal=TERM 5 rm x",
            "env FOO=1 rm x",
            "exec rm x",
            "command rm x",
            "builtin rm x",
            "stdbuf -o0 rm x",
            "busybox rm x",
            "FOO=1 rm x",
        ] {
            assert_eq!(names(command), ["rm"], "{command}");
        }
    }

    #[test]
    fn a_split_string_launcher_reads_the_command_it_carries() {
        assert_eq!(
            read("env -S 'rm x' y"),
            [(
                "rm".to_string(),
                vec!["x".to_string(), "y".to_string()],
                false,
                false
            )]
        );
        assert_eq!(names("env --split-string='rm x'"), ["rm"]);
        assert_eq!(names("env -Srm x"), ["rm"]);
    }

    #[test]
    fn a_data_driven_launcher_marks_the_command_it_feeds() {
        let read = read("xargs rm");
        assert_eq!(read.len(), 1);
        assert_eq!(read.first().map(|entry| entry.3), Some(true));
        assert_eq!(names("xargs -0 rm"), ["rm"]);
    }

    #[test]
    fn an_executable_path_is_reduced_to_its_lowercased_leaf() {
        assert_eq!(names("/usr/bin/rm x"), ["rm"]);
        assert_eq!(names(r"'C:\Windows\System32\DEL.EXE' x"), ["del"]);
        // 引用しない Windows パスの `\` は字句解析の逃がしとして消える (本家も同じ)。
        // 綴りが 1 語に潰れるので葉名は取れない。
        assert_eq!(
            names(r"C:\Windows\System32\DEL.EXE x"),
            ["c:windowssystem32del"]
        );
    }

    #[test]
    fn a_launcher_option_that_cannot_be_read_is_ambiguous_rather_than_absent() {
        let read = read("sudo --unknown-switch rm x");
        assert_eq!(read.len(), 1);
        assert_eq!(read.first().map(|entry| entry.2), Some(true));
        assert_eq!(
            read.first().map(|entry| entry.0.clone()),
            Some(String::new())
        );
    }

    #[test]
    fn an_unreadable_env_switch_is_ambiguous_but_a_missing_split_value_names_nothing() {
        // 本家は知らない `env` のスイッチを ambiguous として返し (`shellInvocation` の
        // `option.startsWith("-")` → `{ambiguous: true}`)、`-S` の値欠落だけを `null` にする。
        let unreadable = read("env --unknown-switch rm x");
        assert_eq!(unreadable.len(), 1, "{unreadable:?}");
        assert_eq!(unreadable.first().map(|entry| entry.2), Some(true));
        assert_eq!(
            unreadable.first().map(|entry| entry.0.clone()),
            Some(String::new())
        );
        assert!(
            names("env -S").is_empty(),
            "-S の値欠落は実行対象を持たない"
        );
    }

    #[test]
    fn a_name_query_and_a_loop_header_name_no_command() {
        assert!(names("command -v rm").is_empty());
        assert_eq!(names("for f in a; do rm $f; done"), ["rm"]);
        assert!(names("").is_empty());
        assert!(names("   ").is_empty());
    }

    #[test]
    fn the_mutation_list_matches_the_upstream_enumeration() {
        for command in [
            "cp a b",
            "dd of=b",
            "install a b",
            "mv a b",
            "rm a",
            "rsync a b",
            "tee a",
            "touch a",
            "truncate -s0 a",
            "unlink a",
            "rmdir a",
            "shred a",
            "move a b",
            "set-content a",
            "sed -i s/a/b/ f",
            "sed --in-place s/a/b/ f",
            "perl -i -pe s/a/b/ f",
            "find . -delete",
            "find . -fprint out",
        ] {
            let mutations = ShellMutation::read_all(command);
            assert!(
                mutations.at(0).is_some_and(ShellMutation::may_mutate),
                "{command}"
            );
        }
    }

    #[test]
    fn a_read_only_command_never_claims_to_mutate() {
        for command in [
            "echo a",
            "cat a",
            "grep -r x .",
            "sed s/a/b/ f",
            "perl -pe s/a/b/ f",
            "find . -name x",
            "mkdir -p a",
        ] {
            let mutations = ShellMutation::read_all(command);
            assert!(
                mutations.at(0).is_some_and(|first| !first.may_mutate()),
                "{command}"
            );
        }
    }
}
