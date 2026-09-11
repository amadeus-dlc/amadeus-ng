//! 登録簿にある依頼の記録先を読むポート。
use super::{IntentRecordView, ReadModelReadError};
/// IDに対応する投影行だけを読み込む。
pub trait IntentRecordDao {
    /// 依頼IDに対応する記録先。
    /// # Errors
    /// 読取I/O、形式不正、ID重複。
    fn find(&self, intent_id: &str) -> Result<Option<IntentRecordView>, ReadModelReadError>;
}
