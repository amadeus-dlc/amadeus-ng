//! 計画回答の結果を操作IDで返すQuery。
use super::{PlanAnswerDao, PlanAnswerView, ReadModelReadError};
/// 集約には依存せずDAOの読取り結果を返す。
#[derive(Debug)]
pub struct PlanAnswerUseCase<D> {
    dao: D,
}
impl<D: PlanAnswerDao> PlanAnswerUseCase<D> {
    /// DAOを注入する。
    #[must_use]
    pub const fn new(dao: D) -> Self {
        Self { dao }
    }
    /// 呼出側が採番した回答操作IDで検索する。
    /// # Errors
    /// リードモデルを読めない場合。
    pub fn execute(&self, id: &str) -> Result<Option<PlanAnswerView>, ReadModelReadError> {
        self.dao.find(id)
    }
}
