//! Claude委譲完了の型不正。
#[derive(Debug, Clone, PartialEq, Eq)]
/// 文字列操作を適用できない入力。
pub enum SubagentEnvelopeError {
    /// last_assistant_messageが文字列ではない。
    MessageType,
}
impl std::fmt::Display for SubagentEnvelopeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("last_assistant_message must be a string")
    }
}
impl std::error::Error for SubagentEnvelopeError {}
