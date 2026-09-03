//! `JumpPhaseDao` の実 Gateway — フェーズの目的地 1 行を `read_next_jump_phase` から引く。

use std::path::Path;

use core_query_use_case::orchestration::{JumpPhaseDao, JumpPhaseView, ReadModelReadError};

use super::read_model_store::ReadModelStore;

/// 自然キー (`execution_id`, `phase` — UNIQUE 索引 `read_next_jump_phase_key`) の 1 行引当。
///
/// 受理判定 (`read_next_jump`) は結合しない — 行が言うのは目的地の位置までで、そこから
/// たどるのはユースケースの仕事である (オーナー裁定 2026-09-03 —
/// `coding-rules/cqrs-boundaries.md` 規則 6)。
const SELECT_PHASE_TARGET: &str = "SELECT target_index, target_slug \
FROM read_next_jump_phase WHERE execution_id = ?1 AND phase = ?2";

/// フェーズの目的地 1 行を返す実装。
#[derive(Debug)]
pub struct JumpPhaseDaoImpl {
    store: ReadModelStore,
}

impl JumpPhaseDaoImpl {
    /// 構造化リードモデルのストアを読取専用で開く。
    ///
    /// # Errors
    ///
    /// ストアを開けない ([`ReadModelReadError`])。
    pub fn open(path: &Path) -> Result<JumpPhaseDaoImpl, ReadModelReadError> {
        Ok(JumpPhaseDaoImpl {
            store: ReadModelStore::open(path)?,
        })
    }
}

impl JumpPhaseDao for JumpPhaseDaoImpl {
    fn find(
        &self,
        execution_id: &str,
        phase: &str,
    ) -> Result<Option<JumpPhaseView>, ReadModelReadError> {
        self.store
            .find_one(SELECT_PHASE_TARGET, &[&execution_id, &phase], |row| {
                Ok(JumpPhaseView::new(row.get(0)?, row.get(1)?))
            })
    }
}
