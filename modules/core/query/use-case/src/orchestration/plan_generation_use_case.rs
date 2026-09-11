//! 実装開始の結果を操作IDで返すQuery。
use super::{PlanGenerationDao, PlanGenerationView, ReadModelReadError};
/// 集約には依存せずDAOの読取り結果を返す。
#[derive(Debug)]
pub struct PlanGenerationUseCase<D> {
    dao: D,
}
impl<D: PlanGenerationDao> PlanGenerationUseCase<D> {
    /// DAOを注入する。
    #[must_use]
    pub const fn new(dao: D) -> Self {
        Self { dao }
    }
    /// 呼出側が採番した開始操作IDで検索する。
    /// # Errors
    /// リードモデルを読めない場合。
    pub fn execute(&self, id: &str) -> Result<Option<PlanGenerationView>, ReadModelReadError> {
        self.dao.find(id)
    }
}
