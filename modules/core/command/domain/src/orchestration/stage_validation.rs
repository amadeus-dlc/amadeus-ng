//! 完了時に観測した検証根拠。監査契約の値を保存し、後日のファイルから再計算しない。
/// 本家の公開語彙に従う検証根拠、または採取できなかった理由。
/// 値は境界が正準化したJSONまたは単一行の診断であり、保存媒体の形式ではない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StageValidation {
    /// グラフ契約と入力・出力の観測。
    Basis(String),
    /// 根拠を採取できなかった理由。成功した根拠として扱わない。
    Warning(String),
}
