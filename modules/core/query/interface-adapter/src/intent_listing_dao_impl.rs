//! `IntentListingDao` の実 Gateway — 登録簿の行と記録ディレクトリを 1 つの並びへ畳む。
//!
//! 媒体は JSON ファイルとディレクトリであり、SQLite の `read_*` 表とは別の面である —
//! したがって [`super::ReadModelDaos`] (1 要求 1 接続) の住人ではなく、依頼ディレクトリの
//! 所在だけを握る ([`super::IntentReposDaoImpl`] と同じ面を別の問いで読む)。
//!
//! # 記録ディレクトリの見分け方
//!
//! `aidlc-state.md` を持つ子ディレクトリだけを記録として数える (upstream `listIntentDirs`
//! 逐語)。カーソル (`active-intent`)、登録簿 (`intents.json`)、その他の紛れ込みを名前の
//! 綴りで除外すると、新しい紛れ込みが増えるたびに除外表が古くなる。

use std::path::{Path, PathBuf};

use core_query_use_case::orchestration::{
    IntentListingDao, IntentListingRowView, ReadModelReadError,
};

use crate::display_slug_from_dir_name;
use crate::registry_row::record_dir_matches;

/// 記録ディレクトリであることの印 (upstream 逐語)。
const STATE_FILE: &str = "aidlc-state.md";

/// 登録簿のファイル名 (upstream 逐語)。
const REGISTRY_FILE: &str = "intents.json";

/// 登録簿に行が無い記録が名乗る状態 (upstream 逐語)。
const UNKNOWN_STATUS: &str = "unknown";

/// 空間の依頼一覧を読む実装。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntentListingDaoImpl {
    intents_dir: PathBuf,
}

impl IntentListingDaoImpl {
    /// 依頼ディレクトリの所在を受け取る (**この型の唯一の構築経路**)。
    #[must_use]
    pub fn new(intents_dir: &Path) -> IntentListingDaoImpl {
        IntentListingDaoImpl {
            intents_dir: intents_dir.to_path_buf(),
        }
    }

    /// `aidlc-state.md` を持つ子ディレクトリの名前を、名前順で並べる。
    fn record_dirs(&self) -> Result<Vec<String>, ReadModelReadError> {
        let entries = match std::fs::read_dir(&self.intents_dir) {
            Ok(entries) => entries,
            // 依頼ディレクトリがまだ無いのは失敗ではない — 空の一覧が正しい観測である。
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => {
                return Err(ReadModelReadError::new(
                    error.kind(),
                    Some(self.intents_dir.clone()),
                ));
            }
        };
        let mut records = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|error| {
                ReadModelReadError::new(error.kind(), Some(self.intents_dir.clone()))
            })?;
            if !entry.path().join(STATE_FILE).is_file() {
                continue;
            }
            if let Some(name) = entry.file_name().to_str() {
                records.push(name.to_string());
            }
        }
        records.sort();
        Ok(records)
    }

    /// 登録簿の行。不在も壊れた JSON も「行が無い」に畳む (upstream `catch` → `[]`)。
    fn registry(&self) -> Vec<serde_json::Value> {
        std::fs::read(self.intents_dir.join(REGISTRY_FILE))
            .ok()
            .and_then(|bytes| serde_json::from_slice::<Vec<serde_json::Value>>(&bytes).ok())
            .unwrap_or_default()
    }
}

impl IntentListingDao for IntentListingDaoImpl {
    fn find(&self) -> Result<Vec<IntentListingRowView>, ReadModelReadError> {
        let dirs = self.record_dirs()?;
        let mut claimed: Vec<&str> = Vec::new();
        let mut rows: Vec<IntentListingRowView> = Vec::new();
        for entry in self.registry() {
            let directory = dirs
                .iter()
                .find(|dir| record_dir_matches(&entry, dir))
                .map(String::as_str);
            if let Some(directory) = directory {
                claimed.push(directory);
            }
            rows.push(IntentListingRowView::new(
                text(&entry, "uuid"),
                text(&entry, "slug"),
                text(&entry, "status"),
                entry
                    .get("repos")
                    .and_then(serde_json::Value::as_array)
                    .map(|repos| {
                        repos
                            .iter()
                            .filter_map(serde_json::Value::as_str)
                            .map(str::to_string)
                            .collect()
                    })
                    .unwrap_or_default(),
                directory.map(str::to_string),
            ));
        }
        // 登録簿に行が無い記録も並べる — 一覧から消すと、切替先として名乗れる記録が
        // 見えないまま「知らない依頼」と拒否される。
        for directory in &dirs {
            if claimed.contains(&directory.as_str()) {
                continue;
            }
            rows.push(IntentListingRowView::new(
                String::new(),
                display_slug_from_dir_name(directory),
                UNKNOWN_STATUS.to_string(),
                Vec::new(),
                Some(directory.clone()),
            ));
        }
        Ok(rows)
    }
}

fn text(entry: &serde_json::Value, key: &str) -> String {
    entry
        .get(key)
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_string()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::*;

    struct Intents {
        root: tempfile::TempDir,
    }

    impl Intents {
        fn new() -> Intents {
            Intents {
                root: tempfile::tempdir().expect("一時ディレクトリ"),
            }
        }

        fn dao(&self) -> IntentListingDaoImpl {
            IntentListingDaoImpl::new(self.root.path())
        }

        fn record(&self, name: &str) {
            let dir = self.root.path().join(name);
            std::fs::create_dir_all(&dir).expect("記録");
            std::fs::write(dir.join(STATE_FILE), "# state\n").expect("状態ファイル");
        }

        fn registry(&self, json: &str) {
            std::fs::write(self.root.path().join(REGISTRY_FILE), json).expect("登録簿");
        }
    }

    #[test]
    fn an_absent_intents_directory_lists_nothing() {
        let root = tempfile::tempdir().expect("一時ディレクトリ");
        let dao = IntentListingDaoImpl::new(&root.path().join("missing"));
        assert_eq!(dao.find(), Ok(Vec::new()));
    }

    /// 登録簿の行は記録ディレクトリと結び付き、値をそのまま運ぶ。
    #[test]
    fn a_registry_row_is_joined_to_its_record_directory() {
        let intents = Intents::new();
        intents.record("260915-auth-service");
        intents.registry(
            r#"[{"uuid":"u-1","slug":"auth-service","status":"active","repos":["app"],"dirName":"260915-auth-service"}]"#,
        );
        let rows = intents.dao().find().expect("一覧");
        assert_eq!(rows.len(), 1);
        let row = rows.first().expect("1 行目");
        assert_eq!(row.uuid(), "u-1");
        assert_eq!(row.slug(), "auth-service");
        assert_eq!(row.status(), "active");
        assert_eq!(row.repos(), ["app".to_string()]);
        assert_eq!(row.directory(), Some("260915-auth-service"));
    }

    /// 登録簿に行が無い記録も並ぶ — 一覧から消すと切替先として名乗れない。
    #[test]
    fn a_record_without_a_registry_row_is_still_listed() {
        let intents = Intents::new();
        intents.record("260915-orphan");
        let rows = intents.dao().find().expect("一覧");
        assert_eq!(rows.len(), 1);
        let row = rows.first().expect("1 行目");
        assert_eq!(row.uuid(), "");
        assert_eq!(row.slug(), "orphan");
        assert_eq!(row.status(), UNKNOWN_STATUS);
        assert_eq!(row.directory(), Some("260915-orphan"));
    }

    /// 登録簿に在るのに記録が無い行は、記録ディレクトリを持たないまま並ぶ。
    #[test]
    fn a_registry_row_without_a_record_keeps_a_null_directory() {
        let intents = Intents::new();
        intents.registry(
            r#"[{"uuid":"u-1","slug":"gone","status":"finished","dirName":"260101-gone"}]"#,
        );
        let rows = intents.dao().find().expect("一覧");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows.first().expect("1 行目").directory(), None);
    }

    /// 壊れた登録簿は「行が無い」に畳み、記録ディレクトリの一覧は失わない。
    #[test]
    fn a_broken_registry_does_not_hide_the_records_on_disk() {
        let intents = Intents::new();
        intents.record("260915-auth-service");
        intents.registry("{ not json");
        let rows = intents.dao().find().expect("一覧");
        assert_eq!(rows.len(), 1);
        let row = rows.first().expect("1 行目");
        assert_eq!(row.directory(), Some("260915-auth-service"));
        assert_eq!(row.status(), UNKNOWN_STATUS);
    }

    /// `aidlc-state.md` を持たない子は記録ではない (カーソルや登録簿の紛れ込み)。
    #[test]
    fn a_directory_without_a_state_file_is_not_a_record() {
        let intents = Intents::new();
        std::fs::create_dir_all(intents.root.path().join(".aidlc-sensors")).expect("紛れ込み");
        assert_eq!(intents.dao().find(), Ok(Vec::new()));
    }

    #[test]
    fn the_display_slug_drops_a_date_stamp_or_an_identifier_suffix() {
        assert_eq!(
            display_slug_from_dir_name("260915-auth-service"),
            "auth-service"
        );
        assert_eq!(
            display_slug_from_dir_name("auth-service-1a2b3c4d"),
            "auth-service"
        );
        assert_eq!(display_slug_from_dir_name("plain"), "plain");
    }

    /// `dirName` の無い行だけが `<slug>-<id8>` の形へ後退する。
    #[test]
    fn a_legacy_row_without_a_dir_name_falls_back_to_the_slug_and_identifier_shape() {
        let intents = Intents::new();
        intents.record("auth-service-1a2b3c4d");
        intents
            .registry(r#"[{"uuid":"0000-0000-1a2b3c4d","slug":"auth-service","status":"active"}]"#);
        let rows = intents.dao().find().expect("一覧");
        assert_eq!(rows.len(), 1);
        assert_eq!(
            rows.first().expect("1 行目").directory(),
            Some("auth-service-1a2b3c4d")
        );
    }
}
