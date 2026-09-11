//! 先頭workspace名詞の構文を、本家が名指す終端ユーティリティへ写す。
use core_query_use_case::orchestration::{Directive, EngineCommand, ReadOnlyVerb};

pub(super) fn draw(tokens: &[String]) -> Directive {
    let Some(noun) = tokens.first().map(String::as_str) else {
        return error("Invalid workspace command.".to_string());
    };
    let second = tokens.get(1).map(String::as_str);
    if noun == "space-create" {
        return second.map_or_else(
            || error("Usage: aidlc space-create <name>".to_string()),
            |name| print(&["space-create", name]),
        );
    }
    match second {
        None => print(&[noun]),
        Some("--json") => print(&[noun, "--json"]),
        Some("help" | "-h") => Directive::Print {
            message: crate::wording::read_only(
                &EngineCommand::ReadOnlyUtility(ReadOnlyVerb::Help).cli_spelling(),
            ),
            narration: None,
        },
        Some(verb @ ("archive" | "rename" | "show" | "birth")) => error(format!(
            "{noun} {verb} is reserved for a future workspace verb and is not implemented yet. Use {noun} switch {verb} to select an existing record with that name."
        )),
        Some("list") => {
            if tokens.get(2).is_some_and(|token| token == "--json") {
                print(&[noun, "--json"])
            } else {
                print(&[noun])
            }
        }
        Some("switch") => tokens.get(2).map_or_else(
            || error(format!("Usage: aidlc {noun} switch <name>")),
            |name| print(&[noun, "switch", name]),
        ),
        Some("create") if noun == "intent" => {
            let mut argv = vec!["intent-create"];
            argv.extend(tokens.iter().skip(2).map(String::as_str));
            print(&argv)
        }
        Some("create") => tokens.get(2).map_or_else(
            || error("Usage: aidlc space create <name>".to_string()),
            |name| print(&["space-create", name]),
        ),
        Some(name) => print(&[noun, name]),
    }
}

fn print(argv: &[&str]) -> Directive {
    let command = EngineCommand::NounTokens(argv.iter().map(|arg| (*arg).to_string()).collect());
    Directive::Print {
        message: format!(
            "Run `{}`, print its output verbatim, then stop.",
            command.cli_spelling()
        ),
        narration: None,
    }
}

const fn error(message: String) -> Directive {
    Directive::Error { message }
}
