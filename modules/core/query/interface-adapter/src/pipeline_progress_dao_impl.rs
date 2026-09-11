//! Pipeline参照投影1表だけのキー引当。
use super::read_model_store::ReadModelStore;
use core_query_use_case::orchestration::{
    PipelineProgressDao, PipelineProgressView, ReadModelReadError,
};
use std::rc::Rc;
/// 読取り専用接続を使うDAO。
#[derive(Debug)]
pub struct PipelineProgressDaoImpl {
    store: Rc<ReadModelStore>,
}
impl PipelineProgressDaoImpl {
    pub(crate) const fn new(store: Rc<ReadModelStore>) -> Self {
        Self { store }
    }
}
impl PipelineProgressDao for PipelineProgressDaoImpl {
    fn find(
        &self,
        execution: &str,
        stage: &str,
        single: bool,
    ) -> Result<Option<PipelineProgressView>, ReadModelReadError> {
        self.store.find_one("SELECT completed FROM read_pipeline_progress WHERE execution_id=?1 AND stage=?2 AND single=?3",&[&execution,&stage,&single],|row|Ok(PipelineProgressView::new(row.get(0)?)))
    }
}
