//! `JumpPhaseDao` の in-memory テストダブル。

use core_query_use_case::orchestration::{JumpPhaseDao, JumpPhaseView, ReadModelReadError};

/// 引当の結果を握るダブル (鍵は見ない — 何を返すかはテストが決める)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InMemoryJumpPhaseDao {
    held: Result<Option<JumpPhaseView>, ReadModelReadError>,
}

impl InMemoryJumpPhaseDao {
    /// 目的地が在る。
    #[must_use]
    pub const fn holding(view: JumpPhaseView) -> InMemoryJumpPhaseDao {
        InMemoryJumpPhaseDao {
            held: Ok(Some(view)),
        }
    }

    /// 目的地を持たないフェーズ (正常な観測)。
    #[must_use]
    pub const fn absent() -> InMemoryJumpPhaseDao {
        InMemoryJumpPhaseDao { held: Ok(None) }
    }

    /// 引けない。
    #[must_use]
    pub const fn failing(error: ReadModelReadError) -> InMemoryJumpPhaseDao {
        InMemoryJumpPhaseDao { held: Err(error) }
    }
}

impl JumpPhaseDao for InMemoryJumpPhaseDao {
    fn find(&self, _: &str, _: &str) -> Result<Option<JumpPhaseView>, ReadModelReadError> {
        self.held.clone()
    }
}
