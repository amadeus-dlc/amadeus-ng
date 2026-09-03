//! `SteeringPlanDao` の in-memory テストダブル。

use core_query_use_case::orchestration::{ReadModelReadError, SteeringPlanDao, SteeringPlanView};

/// 配信計画の行を持ち、**鍵で引く**ダブル。
///
/// 2 つの鍵 (`id` / `id` + 束のダイジェスト) はどちらも行が運ぶ列なので、明示的な鍵引数を
/// 取らず View から読む。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InMemorySteeringPlanDao {
    held: Result<Vec<SteeringPlanView>, ReadModelReadError>,
}

impl InMemorySteeringPlanDao {
    /// 握る内容をそのまま組み立てる (**この型の唯一の構造体リテラル**)。
    const fn new(
        held: Result<Vec<SteeringPlanView>, ReadModelReadError>,
    ) -> InMemorySteeringPlanDao {
        InMemorySteeringPlanDao { held }
    }

    /// 行が 1 つも無い (どの鍵にも当たらない)。
    #[must_use]
    pub const fn empty() -> InMemorySteeringPlanDao {
        InMemorySteeringPlanDao::new(Ok(Vec::new()))
    }

    /// 行を 1 つ足す (鍵はどちらも行の列)。
    #[must_use]
    pub fn with_row(mut self, view: SteeringPlanView) -> InMemorySteeringPlanDao {
        if let Ok(rows) = &mut self.held {
            rows.push(view);
        }
        self
    }

    /// 引けない。
    #[must_use]
    pub const fn failing(error: ReadModelReadError) -> InMemorySteeringPlanDao {
        InMemorySteeringPlanDao::new(Err(error))
    }
}

impl SteeringPlanDao for InMemorySteeringPlanDao {
    fn find(&self, id: &str) -> Result<Option<SteeringPlanView>, ReadModelReadError> {
        let rows = self.held.as_ref().map_err(Clone::clone)?;
        Ok(rows.iter().find(|view| view.id() == id).cloned())
    }

    fn find_bound(
        &self,
        id: &str,
        bundle_digest: &str,
    ) -> Result<Option<SteeringPlanView>, ReadModelReadError> {
        let rows = self.held.as_ref().map_err(Clone::clone)?;
        Ok(rows
            .iter()
            .find(|view| view.id() == id && view.bundle_digest() == bundle_digest)
            .cloned())
    }
}
