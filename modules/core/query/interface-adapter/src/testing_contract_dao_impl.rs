//! テスト契約の指定IDによる1表引当。
use super::read_model_store::ReadModelStore;
use core_query_use_case::orchestration::{
    ReadModelReadError, TestingContractDao, TestingContractView,
};
use std::rc::Rc;
/// 規則を読まず、RMUが構築した行を取得する。
#[derive(Debug)]
pub struct TestingContractDaoImpl {
    store: Rc<ReadModelStore>,
}
impl TestingContractDaoImpl {
    pub(crate) const fn new(store: Rc<ReadModelStore>) -> Self {
        Self { store }
    }
}
impl TestingContractDao for TestingContractDaoImpl {
    fn find(&self, intent_id: &str) -> Result<Option<TestingContractView>, ReadModelReadError> {
        self.store.find_one(
            "SELECT contract,rendered,error FROM read_testing_contract WHERE id=?1",
            &[&intent_id],
            |row| {
                Ok(TestingContractView::new(
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                ))
            },
        )
    }
}
