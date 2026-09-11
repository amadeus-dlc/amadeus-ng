//! UUIDとper-user配置の対応。workflowの進行判断は持たない。
use crate::layout::Layout;
use core_command_domain::workspace::{IntentDirName, SpaceName};
use std::{fs, path::Path};
/// registryと実在recordから観測したintentの場所。
#[derive(Debug, Clone)]
pub(crate) struct IntentLocation {
    layout: Layout,
    uuid: String,
    slug: String,
}
impl IntentLocation {
    const fn new(layout: Layout, uuid: String, slug: String) -> Self {
        Self { layout, uuid, slug }
    }
    pub(crate) fn current(layout: &Layout) -> Option<Self> {
        let record = layout.record_dir()?.file_name()?.to_str()?;
        let rows: serde_json::Value =
            serde_json::from_slice(&fs::read(layout.intents_dir().join("intents.json")).ok()?)
                .ok()?;
        let row = rows
            .as_array()?
            .iter()
            .find(|row| row.get("dirName").and_then(serde_json::Value::as_str) == Some(record))?;
        Self::from_row(layout.project_dir(), layout.space(), row)
    }
    pub(crate) fn find(project: &Path, uuid: &str) -> Option<Self> {
        let mut spaces = fs::read_dir(project.join("aidlc/spaces"))
            .ok()?
            .filter_map(Result::ok)
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        spaces.sort();
        for space in spaces {
            if SpaceName::parse(&space).is_err() {
                continue;
            }
            let Some(rows) = fs::read(
                project
                    .join("aidlc/spaces")
                    .join(&space)
                    .join("intents/intents.json"),
            )
            .ok()
            .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok()) else {
                continue;
            };
            let Some(rows) = rows.as_array() else {
                continue;
            };
            for row in rows {
                if row.get("uuid").and_then(serde_json::Value::as_str) == Some(uuid)
                    && let Some(found) = Self::from_row(project, &space, row)
                {
                    return Some(found);
                }
            }
        }
        None
    }
    fn from_row(project: &Path, space: &str, row: &serde_json::Value) -> Option<Self> {
        let uuid = row.get("uuid")?.as_str()?.to_string();
        if uuid.is_empty() {
            return None;
        }
        let record = IntentDirName::parse(row.get("dirName")?.as_str()?).ok()?;
        let layout = Layout::for_record(project, &SpaceName::parse(space).ok()?, &record);
        if !layout.state_file()?.exists() {
            return None;
        }
        Some(Self::new(layout, uuid, row.get("slug")?.as_str()?.into()))
    }
    pub(crate) const fn layout(&self) -> &Layout {
        &self.layout
    }
    pub(crate) fn uuid(&self) -> &str {
        &self.uuid
    }
    pub(crate) fn slug(&self) -> &str {
        &self.slug
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    use super::*;

    fn write(path: &Path, body: &str) {
        fs::create_dir_all(path.parent().expect("親")).expect("ディレクトリ");
        fs::write(path, body).expect("書込");
    }

    /// registry の行は uuid・dirName・slug が揃い、状態ファイルが実在して初めて場所になる。
    /// 名前が文法外の space や壊れた registry は飛ばして次の space を見る。
    #[test]
    fn find_skips_invalid_spaces_and_rows_until_a_real_record_matches() {
        let root = tempfile::tempdir().expect("一時ディレクトリ");
        let spaces = root.path().join("aidlc/spaces");
        // 文法外の space 名（空白）は候補にならない。
        write(&spaces.join("bad space/intents/intents.json"), "[]");
        // 壊れた JSON、配列でない JSON は飛ばす。
        write(&spaces.join("a-broken/intents/intents.json"), "{not json");
        write(
            &spaces.join("b-object/intents/intents.json"),
            r#"{"uuid":"x"}"#,
        );
        // 空の uuid と状態ファイルの無い記録は場所にならない。
        write(
            &spaces.join("c-rows/intents/intents.json"),
            r#"[{"uuid":"","dirName":"260101-empty-aaaaaaaa","slug":"empty"},
                {"uuid":"ghost","dirName":"260101-ghost-bbbbbbbb","slug":"ghost"},
                {"uuid":"real","dirName":"260101-real-cccccccc","slug":"real"}]"#,
        );
        write(
            &spaces.join("c-rows/intents/260101-real-cccccccc/aidlc-state.md"),
            "# state\n",
        );
        assert!(IntentLocation::find(root.path(), "ghost").is_none());
        assert!(IntentLocation::find(root.path(), "").is_none());
        let found = IntentLocation::find(root.path(), "real").expect("実在する記録");
        assert_eq!(found.uuid(), "real");
        assert_eq!(found.slug(), "real");
        assert_eq!(found.layout().space(), "c-rows");
        assert!(
            found
                .layout()
                .record_dir()
                .is_some_and(|dir| dir.ends_with("260101-real-cccccccc"))
        );
        assert!(IntentLocation::find(root.path(), "missing").is_none());
        let nowhere = tempfile::tempdir().expect("一時ディレクトリ");
        assert!(IntentLocation::find(nowhere.path(), "real").is_none());
    }

    /// `current` はカーソルの記録を registry で引く。registry に無ければ場所にならない。
    #[test]
    fn current_resolves_the_cursor_record_through_the_registry() {
        let root = tempfile::tempdir().expect("一時ディレクトリ");
        let intents = root.path().join("aidlc/spaces/default/intents");
        write(&intents.join("active-intent"), "260101-real-cccccccc\n");
        write(
            &intents.join("260101-real-cccccccc/aidlc-state.md"),
            "# state\n",
        );
        let layout = Layout::resolve(root.path());
        assert!(
            IntentLocation::current(&layout).is_none(),
            "registry が無い"
        );
        write(
            &intents.join("intents.json"),
            r#"[{"uuid":"real","dirName":"260101-real-cccccccc","slug":"real"}]"#,
        );
        let found = IntentLocation::current(&layout).expect("registry の行");
        assert_eq!(found.slug(), "real");
    }
}
