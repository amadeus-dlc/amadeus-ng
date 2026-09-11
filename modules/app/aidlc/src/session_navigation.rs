//! host sessionの機械ローカルなnavigation。承認権限の正本にはしない。
use crate::layout::Layout;
use std::{
    fs,
    path::{Path, PathBuf},
};
/// 検査済みのsession名でだけper-userファイルへ接続する。
#[derive(Debug)]
pub(crate) struct SessionNavigation {
    project: PathBuf,
    session: String,
}
impl SessionNavigation {
    pub(crate) fn new(project: &Path, session: &str) -> Option<Self> {
        Layout::valid_session_id(session).then(|| Self {
            project: project.to_path_buf(),
            session: session.to_string(),
        })
    }
    fn directory(&self) -> PathBuf {
        self.project.join("aidlc/.aidlc-sessions")
    }
    pub(crate) fn stamp(&self) -> Option<String> {
        fs::read(self.directory().join(&self.session))
            .ok()
            .map(|bytes| {
                core_infrastructure::ecmascript::trim(&String::from_utf8_lossy(&bytes)).to_string()
            })
            .filter(|value| !value.is_empty())
    }
    pub(crate) fn binding(&self) -> Option<Layout> {
        Layout::bound_selection(&self.project, &self.session)
    }
    pub(crate) fn write_current(&self) -> std::io::Result<()> {
        fs::create_dir_all(self.directory())?;
        fs::write(
            self.directory().join(".current-session"),
            format!("{}\n", self.session),
        )
    }
    pub(crate) fn write_binding(&self, layout: &Layout) -> std::io::Result<()> {
        use core_infrastructure::canon_json::{
            JsonValue, ObjectMembers, SerializationProfile, serialize,
        };
        fs::create_dir_all(self.directory())?;
        let mut fields = ObjectMembers::new();
        fields.insert("space", JsonValue::String(layout.space().into()));
        fields.insert(
            "intent",
            layout
                .record_dir()
                .and_then(Path::file_name)
                .map_or(JsonValue::Null, |name| {
                    JsonValue::String(name.to_string_lossy().into_owned())
                }),
        );
        fields.insert(
            "boundAt",
            JsonValue::String(
                chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            ),
        );
        fs::write(
            self.directory()
                .join(format!("{}.binding.json", self.session)),
            format!(
                "{}\n",
                serialize(
                    &JsonValue::Object(fields),
                    SerializationProfile::ContractCompact
                )
            ),
        )
    }
    pub(crate) fn write_stamp(&self, uuid: &str) -> std::io::Result<()> {
        if uuid.is_empty() {
            return Ok(());
        }
        fs::create_dir_all(self.directory())?;
        fs::write(self.directory().join(&self.session), format!("{uuid}\n"))
    }
    /// 同じ会話が別の作業を作成した境界を、Stopが一度だけ読める形で記録する。
    pub(crate) fn write_handoff(&self, from: &str, to: &str) -> std::io::Result<()> {
        use core_infrastructure::canon_json::{
            JsonValue, ObjectMembers, SerializationProfile, serialize, to_value,
        };
        if from.is_empty() || to.is_empty() || from == to {
            return Ok(());
        }
        let mut fields = ObjectMembers::new();
        fields.insert("fromIntentUuid", JsonValue::String(from.to_string()));
        fields.insert("toIntentUuid", JsonValue::String(to.to_string()));
        fields.insert(
            "issuedAtMs",
            to_value(&chrono::Utc::now().timestamp_millis()).map_err(std::io::Error::other)?,
        );
        fs::create_dir_all(self.directory())?;
        fs::write(
            self.directory()
                .join(format!("{}.handoff.json", self.session)),
            format!(
                "{}\n",
                serialize(
                    &JsonValue::Object(fields),
                    SerializationProfile::ContractCompact
                )
            ),
        )
    }
    pub(crate) fn clear_stamp(&self) -> std::io::Result<()> {
        remove_if_present(&self.directory().join(&self.session))
    }
    pub(crate) fn offer(&self) -> Option<String> {
        fs::read(
            self.directory()
                .join(format!("{}.rebind-offer", self.session)),
        )
        .ok()
        .map(|bytes| {
            core_infrastructure::ecmascript::trim(&String::from_utf8_lossy(&bytes)).to_string()
        })
        .filter(|text| !text.is_empty())
    }
    pub(crate) fn write_offer(&self, offer: &str) -> std::io::Result<()> {
        if offer.is_empty() {
            return Ok(());
        }
        fs::create_dir_all(self.directory())?;
        fs::write(
            self.directory()
                .join(format!("{}.rebind-offer", self.session)),
            format!("{offer}\n"),
        )
    }
    pub(crate) fn clear_offer(&self) -> std::io::Result<()> {
        remove_if_present(
            &self
                .directory()
                .join(format!("{}.rebind-offer", self.session)),
        )
    }
    pub(crate) fn write_ancestry(&self) {
        crate::session_processes::write_ancestry(&self.project, &self.session);
    }
    pub(crate) fn current_id(project: &Path) -> Option<String> {
        std::env::var("AIDLC_SESSION_OVERRIDE")
            .ok()
            .filter(|session| Layout::valid_session_id(session))
            .or_else(|| crate::session_processes::resolve(project))
    }
    pub(crate) fn write_transcript(project: &Path, session: &str, transcript: &str) {
        if transcript.is_empty()
            || std::env::var("AIDLC_DISABLE_USAGE_TRACKING").as_deref() == Ok("1")
        {
            return;
        }
        let directory = project.join("aidlc/.aidlc-sessions");
        if fs::create_dir_all(&directory).is_err() {
            return;
        }
        if Layout::valid_session_id(session)
            && core_infrastructure::atomic::write_file_atomic(
                &directory.join(format!("{session}.transcript")),
                transcript.as_bytes(),
            )
            .is_err()
        {
            return;
        }
        let _ = core_infrastructure::atomic::write_file_atomic(
            &directory.join("current.transcript"),
            transcript.as_bytes(),
        );
    }
    pub(crate) fn bootstrap(layout: &Layout) {
        use std::io::Write as _;
        let shared = Layout::shared(layout.project_dir());
        let root = shared.aidlc_root();
        if fs::create_dir_all(&root).is_ok() {
            let staged = root.join(format!(
                ".aidlc-active-space-{}-{}.tmp",
                std::process::id(),
                uuid::Uuid::now_v7()
            ));
            if let Ok(mut file) = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&staged)
                && file
                    .write_all(format!("{}\n", shared.space()).as_bytes())
                    .is_ok()
            {
                let _ = fs::hard_link(&staged, root.join("active-space"));
            }
            let _ = fs::remove_file(staged);
        }
        let stub = layout.project_dir().join(".claude/rules/aidlc.md");
        if let Ok(bytes) = fs::read(&stub) {
            let raw = String::from_utf8_lossy(&bytes);
            let next = raw
                .split('\n')
                .map(|line| {
                    let Some(after_at) = line.strip_prefix('@') else {
                        return line.to_string();
                    };
                    let rest = after_at.trim_start_matches("../");
                    let prefix = after_at
                        .get(..after_at.len().saturating_sub(rest.len()))
                        .unwrap_or_default();
                    let Some(path) = rest.strip_prefix("aidlc/spaces/") else {
                        return line.to_string();
                    };
                    let Some((old_space, file)) = path.split_once("/memory/") else {
                        return line.to_string();
                    };
                    if old_space.is_empty() || old_space.contains('/') || file.is_empty() {
                        return line.to_string();
                    }
                    format!("@{prefix}aidlc/spaces/{}/memory/{file}", layout.space())
                })
                .collect::<Vec<_>>()
                .join("\n");
            if next != raw {
                let _ = core_infrastructure::atomic::write_file_atomic(&stub, next.as_bytes());
            }
        }
    }
}

fn remove_if_present(path: &Path) -> std::io::Result<()> {
    match fs::remove_file(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        result => result,
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    use super::*;

    const SESSION: &str = "11111111-2222-4333-8444-555555555555";

    fn navigation(root: &Path) -> SessionNavigation {
        SessionNavigation::new(root, SESSION).expect("有効な session 名")
    }

    /// 印・案内・引継ぎは per-user ファイルとして書かれ、空値は何も書かず、消去は冪等である。
    #[test]
    fn stamps_offers_and_handoffs_are_written_trimmed_and_cleared_idempotently() {
        let root = tempfile::tempdir().expect("一時ディレクトリ");
        let navigation = navigation(root.path());
        let sessions = root.path().join("aidlc/.aidlc-sessions");

        navigation.write_stamp("").expect("空の印は無視");
        assert!(!sessions.join(SESSION).exists());
        navigation.clear_stamp().expect("無い印の消去は成功");
        navigation.write_stamp("uuid-a").expect("印");
        assert_eq!(navigation.stamp().as_deref(), Some("uuid-a"));
        navigation.clear_stamp().expect("印の消去");
        assert_eq!(navigation.stamp(), None);

        navigation.write_offer("").expect("空の案内は無視");
        assert_eq!(navigation.offer(), None);
        navigation
            .write_offer("default/a->default/b")
            .expect("案内");
        assert_eq!(
            fs::read_to_string(sessions.join(format!("{SESSION}.rebind-offer"))).expect("読める"),
            "default/a->default/b\n"
        );
        assert_eq!(navigation.offer().as_deref(), Some("default/a->default/b"));
        navigation.clear_offer().expect("案内の消去");
        navigation.clear_offer().expect("2 度目も成功");
        assert_eq!(navigation.offer(), None);

        navigation
            .write_handoff("same", "same")
            .expect("同一 intent の引継ぎは書かない");
        navigation
            .write_handoff("", "to")
            .expect("空の from は書かない");
        assert!(!sessions.join(format!("{SESSION}.handoff.json")).exists());
        navigation.write_handoff("from", "to").expect("引継ぎ");
        let handoff: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(sessions.join(format!("{SESSION}.handoff.json"))).expect("読める"),
        )
        .expect("JSON");
        assert_eq!(
            handoff.get("fromIntentUuid").and_then(|v| v.as_str()),
            Some("from")
        );
        assert_eq!(
            handoff.get("toIntentUuid").and_then(|v| v.as_str()),
            Some("to")
        );
    }

    /// 印の置き場がディレクトリで塞がれていれば、消去は NotFound 以外の失敗を隠さない。
    #[test]
    fn clearing_a_stamp_that_is_a_directory_surfaces_the_error() {
        let root = tempfile::tempdir().expect("一時ディレクトリ");
        let navigation = navigation(root.path());
        fs::create_dir_all(root.path().join("aidlc/.aidlc-sessions").join(SESSION))
            .expect("同名ディレクトリ");
        assert!(navigation.clear_stamp().is_err());
    }

    /// transcript は per-session と current の 2 か所へ写す。置き場が作れない・書けない
    /// ときは黙って諦める（承認権限ではないため）。
    #[test]
    fn transcripts_are_mirrored_and_write_failures_are_swallowed() {
        let root = tempfile::tempdir().expect("一時ディレクトリ");
        let sessions = root.path().join("aidlc/.aidlc-sessions");
        SessionNavigation::write_transcript(root.path(), SESSION, "");
        assert!(!sessions.exists(), "空の transcript は何も書かない");
        SessionNavigation::write_transcript(root.path(), SESSION, "/tmp/t.jsonl");
        assert_eq!(
            fs::read_to_string(sessions.join(format!("{SESSION}.transcript"))).expect("読める"),
            "/tmp/t.jsonl"
        );
        assert_eq!(
            fs::read_to_string(sessions.join("current.transcript")).expect("読める"),
            "/tmp/t.jsonl"
        );
        // per-session の置き場がディレクトリなら current も更新しない。
        fs::remove_file(sessions.join(format!("{SESSION}.transcript"))).expect("消す");
        fs::create_dir(sessions.join(format!("{SESSION}.transcript"))).expect("塞ぐ");
        SessionNavigation::write_transcript(root.path(), SESSION, "/tmp/other.jsonl");
        assert_eq!(
            fs::read_to_string(sessions.join("current.transcript")).expect("読める"),
            "/tmp/t.jsonl"
        );
        // 置き場そのものが作れなければ何も起きない。
        let blocked = tempfile::tempdir().expect("一時ディレクトリ");
        fs::write(blocked.path().join("aidlc"), "not a directory\n").expect("塞ぐ");
        SessionNavigation::write_transcript(blocked.path(), SESSION, "/tmp/t.jsonl");
        assert!(blocked.path().join("aidlc").is_file());
    }

    /// bootstrap は active-space を 1 度だけ据え、規則スタブの `@` 参照を選択中の space へ
    /// 書き換える。形の違う行は触らない。
    #[test]
    fn bootstrap_rewrites_the_rules_stub_to_the_selected_space() {
        let root = tempfile::tempdir().expect("一時ディレクトリ");
        fs::create_dir_all(root.path().join("aidlc")).expect("aidlc");
        fs::write(root.path().join("aidlc/active-space"), "team-b\n").expect("space カーソル");
        fs::create_dir_all(root.path().join(".claude/rules")).expect("rules");
        let stub = root.path().join(".claude/rules/aidlc.md");
        fs::write(
            &stub,
            "@../../aidlc/spaces/default/memory/org.md\n@../../aidlc/spaces/default/memory/phases/ideation.md\n@../../docs/other.md\n@aidlc/spaces//memory/team.md\nplain line\n",
        )
        .expect("stub");
        let layout = Layout::resolve(root.path());
        assert_eq!(layout.space(), "team-b");
        SessionNavigation::bootstrap(&layout);
        assert_eq!(
            fs::read_to_string(&stub).expect("読める"),
            "@../../aidlc/spaces/team-b/memory/org.md\n@../../aidlc/spaces/team-b/memory/phases/ideation.md\n@../../docs/other.md\n@aidlc/spaces//memory/team.md\nplain line\n"
        );
        assert_eq!(
            fs::read_to_string(root.path().join("aidlc/active-space")).expect("読める"),
            "team-b\n",
            "既存の space カーソルは上書きしない"
        );
        // 変更が無ければ書き直さない（同じ内容のまま）。
        SessionNavigation::bootstrap(&layout);
        assert!(
            fs::read_to_string(&stub)
                .expect("読める")
                .contains("team-b")
        );
    }
}
