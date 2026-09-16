//! `aidlc-utility intent` が運ぶ位置動詞とフラグ（upstream `handleIntent`）。

/// `intent` の位置動詞と `--json`。
///
/// upstream は `positional[1]` を動詞として読み、動詞が無いときは一覧を出す。`--json` の
/// 真偽は `parseArgs` の意味論に従う — 次のトークンが `--` で始まらなければ値として食い、
/// さもなくば `"true"` を入れるので、`--json` 単独は `"true"` になる。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IntentArgs {
    sub: Option<String>,
    json: Option<String>,
}

impl IntentArgs {
    /// 位置動詞（無ければ `None` — upstream は一覧に倒す）。
    #[must_use]
    pub fn sub(&self) -> Option<&str> {
        self.sub.as_deref()
    }

    /// JSON 形で出すか（`--json`）。upstream は `flags.json === "true"` で判定する。
    #[must_use]
    pub fn is_json(&self) -> bool {
        self.json.as_deref() == Some("true")
    }
}

/// `intent` の残り引数から位置動詞とフラグを畳む。
pub(super) fn parse_intent(args: &[String]) -> IntentArgs {
    let mut parsed = IntentArgs::default();
    let mut index = 0;
    while let Some(arg) = args.get(index) {
        let Some(token) = arg.strip_prefix("--") else {
            // 最初の位置トークンだけが動詞である。2 つ目以降は upstream でも読まれない。
            if parsed.sub.is_none() {
                parsed.sub = Some(arg.clone());
            }
            index += 1;
            continue;
        };
        let (key, value) = match token.split_once('=') {
            Some((key, value)) => {
                index += 1;
                (key, value.to_string())
            }
            None => match args.get(index + 1) {
                Some(next) if !next.starts_with("--") => {
                    index += 2;
                    (token, next.clone())
                }
                _ => {
                    index += 1;
                    (token, "true".to_string())
                }
            },
        };
        if key == "json" {
            parsed.json = Some(value);
        }
    }
    parsed
}

#[cfg(test)]
mod tests {
    use super::*;

    fn argv(args: &[&str]) -> Vec<String> {
        args.iter().map(|arg| (*arg).to_string()).collect()
    }

    #[test]
    fn the_first_positional_token_is_the_verb() {
        let parsed = parse_intent(&argv(&["list", "--json"]));
        assert_eq!(parsed.sub(), Some("list"));
        assert!(parsed.is_json());
    }

    /// 動詞が無い起動は、動詞を持たない要求として運ぶ（判断は消費側）。
    #[test]
    fn a_launch_without_a_verb_carries_none() {
        let parsed = parse_intent(&argv(&[]));
        assert_eq!(parsed.sub(), None);
        assert!(!parsed.is_json());
    }

    /// `--json` の次に位置トークンが続くと、upstream はそれを値として食う。
    #[test]
    fn a_flag_swallows_the_following_token_exactly_as_upstream_does() {
        let parsed = parse_intent(&argv(&["--json", "list"]));
        assert_eq!(parsed.sub(), None);
        assert!(!parsed.is_json());
    }

    #[test]
    fn an_explicit_value_is_read_from_the_equals_form() {
        assert!(parse_intent(&argv(&["list", "--json=true"])).is_json());
        assert!(!parse_intent(&argv(&["list", "--json=false"])).is_json());
    }
}
