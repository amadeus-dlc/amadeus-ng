//! 受領した計画回答の、監査配送までの状態。
/// 計画回答の配送・取り消し状態。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlanAnswerState {
    /// ソースの再確認と元の実行への監査配送が未完了。
    Pending,
    /// 監査まで確定した。
    Recorded,
    /// ソース再確認で受領を取り消した。
    Aborted(String),
}
