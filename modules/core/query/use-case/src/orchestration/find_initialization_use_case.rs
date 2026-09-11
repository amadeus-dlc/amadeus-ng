//! 保存済み開始結果を取得する。
use super::{InitializationDao, InitializationView, ReadModelReadError};
/// 依頼IDに対応する開始結果を読む。
#[derive(Debug)]
pub struct FindInitializationUseCase<D: InitializationDao> {
    dao: D,
}
impl<D: InitializationDao> FindInitializationUseCase<D> {
    /// 読取りポートを注入する。
    #[must_use]
    pub const fn new(dao: D) -> Self {
        Self { dao }
    }
    /// 保存済みViewを返す。
    /// # Errors
    /// リードモデルを取得できない場合。
    pub fn execute(
        &self,
        intent_id: &str,
    ) -> Result<Option<InitializationView>, ReadModelReadError> {
        self.dao.find(intent_id)
    }
}
