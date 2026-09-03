//! `SteeringPlanDao` の in-memory テストダブル。

use core_query_use_case::orchestration::{ReadModelReadError, SteeringPlanDao, SteeringPlanView};

/// 引当の結果を握るダブル (鍵は見ない — 何を返すかはテストが決める)。
///
/// 鍵で振り分けないのは、**このダブルが確かめる対象ではない**からである。鍵で引けることは
/// SQLite 実装の契約テストが見る。ここが要るのは「行が在る / 無い / 読めない」の 3 状態を
/// 合成ルート周辺のテストが実 I/O 無しで組めることだけである。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InMemorySteeringPlanDao {
    held: Result<Option<SteeringPlanView>, ReadModelReadError>,
}

impl InMemorySteeringPlanDao {
    /// 計画が在る。
    #[must_use]
    pub const fn holding(view: SteeringPlanView) -> InMemorySteeringPlanDao {
        InMemorySteeringPlanDao {
            held: Ok(Some(view)),
        }
    }

    /// 計画が無い (まだパックしていない — 正常な観測)。
    #[must_use]
    pub const fn absent() -> InMemorySteeringPlanDao {
        InMemorySteeringPlanDao { held: Ok(None) }
    }

    /// 引けない。
    #[must_use]
    pub const fn failing(error: ReadModelReadError) -> InMemorySteeringPlanDao {
        InMemorySteeringPlanDao { held: Err(error) }
    }
}

impl SteeringPlanDao for InMemorySteeringPlanDao {
    fn find(&self, _: &str) -> Result<Option<SteeringPlanView>, ReadModelReadError> {
        self.held.clone()
    }

    fn find_bound(&self, _: &str, _: &str) -> Result<Option<SteeringPlanView>, ReadModelReadError> {
        self.held.clone()
    }
}
