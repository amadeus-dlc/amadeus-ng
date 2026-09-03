//! `PhaseEntryDao` の in-memory テストダブル。

use core_query_use_case::orchestration::{PhaseEntryDao, PhaseEntryView, ReadModelReadError};

/// `read_definition_scope_phase_entry` の 1 行 (自然キー 3 列は View が運ばない)。
#[derive(Debug, Clone, PartialEq, Eq)]
struct Row {
    definition_id: String,
    scope: String,
    phase: String,
    view: PhaseEntryView,
}

/// 定義側のフェーズ入口の行を持ち、**鍵で引く**ダブル。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InMemoryPhaseEntryDao {
    held: Result<Vec<Row>, ReadModelReadError>,
}

impl InMemoryPhaseEntryDao {
    /// 握る内容をそのまま組み立てる (**この型の唯一の構造体リテラル**)。
    const fn new(held: Result<Vec<Row>, ReadModelReadError>) -> InMemoryPhaseEntryDao {
        InMemoryPhaseEntryDao { held }
    }

    /// 行が 1 つも無い (どの鍵にも当たらない)。
    #[must_use]
    pub const fn empty() -> InMemoryPhaseEntryDao {
        InMemoryPhaseEntryDao::new(Ok(Vec::new()))
    }

    /// 行を 1 つ足す (鍵は定義 × scope × フェーズ)。
    #[must_use]
    pub fn with_row(
        mut self,
        definition_id: &str,
        scope: &str,
        phase: &str,
        view: PhaseEntryView,
    ) -> InMemoryPhaseEntryDao {
        if let Ok(rows) = &mut self.held {
            rows.push(Row {
                definition_id: definition_id.to_string(),
                scope: scope.to_string(),
                phase: phase.to_string(),
                view,
            });
        }
        self
    }

    /// 引けない。
    #[must_use]
    pub const fn failing(error: ReadModelReadError) -> InMemoryPhaseEntryDao {
        InMemoryPhaseEntryDao::new(Err(error))
    }
}

impl PhaseEntryDao for InMemoryPhaseEntryDao {
    fn find(
        &self,
        definition_id: &str,
        scope: &str,
        phase: &str,
    ) -> Result<Option<PhaseEntryView>, ReadModelReadError> {
        let rows = self.held.as_ref().map_err(Clone::clone)?;
        Ok(rows
            .iter()
            .find(|row| {
                row.definition_id == definition_id && row.scope == scope && row.phase == phase
            })
            .map(|row| row.view.clone()))
    }
}
