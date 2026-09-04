//! `DefinitionStageDao` の in-memory テストダブル。

use core_query_use_case::orchestration::{
    DefinitionStageDao, DefinitionStageView, ReadModelReadError,
};

/// `read_definition_stage` の 1 行 (鍵は View が運ばない `definition_id` を明示的に持つ)。
#[derive(Debug, Clone, PartialEq, Eq)]
struct Row {
    definition_id: String,
    view: DefinitionStageView,
}

/// ステージ行を持ち、**鍵で引く**ダブル。
///
/// 鍵を見るのは、SQLite 実装と**同じ契約を同じ入力で**満たすためである
/// (`coding-rules/good-examples.md` §契約テスト)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InMemoryDefinitionStageDao {
    held: Result<Vec<Row>, ReadModelReadError>,
}

impl InMemoryDefinitionStageDao {
    /// 握る内容をそのまま組み立てる (**この型の唯一の構造体リテラル**)。
    const fn new(held: Result<Vec<Row>, ReadModelReadError>) -> InMemoryDefinitionStageDao {
        InMemoryDefinitionStageDao { held }
    }

    /// 行が 1 つも無い (どの鍵にも当たらない)。
    #[must_use]
    pub const fn empty() -> InMemoryDefinitionStageDao {
        InMemoryDefinitionStageDao::new(Ok(Vec::new()))
    }

    /// 行を 1 つ足す (鍵は (`definition_id`, `stage_slug`))。
    #[must_use]
    pub fn with_row(
        mut self,
        definition_id: &str,
        view: DefinitionStageView,
    ) -> InMemoryDefinitionStageDao {
        if let Ok(rows) = &mut self.held {
            rows.push(Row {
                definition_id: definition_id.to_string(),
                view,
            });
        }
        self
    }

    /// 引けない。
    #[must_use]
    pub const fn failing(error: ReadModelReadError) -> InMemoryDefinitionStageDao {
        InMemoryDefinitionStageDao::new(Err(error))
    }
}

impl DefinitionStageDao for InMemoryDefinitionStageDao {
    fn find(
        &self,
        definition_id: &str,
        stage_slug: &str,
    ) -> Result<Option<DefinitionStageView>, ReadModelReadError> {
        let rows = self.held.as_ref().map_err(Clone::clone)?;
        Ok(rows
            .iter()
            .find(|row| row.definition_id == definition_id && row.view.stage_slug() == stage_slug)
            .map(|row| row.view.clone()))
    }
}
