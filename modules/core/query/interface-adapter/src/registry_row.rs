//! intent 登録簿 (`intents.json`) の行を記録ディレクトリへ突き合わせる規則
//! (upstream `recordDirMatches`)。
//!
//! この規則を使う消費側は 2 つある — 依頼一覧の `dirName` / `active` 判定
//! ([`super::IntentListingDaoImpl`]) と、repo 引当 ([`super::IntentReposDaoImpl`]) である。
//! 規則は 1 箇所に置いて両方がこれを呼ぶ ([`super::display_slug_from_dir_name`] と同じ形) —
//! 片方だけ直すと、同じ記録に対して一覧と repo 引当が別の行を選ぶ。

/// 登録簿の行が、この記録ディレクトリのものか (upstream `recordDirMatches`)。
///
/// 記録された `dirName` が在ればそれを逐語で突き合わせる。無い行 (spike 以前の行や手書きの
/// フィクスチャ) だけが `<slug>-<id8>` の形へ後退する — slug の接頭辞と、uuid の末尾 hex に
/// 一致する接尾辞である。
pub(crate) fn record_dir_matches(entry: &serde_json::Value, record_dir_name: &str) -> bool {
    if let Some(dir_name) = entry.get("dirName").and_then(serde_json::Value::as_str) {
        return dir_name == record_dir_name;
    }
    let (Some(slug), Some(uuid)) = (
        entry.get("slug").and_then(serde_json::Value::as_str),
        entry.get("uuid").and_then(serde_json::Value::as_str),
    ) else {
        return false;
    };
    let Some(suffix) = record_dir_name.strip_prefix(&format!("{slug}-")) else {
        return false;
    };
    if suffix.is_empty()
        || !suffix
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return false;
    }
    let hex: String = uuid.chars().filter(|c| *c != '-').collect();
    hex.len() >= suffix.len() && hex.ends_with(suffix)
}

#[cfg(test)]
mod tests {
    use super::record_dir_matches;

    fn row(json: &str) -> serde_json::Value {
        serde_json::from_str(json).expect("行")
    }

    /// `dirName` を持つ行は逐語でだけ一致する — 後退一致へ落ちない。
    #[test]
    fn a_row_that_records_its_directory_name_matches_it_verbatim_and_nothing_else() {
        let entry = row(
            r#"{"dirName":"260915-auth-a1b2c3d4","slug":"auth","uuid":"0190aaaa-bbbb-7ccc-9ddd-a1b2c3d4"}"#,
        );
        assert!(record_dir_matches(&entry, "260915-auth-a1b2c3d4"));
        // slug と uuid が後退一致する綴りでも、`dirName` が違えば一致しない。
        assert!(!record_dir_matches(&entry, "auth-a1b2c3d4"));
        assert!(!record_dir_matches(&entry, "260915-auth-a1b2c3d5"));
    }

    /// `dirName` の無い行だけが `<slug>-<id8>` の形へ後退する。
    #[test]
    fn a_row_without_a_directory_name_falls_back_to_the_slug_and_identifier_shape() {
        let entry = row(r#"{"slug":"auth","uuid":"0190aaaa-bbbb-7ccc-9ddd-a1b2c3d4"}"#);
        assert!(record_dir_matches(&entry, "auth-a1b2c3d4"));
        // 大文字の接尾辞は 16 進の曖昧性除去子ではない。
        assert!(!record_dir_matches(&entry, "auth-A1B2C3D4"));
        // 接尾辞が空、uuid の hex に無い、hex より長い綴りは一致しない。
        assert!(!record_dir_matches(&entry, "auth-"));
        assert!(!record_dir_matches(&entry, "auth-ffffffff"));
        assert!(!record_dir_matches(
            &entry,
            &format!("auth-{}", "0".repeat(64))
        ));
        // slug の接頭辞そのものが違えば一致しない。
        assert!(!record_dir_matches(&entry, "billing-a1b2c3d4"));
    }

    /// slug も uuid も無い行は、どの記録にも一致しない。
    #[test]
    fn a_row_without_a_slug_or_a_uuid_matches_nothing() {
        assert!(!record_dir_matches(
            &row(r#"{"slug":"auth"}"#),
            "auth-a1b2c3d4"
        ));
        assert!(!record_dir_matches(
            &row(r#"{"uuid":"0190aaaa-bbbb-7ccc-9ddd-a1b2c3d4"}"#),
            "auth-a1b2c3d4"
        ));
        assert!(!record_dir_matches(&row("{}"), "auth-a1b2c3d4"));
    }
}
