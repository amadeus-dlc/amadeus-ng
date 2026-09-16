//! 開始可否の1表引当。
use super::read_model_store::ReadModelStore;
use core_query_use_case::orchestration::{
    CodeGenerationApprovalDao, CodeGenerationApprovalView, ReadModelReadError,
};
use std::rc::Rc;

/// 実行と対象のキーで保存済みの行だけを読む。
#[derive(Debug)]
pub struct CodeGenerationApprovalDaoImpl {
    store: Rc<ReadModelStore>,
}

impl CodeGenerationApprovalDaoImpl {
    pub(crate) const fn new(store: Rc<ReadModelStore>) -> Self {
        Self { store }
    }
}

impl CodeGenerationApprovalDao for CodeGenerationApprovalDaoImpl {
    fn find(
        &self,
        execution_id: &str,
        target_id: &str,
    ) -> Result<Option<CodeGenerationApprovalView>, ReadModelReadError> {
        self.store.find_one("SELECT ok,reason,unit,contract_hash FROM read_code_generation_approval WHERE execution_id=?1 AND target_id=?2", &[&execution_id, &target_id], |row| {
            Ok(CodeGenerationApprovalView::new(row.get::<_, i64>(0)? != 0, row.get(1)?, row.get(2)?, row.get(3)?))
        })
    }
}
