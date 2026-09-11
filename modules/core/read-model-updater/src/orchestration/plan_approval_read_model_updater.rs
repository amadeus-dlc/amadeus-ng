//! ワークスペース全体の承認状態を管理する集約の投影取得ループ。
use super::{JournalReadError, PlanApprovalJournalReader};
use crate::read_tables::PlanApprovalTables;
/// 集約の履歴を投影し、Queryが読む行を更新する。成功戻り値はunitのみ。
#[derive(Debug)]
pub struct PlanApprovalReadModelUpdater<R> {
    reader: R,
}
impl<R: PlanApprovalJournalReader> PlanApprovalReadModelUpdater<R> {
    /// RMU自身が所有するジャーナルポートを注入する。
    #[must_use]
    pub const fn new(reader: R) -> Self {
        Self { reader }
    }
    /// 保存済みの事実へ投影を追いつかせる。
    /// # Errors
    /// 履歴の読取・復号・投影の失敗。
    pub fn catch_up(&mut self) -> Result<(), JournalReadError> {
        let entries = self.reader.all_events()?;
        let tables = PlanApprovalTables::project(&entries)?;
        self.reader.replace(&tables)
    }
}
