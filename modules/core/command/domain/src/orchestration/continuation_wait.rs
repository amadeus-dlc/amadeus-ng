//! 実行が人間の応答を待っている理由。
/// Stopの差止めを適用しない、実行状態から確定する待機理由。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContinuationWait {
    /// 現ステージの承認または修正待ち。
    GateOrRevision,
    /// 現ステージで提示済みの質問が未回答。
    Decision,
    /// 選定ステージの質問文書が未回答。
    Question,
    /// 人間のprompt後にワークフロー操作を行っていない会話。
    Conversation,
    /// 状態に結び付いた共有再開promptが回答待ち。
    Resume,
}
impl ContinuationWait {
    /// 保存・投影境界へ渡す待機種別。
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::GateOrRevision => "gate-or-revision",
            Self::Decision => "decision",
            Self::Question => "question",
            Self::Conversation => "conversation",
            Self::Resume => "resume",
        }
    }
    /// 保存された待機種別を検査する。
    /// # Errors
    /// 未知の種別の場合。
    pub fn parse(value: &str) -> Result<Self, super::ContinuationError> {
        match value {
            "gate-or-revision" => Ok(Self::GateOrRevision),
            "decision" => Ok(Self::Decision),
            "question" => Ok(Self::Question),
            "conversation" => Ok(Self::Conversation),
            "resume" => Ok(Self::Resume),
            _ => Err(super::ContinuationError::InvalidHistory),
        }
    }
}
