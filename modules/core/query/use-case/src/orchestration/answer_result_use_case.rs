//! 回答結果を指定IDで読む。業務判断・更新は持たない。
use super::{AnswerResultDao, AnswerResultView, ReadModelReadError};
/// 呼出側が既知の回答識別子で結果を引く。
#[derive(Debug)]
pub struct AnswerResultUseCase<D: AnswerResultDao> {
    dao: D,
}
impl<D: AnswerResultDao> AnswerResultUseCase<D> {
    /// DAOを注入する。
    #[must_use]
    pub const fn new(dao: D) -> Self {
        Self { dao }
    }
    /// 投影された結果を返す。
    /// # Errors
    /// リードモデルが読めない場合。
    pub fn execute(&self, answer_id: &str) -> Result<Option<AnswerResultView>, ReadModelReadError> {
        self.dao.find(answer_id)
    }
}
