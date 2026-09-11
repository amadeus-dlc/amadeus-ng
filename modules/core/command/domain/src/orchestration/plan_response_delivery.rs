//! 観測した応答を実行側へ配送する際の集約の判断。
/// 判断は共有承認と実行の保存済み事実から行い、UseCaseはI/Oを配線する。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanResponseDelivery {
    /// 対応するHUMAN_TURNの事実を実行側へ保存する。
    RecordRequired,
    /// 同じ操作の観測は実行側へ保存済み。
    Recorded,
}
