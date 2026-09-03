//! `ScopeDao` の in-memory テストダブル。

use core_query_use_case::orchestration::{ReadModelReadError, ScopeDao, ScopeView};

/// scope カタログの列を名前で握るダブル。
///
/// ここだけは鍵 (scope 名) で振り分ける — 既製 3 scope の引当 ([`ScopeDao::find_stock`]) は
/// trait の既定実装が [`ScopeDao::find`] を 3 回呼ぶ形なので、名前を見ないダブルでは
/// その並びを組めない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InMemoryScopeDao {
    held: Result<Vec<ScopeView>, ReadModelReadError>,
}

impl InMemoryScopeDao {
    /// カタログの列を並べて握る (名前で引き分ける)。
    #[must_use]
    pub const fn holding(views: Vec<ScopeView>) -> InMemoryScopeDao {
        InMemoryScopeDao { held: Ok(views) }
    }

    /// カタログが空 (どの名前も引けない)。
    #[must_use]
    pub const fn absent() -> InMemoryScopeDao {
        InMemoryScopeDao {
            held: Ok(Vec::new()),
        }
    }

    /// 引けない。
    #[must_use]
    pub const fn failing(error: ReadModelReadError) -> InMemoryScopeDao {
        InMemoryScopeDao { held: Err(error) }
    }
}

impl ScopeDao for InMemoryScopeDao {
    fn find(&self, _: &str, scope: &str) -> Result<Option<ScopeView>, ReadModelReadError> {
        match &self.held {
            Err(error) => Err(error.clone()),
            Ok(views) => Ok(views.iter().find(|view| view.scope() == scope).cloned()),
        }
    }
}
