//! state writer 4 種 — 純粋な string→string (upstream `aidlc-lib.ts:6487-6620`, 03 §5.3)。
//! `with_field_if_present` は不在で無言 no-op、`with_field` は不在で Err (「無言 no-op は検出不能な
//! ドリフト」)、`with_field_or_insert` は指定 `## Heading` 末尾に bullet を追記、`without_field`
//! は bullet 行ごと削除 (不在は no-op)。フィールド行文法は `- **<Field>**:[ \t]*(.*)`。
//!
//! `FieldNotFound` は独立ファイルに 1 型 1 ファイルで置く (`one-public-type`)。子モジュール
//! `state_writers::field_not_found`（`state_writers/field_not_found.rs`）として所有し、
//! ここから `pub use` で再輸出する — 兄弟ファイルを跨いだ利便再エクスポートではなく、通常の
//! ファサード（`mod.rs` と同型の所有連鎖）である (`coding-rules/module-visibility.md`)。
//!
//! # 節末尾への挿入はドメインの `append_under_heading` に寄せる（b49）
//!
//! `with_field_or_insert` の挿入経路は upstream の `setOrInsertField` がそのまま
//! `appendUnderHeading` を呼ぶ形である（`aidlc-lib.ts:6612`）。b49 でその機構を
//! ドメインの `workspace::append_under_heading` に置いたので、ここは**同じ関数へ委譲**する
//! （同じ意味論の実装を 2 つ並立させない — `coding-rules/no-backward-compatibility.md`）。
//! 拒否の型も同じ [`HeadingNotFound`] である。
//!
//! **これは挙動の是正でもある**: 旧実装は挿入位置を節末尾の**空行より前**へ巻き戻していたが、
//! upstream は次の `## ` 見出しの**直前**（空行の後ろ）へ入れる — ゴールデン
//! `cli/park/park/state.diff` の実バイトがその形である。
//!
//! [`HeadingNotFound`]: core_command_domain::workspace::HeadingNotFound

use core_command_domain::workspace::{HeadingNotFound, append_under_heading};

use super::wording as msg;

mod field_not_found;

pub use field_not_found::FieldNotFound;

/// フィールド読取 (`getField`) — 最初に一致した行の値を trim して返す。
#[must_use]
pub fn find_field(content: &str, field: &str) -> Option<String> {
    let prefix = format!("- **{field}**:");
    content.lines().find_map(|l| {
        l.strip_prefix(&prefix)
            .map(|v| v.trim_matches([' ', '\t']).to_string())
    })
}

/// 存在すれば置換、不在なら**無言 no-op** (display 専用フィールド向け)。
#[must_use]
pub fn with_field_if_present(content: &str, field: &str, value: &str) -> String {
    replace_field(content, field, value).unwrap_or_else(|| content.to_string())
}

/// 状態機械遷移用 — 不在フィールドは Err (検出不能ドリフトの拒否)。
/// # Errors
///
/// 指定フィールド行が存在しなければ `FieldNotFound` (upstream 逐語の拒否文言つき)。
pub fn with_field(content: &str, field: &str, value: &str) -> Result<String, FieldNotFound> {
    replace_field(content, field, value)
        .ok_or_else(|| FieldNotFound::new(msg::field_not_found_message(field)))
}

/// 存在すれば置換、不在なら指定 `## Heading` セクションの末尾に bullet を追記
/// (フィールド不在はエラーではなく挿入 — upstream `setOrInsertField` `:6599-6613`)。
///
/// 挿入位置は次の `## ` 見出しの**直前**である（機構は
/// [`append_under_heading`] が持つ — b49 でドメインへ寄せた）。
///
/// # Errors
///
/// 指定 `## Heading` が存在しなければ `HeadingNotFound` (見出しの綴りを逐語で添える)。
pub fn with_field_or_insert(
    content: &str,
    heading: &str,
    field: &str,
    value: &str,
) -> Result<String, HeadingNotFound> {
    if let Some(replaced) = replace_field(content, field, value) {
        return Ok(replaced);
    }
    append_under_heading(
        content,
        &format!("## {heading}"),
        &format!("- **{field}**: {value}\n"),
    )
}

/// bullet 行ごと削除 (不在は no-op)。
#[must_use]
pub fn without_field(content: &str, field: &str) -> String {
    let prefix = format!("- **{field}**:");
    let out: Vec<String> = content
        .lines()
        .filter(|l| !l.starts_with(&prefix))
        .map(|s| s.to_string())
        .collect();
    rejoin(&out, content)
}

fn replace_field(content: &str, field: &str, value: &str) -> Option<String> {
    let prefix = format!("- **{field}**:");
    let mut found = false;
    let out: Vec<String> = content
        .lines()
        .map(|l| {
            if !found && l.starts_with(&prefix) {
                found = true;
                format!("- **{field}**: {value}")
            } else {
                l.to_string()
            }
        })
        .collect();
    found.then(|| rejoin(&out, content))
}

fn rejoin(lines: &[String], original: &str) -> String {
    let mut s = lines.join("\n");
    if original.ends_with('\n') {
        s.push('\n');
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\
## Project Information
- **Project**: demo
- **Scope**: feature

## Current Status
- **Status**: Running
";

    #[test]
    fn find_field_returns_the_trimmed_value() {
        assert_eq!(find_field(SAMPLE, "Scope").as_deref(), Some("feature"));
        assert_eq!(find_field(SAMPLE, "Missing"), None);
    }

    #[test]
    fn with_field_if_present_silently_no_ops_on_missing_fields() {
        let out = with_field_if_present(SAMPLE, "Missing", "x");
        assert_eq!(out, SAMPLE);
    }

    #[test]
    fn with_field_refuses_missing_fields_with_the_verbatim_message() {
        let err = with_field(SAMPLE, "Construction Autonomy Mode", "gated").unwrap_err();
        assert_eq!(
            err.message(),
            "Field not found in state file: \"Construction Autonomy Mode\". Cannot update — refusing to silently no-op."
        );
        let ok = with_field(SAMPLE, "Status", "Completed").unwrap();
        assert!(ok.contains("- **Status**: Completed"));
    }

    /// 挿入は**次の `## ` 見出しの直前**に入る（節末尾の空行の後ろ）。upstream の
    /// `setOrInsertField` → `appendUnderHeading` と同じ位置であり、ゴールデン
    /// `cli/park/park/state.diff` の実バイトもこの形である（b49 で是正）。
    #[test]
    fn with_field_or_insert_appends_immediately_before_the_next_heading() {
        let out = with_field_or_insert(
            SAMPLE,
            "Project Information",
            "Parked",
            "2026-08-22T00:00:00Z",
        )
        .unwrap();
        let expected = "\
## Project Information
- **Project**: demo
- **Scope**: feature

- **Parked**: 2026-08-22T00:00:00Z
## Current Status
- **Status**: Running
";
        assert_eq!(out, expected);
        // 不在見出しは、その見出しの綴りを逐語で添えて拒否する（`## ` を含む完全形）。
        let err = with_field_or_insert(SAMPLE, "No Such Heading", "F", "v").unwrap_err();
        assert_eq!(err.as_str(), "## No Such Heading");
        assert_eq!(err, HeadingNotFound::new("## No Such Heading"));
    }

    #[test]
    fn without_field_deletes_the_bullet_line_and_no_ops_when_missing() {
        let out = without_field(SAMPLE, "Scope");
        assert!(!out.contains("Scope"));
        assert_eq!(without_field(SAMPLE, "Missing"), SAMPLE);
    }
}
