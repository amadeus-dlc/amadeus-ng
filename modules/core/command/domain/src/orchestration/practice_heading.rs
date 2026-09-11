//! `PracticeHeading` — 学びが落ちるメモリ層の見出し。

/// メモリ層の `## ` 見出し（正規化済み）。
///
/// 見出しの選定 (fit による routing) は orchestrator の知識であり、この型は**綴りを正す**
/// だけである。空白だけ・空の指定は既定の `## Corrections` へ倒す。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PracticeHeading {
    heading: String,
}

/// 見出しが指定されなかったときの既定（org / team / project すべてが持つ節）。
const DEFAULT_PRACTICE_HEADING: &str = "## Corrections";
/// H2 の接頭。
const H2: &str = "## ";

impl PracticeHeading {
    // 正規化済みの綴りは、この構築口で全状態を初期化する。
    const fn of_heading(heading: String) -> Self {
        Self { heading }
    }

    /// 既定の見出し。
    #[must_use]
    pub fn corrections() -> PracticeHeading {
        PracticeHeading::of_heading(DEFAULT_PRACTICE_HEADING.to_string())
    }

    /// orchestrator が選んだ見出しを正規化する（**この型の唯一の構築経路**）。
    ///
    /// 素の `Corrections` も完全形の `## Corrections` も同じ見出しへ落ちる。空・空白だけの
    /// 指定は既定へ倒す（拒否しない — 見出しの綴りで儀式を止めない）。
    #[must_use]
    pub fn from_routed(routed: &str) -> PracticeHeading {
        let trimmed = routed.trim();
        if trimmed.is_empty() {
            return PracticeHeading::corrections();
        }
        if trimmed.starts_with(H2) {
            return PracticeHeading::of_heading(trimmed.to_string());
        }
        PracticeHeading::of_heading(format!(
            "{H2}{}",
            trimmed.trim_start_matches('#').trim_start()
        ))
    }

    /// `## ` を含む完全形の綴り。
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.heading
    }
}

#[cfg(test)]
mod tests {
    use super::PracticeHeading;

    #[test]
    fn a_bare_name_and_the_full_form_resolve_to_the_same_heading() {
        assert_eq!(
            PracticeHeading::from_routed("Corrections").as_str(),
            "## Corrections"
        );
        assert_eq!(
            PracticeHeading::from_routed("## Corrections").as_str(),
            "## Corrections"
        );
        assert_eq!(
            PracticeHeading::from_routed("  Testing Posture  ").as_str(),
            "## Testing Posture"
        );
        assert_eq!(
            PracticeHeading::from_routed("# Forbidden").as_str(),
            "## Forbidden"
        );
    }

    #[test]
    fn an_empty_pick_falls_back_to_the_default() {
        assert_eq!(PracticeHeading::from_routed("").as_str(), "## Corrections");
        assert_eq!(
            PracticeHeading::from_routed("   ").as_str(),
            "## Corrections"
        );
        assert_eq!(PracticeHeading::corrections().as_str(), "## Corrections");
    }
}
