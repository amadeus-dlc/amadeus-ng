//! `RunStageDao` の in-memory テストダブル。

use core_query_use_case::orchestration::{ReadModelReadError, RunStageDao, RunStageView};

/// run-stage の材料の行を持ち、**鍵で引く**ダブル。
///
/// 3 つの鍵 (自然キー / 代理キー / 自然キー + 2 束縛) はいずれも行が運ぶ列なので、明示的な
/// 鍵引数を取らず View から読む。とりわけ束縛 2 列は**鍵の一部**であって照合ではないので、
/// SQLite 実装と同じく `WHERE` の残余条件として扱う。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InMemoryRunStageDao {
    held: Result<Vec<RunStageView>, ReadModelReadError>,
}

impl InMemoryRunStageDao {
    /// 握る内容をそのまま組み立てる (**この型の唯一の構造体リテラル**)。
    const fn new(held: Result<Vec<RunStageView>, ReadModelReadError>) -> InMemoryRunStageDao {
        InMemoryRunStageDao { held }
    }

    /// 行が 1 つも無い (どの鍵にも当たらない)。
    #[must_use]
    pub const fn empty() -> InMemoryRunStageDao {
        InMemoryRunStageDao::new(Ok(Vec::new()))
    }

    /// 行を 1 つ足す (鍵はすべて行の列)。
    #[must_use]
    pub fn with_row(mut self, view: RunStageView) -> InMemoryRunStageDao {
        if let Ok(rows) = &mut self.held {
            rows.push(view);
        }
        self
    }

    /// 引けない。
    #[must_use]
    pub const fn failing(error: ReadModelReadError) -> InMemoryRunStageDao {
        InMemoryRunStageDao::new(Err(error))
    }

    /// 鍵に当たる行を 1 つ返す (3 動詞が共有する引当)。
    fn find_row(
        &self,
        matches: impl Fn(&RunStageView) -> bool,
    ) -> Result<Option<RunStageView>, ReadModelReadError> {
        let rows = self.held.as_ref().map_err(Clone::clone)?;
        Ok(rows.iter().find(|view| matches(view)).cloned())
    }
}

impl RunStageDao for InMemoryRunStageDao {
    fn find(
        &self,
        definition_id: &str,
        scope: &str,
        stage_slug: &str,
    ) -> Result<Option<RunStageView>, ReadModelReadError> {
        self.find_row(|view| {
            view.definition_id() == definition_id
                && view.scope() == scope
                && view.stage_slug() == stage_slug
        })
    }

    fn find_by_id(&self, id: &str) -> Result<Option<RunStageView>, ReadModelReadError> {
        self.find_row(|view| view.id() == id)
    }

    fn find_bound(
        &self,
        definition_id: &str,
        scope: &str,
        stage_slug: &str,
        route_digest: &str,
        directive_digest: &str,
    ) -> Result<Option<RunStageView>, ReadModelReadError> {
        self.find_row(|view| {
            view.definition_id() == definition_id
                && view.scope() == scope
                && view.stage_slug() == stage_slug
                && view.route_digest() == route_digest
                && view.directive_digest() == directive_digest
        })
    }
}
