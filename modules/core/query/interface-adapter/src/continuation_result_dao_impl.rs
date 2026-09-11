//! 停止判断のread表を要求IDで引く。
use super::read_model_store::ReadModelStore;
use core_query_use_case::orchestration::{
    ContinuationResultDao, ContinuationResultView, ReadModelReadError,
};
use std::rc::Rc;
fn decode(row: &rusqlite::Row<'_>) -> rusqlite::Result<ContinuationResultView> {
    Ok(ContinuationResultView::new(
        row.get(0)?,
        row.get(1)?,
        u64::try_from(row.get::<_, i64>(2)?).map_err(|_| rusqlite::Error::InvalidQuery)?,
        row.get::<_, String>(3)?
            .parse()
            .map_err(|_| rusqlite::Error::InvalidQuery)?,
        row.get(4)?,
        row.get(5)?,
        row.get(6)?,
    ))
}

/// 行を読んでDTOへ写すだけのDAO。
#[derive(Debug)]
pub struct ContinuationResultDaoImpl {
    store: Rc<ReadModelStore>,
}
impl ContinuationResultDaoImpl {
    pub(crate) const fn new(store: Rc<ReadModelStore>) -> Self {
        Self { store }
    }
}
impl ContinuationResultDao for ContinuationResultDaoImpl {
    fn find(&self, id: &str) -> Result<Option<ContinuationResultView>, ReadModelReadError> {
        self.store.find_one(
            "SELECT id,blocked,count,block_cap,wait_reason,counter_published,settled FROM read_continuation_result WHERE id=?1",
            &[&id],
            decode,
        )
    }
    fn unsettled(
        &self,
        aggregate_id: &str,
    ) -> Result<Option<ContinuationResultView>, ReadModelReadError> {
        self.store.find_one("SELECT id,blocked,count,block_cap,wait_reason,counter_published,settled FROM read_continuation_result WHERE aggregate_id=?1 AND settled=0", &[&aggregate_id], decode)
    }
}
