//! `PhaseEntryDao` の実 Gateway — 定義側のフェーズ入口を引く。

use std::path::Path;

use core_query_use_case::orchestration::{PhaseEntryDao, PhaseEntryView, ReadModelReadError};

use super::read_model_store::ReadModelStore;

/// 自然キー (`definition_id`, `scope`, `phase` — UNIQUE 索引
/// `read_definition_scope_phase_entry_key`) の 1 行引当。
const SELECT_PHASE_ENTRY: &str = "SELECT first_stage_slug FROM read_definition_scope_phase_entry \
WHERE definition_id = ?1 AND scope = ?2 AND phase = ?3";

/// 定義側のフェーズ入口を返す実装。
#[derive(Debug)]
pub struct PhaseEntryDaoImpl {
    store: ReadModelStore,
}

impl PhaseEntryDaoImpl {
    /// 構造化リードモデルのストアを読取専用で開く。
    ///
    /// # Errors
    ///
    /// ストアを開けない ([`ReadModelReadError`])。
    pub fn open(path: &Path) -> Result<PhaseEntryDaoImpl, ReadModelReadError> {
        Ok(PhaseEntryDaoImpl {
            store: ReadModelStore::open(path)?,
        })
    }
}

impl PhaseEntryDao for PhaseEntryDaoImpl {
    fn find(
        &self,
        definition_id: &str,
        scope: &str,
        phase: &str,
    ) -> Result<Option<PhaseEntryView>, ReadModelReadError> {
        self.store.find_one(
            SELECT_PHASE_ENTRY,
            &[&definition_id, &scope, &phase],
            |row| Ok(PhaseEntryView::new(row.get(0)?)),
        )
    }
}
