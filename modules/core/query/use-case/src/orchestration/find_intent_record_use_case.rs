//! 依頼IDに対応する登録済みの記録先を返す。
use super::{IntentRecordDao, IntentRecordView, ReadModelReadError};
/// 登録情報を解釈し直さず、DAOの行をそのまま返す。
#[derive(Debug)]
pub struct FindIntentRecordUseCase<D> {
    dao: D,
}
impl<D: IntentRecordDao> FindIntentRecordUseCase<D> {
    /// 登録簿の読取ポートを注入する。
    #[must_use]
    pub const fn new(dao: D) -> Self {
        Self { dao }
    }
    /// 依頼IDで記録先を返す。
    /// # Errors
    /// 登録簿を読み込めない場合。
    pub fn execute(&self, intent_id: &str) -> Result<Option<IntentRecordView>, ReadModelReadError> {
        self.dao.find(intent_id)
    }
}
