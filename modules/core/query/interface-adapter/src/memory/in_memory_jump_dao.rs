//! `JumpDao` の in-memory テストダブル。

use core_query_use_case::orchestration::{JumpDao, JumpView, ReadModelReadError};

/// `read_next_jump` の 1 行 (`execution_id` は View が運ばないので明示的に持つ)。
#[derive(Debug, Clone, PartialEq, Eq)]
struct Row {
    execution_id: String,
    view: JumpView,
}

/// ジャンプの受理判定の行を持ち、**鍵で引く**ダブル。
///
/// 2 つの動詞は**同じ行の集まり**を別の鍵 (目的地の slug / 位置) で引く — SQLite 実装が
/// 1 表を鍵違いで引くのと同じ形である。動詞ごとに別の答えを握ると、slug で引いた行と
/// 位置で引いた行が食い違いうる。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InMemoryJumpDao {
    held: Result<Vec<Row>, ReadModelReadError>,
}

impl InMemoryJumpDao {
    /// 握る内容をそのまま組み立てる (**この型の唯一の構造体リテラル**)。
    const fn new(held: Result<Vec<Row>, ReadModelReadError>) -> InMemoryJumpDao {
        InMemoryJumpDao { held }
    }

    /// 行が 1 つも無い (どの鍵にも当たらない)。
    #[must_use]
    pub const fn empty() -> InMemoryJumpDao {
        InMemoryJumpDao::new(Ok(Vec::new()))
    }

    /// 行を 1 つ足す (鍵は `execution_id` と行の目的地列)。
    #[must_use]
    pub fn with_row(mut self, execution_id: &str, view: JumpView) -> InMemoryJumpDao {
        if let Ok(rows) = &mut self.held {
            rows.push(Row {
                execution_id: execution_id.to_string(),
                view,
            });
        }
        self
    }

    /// 引けない。
    #[must_use]
    pub const fn failing(error: ReadModelReadError) -> InMemoryJumpDao {
        InMemoryJumpDao::new(Err(error))
    }

    /// 鍵に当たる行を 1 つ返す (2 動詞が共有する引当)。
    fn find_row(
        &self,
        execution_id: &str,
        matches: impl Fn(&JumpView) -> bool,
    ) -> Result<Option<JumpView>, ReadModelReadError> {
        let rows = self.held.as_ref().map_err(Clone::clone)?;
        Ok(rows
            .iter()
            .find(|row| row.execution_id == execution_id && matches(&row.view))
            .map(|row| row.view.clone()))
    }
}

impl JumpDao for InMemoryJumpDao {
    fn find(
        &self,
        execution_id: &str,
        target_slug: &str,
    ) -> Result<Option<JumpView>, ReadModelReadError> {
        self.find_row(execution_id, |view| view.target_slug() == target_slug)
    }

    fn find_by_target(
        &self,
        execution_id: &str,
        target_index: u32,
    ) -> Result<Option<JumpView>, ReadModelReadError> {
        self.find_row(execution_id, |view| view.target_index() == target_index)
    }
}
