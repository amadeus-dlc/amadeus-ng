//! `ScopeChangeDao` の in-memory テストダブル。

use core_query_use_case::orchestration::{ReadModelReadError, ScopeChangeDao, ScopeChangeView};

/// 引当の結果を握るダブル (鍵は見ない — 何を返すかはテストが決める)。
///
/// 鍵で振り分けないのは、**このダブルが確かめる対象ではない**からである。鍵で引けることは
/// SQLite 実装の契約テストが見る。ここが要るのは「行が在る / 無い / 読めない」の 3 状態を
/// 合成ルート周辺のテストが実 I/O 無しで組めることだけである。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InMemoryScopeChangeDao {
    held: Result<Option<ScopeChangeView>, ReadModelReadError>,
}

impl InMemoryScopeChangeDao {
    /// 行が在る。
    #[must_use]
    pub const fn holding(view: ScopeChangeView) -> InMemoryScopeChangeDao {
        InMemoryScopeChangeDao {
            held: Ok(Some(view)),
        }
    }

    /// 行が無い (正常な観測であって失敗ではない)。
    #[must_use]
    pub const fn absent() -> InMemoryScopeChangeDao {
        InMemoryScopeChangeDao { held: Ok(None) }
    }

    /// 引けない。
    #[must_use]
    pub const fn failing(error: ReadModelReadError) -> InMemoryScopeChangeDao {
        InMemoryScopeChangeDao { held: Err(error) }
    }
}

impl ScopeChangeDao for InMemoryScopeChangeDao {
    fn find(&self, _: &str, _: &str) -> Result<Option<ScopeChangeView>, ReadModelReadError> {
        self.held.clone()
    }
}
