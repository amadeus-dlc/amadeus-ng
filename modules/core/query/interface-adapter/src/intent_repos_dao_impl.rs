//! `IntentReposDao` の実 Gateway — intent 登録簿 (`intents.json`) の行が持つ `repos` を読む。
//!
//! 媒体は JSON ファイルであり、SQLite の `read_*` 表とは別の面である — したがって
//! [`super::ReadModelDaos`] (1 要求 1 接続) の住人ではなく、登録簿の所在だけを握る
//! ([`super::IntentRecordDaoImpl`] と同じ面を別の問いで読む)。

use std::path::{Path, PathBuf};

use core_query_use_case::orchestration::IntentReposDao;

use crate::registry_row::record_dir_matches;

/// intent 登録簿を読む実装。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntentReposDaoImpl {
    registry: PathBuf,
}

impl IntentReposDaoImpl {
    /// 登録簿の所在を受け取る (**この型の唯一の構築経路**)。
    #[must_use]
    pub fn new(registry: &Path) -> IntentReposDaoImpl {
        IntentReposDaoImpl {
            registry: registry.to_path_buf(),
        }
    }
}

impl IntentReposDao for IntentReposDaoImpl {
    fn find(&self, record_dir_name: &str) -> Vec<String> {
        // 不在・壊れた JSON はどちらも「記録が無い」に畳む (upstream `readIntentRegistry` の
        // `catch` → `[]`)。ここで拒否に変えると、upstream なら名前の後退で答える場面を
        // こちらだけが落としてしまう。
        let Ok(bytes) = std::fs::read(&self.registry) else {
            return Vec::new();
        };
        let Ok(entries) = serde_json::from_slice::<Vec<serde_json::Value>>(&bytes) else {
            return Vec::new();
        };
        entries
            .iter()
            .find(|entry| record_dir_matches(entry, record_dir_name))
            .and_then(|entry| entry.get("repos"))
            .and_then(serde_json::Value::as_array)
            .map(|repos| {
                repos
                    .iter()
                    .filter_map(serde_json::Value::as_str)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default()
    }
}
