//! `WriteTarget` — ハーネスが実行しようとしている書込みの宛先 1 件。
use super::WriteTargetError;
/// 書込み系ツールが名指した宛先パス (POSIX 区切りへ正規化済み)。
///
/// 区切りの正規化を**構築時に済ませる**のは、照合のたびに呼出側が `\` を畳み直すと
/// 畳み忘れた経路だけ保護が外れるからである (upstream は照合の直前に
/// `file.replace(/\\/g, "/")` を置く)。相対・絶対は入力のまま保つ — 宣言成果物の照合は
/// 接尾辞一致であり、根の解決を必要としない。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WriteTarget(String);
impl WriteTarget {
    /// 生の綴りを検査して宛先にする (**この型の唯一の構築経路**)。
    ///
    /// # Errors
    /// 空、または空白だけの綴りの場合。
    pub fn parse(raw: &str) -> Result<WriteTarget, WriteTargetError> {
        if raw.trim().is_empty() {
            return Err(WriteTargetError::Empty);
        }
        Ok(WriteTarget(raw.replace('\\', "/")))
    }
    /// 監査項目・文言へ渡す正準綴り。
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
    /// この宛先が与えられた接尾辞で終わるか。
    #[must_use]
    pub fn ends_with(&self, suffix: &str) -> bool {
        self.0.ends_with(suffix)
    }
    /// 接尾辞を取り除いた先頭側 (末尾一致しないときは `None`)。
    #[must_use]
    pub fn without_suffix(&self, suffix: &str) -> Option<&str> {
        self.0.strip_suffix(suffix)
    }
}
#[cfg(test)]
mod tests {
    use super::{WriteTarget, WriteTargetError};
    #[test]
    fn a_windows_path_is_folded_to_posix_separators_at_construction() {
        let target = WriteTarget::parse(r"C:\record\construction\code-generation\plan.md").unwrap();
        assert_eq!(
            target.as_str(),
            "C:/record/construction/code-generation/plan.md"
        );
        assert!(target.ends_with("/code-generation/plan.md"));
    }
    #[test]
    fn an_empty_or_blank_spelling_names_no_target() {
        assert_eq!(WriteTarget::parse(""), Err(WriteTargetError::Empty));
        assert_eq!(WriteTarget::parse("   "), Err(WriteTargetError::Empty));
    }
    #[test]
    fn the_head_before_a_matching_suffix_is_returned_and_a_miss_is_none() {
        let target = WriteTarget::parse("/r/construction/u1/code-generation/plan.md").unwrap();
        assert_eq!(
            target.without_suffix("/code-generation/plan.md"),
            Some("/r/construction/u1")
        );
        assert_eq!(target.without_suffix("/other/plan.md"), None);
    }
}
