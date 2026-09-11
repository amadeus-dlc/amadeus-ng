//! `aidlc-learnings` のフラグ（§13 の学びの儀式）。

/// `surface` / `persist` が受けるフラグ一式。
///
/// 本家 `aidlc-learnings.ts:1075-1085` の `parseFlags` は `--<name> <value>` だけを拾い、
/// 値を伴わない末尾のフラグは落とす。ここも同じ拾い方をする。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LearningsArgs {
    slug: Option<String>,
    selections_json: Option<String>,
}

impl LearningsArgs {
    /// 検査済みの値を束ねる完全コンストラクタ。
    #[must_use]
    pub const fn new(slug: Option<String>, selections_json: Option<String>) -> LearningsArgs {
        LearningsArgs {
            slug,
            selections_json,
        }
    }

    /// `--slug <stage-slug>`。
    #[must_use]
    pub fn slug(&self) -> Option<&str> {
        self.slug.as_deref()
    }

    /// `--selections-json <path>`。
    #[must_use]
    pub fn selections_json(&self) -> Option<&str> {
        self.selections_json.as_deref()
    }
}

/// フラグ列を畳む。
#[must_use]
pub fn parse_learnings(args: &[String]) -> LearningsArgs {
    let mut flags = LearningsArgs::default();
    let mut index = 0;
    while let Some(name) = args.get(index) {
        let Some(value) = args.get(index + 1) else {
            break;
        };
        match name.as_str() {
            "--slug" => {
                flags = LearningsArgs::new(Some(value.clone()), flags.selections_json.clone());
                index += 1;
            }
            "--selections-json" => {
                flags = LearningsArgs::new(flags.slug.clone(), Some(value.clone()));
                index += 1;
            }
            _ => {}
        }
        index += 1;
    }
    flags
}

#[cfg(test)]
mod tests {
    use super::{LearningsArgs, parse_learnings};

    fn argv(args: &[&str]) -> Vec<String> {
        args.iter().map(|arg| (*arg).to_string()).collect()
    }

    #[test]
    fn the_two_flags_are_folded_in_any_order() {
        assert_eq!(
            parse_learnings(&argv(&[
                "--selections-json",
                "sel.json",
                "--slug",
                "requirements-analysis"
            ])),
            LearningsArgs::new(
                Some("requirements-analysis".to_string()),
                Some("sel.json".to_string())
            )
        );
    }

    #[test]
    fn a_flag_without_a_value_is_dropped() {
        assert_eq!(
            parse_learnings(&argv(&["--slug"])),
            LearningsArgs::default()
        );
        assert_eq!(
            parse_learnings(&argv(&["--slug", "x", "--selections-json"])),
            LearningsArgs::new(Some("x".to_string()), None)
        );
    }

    #[test]
    fn unknown_flags_are_ignored() {
        assert_eq!(
            parse_learnings(&argv(&["--project-dir", "/tmp", "--slug", "x"])),
            LearningsArgs::new(Some("x".to_string()), None)
        );
    }
}
