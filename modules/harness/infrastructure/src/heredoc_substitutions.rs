//! 非引用heredocの中で実行されるコマンド置換。
use super::{ShellParseError, ShellSubstitutions, compile_shell_pattern};
use core_infrastructure::collections::Collection;
use std::collections::VecDeque;
struct PendingHeredoc {
    delimiter: String,
    strip_tabs: bool,
    executable: bool,
    lines: Vec<String>,
}
/// 閉じた非引用heredocから、実行される本文だけを出現順に返す。
/// # Errors
/// 固定パターンが不正な場合。
pub fn heredoc_substitution_bodies(command: &str) -> Result<Collection<String>, ShellParseError> {
    let pattern =
        compile_shell_pattern(r#"<<(-)?\s*(?:'([^']+)'|"([^"]+)"|([A-Za-z_][A-Za-z0-9_]*))"#)?;
    let mut pending = VecDeque::<PendingHeredoc>::new();
    let mut bodies = Collection::empty();
    for line in command.split('\n') {
        if let Some(active) = pending.front_mut() {
            let candidate = if active.strip_tabs {
                line.trim_start_matches('\t')
            } else {
                line
            };
            if candidate == active.delimiter {
                if active.executable {
                    bodies = bodies
                        .combine(ShellSubstitutions::parse(&active.lines.join("\n")).bodies());
                }
                pending.pop_front();
            } else {
                active.lines.push(line.to_string());
            }
            continue;
        }
        for found in pattern.captures_iter(line) {
            if let Some(delimiter) = found
                .get(2)
                .or_else(|| found.get(3))
                .or_else(|| found.get(4))
            {
                pending.push_back(PendingHeredoc {
                    delimiter: delimiter.as_str().to_string(),
                    strip_tabs: found.get(1).is_some(),
                    executable: found.get(4).is_some(),
                    lines: Vec::new(),
                });
            }
        }
    }
    Ok(bodies)
}
