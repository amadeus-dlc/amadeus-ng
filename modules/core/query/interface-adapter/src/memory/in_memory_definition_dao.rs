//! `DefinitionDao` の in-memory テストダブル。

use core_query_use_case::orchestration::{
    DefinitionDao, DefinitionSummaryView, ReadModelReadError,
};

/// `read_definition` の 1 行 (鍵 `id` は View が運ばないので明示的に持つ)。
#[derive(Debug, Clone, PartialEq, Eq)]
struct Row {
    definition_id: String,
    view: DefinitionSummaryView,
}

/// 定義要約の行を持ち、**鍵で引く**ダブル。
///
/// 鍵を見るのは、SQLite 実装と**同じ契約を同じ入力で**満たすためである — 契約テストは
/// ジェネリック関数 1 本を両実装に走らせるので、鍵を無視するダブルでは「当たらなければ
/// `Ok(None)`」を満たせない (`coding-rules/good-examples.md` §契約テスト)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InMemoryDefinitionDao {
    held: Result<Vec<Row>, ReadModelReadError>,
}

impl InMemoryDefinitionDao {
    /// 握る内容をそのまま組み立てる (**この型の唯一の構造体リテラル**)。
    const fn new(held: Result<Vec<Row>, ReadModelReadError>) -> InMemoryDefinitionDao {
        InMemoryDefinitionDao { held }
    }

    /// 行が 1 つも無い (どの鍵にも当たらない)。
    #[must_use]
    pub const fn empty() -> InMemoryDefinitionDao {
        InMemoryDefinitionDao::new(Ok(Vec::new()))
    }

    /// 行を 1 つ足す (鍵は `definition_id`)。
    #[must_use]
    pub fn with_row(
        mut self,
        definition_id: &str,
        view: DefinitionSummaryView,
    ) -> InMemoryDefinitionDao {
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
    pub const fn failing(error: ReadModelReadError) -> InMemoryDefinitionDao {
        InMemoryDefinitionDao::new(Err(error))
    }
}

impl DefinitionDao for InMemoryDefinitionDao {
    fn find(
        &self,
        definition_id: &str,
    ) -> Result<Option<DefinitionSummaryView>, ReadModelReadError> {
        let rows = self.held.as_ref().map_err(Clone::clone)?;
        Ok(rows
            .iter()
            .find(|row| row.definition_id == definition_id)
            .map(|row| row.view.clone()))
    }
}
