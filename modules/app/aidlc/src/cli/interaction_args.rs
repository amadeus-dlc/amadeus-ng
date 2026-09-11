//! logの質問・回答フラグを未解釈で保持する入力境界。
use std::collections::BTreeMap;

/// 配布parseFlagsに従うフラグと構文エラー。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InteractionArgs {
    flags: BTreeMap<String, String>,
    parse_error: Option<String>,
}
impl InteractionArgs {
    /// 名前を指定して生値を読む。
    #[must_use]
    pub fn value(&self, name: &str) -> Option<&str> {
        self.flags.get(name).map(String::as_str)
    }
    /// 値を欠くフラグ。
    #[must_use]
    pub fn parse_error(&self) -> Option<&str> {
        self.parse_error.as_deref()
    }
}
pub(super) fn parse_interaction(args: &[String]) -> InteractionArgs {
    let mut result = InteractionArgs::default();
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        let Some(name) = arg.strip_prefix("--") else {
            continue;
        };
        let value = if matches!(
            name,
            "single" | "retry-pending" | "stage-level" | "exact-option-labels"
        ) {
            "true"
        } else {
            match args.next().filter(|value| !value.starts_with("--")) {
                Some(value) => value,
                None => {
                    result.parse_error = Some(format!("Missing value for --{name}"));
                    break;
                }
            }
        };
        result.flags.insert(name.to_string(), value.to_string());
    }
    result
}
