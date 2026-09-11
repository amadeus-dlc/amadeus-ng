//! シェル文字列から実行位置を探すための、引用・文書領域の投影。
use super::{ShellParseError, compile_shell_pattern as expression};
use std::collections::VecDeque;
/// 原文を保持し、書込みやプロセス実行をしない観測値。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellText {
    text: String,
}
impl ShellText {
    /// 原文を保持する。
    #[must_use]
    pub const fn new(text: String) -> Self {
        Self { text }
    }
    /// heredocの本文だけを隠す。引用内の実行置換は別の解析で扱う。
    /// # Errors
    /// 固定パターンが不正な場合。
    pub fn without_heredocs(&self) -> Result<String, ShellParseError> {
        mask_heredocs(&self.text)
    }
    /// 引用中の区切り・heredoc本文・未実行の関数定義を隠した文字列。
    /// # Errors
    /// 固定パターンの不正または文字位置の不整合。
    pub fn invocation_text(&self) -> Result<String, ShellParseError> {
        mask_functions(&mask_heredocs(&mask_quotes(&self.text))?)
    }
}
fn mask_quotes(command: &str) -> String {
    let mut chars: Vec<char> = command.chars().collect();
    let mut i = 0;
    while let Some(quote) = chars.get(i).copied() {
        if !matches!(quote, '\'' | '"' | '`') {
            i += 1;
            continue;
        }
        let mut end = i + 1;
        let mut escaped = false;
        while let Some(ch) = chars.get(end).copied() {
            if quote != '\'' && !escaped && ch == '\\' {
                escaped = true;
                end += 1;
                continue;
            }
            if !escaped && ch == quote {
                break;
            }
            escaped = false;
            end += 1;
        }
        if end >= chars.len() {
            end = chars.len().saturating_sub(1);
        }
        let multiline = chars.iter().skip(i).take(end - i + 1).any(|ch| *ch == '\n');
        let mut depth = 0;
        let mut j = i;
        while j <= end {
            let starts = quote == '"'
                && chars.get(j) == Some(&'$')
                && chars.get(j + 1) == Some(&'(')
                && (j == 0 || chars.get(j - 1) != Some(&'\\'));
            if starts {
                depth += 1;
                j += 2;
                continue;
            }
            if quote == '"'
                && depth > 0
                && chars.get(j) == Some(&')')
                && (j == 0 || chars.get(j - 1) != Some(&'\\'))
            {
                depth -= 1;
                j += 1;
                continue;
            }
            if depth > 0 {
                j += 1;
                continue;
            }
            if let Some(ch) = chars.get_mut(j)
                && ((multiline && *ch != '\n')
                    || (!multiline && matches!(*ch, '&' | '|' | ';' | '(' | '{')))
            {
                *ch = ' ';
            }
            j += 1;
        }
        i = end + 1;
    }
    chars.into_iter().collect()
}
struct Heredoc {
    delimiter: String,
    strip_tabs: bool,
}
fn mask_heredocs(command: &str) -> Result<String, ShellParseError> {
    let pattern = expression(r#"<<(-)?\s*(?:'([^']+)'|"([^"]+)"|([A-Za-z_][A-Za-z0-9_]*))"#)?;
    let mut pending = VecDeque::<Heredoc>::new();
    let mut lines = Vec::new();
    for line in command.split('\n') {
        if let Some(active) = pending.front() {
            let candidate = if active.strip_tabs {
                line.trim_start_matches('\t')
            } else {
                line
            };
            let closes = candidate == active.delimiter;
            lines.push(" ".repeat(line.encode_utf16().count()));
            if closes {
                pending.pop_front();
            }
            continue;
        }
        for found in pattern.captures_iter(line) {
            if let Some(delimiter) = found
                .get(2)
                .or_else(|| found.get(3))
                .or_else(|| found.get(4))
            {
                pending.push_back(Heredoc {
                    delimiter: delimiter.as_str().to_string(),
                    strip_tabs: found.get(1).is_some(),
                });
            }
        }
        lines.push(line.to_string());
    }
    Ok(lines.join("\n"))
}
fn mask_functions(command: &str) -> Result<String, ShellParseError> {
    if !command.contains('{') {
        return Ok(command.to_string());
    }
    let pattern = expression(
        r"(?:^|[;\n])[ \t]*(?:(?:function[ \t]+)?[A-Za-z_][A-Za-z0-9_]*[ \t]*\([ \t]*\)|function[ \t]+[A-Za-z_][A-Za-z0-9_]*)[ \t\n]*\{",
    )?;
    let mut chars: Vec<char> = command.chars().collect();
    let mut last_index = 0;
    loop {
        let source: String = chars.iter().collect();
        let Some(found) = pattern.find_at(&source, byte_for_utf16(&source, last_index)) else {
            break;
        };
        let prefix = source
            .get(..found.start())
            .ok_or(ShellParseError::Boundary {
                offset: found.start(),
            })?;
        let start_index = prefix.encode_utf16().count();
        let brace = found.as_str().rfind('{').ok_or(ShellParseError::Boundary {
            offset: found.start(),
        })?;
        // 固定本家はRegExpのUTF-16位置をspread配列の位置に渡す。この観測差も保持する。
        let open = start_index + brace;
        let mut depth = 0i64;
        let mut quote = None;
        let mut escaped = false;
        let mut end = open;
        while let Some(ch) = chars.get(end).copied() {
            if let Some(active) = quote {
                if active != '\'' && !escaped && ch == '\\' {
                    escaped = true;
                    end += 1;
                    continue;
                }
                if !escaped && ch == active {
                    quote = None;
                }
                escaped = false;
                end += 1;
                continue;
            }
            if matches!(ch, '\'' | '"' | '`') {
                quote = Some(ch);
            } else if ch == '{' {
                depth += 1;
            } else if ch == '}' {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
            end += 1;
        }
        if depth != 0 {
            break;
        }
        let start = start_index + usize::from(found.as_str().starts_with([';', '\n']));
        for ch in chars.iter_mut().take(end + 1).skip(start) {
            if *ch != '\n' {
                *ch = ' ';
            }
        }
        last_index = end + 1;
    }
    Ok(chars.into_iter().collect())
}
fn byte_for_utf16(text: &str, offset: usize) -> usize {
    let mut units = 0;
    for (byte, ch) in text.char_indices() {
        if units >= offset {
            return byte;
        }
        units += ch.len_utf16();
    }
    text.len()
}

#[cfg(test)]
mod tests {
    use super::ShellText;

    fn invocation(command: &str) -> String {
        ShellText::new(command.to_string())
            .invocation_text()
            .expect("固定パターンは常に妥当")
    }

    #[test]
    fn separators_inside_quotes_are_hidden_but_the_command_shape_stays() {
        assert_eq!(invocation("echo 'a;b' ; rm x"), "echo 'a b' ; rm x");
        assert_eq!(invocation("echo \"a|b\" && rm x"), "echo \"a b\" && rm x");
        // 引用の中のコマンド置換は隠さない (別の解析が読む)。
        assert_eq!(invocation("echo \"$(rm x; ls)\""), "echo \"$(rm x; ls)\"");
    }

    #[test]
    fn an_unterminated_quote_hides_separators_to_the_end() {
        assert_eq!(invocation("echo \"a; rm x"), "echo \"a  rm x");
        // 複数行にまたがる引用は改行以外を全て隠す。
        assert_eq!(invocation("echo 'a\n rm x"), "echo   \n     ");
    }

    #[test]
    fn a_heredoc_body_is_blanked_line_by_line() {
        let text = ShellText::new("cat <<EOF\nrm x\nEOF\nrm y".to_string());
        assert_eq!(
            text.without_heredocs().unwrap(),
            "cat <<EOF\n    \n   \nrm y"
        );
        let tabbed = ShellText::new("cat <<-'EOF'\n\trm x\n\tEOF\nrm y".to_string());
        assert_eq!(
            tabbed.without_heredocs().unwrap(),
            "cat <<-'EOF'\n     \n    \nrm y"
        );
    }

    #[test]
    fn a_function_definition_is_blanked_while_the_call_site_stays() {
        assert_eq!(invocation("f() { rm x; }; rm y"), "             ; rm y");
        assert_eq!(
            invocation("function f {\n rm x\n}\nrm y"),
            "            \n     \n \nrm y"
        );
        // 定義の中の引用は `}` を閉じない。逃がした引用符も閉じない。
        assert_eq!(
            invocation("f() { echo \"}\" ; }; rm y"),
            "                  ; rm y"
        );
        assert_eq!(
            invocation("f() { echo \"\\\"}\" ; }; rm y"),
            "                    ; rm y"
        );
        assert_eq!(
            invocation("f() { echo '}' ; }; rm y"),
            "                  ; rm y"
        );
        assert_eq!(
            invocation("f() { echo `}` ; }; rm y"),
            "                  ; rm y"
        );
        // 定義だけで終わる文字列は全体が隠れる。
        assert_eq!(invocation("f() { rm x; }"), "             ");
        // 閉じない定義は隠さない。
        assert_eq!(invocation("f() { rm x; rm y"), "f() { rm x; rm y");
        assert_eq!(invocation("rm { x"), "rm { x");
    }
}
