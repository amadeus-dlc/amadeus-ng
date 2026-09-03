//! `JumpDao` の in-memory テストダブル。

use core_query_use_case::orchestration::{JumpDao, JumpView, ReadModelReadError};

/// ジャンプ引当の結果を握るダブル (鍵は見ない — 2 つの動詞は別々に握る)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InMemoryJumpDao {
    by_slug: Result<Option<JumpView>, ReadModelReadError>,
    by_target: Result<Option<JumpView>, ReadModelReadError>,
}

impl InMemoryJumpDao {
    /// 2 つの動詞それぞれの答えを握る。
    #[must_use]
    pub const fn new(
        by_slug: Result<Option<JumpView>, ReadModelReadError>,
        by_target: Result<Option<JumpView>, ReadModelReadError>,
    ) -> InMemoryJumpDao {
        InMemoryJumpDao { by_slug, by_target }
    }

    /// どちらの動詞も行を返さない。
    #[must_use]
    pub const fn absent() -> InMemoryJumpDao {
        InMemoryJumpDao::new(Ok(None), Ok(None))
    }
}

impl JumpDao for InMemoryJumpDao {
    fn find(&self, _: &str, _: &str) -> Result<Option<JumpView>, ReadModelReadError> {
        self.by_slug.clone()
    }

    fn find_by_target(&self, _: &str, _: u32) -> Result<Option<JumpView>, ReadModelReadError> {
        self.by_target.clone()
    }
}
