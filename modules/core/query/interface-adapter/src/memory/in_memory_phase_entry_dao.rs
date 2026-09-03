//! `PhaseEntryDao` の in-memory テストダブル。

use core_query_use_case::orchestration::{PhaseEntryDao, PhaseEntryView, ReadModelReadError};

/// 引当の結果を握るダブル (鍵は見ない — 何を返すかはテストが決める)。
///
/// 鍵で振り分けないのは、**このダブルが確かめる対象ではない**からである。鍵で引けることは
/// SQLite 実装の契約テストが見る。ここが要るのは「行が在る / 無い / 読めない」の 3 状態を
/// 合成ルート周辺のテストが実 I/O 無しで組めることだけである。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InMemoryPhaseEntryDao {
    held: Result<Option<PhaseEntryView>, ReadModelReadError>,
}

impl InMemoryPhaseEntryDao {
    /// 行が在る。
    #[must_use]
    pub const fn holding(view: PhaseEntryView) -> InMemoryPhaseEntryDao {
        InMemoryPhaseEntryDao {
            held: Ok(Some(view)),
        }
    }

    /// 行が無い (正常な観測であって失敗ではない)。
    #[must_use]
    pub const fn absent() -> InMemoryPhaseEntryDao {
        InMemoryPhaseEntryDao { held: Ok(None) }
    }

    /// 引けない。
    #[must_use]
    pub const fn failing(error: ReadModelReadError) -> InMemoryPhaseEntryDao {
        InMemoryPhaseEntryDao { held: Err(error) }
    }
}

impl PhaseEntryDao for InMemoryPhaseEntryDao {
    fn find(
        &self,
        _: &str,
        _: &str,
        _: &str,
    ) -> Result<Option<PhaseEntryView>, ReadModelReadError> {
        self.held.clone()
    }
}
