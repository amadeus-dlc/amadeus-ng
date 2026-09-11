//! 共有承認イベントと投影の読書きポート。
use super::{JournalReadError, PlanApprovalJournalEntry};
use crate::read_tables::PlanApprovalTables;
/// イベントを読み、投影とそのチェックポイントを同一DBへ書く。
pub trait PlanApprovalJournalReader {
    /// ワークスペース全体の承認集約の履歴。
    /// # Errors
    /// I/O・復号・通番の不正。
    fn all_events(&self) -> Result<Vec<PlanApprovalJournalEntry>, JournalReadError>;
    /// 計算済み投影とチェックポイントを一緒に確定する。
    /// # Errors
    /// I/O・古い投影・アンカー不一致。
    fn replace(&mut self, tables: &PlanApprovalTables) -> Result<(), JournalReadError>;
}
