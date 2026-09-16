//! 記録ディレクトリ名から表示用の短い名前を取る (upstream `displaySlugFromDirName`)。
//!
//! 登録簿に行が無い記録をどう名乗らせるかは、依頼一覧と端末の状態行の**両方**が同じ規則で
//! 決める。upstream 自身は `aidlc-lib.ts` の 1 実装を状態行のローカル写しが複製しているが、
//! こちらは 1 箇所に置いて両方の消費側がこれを呼ぶ — 片方だけ直すと同じ記録が 2 つの名前で
//! 現れる。

/// 記録ディレクトリ名から表示用の短い名前を取る。
///
/// 先頭が 6 桁の日付ならその後ろを名前とし、そうでなければ末尾の小文字 16 進の曖昧性除去子を
/// 落とす。どちらにも当てはまらなければディレクトリ名そのままである。
#[must_use]
pub fn display_slug_from_dir_name(directory: &str) -> String {
    if let Some((stamp, rest)) = directory.split_once('-')
        && stamp.len() == 6
        && stamp.bytes().all(|byte| byte.is_ascii_digit())
    {
        return rest.to_string();
    }
    match directory.rsplit_once('-') {
        Some((head, suffix))
            if !suffix.is_empty()
                && suffix
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase()) =>
        {
            head.to_string()
        }
        _ => directory.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::display_slug_from_dir_name;

    /// 先頭の 6 桁日付を落とす形が最優先である。
    #[test]
    fn a_dated_record_name_drops_its_stamp() {
        assert_eq!(display_slug_from_dir_name("260907-selfhost"), "selfhost");
        assert_eq!(
            display_slug_from_dir_name("260907-selfhost-a1b2c3d4"),
            "selfhost-a1b2c3d4"
        );
    }

    /// 日付でなければ、末尾の小文字 16 進の曖昧性除去子だけを落とす。
    #[test]
    fn an_id_suffixed_record_name_drops_its_disambiguator() {
        assert_eq!(display_slug_from_dir_name("selfhost-a1b2c3d4"), "selfhost");
        // 大文字を含む接尾辞は 16 進の曖昧性除去子ではない。
        assert_eq!(
            display_slug_from_dir_name("selfhost-A1B2C3D4"),
            "selfhost-A1B2C3D4"
        );
        // 区切りはあるが接尾辞が空なら落とさない。
        assert_eq!(display_slug_from_dir_name("selfhost-"), "selfhost-");
    }

    /// どちらの形でもない名前はそのまま名乗る。
    #[test]
    fn a_plain_record_name_is_kept() {
        assert_eq!(display_slug_from_dir_name("selfhost"), "selfhost");
        assert_eq!(display_slug_from_dir_name(""), "");
    }
}
