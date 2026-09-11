//! シェルコマンド 1 本が書き換えうるファイルの列。
use super::{
    shell_mutation::{PERL_SHORT_VALUES, SED_LONG_VALUES, SED_SHORT_VALUES, ShellMutation},
    shell_options::{ShellOptions, attached_path_option_values},
};
use core_infrastructure::collections::{Collection, FirstClassCollection};
use std::path::{Component, Path, PathBuf};

/// `Bash` の 1 呼出しが書き換えうる宛先を、絶対パスの綴りで並べた列。
///
/// upstream `hooks/review-freeze-command.ts` の `shellWriteTargets` に対応する。
/// **シェルを実行しない** — 出力リダイレクトの送り先と、変更系コマンドの操作対象を
/// 綴りから読むだけである。ただし `cp` / `install` / `mv` の宛先がディレクトリかどうかだけは
/// 実際のファイルシステムへ問い合わせる (upstream `statSync` と同じ位置)。
///
/// **保証は「書き換えうる」であって「書き換える」ではない。** 読取り専用のコマンドは宛先を
/// 生まないが、綴りを読み切れないラッパー (`sudo --unknown` など) は作業ディレクトリ自身を
/// 宛先として並べる — 「無害」ではなく「読めない」を表すためである。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellWriteTargets {
    items: Vec<String>,
}
impl ShellWriteTargets {
    /// 重複を除いた宛先列を固定する (**この型の唯一の構築経路**)。
    const fn new(items: Vec<String>) -> Self {
        Self { items }
    }
    /// コマンド文字列から宛先を読む。`cwd` は相対綴りを解決する基点。
    #[must_use]
    pub fn parse(command: &str, cwd: &Path) -> ShellWriteTargets {
        let mut found = Found::new(cwd);
        scan_redirections(command, &mut found);
        let mutations = ShellMutation::read_all(command);
        for index in 0..mutations.len() {
            if let Some(mutation) = mutations.at(index) {
                scan_invocation(mutation, &mut found);
            }
        }
        ShellWriteTargets::new(found.into_items())
    }
    /// 宛先の件数。
    #[must_use]
    pub const fn len(&self) -> usize {
        self.items.len()
    }
    /// 宛先が 1 つも無いか。
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
    /// 発見順の添字参照。範囲外は `None`。
    #[must_use]
    pub fn at(&self, index: usize) -> Option<&str> {
        self.items.get(index).map(String::as_str)
    }
}
impl FirstClassCollection for ShellWriteTargets {
    type Item<'a> = &'a str;
    type Filtered = Self;
    fn len(&self) -> usize {
        Self::len(self)
    }
    fn at(&self, index: usize) -> Option<&str> {
        Self::at(self, index)
    }
    fn fold_left<'a, A>(&'a self, initial: A, mut fold: impl FnMut(A, &'a str) -> A) -> A {
        self.items
            .iter()
            .fold(initial, |acc, target| fold(acc, target))
    }
    fn filter(&self, mut predicate: impl FnMut(&str) -> bool) -> Self {
        Self::new(
            self.items
                .iter()
                .filter(|target| predicate(target))
                .cloned()
                .collect(),
        )
    }
}

/// 走査中の宛先の積み上げ。発見順を保ち、同じ綴りは 1 度だけ残す。
struct Found {
    cwd: PathBuf,
    items: Vec<String>,
}
impl Found {
    /// 基点だけを与えて空から始める (**この型の唯一の構築経路**)。
    fn new(cwd: &Path) -> Self {
        Self {
            cwd: cwd.to_path_buf(),
            items: Vec::new(),
        }
    }
    /// 生の綴りを正規化して積む。宛先にならない綴りは捨てる。
    fn add(&mut self, raw: &str) {
        if let Some(target) = normalize(raw, &self.cwd)
            && !self.items.contains(&target)
        {
            self.items.push(target);
        }
    }
    /// 作業ディレクトリ自身を積む。
    fn add_cwd(&mut self) {
        let cwd = self.cwd.to_string_lossy().into_owned();
        self.add(&cwd);
    }
    /// その綴りが実在のディレクトリを指すか。
    fn is_directory(&self, raw: &str) -> bool {
        normalize(raw, &self.cwd)
            .and_then(|target| std::fs::metadata(&target).ok())
            .is_some_and(|meta| meta.is_dir())
    }
    /// 宛先と、ディレクトリ宛のときは各源の葉名を足した宛先候補を積む。
    fn add_destination(
        &mut self,
        destination: Option<&str>,
        sources: &Collection<String>,
        directory: bool,
    ) {
        let Some(destination) = destination else {
            return;
        };
        self.add(destination);
        if !directory {
            return;
        }
        let Some(resolved) = normalize(destination, &self.cwd) else {
            return;
        };
        // cp/install/mv はディレクトリ宛を取る。走査前のファイルシステムを見ずに、
        // 各源の葉名を足した候補も宛先として並べる。
        let children = sources.fold_left(Vec::new(), |mut acc: Vec<String>, source| {
            if let Some(source) = normalize(source, &self.cwd)
                && let Some(leaf) = Path::new(&source).file_name()
            {
                acc.push(
                    Path::new(&resolved)
                        .join(leaf)
                        .to_string_lossy()
                        .into_owned(),
                );
            }
            acc
        });
        for child in children {
            self.add(&child);
        }
    }
    /// 積み上げた宛先を発見順で取り出す。
    fn into_items(self) -> Vec<String> {
        self.items
    }
}

/// 引用の外にある出力リダイレクトの送り先を拾う。
///
/// `printf x>>file` のような詰めた綴りも、引用された綴りも、`$PWD` 起点の綴りも拾う。
fn scan_redirections(command: &str, found: &mut Found) {
    let chars: Vec<char> = command.chars().collect();
    let mut quote: Option<char> = None;
    let mut escaped = false;
    let mut i = 0;
    while let Some(ch) = chars.get(i).copied() {
        i += 1;
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' && quote != Some('\'') {
            escaped = true;
            continue;
        }
        if let Some(active) = quote {
            if ch == active {
                quote = None;
            }
            continue;
        }
        if matches!(ch, '\'' | '"') {
            quote = Some(ch);
            continue;
        }
        if ch != '>' {
            continue;
        }
        let mut start = i;
        if matches!(chars.get(start).copied(), Some('>' | '|')) {
            start += 1;
        }
        while chars
            .get(start)
            .copied()
            .is_some_and(core_infrastructure::ecmascript::is_whitespace)
        {
            start += 1;
        }
        // `2>&1` `2>&-` は記述子の複製・閉鎖、`>&file` はファイルへの書込み。
        if chars.get(start).copied() == Some('&') {
            let Some((word, end)) = word_at(&chars, start + 1) else {
                continue;
            };
            if word == "-" || word.bytes().all(|byte| byte.is_ascii_digit()) {
                continue;
            }
            found.add(&word);
            i = end;
            continue;
        }
        let Some((word, end)) = word_at(&chars, start) else {
            continue;
        };
        found.add(&word);
        i = end;
    }
}

/// コマンド 1 本から、宛先・その場書換えの操作対象だけを拾う。
///
/// 読取り専用の被演算子を持つコマンドでは、宛先と `-i` の対象だけが候補になる。
fn scan_invocation(mutation: &ShellMutation, found: &mut Found) {
    if mutation.is_ambiguous() {
        found.add_cwd();
        return;
    }
    if mutation.is_data_driven() && mutation.may_mutate() {
        found.add_cwd();
    }
    let name = mutation.name();
    let args = mutation.args();
    if name == "dd" {
        for index in 0..args.len() {
            if let Some(arg) = args.at(index).filter(|arg| arg.starts_with("of=")) {
                found.add(arg);
            }
        }
        return;
    }

    let basic = ShellOptions::parse(args, &[], &[]);
    let attached = attached_path_option_values(args, &POWERSHELL_PATH_OPTIONS);
    if basic.operands().is_empty() && attached.is_empty() && name != "find" {
        return;
    }

    match name {
        "cp" => copy_like(
            &ShellOptions::parse(args, &CP_SHORT_VALUES, &CP_LONG_VALUES),
            found,
        ),
        "install" => install(args, found),
        "mv" => {
            let parsed = ShellOptions::parse(args, &CP_SHORT_VALUES, &CP_LONG_VALUES);
            let sources = move_sources(&parsed);
            for index in 0..sources.len() {
                if let Some(source) = sources.at(index) {
                    found.add(source);
                }
            }
            copy_like(&parsed, found);
        }
        "rm" | "tee" | "touch" | "truncate" | "unlink" => {
            let parsed = match name {
                "touch" => ShellOptions::parse(args, &TOUCH_SHORT_VALUES, &TOUCH_LONG_VALUES),
                "truncate" => {
                    ShellOptions::parse(args, &TRUNCATE_SHORT_VALUES, &TRUNCATE_LONG_VALUES)
                }
                _ => basic,
            };
            add_all(parsed.operands(), found);
        }
        "sed" => in_place(
            args,
            &SED_SHORT_VALUES,
            &SED_LONG_VALUES,
            &SED_PROGRAM_OPTIONS,
            found,
        ),
        "perl" => in_place(args, &PERL_SHORT_VALUES, &[], &PERL_PROGRAM_OPTIONS, found),
        "find" => find(args, found),
        "copy-item" => {
            if let Some(destination) = basic.last_operand() {
                found.add(destination);
            }
            for value in attached_path_option_values(args, &["destination"]) {
                found.add(&value);
            }
        }
        "rsync" => {
            if let Some(destination) = basic.last_operand() {
                found.add(destination);
            }
            if basic.has("--remove-source-files") {
                add_all(&basic.operands_before_last(), found);
            }
        }
        _ => {
            if mutation.is_static_remove()
                || mutation.is_static_move()
                || mutation.is_static_content()
            {
                add_all(basic.operands(), found);
                for value in attached {
                    found.add(&value);
                }
            }
        }
    }
}

/// `cp` / `mv` / `install` の宛先とディレクトリ宛の候補を積む。
fn copy_like(parsed: &ShellOptions, found: &mut Found) {
    let target_directory = parsed.last_value(&TARGET_DIRECTORY_OPTIONS);
    let has_target_directory = target_directory.is_some();
    let destination = target_directory.or_else(|| parsed.last_operand());
    let sources = move_sources(parsed);
    let directory = has_target_directory
        || sources.len() > 1
        || destination.is_some_and(|destination| found.is_directory(destination));
    found.add_destination(destination, &sources, directory);
}

/// `-t` があれば全被演算子、無ければ最後を除いた被演算子が源になる。
fn move_sources(parsed: &ShellOptions) -> Collection<String> {
    if parsed.last_value(&TARGET_DIRECTORY_OPTIONS).is_some() {
        parsed.operands_from(0)
    } else {
        parsed.operands_before_last()
    }
}

/// `install` は `-d` でディレクトリ新設、それ以外は `cp` と同じ宛先の読み。
fn install(args: &Collection<String>, found: &mut Found) {
    let parsed = ShellOptions::parse(args, &INSTALL_SHORT_VALUES, &INSTALL_LONG_VALUES);
    if parsed.has("-d") || parsed.has("--directory") {
        add_all(parsed.operands(), found);
        return;
    }
    copy_like(&parsed, found);
}

/// `sed -i` / `perl -i` の書換え対象を積む。プログラム本体は対象にしない。
fn in_place(
    args: &Collection<String>,
    short_values: &[&str],
    long_values: &[&str],
    program_options: &[&str],
    found: &mut Found,
) {
    let parsed = ShellOptions::parse(args, short_values, long_values);
    if !parsed.has("-i") && !parsed.has("--in-place") {
        return;
    }
    // プログラム本体がスイッチで与えられていなければ、最初の被演算子がプログラムである。
    let from_option = parsed.has_any_value(program_options);
    add_all(&parsed.operands_from(usize::from(!from_option)), found);
}

/// `find` の削除対象と出力ファイルを積む。
fn find(args: &Collection<String>, found: &mut Found) {
    let list = args.fold_left(Vec::new(), |mut acc: Vec<String>, arg| {
        acc.push(arg.clone());
        acc
    });
    if list.iter().any(|arg| arg == "-delete") {
        for root in traversal_roots(&list) {
            found.add(&root);
        }
    }
    let mut index = 0;
    while let Some(arg) = list.get(index) {
        if ["-fprint", "-fprint0", "-fls"].contains(&arg.as_str()) {
            index += 1;
            if let Some(output) = list.get(index) {
                found.add(output);
            }
        } else if arg == "-fprintf" {
            index += 1;
            if let Some(output) = list.get(index) {
                found.add(output);
            }
            index += 1;
        }
        index += 1;
    }
}

/// `find` が走査する起点。指定が無ければカレント 1 件。
fn traversal_roots(args: &[String]) -> Vec<String> {
    let mut index = 0;
    while args
        .get(index)
        .is_some_and(|arg| ["-H", "-L", "-P"].contains(&arg.as_str()))
    {
        index += 1;
    }
    while args.get(index).is_some_and(|arg| arg.starts_with("-D")) {
        index += if args.get(index).is_some_and(|arg| arg == "-D") {
            2
        } else {
            1
        };
    }
    if args.get(index).is_some_and(|arg| {
        arg.strip_prefix("-O")
            .is_some_and(|rest| !rest.is_empty() && rest.bytes().all(|b| b.is_ascii_digit()))
    }) {
        index += 1;
    }
    let mut roots: Vec<String> = Vec::new();
    while let Some(arg) = args.get(index) {
        if arg == "!" || arg == "(" || arg == ")" || arg == "," || arg.starts_with('-') {
            break;
        }
        roots.push(arg.clone());
        index += 1;
    }
    if roots.is_empty() {
        return vec![".".to_string()];
    }
    roots
}

/// 列の全要素を宛先として積む。
fn add_all(operands: &Collection<String>, found: &mut Found) {
    for index in 0..operands.len() {
        if let Some(operand) = operands.at(index) {
            found.add(operand);
        }
    }
}

/// 位置 `start` から 1 語を読む。引用が閉じない・空語なら `None`。
fn word_at(chars: &[char], start: usize) -> Option<(String, usize)> {
    let mut word = String::new();
    let mut quote: Option<char> = None;
    let mut escaped = false;
    let mut i = start;
    while let Some(ch) = chars.get(i).copied() {
        if escaped {
            word.push(ch);
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
        if core_infrastructure::ecmascript::is_whitespace(ch)
            || matches!(ch, ';' | '|' | '&' | '(' | ')' | '<' | '>')
        {
            break;
        }
        word.push(ch);
        i += 1;
    }
    (quote.is_none() && !word.is_empty()).then_some((word, i))
}

/// 生の綴りを絶対パスの宛先へ直す。宛先にならない綴りは `None`。
///
/// `$PWD` 起点だけを解決し、それ以外の展開・グロブを含む綴りは宛先にしない —
/// 実行前に確定しないものを凍結の材料にしないためである。
fn normalize(target: &str, cwd: &Path) -> Option<String> {
    let stripped = target.strip_prefix("of=").unwrap_or(target);
    let trimmed =
        stripped.trim_matches(|ch| matches!(ch, ',' | ':' | '[' | ']' | '{' | '}' | '(' | ')'));
    // `${PWD}` 単独は直前の `trim_matches` が末尾の `}` を剥がして `${PWD` になり、`$` を
    // 含むので宛先にならない (本家 `normalizeShellTarget` も同じ順で剥がすため同じ観測)。
    let cleaned = if trimmed == "$PWD" {
        cwd.to_string_lossy().into_owned()
    } else if let Some(rest) = trimmed
        .strip_prefix("$PWD/")
        .or_else(|| trimmed.strip_prefix(BRACED_PWD_PREFIX))
    {
        cwd.join(rest).to_string_lossy().into_owned()
    } else {
        trimmed.to_string()
    };
    if cleaned.is_empty() || cleaned.contains(['$', '`', '*', '?']) {
        return None;
    }
    Some(resolve_lexically(cwd, &cleaned))
}

/// Node の `path.resolve` と同じ字句的な解決。ファイルシステムを読まない。
///
/// `cwd` が相対のときは相対のまま正規化する — upstream はプロセスの作業ディレクトリを
/// 継ぎ足すが、フックが受け取る `cwd` は常に絶対であり、接尾辞一致の照合は根を要求しない。
fn resolve_lexically(cwd: &Path, target: &str) -> String {
    let joined = if Path::new(target).is_absolute() {
        PathBuf::from(target)
    } else {
        cwd.join(target)
    };
    let mut parts: Vec<String> = Vec::new();
    let mut absolute = false;
    for component in joined.components() {
        match component {
            Component::RootDir => absolute = true,
            Component::CurDir => {}
            Component::ParentDir => {
                parts.pop();
            }
            Component::Normal(part) => parts.push(part.to_string_lossy().into_owned()),
            Component::Prefix(prefix) => {
                parts.push(prefix.as_os_str().to_string_lossy().into_owned());
            }
        }
    }
    let body = parts.join("/");
    if absolute {
        return format!("/{body}");
    }
    if body.is_empty() {
        return ".".to_string();
    }
    body
}

/// `${PWD}/` の綴り (テンプレート展開を避けて組む)。
const BRACED_PWD_PREFIX: &str = "${PWD}/";
/// PowerShell 系のパス指定スイッチ (逐語)。
const POWERSHELL_PATH_OPTIONS: [&str; 5] =
    ["path", "literalpath", "destination", "newname", "filepath"];
/// `cp` / `mv` の値を取る短いスイッチ。
const CP_SHORT_VALUES: [&str; 2] = ["-S", "-t"];
/// `cp` / `mv` の値を取る長いスイッチ。
const CP_LONG_VALUES: [&str; 2] = ["--suffix", "--target-directory"];
/// 宛先ディレクトリを指すスイッチ (掲載順)。
const TARGET_DIRECTORY_OPTIONS: [&str; 2] = ["-t", "--target-directory"];
/// `install` の値を取る短いスイッチ。
const INSTALL_SHORT_VALUES: [&str; 5] = ["-g", "-m", "-o", "-S", "-t"];
/// `install` の値を取る長いスイッチ。
const INSTALL_LONG_VALUES: [&str; 5] = [
    "--group",
    "--mode",
    "--owner",
    "--suffix",
    "--target-directory",
];
/// `touch` の値を取る短いスイッチ。
const TOUCH_SHORT_VALUES: [&str; 3] = ["-d", "-r", "-t"];
/// `touch` の値を取る長いスイッチ。
const TOUCH_LONG_VALUES: [&str; 3] = ["--date", "--reference", "--time"];
/// `truncate` の値を取る短いスイッチ。
const TRUNCATE_SHORT_VALUES: [&str; 2] = ["-r", "-s"];
/// `truncate` の値を取る長いスイッチ。
const TRUNCATE_LONG_VALUES: [&str; 2] = ["--reference", "--size"];
/// `sed` のプログラム本体を与えるスイッチ (逐語)。
const SED_PROGRAM_OPTIONS: [&str; 4] = ["-e", "-f", "--expression", "--file"];
/// `perl` のプログラム本体を与えるスイッチ (逐語)。
const PERL_PROGRAM_OPTIONS: [&str; 2] = ["-e", "-E"];

#[cfg(test)]
mod tests {
    use super::ShellWriteTargets;
    use core_infrastructure::collections::FirstClassCollection;
    use std::path::Path;

    fn targets(command: &str, cwd: &Path) -> Vec<String> {
        ShellWriteTargets::parse(command, cwd).fold_left(Vec::new(), |mut acc, target| {
            acc.push(target.to_string());
            acc
        })
    }

    fn at_root(command: &str) -> Vec<String> {
        targets(command, Path::new("/r"))
    }

    #[test]
    fn an_output_redirection_names_its_file() {
        assert_eq!(at_root("printf x >> /r/a.md"), ["/r/a.md"]);
        assert_eq!(at_root("printf x>/r/a.md"), ["/r/a.md"]);
        assert_eq!(at_root("printf x >| /r/a.md"), ["/r/a.md"]);
        assert_eq!(at_root(r#"printf x > "/r/a b.md""#), ["/r/a b.md"]);
        assert_eq!(at_root("printf x >&/r/a.md"), ["/r/a.md"]);
    }

    #[test]
    fn a_numbered_redirection_names_a_file_but_a_duplication_names_none() {
        assert_eq!(at_root("cmd 2>/r/log"), ["/r/log"]);
        assert!(at_root("cmd 2>&1").is_empty());
        assert!(at_root("cmd >&-").is_empty());
        assert_eq!(at_root("cmd &>/r/log"), ["/r/log"]);
    }

    #[test]
    fn a_descriptor_number_never_becomes_an_operand_of_the_command() {
        // 是正前の本家は `2>&1` の `2` を `rm` の被演算子として拾い、`1` を別コマンドに
        // していた (`scripts/aidlc-sync/patches/shell-redirection-tokens.patch`)。
        assert_eq!(at_root("rm /r/a.md 2>&1"), ["/r/a.md"]);
        assert_eq!(at_root("rm /r/a.md &> /r/log"), ["/r/log", "/r/a.md"]);
        assert_eq!(at_root("rm /r/a.md 2>&-"), ["/r/a.md"]);
    }

    #[test]
    fn a_relative_redirection_is_resolved_against_the_working_directory() {
        assert_eq!(at_root("printf x > a.md"), ["/r/a.md"]);
        assert_eq!(at_root("printf x > ./sub/../a.md"), ["/r/a.md"]);
        assert_eq!(at_root("printf x > $PWD/a.md"), ["/r/a.md"]);
    }

    #[test]
    fn an_unresolvable_spelling_is_never_a_target() {
        // 展開・グロブは実行前に確定しないので凍結の材料にしない。
        for command in [
            "printf x > $OUT/a.md",
            "printf x > /r/*.md",
            "printf x > /r/a?.md",
            "rm $HOME/a.md",
        ] {
            assert!(at_root(command).is_empty(), "{command}");
        }
    }

    #[test]
    fn a_removal_names_every_operand_and_a_read_names_none() {
        assert_eq!(at_root("rm -rf /r/a.md /r/b.md"), ["/r/a.md", "/r/b.md"]);
        assert_eq!(at_root("touch /r/a.md"), ["/r/a.md"]);
        assert_eq!(at_root("tee /r/a.md"), ["/r/a.md"]);
        assert_eq!(at_root("truncate -s 0 /r/a.md"), ["/r/a.md"]);
        assert_eq!(at_root("unlink /r/a.md"), ["/r/a.md"]);
        for command in ["cat /r/a.md", "grep -r x /r", "ls /r", "mkdir -p /r/x"] {
            assert!(at_root(command).is_empty(), "{command}");
        }
    }

    #[test]
    fn a_copy_names_its_destination_and_a_move_names_both_ends() {
        assert_eq!(at_root("cp /r/a.md /r/b.md"), ["/r/b.md"]);
        assert_eq!(at_root("mv /r/a.md /r/b.md"), ["/r/a.md", "/r/b.md"]);
        assert_eq!(at_root("install /r/a.md /r/b.md"), ["/r/b.md"]);
        assert_eq!(at_root("install -d /r/x /r/y"), ["/r/x", "/r/y"]);
    }

    #[test]
    fn a_directory_destination_also_names_each_child_it_would_receive() {
        assert_eq!(
            at_root("cp -t /r/out /r/a.md /r/b.md"),
            ["/r/out", "/r/out/a.md", "/r/out/b.md"]
        );
        // 源が複数なら宛先はディレクトリと分かる。
        assert_eq!(
            at_root("cp /r/a.md /r/b.md /r/out"),
            ["/r/out", "/r/out/a.md", "/r/out/b.md"]
        );
    }

    #[test]
    fn an_existing_directory_destination_is_detected_from_the_filesystem() {
        let temp = tempfile::tempdir().unwrap();
        let out = temp.path().join("out");
        std::fs::create_dir(&out).unwrap();
        let command = format!("cp a.md {}", out.to_string_lossy());
        let found = targets(&command, temp.path());
        assert_eq!(found.len(), 2, "{found:?}");
        assert!(
            found.contains(&out.to_string_lossy().into_owned()),
            "{found:?}"
        );
        assert!(
            found.contains(&out.join("a.md").to_string_lossy().into_owned()),
            "{found:?}"
        );
    }

    #[test]
    fn an_in_place_edit_names_the_file_and_a_plain_edit_names_none() {
        assert_eq!(at_root("sed -i s/x/y/ /r/a.md"), ["/r/a.md"]);
        assert_eq!(at_root("sed -i -e s/x/y/ /r/a.md"), ["/r/a.md"]);
        assert_eq!(at_root("sed --in-place s/x/y/ /r/a.md"), ["/r/a.md"]);
        assert!(at_root("sed s/x/y/ /r/a.md").is_empty());
        assert_eq!(at_root("perl -i -pe s/x/y/ /r/a.md"), ["/r/a.md"]);
        assert!(at_root("perl -pe s/x/y/ /r/a.md").is_empty());
    }

    #[test]
    fn a_find_names_its_roots_only_when_it_deletes() {
        assert_eq!(at_root("find /r/x -delete"), ["/r/x"]);
        assert_eq!(at_root("find -delete"), ["/r"]);
        assert!(at_root("find /r/x -name a.md").is_empty());
        assert_eq!(at_root("find /r/x -fprint /r/out"), ["/r/out"]);
        assert_eq!(at_root("find /r/x -fprintf /r/out %p"), ["/r/out"]);
    }

    #[test]
    fn a_data_stream_or_an_unreadable_launcher_names_the_working_directory() {
        assert_eq!(at_root("xargs rm"), ["/r"]);
        assert_eq!(at_root("sudo --unknown-switch rm /r/a.md"), ["/r"]);
        assert_eq!(at_root("env --unknown-switch rm /r/a.md"), ["/r"]);
        // 読取り専用のコマンドを流し込むだけなら宛先にならない。
        assert!(at_root("xargs cat").is_empty());
    }

    #[test]
    fn a_dd_output_operand_is_read_without_its_prefix() {
        assert_eq!(at_root("dd if=/r/a.md of=/r/b.md"), ["/r/b.md"]);
    }

    #[test]
    fn a_powershell_command_names_its_operands_and_attached_paths() {
        assert_eq!(at_root("remove-item /r/a.md"), ["/r/a.md"]);
        assert_eq!(at_root("set-content -Path:/r/a.md"), ["/r/a.md"]);
        assert_eq!(at_root("copy-item /r/a.md /r/b.md"), ["/r/b.md"]);
    }

    #[test]
    fn an_rsync_names_its_destination_and_its_sources_only_when_it_removes_them() {
        assert_eq!(at_root("rsync /r/a.md /r/b.md"), ["/r/b.md"]);
        assert_eq!(
            at_root("rsync --remove-source-files /r/a.md /r/b.md"),
            ["/r/b.md", "/r/a.md"]
        );
    }

    #[test]
    fn each_command_of_a_chain_contributes_and_duplicates_collapse() {
        assert_eq!(at_root("echo hi && rm /r/a.md"), ["/r/a.md"]);
        assert_eq!(at_root("rm /r/a.md; rm /r/a.md"), ["/r/a.md"]);
        assert_eq!(at_root("rm /r/a.md | tee /r/b.md"), ["/r/a.md", "/r/b.md"]);
        assert_eq!(at_root("sudo rm /r/a.md"), ["/r/a.md"]);
    }

    #[test]
    fn a_launcher_that_carries_the_command_still_names_the_target() {
        assert_eq!(at_root("env FOO=1 rm /r/a.md"), ["/r/a.md"]);
        assert_eq!(at_root("timeout 5 rm /r/a.md"), ["/r/a.md"]);
        assert_eq!(at_root("nohup rm /r/a.md"), ["/r/a.md"]);
    }

    #[test]
    fn the_collection_reports_its_count_and_indexes_in_discovery_order() {
        let found = ShellWriteTargets::parse("rm /r/a.md /r/b.md", Path::new("/r"));
        assert_eq!(found.len(), 2);
        assert_eq!(found.at(0), Some("/r/a.md"));
        assert_eq!(found.at(1), Some("/r/b.md"));
        assert_eq!(found.at(2), None, "範囲外は None");
        // FirstClassCollection 経由でも同じ答えを返す。
        assert_eq!(FirstClassCollection::len(&found), 2);
        assert_eq!(FirstClassCollection::at(&found, 1), Some("/r/b.md"));
        let empty = ShellWriteTargets::parse("cat /r/a.md", Path::new("/r"));
        assert_eq!(empty.len(), 0);
        assert!(empty.is_empty());
    }

    #[test]
    fn a_filter_keeps_only_the_matching_targets_in_order() {
        let found = ShellWriteTargets::parse("rm /r/a.md /r/b.txt /r/c.md", Path::new("/r"));
        let markdown = found.filter(|target| target.ends_with(".md"));
        assert_eq!(markdown.len(), 2);
        assert_eq!(markdown.at(0), Some("/r/a.md"));
        assert_eq!(markdown.at(1), Some("/r/c.md"));
        assert!(found.filter(|_| false).is_empty());
    }

    #[test]
    fn an_attached_path_switch_without_operands_names_no_copy_destination() {
        // `-NewName:` は PowerShell 系の張り付き値であり、cp/mv の被演算子にならないため
        // 宛先が無い。宛先が無ければ何も積まない (add_destination の早期 return)。
        assert!(at_root("cp -NewName:/r/a.md").is_empty());
        assert!(at_root("mv -NewName:/r/a.md").is_empty());
    }

    #[test]
    fn an_unresolvable_directory_destination_names_no_child_either() {
        // 源が複数なので宛先はディレクトリだが、`$OUT` は実行前に確定しないため
        // 宛先も各源の葉名候補も生まない。
        assert!(at_root("cp /r/a.md /r/b.md $OUT").is_empty());
        assert!(at_root("cp -t $OUT /r/a.md").is_empty());
    }

    #[test]
    fn an_escaped_or_quoted_angle_bracket_is_not_a_redirection() {
        assert!(at_root(r"echo a\>b").is_empty());
        assert!(at_root(r#"echo "x > /r/a.md""#).is_empty());
        assert!(at_root("echo 'x > /r/a.md'").is_empty());
        // 逃がした `\` の次の `>` は普通のリダイレクトである。
        assert_eq!(at_root(r"echo \\> /r/a.md"), ["/r/a.md"]);
        // 引用の中の `\"` は引用を閉じない。
        assert!(at_root(r#"echo "a\" > /r/a.md""#).is_empty());
    }

    #[test]
    fn a_dangling_redirection_names_nothing() {
        assert!(at_root("printf x >").is_empty());
        assert!(at_root("printf x >&").is_empty());
        assert!(at_root("printf x > \"unterminated").is_empty());
        assert!(at_root("printf x >& \"unterminated").is_empty());
    }

    #[test]
    fn a_redirection_target_ends_at_a_control_operator() {
        assert_eq!(
            at_root("printf x > /r/a.md && rm /r/b.md"),
            ["/r/a.md", "/r/b.md"]
        );
        assert_eq!(
            at_root("printf x >/r/a.md;rm /r/b.md"),
            ["/r/a.md", "/r/b.md"]
        );
        assert_eq!(at_root(r"printf x > /r/a\ b.md"), ["/r/a b.md"]);
    }

    #[test]
    fn a_copy_item_names_its_attached_destination_as_well() {
        assert_eq!(
            at_root("copy-item /r/a.md -Destination:/r/b.md"),
            ["/r/a.md", "/r/b.md"]
        );
    }

    #[test]
    fn a_find_skips_its_leading_traversal_switches_before_the_roots() {
        assert_eq!(at_root("find -L /r/x -delete"), ["/r/x"]);
        assert_eq!(at_root("find -H -P /r/x /r/y -delete"), ["/r/x", "/r/y"]);
        assert_eq!(at_root("find -D tree /r/x -delete"), ["/r/x"]);
        assert_eq!(at_root("find -Dtree /r/x -delete"), ["/r/x"]);
        assert_eq!(at_root("find -O2 /r/x -delete"), ["/r/x"]);
        // `-O` の後ろが数字でなければ起点の指定が無いとみなし、カレントが起点になる。
        assert_eq!(at_root("find -Ox -delete"), ["/r"]);
        assert_eq!(at_root("find ! -name a -delete"), ["/r"]);
    }

    #[test]
    fn the_working_directory_spelling_resolves_to_the_working_directory() {
        assert_eq!(at_root("rm -rf $PWD"), ["/r"]);
        assert_eq!(at_root("rm -rf ${PWD}/a.md"), ["/r/a.md"]);
        assert_eq!(at_root("printf x > $PWD"), ["/r"]);
    }

    #[test]
    fn a_relative_working_directory_is_resolved_lexically_without_a_root() {
        assert_eq!(targets("printf x > a.md", Path::new(".")), ["a.md"]);
        assert_eq!(
            targets("printf x > ./sub/../a.md", Path::new("rel")),
            ["rel/a.md"]
        );
        assert_eq!(targets("rm -rf .", Path::new(".")), ["."]);
        assert_eq!(targets("rm -rf ..", Path::new("rel")), ["."]);
    }

    #[test]
    fn an_empty_or_read_only_command_names_nothing() {
        assert!(at_root("").is_empty());
        assert!(at_root("   ").is_empty());
        assert!(ShellWriteTargets::parse("", Path::new("/r")).is_empty());
    }
}
