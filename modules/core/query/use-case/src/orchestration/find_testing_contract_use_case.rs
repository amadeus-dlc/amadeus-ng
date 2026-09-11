//! 保存済みテスト契約を指定IDで読む。
use super::{ReadModelReadError, TestingContractDao, TestingContractView};
/// 規則の再解釈をせず、リードモデルを返す。
#[derive(Debug)]
pub struct FindTestingContractUseCase<D: TestingContractDao> {
    dao: D,
}
impl<D: TestingContractDao> FindTestingContractUseCase<D> {
    /// 読取りポートを注入する。
    #[must_use]
    pub const fn new(dao: D) -> Self {
        Self { dao }
    }
    /// 依頼に対応する契約を取得する。
    /// # Errors
    /// DAOからの読取りに失敗した場合。
    pub fn execute(
        &self,
        intent_id: &str,
    ) -> Result<Option<TestingContractView>, ReadModelReadError> {
        self.dao.find(intent_id)
    }
}
