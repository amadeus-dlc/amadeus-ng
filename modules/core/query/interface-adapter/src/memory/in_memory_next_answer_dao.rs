//! `NextAnswerDao` の in-memory テストダブル。

use core_query_use_case::orchestration::{NextAnswerDao, NextAnswerView, ReadModelReadError};

/// `read_next_answer` の 1 行 (`request_kind` は View が運ばないので明示的に持つ)。
#[derive(Debug, Clone, PartialEq, Eq)]
struct Row {
    request_kind: String,
    view: NextAnswerView,
}

/// `next` の答えの行を持ち、**鍵で引く**ダブル。
///
/// 明示的に取る鍵は **View が運ばない列だけ** (`request_kind`) で、実行の識別子は行から
/// 読む — 鍵と行の値がずれる余地を作らない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InMemoryNextAnswerDao {
    held: Result<Vec<Row>, ReadModelReadError>,
}

impl InMemoryNextAnswerDao {
    /// 握る内容をそのまま組み立てる (**この型の唯一の構造体リテラル**)。
    const fn new(held: Result<Vec<Row>, ReadModelReadError>) -> InMemoryNextAnswerDao {
        InMemoryNextAnswerDao { held }
    }

    /// 行が 1 つも無い (どの鍵にも当たらない)。
    #[must_use]
    pub const fn empty() -> InMemoryNextAnswerDao {
        InMemoryNextAnswerDao::new(Ok(Vec::new()))
    }

    /// 行を 1 つ足す (鍵は `request_kind` と行の `execution_id`)。
    #[must_use]
    pub fn with_row(mut self, request_kind: &str, view: NextAnswerView) -> InMemoryNextAnswerDao {
        if let Ok(rows) = &mut self.held {
            rows.push(Row {
                request_kind: request_kind.to_string(),
                view,
            });
        }
        self
    }

    /// 引けない。
    #[must_use]
    pub const fn failing(error: ReadModelReadError) -> InMemoryNextAnswerDao {
        InMemoryNextAnswerDao::new(Err(error))
    }
}

impl NextAnswerDao for InMemoryNextAnswerDao {
    fn find(
        &self,
        execution_id: &str,
        request_kind: &str,
    ) -> Result<Option<NextAnswerView>, ReadModelReadError> {
        let rows = self.held.as_ref().map_err(Clone::clone)?;
        Ok(rows
            .iter()
            .find(|row| row.view.execution_id() == execution_id && row.request_kind == request_kind)
            .map(|row| row.view.clone()))
    }
}
