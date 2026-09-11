//! 計画回答を元の実行へ監査記録する際の判断。
/// 判断は受領状態・ソース指紋・実行の保存事実が所有する。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanAnswerDelivery {
    /// ソース再確認済み。元の実行へ監査を記録する。
    RecordRequired,
    /// 元の実行には同じ操作の監査が保存済み。
    Recorded,
    /// ソースが変わったため受領を取り消す。
    RejectCertification,
}
