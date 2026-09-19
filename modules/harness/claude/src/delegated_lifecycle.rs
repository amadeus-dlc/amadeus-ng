//! 委譲されたClaudeエージェントの進行コマンドを、実行位置で識別する。
use core_infrastructure::collections::FirstClassCollection;
use harness_infrastructure::{
    ShellInvocation, ShellParseError, ShellText, ShellWords, split_shell_segments,
};
use std::collections::BTreeMap;
pub(super) fn detect(command: &str) -> Result<Option<String>, ShellParseError> {
    at_depth(command, 0)
}
fn at_depth(command: &str, depth: usize) -> Result<Option<String>, ShellParseError> {
    if depth > 8 {
        return Ok(Some(
            "nested shell command beyond guard inspection limit".to_string(),
        ));
    }
    let source = ShellText::new(command.to_string()).without_heredocs()?;
    let substitutions = harness_infrastructure::ShellSubstitutions::parse(&source);
    let bodies = harness_infrastructure::heredoc_substitution_bodies(command)?
        .combine(substitutions.bodies());
    for index in 0..bodies.len() {
        if let Some(body) = bodies.at(index) {
            let nested = at_depth(body, depth + 1)?;
            if nested.is_some() {
                return Ok(nested);
            }
        }
    }
    let segments = split_shell_segments(substitutions.masked());
    let mut assignments = BTreeMap::<String, String>::new();
    for index in 0..segments.len() {
        let Some(segment) = segments.at(index) else {
            continue;
        };
        let words = ShellWords::parse(segment);
        if !words.is_empty()
            && words.fold_left(true, |valid, word| valid && assignment(word).is_some())
        {
            words.fold_left((), |(), word| {
                if let Some((name, value)) = assignment(word) {
                    assignments.insert(name.to_string(), value.to_string());
                }
            });
            continue;
        }
        let mut argv = match ShellInvocation::parse(segment) {
            ShellInvocation::Known(words) => words,
            ShellInvocation::Uninspectable => {
                return Ok(Some(
                    "execution wrapper beyond guard inspection".to_string(),
                ));
            }
        };
        if let Some(variable) = variable(argv.at(0).unwrap_or("")) {
            let resolved = assignments.get(variable).map_or_else(
                || ShellWords::new(Vec::new()),
                |value| ShellWords::parse(value),
            );
            if resolved.len() != 1 {
                return Ok(Some(
                    "dynamic executable beyond guard inspection".to_string(),
                ));
            }
            argv = argv.replace_range(0, 1, &resolved);
        }
        if argv.at(0).is_some_and(|word| word.contains('$')) {
            return Ok(Some(
                "dynamic executable beyond guard inspection".to_string(),
            ));
        }
        let executable = basename(argv.at(0).unwrap_or(""));
        if executable == "eval" {
            let args = argv.suffix(if argv.at(1) == Some("--") { 2 } else { 1 });
            let nested = at_depth(&args.join(" "), depth + 1)?;
            if matches!(
                nested.as_deref(),
                Some(
                    "dynamic executable beyond guard inspection"
                        | "dynamic shell command beyond guard inspection"
                )
            ) {
                return Ok(Some(
                    "dynamic eval shell command beyond guard inspection".to_string(),
                ));
            }
            if nested.is_some() {
                return Ok(nested);
            }
            if segment.contains(['$', '`', '\\']) {
                return Ok(Some(
                    "dynamic eval shell command beyond guard inspection".to_string(),
                ));
            }
            continue;
        }
        let shell = executable.strip_suffix(".exe").unwrap_or(&executable);
        if matches!(shell, "sh" | "bash" | "dash" | "ash" | "ksh" | "zsh") {
            let mut i = 1;
            while let Some(option) = argv.at(i) {
                if matches!(
                    option,
                    "-O" | "+O" | "-o" | "+o" | "--rcfile" | "--init-file"
                ) {
                    i += 2;
                    continue;
                }
                if option == "-c"
                    || option.strip_prefix('-').is_some_and(|flags| {
                        flags.contains('c') && flags.chars().all(|ch| ch.is_ascii_alphabetic())
                    })
                {
                    let command_index = i + 1 + usize::from(argv.at(i + 1) == Some("--"));
                    let mut command = argv.at(command_index).unwrap_or("");
                    if let Some(name) = variable(command) {
                        let Some(value) = assignments.get(name) else {
                            return Ok(Some(
                                "dynamic shell command beyond guard inspection".to_string(),
                            ));
                        };
                        command = value;
                    }
                    if command.contains('$') {
                        return Ok(Some(
                            "dynamic shell command beyond guard inspection".to_string(),
                        ));
                    }
                    let nested = at_depth(command, depth + 1)?;
                    if nested.is_some() {
                        return Ok(nested);
                    }
                    break;
                }
                if !option.starts_with('-') {
                    break;
                }
                i += 1;
            }
            continue;
        }
        let (script, args) = if matches!(executable.as_str(), "bun" | "bun.exe") {
            let Some(invocation) = bun_script(&argv) else {
                continue;
            };
            invocation
        } else {
            (executable, argv.suffix(1))
        };
        let positional = without_project_dir(&args);
        let verb = positional.at(0).unwrap_or("");
        match script.as_str() {
            "aidlc-orchestrate.ts" if matches!(verb, "next" | "continue" | "report" | "park") => {
                return Ok(Some(format!("{script} {verb}")));
            }
            "aidlc-state.ts" if state_mutation(verb) => {
                return Ok(Some(format!("{script} {verb}")));
            }
            "aidlc-jump.ts" if verb == "execute" => return Ok(Some(format!("{script} {verb}"))),
            "aidlc-utility.ts" => {
                if let Some(found) = utility(&script, &args) {
                    return Ok(Some(found));
                }
            }
            "aidlc.ts" | "aidlc" | "aidlc.exe" => {
                let prefix = if script == "aidlc.ts" {
                    "aidlc.ts"
                } else {
                    "aidlc"
                };
                if let Some(found) = dispatcher(prefix, &positional) {
                    return Ok(Some(found));
                }
            }
            _ => (),
        }
    }
    Ok(None)
}
fn basename(word: &str) -> String {
    word.replace('\\', "/")
        .rsplit('/')
        .next()
        .unwrap_or("")
        .to_string()
}
fn identifier(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|ch| ch.is_ascii_alphabetic() || ch == '_')
        && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}
fn assignment(word: &str) -> Option<(&str, &str)> {
    word.split_once('=').filter(|(name, _)| identifier(name))
}
fn variable(word: &str) -> Option<&str> {
    word.strip_prefix("${")
        .and_then(|word| word.strip_suffix('}'))
        .or_else(|| word.strip_prefix('$'))
        .filter(|word| identifier(word))
}
fn bun_script(words: &ShellWords) -> Option<(String, ShellWords)> {
    let mut cursor = 1;
    let options = |cursor: &mut usize| {
        while let Some(option) = words.at(*cursor).filter(|word| word.starts_with('-')) {
            if option == "--" {
                *cursor += 1;
                return true;
            }
            if matches!(option, "-e" | "--eval" | "-p" | "--print") {
                return false;
            }
            *cursor += if matches!(
                option,
                "-C" | "--cwd"
                    | "-r"
                    | "--preload"
                    | "--define"
                    | "--loader"
                    | "--conditions"
                    | "--env-file"
                    | "--config"
            ) {
                2
            } else {
                1
            };
        }
        true
    };
    if !options(&mut cursor) {
        return None;
    }
    if words.at(cursor) == Some("run") {
        cursor += 1;
        if !options(&mut cursor) {
            return None;
        }
    }
    let script = basename(words.at(cursor)?);
    if script.is_empty() {
        return None;
    }
    Some((script, words.suffix(cursor + 1)))
}
fn without_project_dir(args: &ShellWords) -> ShellWords {
    let mut words = Vec::new();
    let mut i = 0;
    while let Some(word) = args.at(i) {
        if word == "--project-dir" {
            i += 2;
        } else {
            words.push(word.to_string());
            i += 1;
        }
    }
    ShellWords::new(words)
}
fn state_mutation(verb: &str) -> bool {
    super::state_transition_guard::blocked_state_transition(verb)
        || matches!(
            verb,
            "set-skeleton-stance"
                | "set-construction-iteration"
                | "set-unit-ownership"
                | "set-unit-gate-rhythm"
                | "acknowledge-compaction"
                | "reuse-artifact"
                | "practices-event"
                | "practices-promote"
                | "fork"
                | "merge"
                | "unpark"
        )
}
/// 二段形が前置する名前空間トークン。
const ENGINE_NAMESPACE: &str = "engine";
/// ディスパッチャ面の進行コマンドを、名前空間を剥がしてから分類する。
///
/// 本家 `delegatedDispatcherCommand` (`.claude/hooks/aidlc-state-transition-guard.ts:774-778`)
/// と同じく、先頭の名前空間トークンを剥がしてから一段形と同じ規則で読み、拒否文の接頭辞には
/// 剥がした名前空間を戻す。剥がさないと `aidlc engine intent create` がどの腕にも当たらず、
/// 委譲エージェントが作業記録を作れてしまう。
///
/// 本家が併せて剥がす `system` は写さない。`EngineRoute::resolve`
/// (`modules/app/aidlc/src/cli/engine_route.rs:320-323`) は先頭が `engine` のときだけ働くので、
/// `aidlc system …` で届く native の経路が無く、守る不変条件が無いためである。
///
/// `orchestrate` の 4 動詞は二段形も一段形と同じ進行コマンドとして分類する。native は
/// `WIRED` (`engine_route.rs:53-56`) で `orchestrate next|continue|report|park` を運ぶので、
/// 一段形だけを見ていると委譲エージェントが二段形でガードを抜ける。読み取り専用の
/// `orchestrate help` は本家の一段形と同じく素通りさせる。
fn dispatcher(prefix: &str, args: &ShellWords) -> Option<String> {
    let namespaced = args.at(0) == Some(ENGINE_NAMESPACE);
    let prefix = if namespaced {
        format!("{prefix} {ENGINE_NAMESPACE}")
    } else {
        prefix.to_string()
    };
    let args = if namespaced {
        args.suffix(1)
    } else {
        args.clone()
    };
    let group = args.at(0).unwrap_or("");
    let verb = args.at(1).unwrap_or("");
    if matches!(
        group,
        "next"
            | "continue"
            | "report"
            | "park"
            | "--resume"
            | "--scope"
            | "scope-change"
            | "config-change"
            | "compose"
            | "recompose"
            | "init"
    ) {
        return Some(format!("{prefix} {group}"));
    }
    if (group == "scope" && verb == "change")
        || (group == "orchestrate" && matches!(verb, "next" | "continue" | "report" | "park"))
        || (group == "state" && state_mutation(verb))
        || (group == "jump" && verb == "execute")
        || (group == "config" && verb == "set")
    {
        return Some(format!("{prefix} {group} {verb}"));
    }
    workspace(&prefix, &args)
}
fn utility(prefix: &str, args: &ShellWords) -> Option<String> {
    let mut positional = Vec::new();
    let mut i = 0;
    while let Some(word) = args.at(i) {
        if word.starts_with("--") {
            i += if !word.contains('=')
                && args.at(i + 1).is_some_and(|next| !next.starts_with("--"))
            {
                2
            } else {
                1
            };
        } else {
            positional.push(word.to_string());
            i += 1;
        }
    }
    let args = ShellWords::new(positional);
    let verb = args.at(0).unwrap_or("");
    if matches!(
        verb,
        "scope-change"
            | "config-change"
            | "recompose"
            | "intent-create"
            | "state-init"
            | "space-create"
    ) {
        Some(format!("{prefix} {verb}"))
    } else {
        workspace(prefix, &args)
    }
}
fn workspace(prefix: &str, args: &ShellWords) -> Option<String> {
    let noun = args.at(0)?;
    let verb = args.at(1)?;
    if verb == "--help" {
        return None;
    }
    if noun == "space-create" {
        return Some(format!("{prefix} space-create"));
    }
    if !matches!(noun, "space" | "intent")
        || matches!(
            verb,
            "--json" | "help" | "-h" | "list" | "archive" | "rename" | "show" | "birth"
        )
    {
        return None;
    }
    if verb == "switch" {
        return args.at(2).map(|_| format!("{prefix} {noun} switch"));
    }
    if verb == "create" {
        return if noun == "intent" {
            Some(format!("{prefix} intent create"))
        } else {
            args.at(2).map(|_| format!("{prefix} space create"))
        };
    }
    Some(format!("{prefix} {noun} {verb}"))
}

#[cfg(test)]
mod tests {
    use super::detect;

    fn found(command: &str) -> Option<String> {
        detect(command).expect("固定パターンは常に妥当")
    }

    #[test]
    fn a_lifecycle_command_inside_a_substitution_or_heredoc_is_still_found() {
        assert_eq!(
            found("echo $(bun aidlc-orchestrate.ts next)").as_deref(),
            Some("aidlc-orchestrate.ts next")
        );
        assert_eq!(
            found("echo `bun aidlc-orchestrate.ts report`").as_deref(),
            Some("aidlc-orchestrate.ts report")
        );
        assert_eq!(found("echo $(ls) && ls").as_deref(), None);
    }

    #[test]
    fn a_dynamic_executable_spelling_is_refused_rather_than_guessed() {
        assert_eq!(
            found("$X/bin/bun aidlc-orchestrate.ts next").as_deref(),
            Some("dynamic executable beyond guard inspection")
        );
        assert_eq!(
            found("$CMD next").as_deref(),
            Some("dynamic executable beyond guard inspection")
        );
        // 直前の代入で 1 語に解決できれば、その語で判定する。
        assert_eq!(
            found("CMD=bun; $CMD aidlc-orchestrate.ts next").as_deref(),
            Some("aidlc-orchestrate.ts next")
        );
        assert_eq!(
            found("CMD='bun run'; $CMD aidlc-orchestrate.ts next").as_deref(),
            Some("dynamic executable beyond guard inspection")
        );
    }

    #[test]
    fn an_eval_is_inspected_through_its_argument_and_refused_when_dynamic() {
        assert_eq!(
            found("eval 'bun aidlc-orchestrate.ts next'").as_deref(),
            Some("aidlc-orchestrate.ts next")
        );
        assert_eq!(
            found("eval -- 'bun aidlc-orchestrate.ts park'").as_deref(),
            Some("aidlc-orchestrate.ts park")
        );
        assert_eq!(
            found("eval '$CMD next'").as_deref(),
            Some("dynamic eval shell command beyond guard inspection")
        );
        assert_eq!(
            found("eval 'echo $x'").as_deref(),
            Some("dynamic eval shell command beyond guard inspection")
        );
        assert_eq!(found("eval 'echo hi'; ls").as_deref(), None);
    }

    #[test]
    fn a_shell_dash_c_is_inspected_through_its_script_string() {
        assert_eq!(
            found("sh -c 'bun aidlc-orchestrate.ts next'").as_deref(),
            Some("aidlc-orchestrate.ts next")
        );
        assert_eq!(
            found("bash -xc 'bun aidlc-orchestrate.ts next'").as_deref(),
            Some("aidlc-orchestrate.ts next")
        );
        assert_eq!(
            found("zsh -o pipefail -c -- 'bun aidlc-orchestrate.ts next'").as_deref(),
            Some("aidlc-orchestrate.ts next")
        );
        assert_eq!(
            found("sh -c \"$CMD\"").as_deref(),
            Some("dynamic shell command beyond guard inspection")
        );
        assert_eq!(
            found("CMD='echo $x'; sh -c \"$CMD\"").as_deref(),
            Some("dynamic shell command beyond guard inspection")
        );
        assert_eq!(
            found("sh -c 'echo $x'").as_deref(),
            Some("dynamic shell command beyond guard inspection")
        );
        assert_eq!(found("sh -c 'echo hi'; ls").as_deref(), None);
        assert_eq!(found("sh -x script.sh; ls").as_deref(), None);
        assert_eq!(found("sh script.sh").as_deref(), None);
        assert_eq!(found("bash.exe -c 'ls'").as_deref(), None);
    }

    #[test]
    fn bun_switches_are_skipped_before_the_script_and_an_eval_is_opaque() {
        for command in [
            "bun -- aidlc-orchestrate.ts next",
            "bun --cwd /x aidlc-orchestrate.ts next",
            "bun --define a=b --loader .x:text aidlc-orchestrate.ts next",
            "bun --conditions dev --env-file .env --config bunfig.toml aidlc-orchestrate.ts next",
            "bun -r ./pre.ts --preload ./pre.ts aidlc-orchestrate.ts next",
            "bun --hot aidlc-orchestrate.ts next",
            "bun run --hot aidlc-orchestrate.ts next",
            "bun run -- aidlc-orchestrate.ts next",
            "/usr/local/bin/bun.exe run .claude/tools/aidlc-orchestrate.ts next",
        ] {
            assert_eq!(
                found(command).as_deref(),
                Some("aidlc-orchestrate.ts next"),
                "{command}"
            );
        }
        assert_eq!(found("bun -e 'x'").as_deref(), None);
        assert_eq!(found("bun run --eval 'x'").as_deref(), None);
        assert_eq!(found("bun run /").as_deref(), None);
        assert_eq!(found("bun").as_deref(), None);
    }

    #[test]
    fn the_utility_verbs_are_read_after_dropping_switches() {
        assert_eq!(
            found("bun aidlc-utility.ts --json intent-create").as_deref(),
            None,
            "--json は次の語を値として消費する"
        );
        assert_eq!(
            found("bun aidlc-utility.ts --scope=x intent-create").as_deref(),
            Some("aidlc-utility.ts intent-create")
        );
        assert_eq!(
            found("bun aidlc-utility.ts --json --quiet=1 state-init").as_deref(),
            Some("aidlc-utility.ts state-init")
        );
        assert_eq!(
            found("bun aidlc-utility.ts space-create x").as_deref(),
            Some("aidlc-utility.ts space-create")
        );
        assert_eq!(
            found("bun aidlc-utility.ts intent switch x").as_deref(),
            Some("aidlc-utility.ts intent switch")
        );
        assert_eq!(found("bun aidlc-utility.ts help").as_deref(), None);
        assert_eq!(found("bun aidlc-utility.ts intent list").as_deref(), None);
    }

    #[test]
    fn the_dispatcher_workspace_verbs_are_classified_by_noun_and_verb() {
        assert_eq!(found("bun aidlc.ts intent --help").as_deref(), None);
        assert_eq!(
            found("bun aidlc.ts space-create x").as_deref(),
            Some("aidlc.ts space-create")
        );
        assert_eq!(
            found("aidlc intent create").as_deref(),
            Some("aidlc intent create")
        );
        assert_eq!(
            found("aidlc space create x").as_deref(),
            Some("aidlc space create")
        );
        assert_eq!(found("aidlc space create").as_deref(), None);
        assert_eq!(
            found("aidlc.exe intent switch x").as_deref(),
            Some("aidlc intent switch")
        );
        assert_eq!(found("aidlc intent switch").as_deref(), None);
        assert_eq!(
            found("aidlc intent delete x").as_deref(),
            Some("aidlc intent delete")
        );
        assert_eq!(found("aidlc intent show x").as_deref(), None);
        assert_eq!(found("aidlc other verb").as_deref(), None);
        assert_eq!(
            found("aidlc --project-dir /x next").as_deref(),
            Some("aidlc next")
        );
        assert_eq!(
            found("aidlc state fork").as_deref(),
            Some("aidlc state fork")
        );
        assert_eq!(
            found("aidlc jump execute").as_deref(),
            Some("aidlc jump execute")
        );
        assert_eq!(
            found("bun aidlc-jump.ts execute").as_deref(),
            Some("aidlc-jump.ts execute")
        );
        assert_eq!(
            found("bun aidlc-state.ts practices-promote").as_deref(),
            Some("aidlc-state.ts practices-promote")
        );
    }

    /// 二段形は名前空間を剥がしてから分類され、拒否文には剥がした `engine` が戻る。
    ///
    /// 剥がしが過剰でも過少でもないことを、同じ名詞の読み取り専用動詞（`intent list`）、
    /// 第 2 の条件が受ける組（`state fork`）、名前空間だけで動詞が無い境界（`aidlc engine`）と
    /// 並べて固定する。
    #[test]
    fn the_engine_namespace_is_stripped_before_the_dispatcher_classifies_the_verb() {
        assert_eq!(
            found("aidlc engine intent create --scope bugfix --arguments='fix the crash'")
                .as_deref(),
            Some("aidlc engine intent create")
        );
        assert_eq!(found("aidlc engine intent list").as_deref(), None);
        assert_eq!(
            found("aidlc engine state fork").as_deref(),
            Some("aidlc engine state fork")
        );
        assert_eq!(found("aidlc engine").as_deref(), None);
    }

    /// `orchestrate` の 4 動詞は二段形でも進行コマンドである。
    ///
    /// native は `engine_route.rs` の `WIRED` で `orchestrate next|continue|report|park` を
    /// 運ぶので、一段形だけを見ていると委譲エージェントが二段形でガードを抜ける。
    /// 読み取り専用の `orchestrate help` が素通りすることも併せて固定する。
    #[test]
    fn the_two_part_orchestrate_verbs_are_progress_commands_like_their_one_part_forms() {
        for verb in ["next", "continue", "report", "park"] {
            assert_eq!(
                found(&format!("aidlc engine orchestrate {verb}")).as_deref(),
                Some(format!("aidlc engine orchestrate {verb}").as_str())
            );
            assert_eq!(
                found(&format!("aidlc orchestrate {verb}")).as_deref(),
                Some(format!("aidlc orchestrate {verb}").as_str())
            );
        }
        assert_eq!(found("aidlc engine orchestrate help").as_deref(), None);
    }

    #[test]
    fn an_uninspectable_wrapper_and_a_deep_nesting_are_refused() {
        assert_eq!(
            found("env --unknown bun aidlc-orchestrate.ts next").as_deref(),
            Some("execution wrapper beyond guard inspection")
        );
        let nested = (0..9).fold("ls".to_string(), |acc, _| {
            format!("sh -c '{}'", acc.replace('\'', "'\\''"))
        });
        assert_eq!(
            found(&nested).as_deref(),
            Some("nested shell command beyond guard inspection limit")
        );
    }
}
