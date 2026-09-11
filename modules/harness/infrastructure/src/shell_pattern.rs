//! シェル観測の固定パターンの文字集合。
use super::ShellParseError;
/// 空白と単語境界をECMAScriptの非Unicodeモードに合わせる。
/// この入口の固定パターンは大小無視や長さ制限を持たない。
/// # Errors
/// パターンが不正な場合。
pub fn compile_shell_pattern(pattern: &str) -> Result<regex::Regex, ShellParseError> {
    let whitespace = r"[\x{0009}-\x{000D}\x{0020}\x{00A0}\x{1680}\x{2000}-\x{200A}\x{2028}\x{2029}\x{202F}\x{205F}\x{3000}\x{FEFF}]";
    Ok(regex::Regex::new(
        &pattern
            .replace(r"\s", whitespace)
            .replace(r"\b", r"(?-u:\b)"),
    )?)
}
