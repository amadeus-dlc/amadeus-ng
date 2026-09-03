//! `SteeringPartDao` の in-memory テストダブル。

use core_query_use_case::orchestration::{ReadModelReadError, SteeringPartDao, SteeringPartView};

/// 配信の部の行を持ち、**鍵で引く**ダブル。
///
/// 鍵 (計画の FK と部番号) はどちらも行が運ぶ列なので、明示的な鍵引数を取らず View から
/// 読む。終端 (`Ok(None)`) は行の有無がそのまま表す。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InMemorySteeringPartDao {
    held: Result<Vec<SteeringPartView>, ReadModelReadError>,
}

impl InMemorySteeringPartDao {
    /// 握る内容をそのまま組み立てる (**この型の唯一の構造体リテラル**)。
    const fn new(
        held: Result<Vec<SteeringPartView>, ReadModelReadError>,
    ) -> InMemorySteeringPartDao {
        InMemorySteeringPartDao { held }
    }

    /// 行が 1 つも無い (どの鍵にも当たらない)。
    #[must_use]
    pub const fn empty() -> InMemorySteeringPartDao {
        InMemorySteeringPartDao::new(Ok(Vec::new()))
    }

    /// 行を 1 つ足す (鍵はどちらも行の列)。
    #[must_use]
    pub fn with_row(mut self, view: SteeringPartView) -> InMemorySteeringPartDao {
        if let Ok(rows) = &mut self.held {
            rows.push(view);
        }
        self
    }

    /// 引けない。
    #[must_use]
    pub const fn failing(error: ReadModelReadError) -> InMemorySteeringPartDao {
        InMemorySteeringPartDao::new(Err(error))
    }
}

impl SteeringPartDao for InMemorySteeringPartDao {
    fn find(
        &self,
        steering_plan_id: &str,
        part_index: u32,
    ) -> Result<Option<SteeringPartView>, ReadModelReadError> {
        let rows = self.held.as_ref().map_err(Clone::clone)?;
        Ok(rows
            .iter()
            .find(|view| {
                view.steering_plan_id() == steering_plan_id && view.part_index() == part_index
            })
            .cloned())
    }
}
