//! `AuditRedaction` — 監査ブロックの値に現れる project dir を `<project-dir>` へ伏せる規則。
//!
//! upstream `renderAuditBlock` (`aidlc-audit.ts:512-534`) は監査ブロックの**すべての値**へ
//! `redactProjectDirPrefix(value, projectDir)` (`aidlc-lib.ts:22167-22203`) を掛ける。
//! 置換の規則はそこに書かれたとおりである:
//!
//! - 伏せる綴りは project dir の**絶対形**と**実パス** (symlink を解いた形。解けなければ
//!   絶対形だけ) の 2 つに、それぞれの `\` ↔ `/` を入れ替えた形を足した集合
//! - 長い綴りから順に、左から右へ照合する
//! - 一致の**直後**が `/`・`\`・ECMAScript の空白・末尾のいずれでもなければ、その一致は
//!   別名の一部 (`<project>-neighbor/…`) なので置換しない。**直前**の境界は見ない
//!   (upstream も見ない)
//!
//! 本 build ではこの規則を**描画の出口** (取得ループがシャードへ追記する直前) に 1 度だけ
//! 掛ける。監査ブロックの見出し・`Timestamp`・`Event`・項目名は閉じた語彙で project dir を
//! 含まないので、ブロック全体に掛けても値ごとに掛けたのと同じ結果になる。値の中の改行は
//! `AuditFieldValue` が構築時に `\n` の 2 文字へ畳んでいるため、upstream が「値の末尾」と
//! 見る位置は本 build では `\` (畳んだ改行の先頭) か行末の LF に当たり、どちらも境界として
//! 通る。標準エラーへ出す拒否文言は upstream も生の綴りのままなので、ここでは伏せない。

use std::path::Path;

/// 置換後の綴り (upstream の逐語)。
const PLACEHOLDER: &str = "<project-dir>";

/// 監査値の project dir を伏せる規則 1 組。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditRedaction {
    /// 伏せる綴り (長い順、重複なし、空文字なし)。
    variants: Vec<String>,
}

impl AuditRedaction {
    /// 綴りの集合を固定する (**この型の唯一の構築経路**)。
    fn new(variants: Vec<String>) -> AuditRedaction {
        let mut variants: Vec<String> = variants
            .into_iter()
            .filter(|variant| !variant.is_empty())
            .collect();
        variants.sort_by_key(|variant| std::cmp::Reverse(variant.len()));
        variants.dedup();
        AuditRedaction { variants }
    }

    /// project dir から伏せる綴りの集合を組む (upstream `redactProjectDirPrefix` の
    /// `variants` と同じ 4 形まで)。実パスは読めるときだけ足す。
    #[must_use]
    pub fn for_project_dir(project_dir: &Path) -> AuditRedaction {
        let absolute = std::path::absolute(project_dir)
            .unwrap_or_else(|_| project_dir.to_path_buf())
            .to_string_lossy()
            .into_owned();
        let mut variants = vec![absolute];
        if let Ok(real) = std::fs::canonicalize(project_dir) {
            variants.push(real.to_string_lossy().into_owned());
        }
        for variant in variants.clone() {
            variants.push(variant.replace('\\', "/"));
            variants.push(variant.replace('/', "\\"));
        }
        AuditRedaction::new(variants)
    }

    /// 伏せる綴りを持たない規則 (project dir を導けない配置)。`redact` は恒等写像になる。
    #[must_use]
    pub fn none() -> AuditRedaction {
        AuditRedaction::new(Vec::new())
    }

    /// 文字列の中の project dir をすべて `<project-dir>` へ置き換える。
    #[must_use]
    pub fn redact(&self, text: &str) -> String {
        let mut result = text.to_string();
        for variant in &self.variants {
            let mut offset = 0;
            while let Some(relative) = result.get(offset..).and_then(|tail| tail.find(variant)) {
                let begin = offset + relative;
                let end = begin + variant.len();
                let next = result.get(end..).and_then(|tail| tail.chars().next());
                if next.is_some_and(|ch| ch != '/' && ch != '\\' && !is_js_whitespace(ch)) {
                    offset = end;
                } else {
                    result.replace_range(begin..end, PLACEHOLDER);
                    offset = begin + PLACEHOLDER.len();
                }
            }
        }
        result
    }
}

/// ECMAScript の `\s` (upstream の境界判定 `!/\s/.test(next)`)。
const fn is_js_whitespace(ch: char) -> bool {
    matches!(
        ch,
        '\u{9}'..='\u{d}'
            | ' '
            | '\u{a0}'
            | '\u{1680}'
            | '\u{2000}'..='\u{200a}'
            | '\u{2028}'
            | '\u{2029}'
            | '\u{202f}'
            | '\u{205f}'
            | '\u{3000}'
            | '\u{feff}'
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_redaction_uses_the_upstream_trailing_boundary() {
        let temporary = tempfile::tempdir().unwrap();
        let redaction = AuditRedaction::for_project_dir(temporary.path());
        let root = temporary.path().to_string_lossy();
        for suffix in ["", "/file", "\\file", "\targ", "\u{feff}arg"] {
            assert_eq!(
                redaction.redact(&format!("{root}{suffix}")),
                format!("<project-dir>{suffix}")
            );
        }
        for suffix in ["-neighbor/file", "'", "\"", "\u{85}arg"] {
            let input = format!("{root}{suffix}");
            assert_eq!(redaction.redact(&input), input);
        }
        assert_eq!(
            redaction.redact(&format!("prefix{root}/file {root}/other")),
            "prefix<project-dir>/file <project-dir>/other"
        );
        assert_eq!(
            redaction.redact(&format!("{}\\file", root.replace('/', "\\"))),
            "<project-dir>\\file"
        );
    }

    #[cfg(unix)]
    #[test]
    fn project_redaction_also_recognizes_the_resolved_directory() {
        let temporary = tempfile::tempdir().unwrap();
        let actual = temporary.path().join("actual");
        std::fs::create_dir(&actual).unwrap();
        let alias = temporary.path().join("alias");
        std::os::unix::fs::symlink(&actual, &alias).unwrap();
        let redaction = AuditRedaction::for_project_dir(&alias);
        let resolved = std::fs::canonicalize(&actual).unwrap();
        assert_eq!(
            redaction.redact(&format!(
                "{}/file {}/file",
                alias.display(),
                resolved.display()
            )),
            "<project-dir>/file <project-dir>/file"
        );
    }

    #[test]
    fn a_whole_audit_block_is_redacted_only_in_its_values() {
        // 見出し・Timestamp・Event・項目名は閉じた語彙なので、ブロック全体に掛けても
        // 値だけが変わる。畳んだ改行 (`\n` の 2 文字) の直前も境界として通る。
        let temporary = tempfile::tempdir().unwrap();
        let redaction = AuditRedaction::for_project_dir(temporary.path());
        let root = temporary.path().to_string_lossy();
        let block = format!(
            "\n## Error Logged\n**Timestamp**: 2026-09-10T00:00:00Z\n**Event**: ERROR_LOGGED\n\
             **Tool**: aidlc-log\n**Command**: aidlc-log decision --project-dir {root}\n\
             **Error**: {root}\\nsecond line\n\n---\n"
        );
        assert_eq!(
            redaction.redact(&block),
            "\n## Error Logged\n**Timestamp**: 2026-09-10T00:00:00Z\n**Event**: ERROR_LOGGED\n\
             **Tool**: aidlc-log\n**Command**: aidlc-log decision --project-dir <project-dir>\n\
             **Error**: <project-dir>\\nsecond line\n\n---\n"
        );
    }

    #[test]
    fn a_redaction_without_a_project_dir_leaves_the_text_alone() {
        let text = "/Users/someone/project/aidlc/spaces/default";
        assert_eq!(AuditRedaction::none().redact(text), text);
        assert!(!AuditRedaction::none().redact("").contains(PLACEHOLDER));
    }

    #[test]
    fn the_placeholder_itself_is_never_matched_again() {
        // 置換済みの綴りは伏せる集合に無いので、2 度掛けても増殖しない。
        let temporary = tempfile::tempdir().unwrap();
        let redaction = AuditRedaction::for_project_dir(temporary.path());
        let once = redaction.redact(&format!("{}/x", temporary.path().display()));
        assert_eq!(redaction.redact(&once), once);
    }
}
