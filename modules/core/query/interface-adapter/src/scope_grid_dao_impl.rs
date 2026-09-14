//! `ScopeGridDao` の実 Gateway — `scope-grid.json` を読んで 1 scope 分の割当を写す。
//!
//! 媒体は JSON ファイル (compile コンテキストの投影 = リードモデル) であり、それを読む実装が
//! クエリ側に在るのは規則どおりである (`coding-rules/cqrs-boundaries.md` 規則 6/7)。

use std::collections::BTreeMap;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use core_query_use_case::orchestration::{ReadModelReadError, ScopeActionsView, ScopeGridDao};

/// scope グリッドを 1 面読む実装。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopeGridDaoImpl {
    scope_grid: PathBuf,
}

impl ScopeGridDaoImpl {
    /// 配布データ置き場 (`<harness>/tools/data`) を受け取る (**この型の唯一の構築経路**)。
    #[must_use]
    pub fn new(definition_data_dir: &Path) -> ScopeGridDaoImpl {
        ScopeGridDaoImpl {
            scope_grid: definition_data_dir.join("scope-grid.json"),
        }
    }
}

impl ScopeGridDao for ScopeGridDaoImpl {
    fn find(&self, scope: &str) -> Result<Option<ScopeActionsView>, ReadModelReadError> {
        let raw = match std::fs::read_to_string(&self.scope_grid) {
            Ok(text) => text,
            Err(error) => {
                return Err(ReadModelReadError::new(
                    error.kind(),
                    Some(self.scope_grid.clone()),
                ));
            }
        };
        let parsed: serde_json::Value = serde_json::from_str(&raw).map_err(|_| {
            ReadModelReadError::new(ErrorKind::InvalidData, Some(self.scope_grid.clone()))
        })?;
        let Some(entry) = parsed.get(scope) else {
            return Ok(None);
        };
        let mut actions = BTreeMap::new();
        if let Some(stages) = entry.get("stages").and_then(serde_json::Value::as_object) {
            for (slug, action) in stages {
                if let Some(action) = action.as_str() {
                    actions.insert(slug.clone(), action.to_string());
                }
            }
        }
        Ok(Some(ScopeActionsView::new(scope.to_string(), actions)))
    }
}
