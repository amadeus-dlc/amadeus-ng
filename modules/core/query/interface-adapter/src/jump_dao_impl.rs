//! `JumpDao` の実 Gateway — ジャンプ先ごとの受理判定を `read_next_jump` から引く。

use std::path::Path;

use core_query_use_case::orchestration::{JumpDao, JumpView, ReadModelReadError};
use rusqlite::Row;

use super::read_model_store::ReadModelStore;

/// `read_next_jump` を 1 表で引く SELECT を、`WHERE` 句の literal から組む。
macro_rules! select_jump {
    ($where_clause:literal) => {
        concat!(
            "SELECT target_index, target_slug, outcome, refusal FROM read_next_jump WHERE ",
            $where_clause
        )
    };
}

/// 実行 × ジャンプ先 slug の 1 行引当 (索引 `read_next_jump_target_slug`)。
///
/// `target_slug` は自然キーではないが、1 つの実行の計画に同じ slug は 2 度現れないので
/// 等値で 1 行に定まる。
const SELECT_BY_TARGET_SLUG: &str = select_jump!("execution_id = ?1 AND target_slug = ?2");

/// 実行 × ジャンプ先の位置 (自然キー — UNIQUE 索引 `read_next_jump_key`)。
///
/// フェーズ表 (`read_next_jump_phase`) が言う目的地の位置をそのまま鍵にする口である。
/// 2 表を結合しないのは、たどるのがユースケースの仕事だからである (オーナー裁定
/// 2026-09-03 — `coding-rules/cqrs-boundaries.md` 規則 6)。
const SELECT_BY_TARGET_INDEX: &str = select_jump!("execution_id = ?1 AND target_index = ?2");

/// ジャンプの受理判定を返す実装 (2 動詞とも同じ 1 表を鍵違いで引く)。
#[derive(Debug)]
pub struct JumpDaoImpl {
    store: ReadModelStore,
}

impl JumpDaoImpl {
    /// 構造化リードモデルのストアを読取専用で開く。
    ///
    /// # Errors
    ///
    /// ストアを開けない ([`ReadModelReadError`])。
    pub fn open(path: &Path) -> Result<JumpDaoImpl, ReadModelReadError> {
        Ok(JumpDaoImpl {
            store: ReadModelStore::open(path)?,
        })
    }
}

/// 4 列を 1 行の写しへ。
fn jump_row(row: &Row<'_>) -> rusqlite::Result<JumpView> {
    Ok(JumpView::new(
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
    ))
}

impl JumpDao for JumpDaoImpl {
    fn find(
        &self,
        execution_id: &str,
        target_slug: &str,
    ) -> Result<Option<JumpView>, ReadModelReadError> {
        self.store.find_one(
            SELECT_BY_TARGET_SLUG,
            &[&execution_id, &target_slug],
            jump_row,
        )
    }

    fn find_by_target(
        &self,
        execution_id: &str,
        target_index: u32,
    ) -> Result<Option<JumpView>, ReadModelReadError> {
        self.store.find_one(
            SELECT_BY_TARGET_INDEX,
            &[&execution_id, &target_index],
            jump_row,
        )
    }
}
