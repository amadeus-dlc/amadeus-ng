//! 本家2.7.1の記録名ベース（YYMMDD-label）。重複の番号付けは作成時の予約が担う。

use core_command_domain::workspace::{IntentDirName, IntentDirNameError};

/// 本家slugify(label, 24)の上限。
const MAX_LABEL_LEN: usize = 24;

/// ラベルが無いときの既定（upstream の `DEFAULT_SCOPE` 相当の位置づけ — 名前は必ず要る）。
const FALLBACK_LABEL: &str = "intent";

/// `<YYMMDD>-<label>` の予約前の名前を組む。
///
/// `label` と `description` はどちらも人間の自由記述なので、**kebab へ正規化してから**
/// 使う（`IntentDirName` は正規化せず受理か拒否のみなので、整えるのは呼出側の仕事である）。
///
/// # Errors
///
/// 整えた結果が `IntentDirName` の文法に合わない場合。日付は呼出側が `YYMMDD` で渡す。
pub fn compose(
    yymmdd: &str,
    label: Option<&str>,
    description: Option<&str>,
    scope: &str,
) -> Result<IntentDirName, IntentDirNameError> {
    let source = label
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .or_else(|| description.map(str::trim).filter(|value| !value.is_empty()));
    let slug = source.map_or_else(|| scope.to_string(), |value| kebab(value, MAX_LABEL_LEN));
    let slug = if slug.is_empty() {
        FALLBACK_LABEL.to_string()
    } else {
        slug
    };
    IntentDirName::parse(&format!("{yymmdd}-{slug}"))
}

/// 自由記述を kebab へ整える — 小文字化し、`[a-z0-9]` 以外の連なりを 1 つの `-` に畳む。
fn kebab(value: &str, max: usize) -> String {
    let mut out = String::new();
    let mut pending_separator = false;
    for c in value.chars() {
        if c.is_ascii_alphanumeric() {
            if pending_separator && !out.is_empty() {
                out.push('-');
            }
            pending_separator = false;
            out.extend(c.to_lowercase());
            if out.chars().count() >= max {
                break;
            }
        } else {
            pending_separator = true;
        }
    }
    let out = out.trim_matches('-');
    if out.starts_with(|c: char| c.is_ascii_lowercase()) {
        out.to_string()
    } else {
        format!("intent-{out}").trim_end_matches('-').to_string()
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::*;

    #[test]
    fn the_label_becomes_the_readable_middle_segment() {
        let name = compose("260831", Some("fix crash"), None, "classic").expect("文法内");
        assert_eq!(name.as_str(), "260831-fix-crash");
    }

    /// ラベルが無ければ自由記述を切り詰めて代用する（upstream の bare-run の振る舞い）。
    #[test]
    fn without_a_label_the_description_is_truncated_into_one() {
        let name =
            compose("260831", None, Some("Fix the duplicate todos"), "classic").expect("文法内");
        assert_eq!(name.as_str(), "260831-fix-the-duplicate-todos");
    }

    /// どちらも無ければ既定のラベルで名前を作る（名前は必ず要る）。
    #[test]
    fn without_either_a_fallback_label_is_used() {
        let name = compose("260831", None, None, "classic").expect("文法内");
        assert_eq!(name.as_str(), "260831-classic");
    }

    /// 記号や大文字は kebab へ畳む（`IntentDirName` は正規化しないので呼出側の責務）。
    #[test]
    fn punctuation_and_case_are_folded_into_kebab() {
        let name =
            compose("260831", Some("  Fix:: THE  Crash!! "), None, "classic").expect("文法内");
        assert_eq!(name.as_str(), "260831-fix-the-crash");
    }

    /// 空白だけのラベルは「無い」と同じに扱い、自由記述へ落ちる。
    #[test]
    fn a_blank_label_falls_through_to_the_description() {
        let name = compose("260831", Some("   "), Some("auth service"), "classic").expect("文法内");
        assert_eq!(name.as_str(), "260831-auth-service");
    }

    /// 記号しか無いラベルも既定へ落ちる（空の区間を作って文法違反にしない）。
    #[test]
    fn a_label_with_no_alphanumerics_falls_back() {
        let name = compose("260831", Some("!!! ???"), None, "classic").expect("文法内");
        assert_eq!(name.as_str(), "260831-intent");
    }

    /// 長い記述は切り詰められ、全体が 64 字上限に収まる。
    #[test]
    fn a_long_description_is_truncated_within_the_name_limit() {
        let long = "a".repeat(200);
        let name = compose("260831", Some(&long), None, "classic").expect("文法内");
        let composed = name.as_str();
        assert!(composed.chars().count() <= 64, "{composed}");
        assert_eq!(name.as_str(), "260831-aaaaaaaaaaaaaaaaaaaaaaaa");
    }

    /// 本家2.7.1の初回名は日付とラベルだけでありIDは含まない。
    #[test]
    fn the_first_record_name_has_no_identifier_suffix() {
        let name = compose("260831", Some("work"), None, "classic").expect("文法内");
        assert_eq!(name.as_str(), "260831-work");
    }
}
