//! 共有承認イベントと投影の読書きポート。
use super::{JournalReadError, PlanApprovalJournalEntry};
use crate::read_tables::PlanApprovalTables;
use core_command_domain::orchestration::PlanReceipts;

/// 共有承認の履歴が今持っている、保護された受領。
///
/// 開始可否の投影が要るのは集約そのものではなく受領だけである。受領は値なので、空間側の
/// 取得ループへは値として渡せる（集約を合成ルートへ持ち出さずに済む）。誕生の無い履歴は
/// 「受領が 1 件も無い」であって失敗ではない — まだ承認が記録されていないだけである。
///
/// # Errors
/// 履歴の読取・復号・通番の不正。
pub fn plan_approval_receipts<R: PlanApprovalJournalReader + ?Sized>(
    reader: &R,
) -> Result<PlanReceipts, JournalReadError> {
    Ok(crate::read_tables::replay_runtime(&reader.all_events()?)?
        .map_or_else(PlanReceipts::default, |runtime| runtime.receipts().clone()))
}
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
