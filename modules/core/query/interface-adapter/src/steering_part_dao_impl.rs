//! `SteeringPartDao` の実 Gateway — 配信の 1 部を `read_steering_part` から引く。

use std::path::Path;

use core_query_use_case::orchestration::{ReadModelReadError, SteeringPartDao, SteeringPartView};

use super::read_model_store::ReadModelStore;

/// 計画 × 部番号の 1 行引当 (索引 `read_steering_part_plan`)。
///
/// 部番号は**呼び手が渡す鍵**である — `next` が 1 部目から配るという要求の形は
/// [`SteeringPartDao::FIRST_PART`] がポート面で持ち、SQL のリテラルには焼かない
/// (オーナー裁定 2026-09-03 — 非正規化の焼き込みをしない)。
const SELECT_STEERING_PART: &str = "SELECT steering_plan_id, phase, part_index, rules_content \
FROM read_steering_part WHERE steering_plan_id = ?1 AND part_index = ?2";

/// 配信の 1 部を返す実装。
#[derive(Debug)]
pub struct SteeringPartDaoImpl {
    store: ReadModelStore,
}

impl SteeringPartDaoImpl {
    /// 構造化リードモデルのストアを読取専用で開く。
    ///
    /// # Errors
    ///
    /// ストアを開けない ([`ReadModelReadError`])。
    pub fn open(path: &Path) -> Result<SteeringPartDaoImpl, ReadModelReadError> {
        Ok(SteeringPartDaoImpl {
            store: ReadModelStore::open(path)?,
        })
    }
}

impl SteeringPartDao for SteeringPartDaoImpl {
    fn find(
        &self,
        steering_plan_id: &str,
        part_index: u32,
    ) -> Result<Option<SteeringPartView>, ReadModelReadError> {
        self.store.find_one(
            SELECT_STEERING_PART,
            &[&steering_plan_id, &part_index],
            |row| {
                Ok(SteeringPartView::new(
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                ))
            },
        )
    }
}
