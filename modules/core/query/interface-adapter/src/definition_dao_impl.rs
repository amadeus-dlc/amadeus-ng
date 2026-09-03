//! `DefinitionDao` の実 Gateway — 定義 1 行の要約を引く。

use std::path::Path;

use core_query_use_case::orchestration::{
    DefinitionDao, DefinitionSummaryView, ReadModelReadError,
};

use super::read_model_store::ReadModelStore;

/// 主キー `id` (= 定義の系譜 ID) の 1 行引当。
const SELECT_DEFINITION: &str =
    "SELECT revision, stage_count, scope_count FROM read_definition WHERE id = ?1";

/// 定義 1 行の要約を返す実装。
#[derive(Debug)]
pub struct DefinitionDaoImpl {
    store: ReadModelStore,
}

impl DefinitionDaoImpl {
    /// 構造化リードモデルのストアを読取専用で開く。
    ///
    /// # Errors
    ///
    /// ストアを開けない ([`ReadModelReadError`])。
    pub fn open(path: &Path) -> Result<DefinitionDaoImpl, ReadModelReadError> {
        Ok(DefinitionDaoImpl {
            store: ReadModelStore::open(path)?,
        })
    }
}

impl DefinitionDao for DefinitionDaoImpl {
    fn find(
        &self,
        definition_id: &str,
    ) -> Result<Option<DefinitionSummaryView>, ReadModelReadError> {
        self.store
            .find_one(SELECT_DEFINITION, &[&definition_id], |row| {
                Ok(DefinitionSummaryView::new(
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                ))
            })
    }
}
