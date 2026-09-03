//! `SteeringPartDao` の in-memory テストダブル。

use core_query_use_case::orchestration::{ReadModelReadError, SteeringPartDao, SteeringPartView};

/// 引当の結果を握るダブル (鍵は見ない — 何を返すかはテストが決める)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InMemorySteeringPartDao {
    held: Result<Option<SteeringPartView>, ReadModelReadError>,
}

impl InMemorySteeringPartDao {
    /// 部が在る。
    #[must_use]
    pub const fn holding(view: SteeringPartView) -> InMemorySteeringPartDao {
        InMemorySteeringPartDao {
            held: Ok(Some(view)),
        }
    }

    /// その番号の部が無い (終端 — 正常な観測)。
    #[must_use]
    pub const fn absent() -> InMemorySteeringPartDao {
        InMemorySteeringPartDao { held: Ok(None) }
    }

    /// 引けない。
    #[must_use]
    pub const fn failing(error: ReadModelReadError) -> InMemorySteeringPartDao {
        InMemorySteeringPartDao { held: Err(error) }
    }
}

impl SteeringPartDao for InMemorySteeringPartDao {
    fn find(&self, _: &str, _: u32) -> Result<Option<SteeringPartView>, ReadModelReadError> {
        self.held.clone()
    }
}
