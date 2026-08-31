//! 記録ディレクトリ名の組み立て — `<YYMMDD>-<label>-<id8>`。
//!
//! upstream の birth は「uuid を鋳造 → dirName を解決 → mkdir」の順で進み、名前は
//! 日付・人間が読めるラベル・識別子の先頭 8 桁からできている（`aidlc-lib.ts`
//! `recordDirMatches` が `<slug>-<id8>` の対応を検査している）。
//!
//! ラベルは**コンダクタ（LLM）が付ける** — `next` の誕生 print が
//! `--label "<2-3 word kebab essence>"` を名指しするのはそのためで、エンジンは要約できない。
//! ラベルが無い呼出でも名前は要るので、自由記述を切り詰めて代用する（upstream の
//! 「A bare run without --label still births a sane name by truncating --arguments」）。

use core_command_domain::orchestration::IntentId;
use core_command_domain::workspace::{IntentDirName, IntentDirNameError};

/// 名前に載せる識別子の桁数（`recordDirMatches` の `id8`）。
const ID_SUFFIX_LEN: usize = 8;

/// ラベル部分の最大文字数（全体 64 字上限のうち、日付 7 字と id8 9 字を除いた余裕から）。
const MAX_LABEL_LEN: usize = 40;

/// ラベルが無いときの既定（upstream の `DEFAULT_SCOPE` 相当の位置づけ — 名前は必ず要る）。
const FALLBACK_LABEL: &str = "work";

/// `<YYMMDD>-<label>-<id8>` を組む。
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
    id: &IntentId,
) -> Result<IntentDirName, IntentDirNameError> {
    let source = label
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .or_else(|| description.map(str::trim).filter(|value| !value.is_empty()));
    let slug = source.map_or_else(
        || FALLBACK_LABEL.to_string(),
        |value| kebab(value, MAX_LABEL_LEN),
    );
    let slug = if slug.is_empty() {
        FALLBACK_LABEL.to_string()
    } else {
        slug
    };
    IntentDirName::parse(&format!("{yymmdd}-{slug}-{}", id_suffix(id)))
}

/// 識別子の先頭 8 桁（`-` を除いた 16 進）。
fn id_suffix(id: &IntentId) -> String {
    id.as_str()
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .take(ID_SUFFIX_LEN)
        .collect()
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
    out.trim_matches('-').to_string()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::*;

    fn id() -> IntentId {
        IntentId::parse("01a02785-1bd8-76eb-aeea-5aa303ebd5b6").expect("UUIDv7")
    }

    #[test]
    fn the_label_becomes_the_readable_middle_segment() {
        let name = compose("260831", Some("fix crash"), None, &id()).expect("文法内");
        assert_eq!(name.as_str(), "260831-fix-crash-01a02785");
    }

    /// ラベルが無ければ自由記述を切り詰めて代用する（upstream の bare-run の振る舞い）。
    #[test]
    fn without_a_label_the_description_is_truncated_into_one() {
        let name = compose("260831", None, Some("Fix the duplicate todos"), &id()).expect("文法内");
        assert_eq!(name.as_str(), "260831-fix-the-duplicate-todos-01a02785");
    }

    /// どちらも無ければ既定のラベルで名前を作る（名前は必ず要る）。
    #[test]
    fn without_either_a_fallback_label_is_used() {
        let name = compose("260831", None, None, &id()).expect("文法内");
        assert_eq!(name.as_str(), "260831-work-01a02785");
    }

    /// 記号や大文字は kebab へ畳む（`IntentDirName` は正規化しないので呼出側の責務）。
    #[test]
    fn punctuation_and_case_are_folded_into_kebab() {
        let name = compose("260831", Some("  Fix:: THE  Crash!! "), None, &id()).expect("文法内");
        assert_eq!(name.as_str(), "260831-fix-the-crash-01a02785");
    }

    /// 空白だけのラベルは「無い」と同じに扱い、自由記述へ落ちる。
    #[test]
    fn a_blank_label_falls_through_to_the_description() {
        let name = compose("260831", Some("   "), Some("auth service"), &id()).expect("文法内");
        assert_eq!(name.as_str(), "260831-auth-service-01a02785");
    }

    /// 記号しか無いラベルも既定へ落ちる（空の区間を作って文法違反にしない）。
    #[test]
    fn a_label_with_no_alphanumerics_falls_back() {
        let name = compose("260831", Some("!!! ???"), None, &id()).expect("文法内");
        assert_eq!(name.as_str(), "260831-work-01a02785");
    }

    /// 長い記述は切り詰められ、全体が 64 字上限に収まる。
    #[test]
    fn a_long_description_is_truncated_within_the_name_limit() {
        let long = "a".repeat(200);
        let name = compose("260831", Some(&long), None, &id()).expect("文法内");
        let composed = name.as_str();
        assert!(composed.chars().count() <= 64, "{composed}");
        assert!(name.as_str().ends_with("-01a02785"));
    }

    /// 名前の末尾は識別子の先頭 8 桁である（`recordDirMatches` の対応）。
    #[test]
    fn the_name_ends_with_the_first_eight_identifier_digits() {
        let name = compose("260831", Some("work"), None, &id()).expect("文法内");
        assert!(name.as_str().ends_with("-01a02785"));
    }
}
