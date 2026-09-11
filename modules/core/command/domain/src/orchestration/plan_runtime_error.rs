//! 共有承認の操作を確定できない理由。
/// 業務上の拒否と、再開時に照合する操作状態を区別する。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlanRuntimeError {
    /// 対応する計画回答が監査待ちではない。
    NoPendingAnswer,
    /// 受領公開中にソースが変わった。
    ReceiptSourceChanged,
    /// 先行する受領の監査配送を先に回復する必要がある。
    PendingAnswer,
    /// 計画回答の元の実行と一致しない。
    AnswerTargetMismatch,
    /// 元の実行へ計画回答の監査が未保存。
    AnswerNotRecorded,
    /// 再構成する状態の不変条件が成立しない。
    InvalidState,
    /// 開始許可公開後の照合が未完了。
    PendingGeneration,
    /// 指定した開始操作は照合待ちではない。
    NoPendingGeneration,
    /// 初回作成として扱えない既存の承認操作がある。
    AlreadyInUse,
    /// このセッションには保護された提示がない。
    NoPendingChallenge,
    /// 先行する応答の記録を先に回復する必要がある。
    PendingResponse,
    /// 対応する応答の準備がない。
    NoPreparedResponse,
    /// 応答の観測先が一致しない。
    ResponseTargetMismatch,
    /// 実行側への観測記録がまだ保存されていない。
    ResponseNotRecorded,
    /// 元の発行とは異なるspaceまたは実行を渡された。
    InvalidationTargetMismatch,
    /// 先行する指示発行の結果を先に回復する必要がある。
    PendingInvalidation,
    /// 同じ識別子の操作は既に確定している。
    OperationAlreadyApplied,
    /// 対応する準備が存在しない。
    UnknownInvalidation,
}
impl std::fmt::Display for PlanRuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::PendingGeneration => "pending generation publication must be recovered before approval operations",
            Self::NoPendingGeneration => "no pending generation publication",
            Self::ReceiptSourceChanged => "Plan Approval source changed during receipt certification; present the current plan again",
            Self::PendingAnswer => {
                "pending Plan Approval answer must be recovered before approval operations"
            }
            Self::NoPendingAnswer => "no pending Plan Approval answer",
            Self::AnswerTargetMismatch => {
                "Plan Approval answer target does not match its source execution"
            }
            Self::AnswerNotRecorded => {
                "Plan Approval answer has not been recorded by its source execution"
            }
            Self::InvalidationTargetMismatch => {
                "Plan Approval invalidation target does not match the original publication"
            }
            Self::NoPendingChallenge => "no pending Plan Approval challenge for this session",
            Self::NoPreparedResponse => "no prepared Plan Approval response",
            Self::ResponseTargetMismatch => {
                "Plan Approval response target does not match its observation"
            }
            Self::ResponseNotRecorded => {
                "Plan Approval response has not been recorded by its source execution"
            }
            Self::PendingResponse => {
                "pending human response must be recovered before Plan Approval"
            }
            Self::AlreadyInUse => "shared approval runtime already contains workflow operations",
            Self::InvalidState => "invalid Plan Approval runtime state",
            Self::PendingInvalidation => {
                "pending directive publication must be recovered before Plan Approval"
            }
            Self::OperationAlreadyApplied => "Plan Approval operation has already been applied",
            Self::UnknownInvalidation => "Plan Approval invalidation was not prepared",
        })
    }
}
impl std::error::Error for PlanRuntimeError {}
