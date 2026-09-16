//! `reuse-artifact` の位置引数とフラグを、本家 parseFlags の構文で未解釈のまま保持する。
use std::collections::BTreeMap;
/// 成果物再利用の受領の入力。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReuseArtifactArgs {
    slug: Option<String>,
    flags: BTreeMap<String, String>,
    parse_error: Option<String>,
}
impl ReuseArtifactArgs {
    /// 位置引数のステージ slug。
    #[must_use]
    pub fn slug(&self) -> Option<&str> {
        self.slug.as_deref()
    }
    /// フラグの生値。空値と省略を区別する。
    #[must_use]
    pub fn value(&self, name: &str) -> Option<&str> {
        self.flags.get(name).map(String::as_str)
    }
    /// 最初の構文違反。
    #[must_use]
    pub fn parse_error(&self) -> Option<&str> {
        self.parse_error.as_deref()
    }
}
pub(super) fn parse_reuse_artifact(args: &[String]) -> ReuseArtifactArgs {
    let mut slug = None;
    let mut flags = BTreeMap::new();
    let mut parse_error = None;
    let mut index = 0;
    while let Some(arg) = args.get(index) {
        index += 1;
        let Some(name) = arg.strip_prefix("--") else {
            if slug.is_none() {
                slug = Some(arg.clone());
            }
            continue;
        };
        if name == "single" {
            flags.insert(name.to_string(), "true".to_string());
            continue;
        }
        let Some(value) = args.get(index) else {
            parse_error = Some(crate::wording::flag_expects_a_value(arg));
            break;
        };
        if value.starts_with("--") {
            parse_error = Some(crate::wording::flag_expects_a_value_got_flag(arg, value));
            break;
        }
        index += 1;
        flags.insert(name.to_string(), value.clone());
    }
    ReuseArtifactArgs {
        slug,
        flags,
        parse_error,
    }
}
