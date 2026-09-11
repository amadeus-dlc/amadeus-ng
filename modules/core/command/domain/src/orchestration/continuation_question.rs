//! 停止判断へ渡す、作業者が書いた質問文書の観測。
/// 文書の元テキストを保持する値。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContinuationQuestion {
    text: String,
}
impl ContinuationQuestion {
    /// 読み取った原文を受ける。
    #[must_use]
    pub const fn new(text: String) -> Self {
        Self { text }
    }
    /// 本家の未回答タグ（空白またはunderscoreのみ）があるか。
    #[must_use]
    pub fn is_unanswered(&self) -> bool {
        self.text
            .split(['\n', '\r', '\u{2028}', '\u{2029}'])
            .any(|line| {
                line.rsplit_once("[Answer]:").is_some_and(|(_, value)| {
                    let value = value.trim_matches([' ', '\t']);
                    value.bytes().all(|byte| byte == b'_')
                })
            })
    }
}
