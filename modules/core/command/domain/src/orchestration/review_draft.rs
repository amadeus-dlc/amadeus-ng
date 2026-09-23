//! レビュアーが書いた単独のレビュー（下書き）1 本。
/// 試行 ID のディレクトリに置かれた `<iteration>.review.md` の原文。
///
/// upstream 2.8.2 のレビュアーは成果物へ追記せず、依頼が返す `reviewFile` へレビューを書く
/// （`stage-protocol-reviewer.md` §12a Flow 2）。入力境界は判定の iteration に当たる下書きを
/// 試行ごとに観測して渡し、どれを判定の証拠とするかは集約（依頼の試行 ID）が決める。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewDraft {
    attempt: String,
    body: Vec<u8>,
}

impl ReviewDraft {
    /// 置かれていた試行 ID と原文を束ねる。
    #[must_use]
    pub const fn new(attempt: String, body: Vec<u8>) -> Self {
        Self { attempt, body }
    }

    /// 置かれていた試行 ID のディレクトリ名。
    #[must_use]
    pub fn attempt(&self) -> &str {
        &self.attempt
    }

    /// 原文。
    #[must_use]
    pub fn body(&self) -> &[u8] {
        &self.body
    }
}
