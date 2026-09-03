//! `ExecutionDao` の実 Gateway — 実行の現在地 1 行を `read_execution` から引く。

use std::rc::Rc;

use core_query_use_case::orchestration::{ExecutionDao, ExecutionView, ReadModelReadError};
use rusqlite::Row;

use super::read_model_store::ReadModelStore;

/// `read_execution` を 1 表で引く SELECT を、`WHERE` 句の literal から組む。
///
/// 列は DDL と同じ並びで、この順で [`execution_row`] が読む。定義の識別子は**結合しない**
/// — それは intent の持ち物なので、要る呼び手は `intent_id` の FK をたどる
/// (オーナー裁定 2026-09-03 — `coding-rules/cqrs-boundaries.md` 規則 6)。
macro_rules! select_execution {
    ($where_clause:literal) => {
        concat!(
            "SELECT id, intent_id, scope, status, cursor_slug, parked_at_slug, parked_active, \
             state_binding FROM read_execution WHERE ",
            $where_clause
        )
    };
}

/// 主キー (実行の識別子) の 1 行引当。
const SELECT_BY_ID: &str = select_execution!("id = ?1");

/// 状態束縛の引当 (索引 `read_execution_state_binding`)。
///
/// 束縛は実行ごとに違う値になるので、等値で高々 1 行に定まる。
const SELECT_BY_STATE_BINDING: &str = select_execution!("state_binding = ?1");

/// 8 列を 1 行の写しへ。
fn execution_row(row: &Row<'_>) -> rusqlite::Result<ExecutionView> {
    Ok(ExecutionView::new(
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
        row.get(4)?,
        row.get(5)?,
        row.get(6)?,
        row.get(7)?,
    ))
}

/// 実行の現在地 1 行を返す実装 (2 動詞とも同じ 1 表を鍵違いで引く)。
#[derive(Debug)]
pub struct ExecutionDaoImpl {
    store: Rc<ReadModelStore>,
}

impl ExecutionDaoImpl {
    /// 1 要求ぶんの共有ストアを受け取る (**この型の唯一の構築経路**)。
    ///
    /// 開くのは [`super::ReadModelDaos`] 1 か所で、12 実装はその 1 接続を分け合う。
    /// 実装ごとに開くと、多段の引当が別々のスナップショットを見る余地が残る。
    #[must_use]
    pub(crate) const fn new(store: Rc<ReadModelStore>) -> ExecutionDaoImpl {
        ExecutionDaoImpl { store }
    }
}

impl ExecutionDao for ExecutionDaoImpl {
    fn find(&self, execution_id: &str) -> Result<Option<ExecutionView>, ReadModelReadError> {
        self.store
            .find_one(SELECT_BY_ID, &[&execution_id], execution_row)
    }

    fn find_by_state_binding(
        &self,
        state_binding: &str,
    ) -> Result<Option<ExecutionView>, ReadModelReadError> {
        self.store
            .find_one(SELECT_BY_STATE_BINDING, &[&state_binding], execution_row)
    }
}
