//! 指定された回答IDの結果だけを読む。
use super::read_model_store::ReadModelStore;
use core_query_use_case::orchestration::{AnswerResultDao, AnswerResultView, ReadModelReadError};
use std::rc::Rc;
/// 回答結果のSQL読取り。
#[derive(Debug)]
pub struct AnswerResultDaoImpl {
    store: Rc<ReadModelStore>,
}
impl AnswerResultDaoImpl {
    pub(crate) const fn new(store: Rc<ReadModelStore>) -> Self {
        Self { store }
    }
}
impl AnswerResultDao for AnswerResultDaoImpl {
    fn find(&self, answer_id: &str) -> Result<Option<AnswerResultView>, ReadModelReadError> {
        self.store.find_one(
            "SELECT execution_id, stage, disposition FROM read_answer_result WHERE answer_id=?1",
            &[&answer_id],
            |row| Ok(AnswerResultView::new(row.get(0)?, row.get(1)?, row.get(2)?)),
        )
    }
}
