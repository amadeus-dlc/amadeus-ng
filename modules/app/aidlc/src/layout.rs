//! ワークスペースの配置解決 — どのファイルがどこにあるかを 1 か所で決める。
//!
//! upstream `aidlc-lib.ts` の `activeSpace` / `activeIntent` / `recordDir` /
//! `stateFilePath` に対応する。**カーソルを読むのは合成ルートの仕事**である — record の
//! 所在はマシンローカルな navigation であって、どちらの側のドメインでもない。
//!
//! # カーソルは per-user で、コミットされない
//!
//! `aidlc/active-space` と `aidlc/spaces/<space>/intents/active-intent` は `.gitignore`
//! 済みである（同僚が別の intent を指していてよいので、共有された状態にしてはならない）。
//! したがって**どちらも無いのが正常な状態**であり、読めなければ既定へ倒す。

use core_infrastructure::atomic::write_file_atomic;
use std::fs;
use std::path::{Path, PathBuf};

/// 既定の space 名（ディスクに何も無くても常に有効な特例 — 11 §2.1）。
pub(crate) const DEFAULT_SPACE: &str = "default";

/// ワークスペース根から導いた配置。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Layout {
    project_dir: PathBuf,
    space: String,
    record_dir: Option<PathBuf>,
}

impl Layout {
    /// この会話が新しいintentへ移った直後か。per-user navigationの照合だけを行う。
    #[must_use]
    pub fn has_current_handoff(&self, session: &str, at: chrono::DateTime<chrono::Utc>) -> bool {
        let Some(handoff) = SessionHandoff::read(self, session) else {
            return false;
        };
        if !handoff.is_fresh(at) {
            return false;
        }
        let stamp =
            fs::read_to_string(self.aidlc_root().join(".aidlc-sessions").join(session)).ok();
        if stamp.as_deref().map(core_infrastructure::ecmascript::trim)
            != Some(handoff.to_intent.as_str())
        {
            return false;
        }
        let Some(record) = self
            .record_dir()
            .and_then(Path::file_name)
            .and_then(|name| name.to_str())
        else {
            return false;
        };
        let Some(registry) = fs::read(self.intents_dir().join("intents.json"))
            .ok()
            .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
        else {
            return false;
        };
        registry.as_array().is_some_and(|entries| {
            entries.iter().any(|entry| {
                entry.get("uuid").and_then(serde_json::Value::as_str)
                    == Some(handoff.to_intent.as_str())
                    && entry.get("dirName").and_then(serde_json::Value::as_str) == Some(record)
            })
        })
    }

    /// 一回限りの会話切替印を消費する。状態・監査・承認の正本は変更しない。
    /// # Errors
    /// 印が存在するが削除できない場合。
    pub fn clear_handoff(&self, session: &str) -> std::io::Result<()> {
        if !Self::valid_session_id(session) {
            return Ok(());
        }
        match fs::remove_file(
            self.aidlc_root()
                .join(".aidlc-sessions")
                .join(format!("{session}.handoff.json")),
        ) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            result => result,
        }
    }

    /// 古い・未来時刻の切替印だけを清掃する。読めない印は勝手に解釈しない。
    /// # Errors
    /// 期限外の印を削除できない場合。
    pub fn discard_expired_handoff(
        &self,
        session: &str,
        at: chrono::DateTime<chrono::Utc>,
    ) -> std::io::Result<()> {
        if SessionHandoff::read(self, session).is_some_and(|handoff| !handoff.is_fresh(at)) {
            self.clear_handoff(session)
        } else {
            Ok(())
        }
    }
    const fn new(project_dir: PathBuf, space: String, record_dir: Option<PathBuf>) -> Self {
        Self {
            project_dir,
            space,
            record_dir,
        }
    }
    /// 特定の記録を参照する配置。利用者のactive-intentは更新しない。
    #[must_use]
    pub fn for_record(
        project_dir: &Path,
        space: &core_command_domain::workspace::SpaceName,
        record: &core_command_domain::workspace::IntentDirName,
    ) -> Self {
        Self::new(
            project_dir.to_path_buf(),
            space.as_str().to_string(),
            Some(
                project_dir
                    .join("aidlc/spaces")
                    .join(space.as_str())
                    .join("intents")
                    .join(record.as_str()),
            ),
        )
    }

    /// カーソルを読んで配置を決める。
    ///
    /// active-intent カーソルが実在する記録を名指さず、記録も 1 つに定まらなければ
    /// `record_dir` は `None` になる — intent がまだ生まれていない fresh なワークスペースの
    /// 正常な姿である (選び方は [`Layout::shared`])。
    #[must_use]
    pub fn resolve(project_dir: &Path) -> Layout {
        Self::resolve_for_session(project_dir, None)
    }

    /// payloadのsessionを優先し、その会話に固定された配置を読む。共有カーソルは書き換えない。
    #[must_use]
    pub fn resolve_for_session(project_dir: &Path, payload_session: Option<&str>) -> Layout {
        let environment = if payload_session.is_some_and(Self::valid_session_id) {
            None
        } else {
            crate::session_navigation::SessionNavigation::current_id(project_dir)
        };
        let session = payload_session
            .filter(|session| Self::valid_session_id(session))
            .or_else(|| {
                environment
                    .as_deref()
                    .filter(|session| Self::valid_session_id(session))
            });
        if let Some(binding) =
            session.and_then(|session| Self::bound_selection(project_dir, session))
        {
            return binding;
        }
        Self::shared(project_dir)
    }

    /// セッション上書きを適用しない共有navigationの観測。
    ///
    /// 記録の選び方は upstream `activeIntent` (`aidlc-lib.ts:1633-1653`) の順である —
    /// カーソルが**実在する記録**を名指すならそれ、さもなくば記録が**ちょうど 1 つ**ならそれ
    /// (lone intent)、0 か 2 つ以上なら無し (推測しない。どれを指すかは動詞側が人間に問う)。
    /// 実在しない記録を指す古いカーソルは無視される (固定 2.7.1 の実走行 case 79 `dangling`)。
    ///
    /// 「実在する」は upstream どおり `aidlc-state.md` の有無で判定する
    /// (`existsSync(join(dir, raw, "aidlc-state.md"))`、裁定 F-H1 = B)。状態ファイルを失った
    /// 記録はカーソルが名指していても選ばれず、唯一記録の数にも入らない (`listIntentDirs`)。
    /// したがって `runtime::catch_up` の `restore_missing_files` が `next` から届くのは、
    /// 記録が (状態ファイルを持って) 解決できたうえで監査シャードや memory の投影だけが
    /// 失われている場合に限られる。
    pub(crate) fn shared(project_dir: &Path) -> Self {
        let aidlc = project_dir.join("aidlc");
        let space =
            read_cursor(&aidlc.join("active-space")).unwrap_or_else(|| DEFAULT_SPACE.into());
        let intents = aidlc.join("spaces").join(&space).join("intents");
        let record_dir = read_cursor(&intents.join("active-intent"))
            .map(|name| intents.join(name))
            .filter(|record| is_record(record))
            .or_else(|| lone_record(&intents));
        Self::new(project_dir.to_path_buf(), space, record_dir)
    }

    pub(crate) fn valid_session_id(session: &str) -> bool {
        !session.is_empty()
            && session.len() <= 180
            && ![".", ".."].contains(&session)
            && !session.starts_with('-')
            && !session.ends_with('-')
            && session
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte))
    }

    pub(crate) fn bound_selection(project: &Path, session: &str) -> Option<Self> {
        use core_command_domain::workspace::{IntentDirName, SpaceName};
        let bytes = fs::read(
            project
                .join("aidlc/.aidlc-sessions")
                .join(format!("{session}.binding.json")),
        )
        .ok()?;
        let binding: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
        let space = SpaceName::parse(binding.get("space")?.as_str()?).ok()?;
        if binding.get("boundAt")?.as_str()?.is_empty() {
            return None;
        }
        if binding.get("intent")?.is_null() {
            return Some(Self::new(
                project.to_path_buf(),
                space.as_str().to_string(),
                None,
            ));
        }
        let record = IntentDirName::parse(binding.get("intent")?.as_str()?).ok()?;
        let selected = Self::for_record(project, &space, &record);
        selected.state_file()?.exists().then_some(selected)
    }

    /// ワークスペース根。
    #[must_use]
    pub fn project_dir(&self) -> &Path {
        &self.project_dir
    }

    /// `aidlc/` ディレクトリ。
    #[must_use]
    pub fn aidlc_root(&self) -> PathBuf {
        self.project_dir.join("aidlc")
    }

    /// 有効な space 名。
    #[must_use]
    pub fn space(&self) -> &str {
        &self.space
    }

    /// intent の記録ディレクトリ（カーソルが無ければ `None`）。
    #[must_use]
    pub fn record_dir(&self) -> Option<&Path> {
        self.record_dir.as_deref()
    }

    /// `aidlc/spaces/<space>/intents/`。
    #[must_use]
    pub fn intents_dir(&self) -> PathBuf {
        self.aidlc_root()
            .join("spaces")
            .join(&self.space)
            .join("intents")
    }

    /// 実行状態リードモデル `aidlc-state.md`（record が無ければ `None`）。
    #[must_use]
    pub fn state_file(&self) -> Option<PathBuf> {
        self.record_dir
            .as_ref()
            .map(|dir| dir.join("aidlc-state.md"))
    }

    /// 監査シャードの置き場 `<record>/audit/`（record が無ければ `None`）。
    #[must_use]
    pub fn audit_dir(&self) -> Option<PathBuf> {
        self.record_dir.as_ref().map(|dir| dir.join("audit"))
    }

    /// active-space の memory 層。
    #[must_use]
    pub fn memory_dir(&self) -> PathBuf {
        self.aidlc_root()
            .join("spaces")
            .join(&self.space)
            .join("memory")
    }

    /// `aidlc/spaces/<space>/codekb/<repo>` — リポジトリごとの durable な知識の置き場。
    ///
    /// intent の記録ディレクトリの**下ではない** — codekb はリポジトリ名で鍵付けられ、
    /// space の中の全 intent が共有する（memory / knowledge と同じ space 直下の兄弟）。
    #[must_use]
    pub fn codekb_dir(&self, repo: &str) -> PathBuf {
        self.aidlc_root()
            .join("spaces")
            .join(&self.space)
            .join("codekb")
            .join(repo)
    }

    /// [`Layout::codekb_dir`] のワークスペース相対形（区切りは常に `/`）。
    ///
    /// 出力に載る綴りなので、ホスト OS の区切りに依らず posix 形で組む。
    #[must_use]
    pub fn relative_codekb_dir(&self, repo: &str) -> String {
        format!("aidlc/spaces/{}/codekb/{repo}", self.space)
    }

    /// ハーネス根（`.claude` — 既定の 1 ハーネス）。
    ///
    /// upstream の `harnessDir()` は `.claude` / `.kiro` / `.codex` を配置から判別するが、
    /// b29 の範囲は claude 1 面なので固定である。多ハーネス化は後続 Bolt。
    #[must_use]
    pub fn harness_dir(&self) -> PathBuf {
        self.project_dir.join(".claude")
    }

    /// 定義 3 入力の置き場（`<harness>/tools/data/`）。
    #[must_use]
    pub fn definition_data_dir(&self) -> PathBuf {
        self.harness_dir().join("tools").join("data")
    }

    /// scope identity ファイルの置き場（`<harness>/scopes/`）。
    #[must_use]
    pub fn scopes_dir(&self) -> PathBuf {
        self.harness_dir().join("scopes")
    }

    /// ステージ本体ファイルの置き場（`<harness>/aidlc-common/stages`）。
    #[must_use]
    pub fn stage_library_dir(&self) -> PathBuf {
        self.harness_dir().join("aidlc-common").join("stages")
    }

    /// エージェントペルソナの置き場（`<harness>/agents`）。
    #[must_use]
    pub fn agent_dir(&self) -> PathBuf {
        self.harness_dir().join("agents")
    }

    /// active-intent カーソルを据える（intent 鋳造の直後に合成ルートが書く）。
    ///
    /// 書込は**不可分**である。`fs::write` は切り詰めてから書くので、その隙に読んだ側は
    /// 空のカーソルを見る — `read_cursor` は空を「無い」と読むので、鋳造直後の `next` が
    /// record を解決できず、追いつきが素通りする。tmp + rename なら読み手が見るのは
    /// 常に古い値か新しい値のどちらかである。
    ///
    /// # Errors
    ///
    /// ディレクトリを作れない・書けない場合の I/O エラー。
    pub fn point_at(&self, record_dir_name: &str) -> std::io::Result<()> {
        let intents = self.intents_dir();
        fs::create_dir_all(&intents)?;
        write_file_atomic(
            &intents.join("active-intent"),
            format!("{record_dir_name}\n").as_bytes(),
        )
    }
}

/// 検査済みのper-user切替先。fromはread時の非空・別intent検査にだけ使う。
struct SessionHandoff {
    to_intent: String,
    issued_at: f64,
}
impl SessionHandoff {
    const fn new(to_intent: String, issued_at: f64) -> Self {
        Self {
            to_intent,
            issued_at,
        }
    }
    fn read(layout: &Layout, session: &str) -> Option<Self> {
        if !Layout::valid_session_id(session) {
            return None;
        }
        let bytes = fs::read(
            layout
                .aidlc_root()
                .join(".aidlc-sessions")
                .join(format!("{session}.handoff.json")),
        )
        .ok()?;
        let value: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
        let from = value.get("fromIntentUuid")?.as_str()?;
        let to = value.get("toIntentUuid")?.as_str()?;
        let at = value.get("issuedAtMs")?.as_f64()?;
        if from.is_empty() || to.is_empty() || from == to || !at.is_finite() {
            return None;
        }
        Some(Self::new(to.to_string(), at))
    }
    fn is_fresh(&self, at: chrono::DateTime<chrono::Utc>) -> bool {
        serde_json::Number::from(at.timestamp_millis())
            .as_f64()
            .is_some_and(|now| self.issued_at <= now && now - self.issued_at <= 300_000.0)
    }
}

/// 記録ディレクトリか — `aidlc-state.md` を持つものだけが記録である (upstream
/// `listIntentDirs` / `activeIntent` の `existsSync(join(dir, name, "aidlc-state.md"))`)。
fn is_record(directory: &Path) -> bool {
    directory.join("aidlc-state.md").exists()
}

/// 記録がちょうど 1 つならその場所 (upstream の lone-intent 後退)。カーソル・registry・
/// 状態ファイルの無いディレクトリは数えない。
fn lone_record(intents: &Path) -> Option<PathBuf> {
    let mut records = fs::read_dir(intents)
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| is_record(path))
        .collect::<Vec<_>>();
    records.sort();
    (records.len() == 1).then(|| records.remove(0))
}

/// カーソルファイルを読む — 空白のみ・読めないは「無い」と同じに扱う。
fn read_cursor(path: &Path) -> Option<String> {
    let value = fs::read_to_string(path).ok()?;
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(trimmed.to_string())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::*;

    fn workspace() -> tempfile::TempDir {
        tempfile::tempdir().expect("一時ディレクトリ")
    }

    /// カーソルがどちらも無いのは fresh なワークスペースの正常な姿である。
    #[test]
    fn a_fresh_workspace_resolves_to_the_default_space_with_no_record() {
        let root = workspace();

        let layout = Layout::resolve(root.path());

        assert_eq!(layout.space(), "default");
        assert_eq!(layout.record_dir(), None);
        assert_eq!(layout.state_file(), None);
        assert_eq!(layout.audit_dir(), None);
    }

    /// `intents/<name>/aidlc-state.md` を持つ記録を 1 つ据える。
    fn record_with_state(root: &Path, name: &str) {
        let record = root.join("aidlc/spaces/default/intents").join(name);
        fs::create_dir_all(&record).expect("record");
        fs::write(record.join("aidlc-state.md"), "# AI-DLC State\n").expect("state");
    }

    #[test]
    fn the_active_intent_cursor_names_the_record_directory() {
        let root = workspace();
        record_with_state(root.path(), "260831-fix-crash-abcd1234");
        let layout = Layout::resolve(root.path());
        layout
            .point_at("260831-fix-crash-abcd1234")
            .expect("カーソル");

        let layout = Layout::resolve(root.path());

        let expected = root
            .path()
            .join("aidlc/spaces/default/intents/260831-fix-crash-abcd1234");
        assert_eq!(layout.record_dir(), Some(expected.as_path()));
        assert_eq!(layout.state_file(), Some(expected.join("aidlc-state.md")));
        assert_eq!(layout.audit_dir(), Some(expected.join("audit")));
    }

    #[test]
    fn the_active_space_cursor_selects_the_space() {
        let root = workspace();
        let aidlc = root.path().join("aidlc");
        fs::create_dir_all(&aidlc).expect("aidlc");
        fs::write(aidlc.join("active-space"), "team-b\n").expect("space カーソル");

        let layout = Layout::resolve(root.path());

        assert_eq!(layout.space(), "team-b");
        assert_eq!(
            layout.memory_dir(),
            root.path().join("aidlc/spaces/team-b/memory")
        );
    }

    /// 空白だけのカーソルは「無い」と同じ（書きかけのファイルで別の場所を指さない）。
    #[test]
    fn a_blank_cursor_is_treated_as_absent() {
        let root = workspace();
        let intents = root.path().join("aidlc/spaces/default/intents");
        fs::create_dir_all(&intents).expect("intents");
        fs::write(intents.join("active-intent"), "   \n").expect("空のカーソル");

        assert_eq!(Layout::resolve(root.path()).record_dir(), None);
    }

    /// カーソルは改行付きで書かれ、読み戻しで trim される（往復する）。
    #[test]
    fn the_cursor_round_trips_through_disk() {
        let root = workspace();
        record_with_state(root.path(), "260831-work-deadbeef");
        Layout::resolve(root.path())
            .point_at("260831-work-deadbeef")
            .expect("カーソル");

        let written = fs::read_to_string(
            root.path()
                .join("aidlc/spaces/default/intents/active-intent"),
        )
        .expect("読める");

        assert_eq!(written, "260831-work-deadbeef\n");
        assert_eq!(
            Layout::resolve(root.path())
                .record_dir()
                .and_then(|d| d.file_name())
                .and_then(|n| n.to_str()),
            Some("260831-work-deadbeef")
        );
    }

    /// upstream `activeIntent` (`aidlc-lib.ts:1633-1653`) の判定順: カーソルが**実在する記録**
    /// (`aidlc-state.md` を持つ) を名指すならそれ、さもなくば記録が**ちょうど 1 つ**ならそれ、
    /// それ以外は無し。固定 2.7.1 の実走行 (stage-rules closeout case 79 `dangling`) は、
    /// 実在しない `gone` を指すカーソルを無視して唯一の記録 `rec-0001` を選んだ。
    #[test]
    fn a_cursor_that_names_an_absent_record_falls_back_to_the_lone_record() {
        let root = workspace();
        record_with_state(root.path(), "rec-0001");
        let intents = root.path().join("aidlc/spaces/default/intents");
        fs::write(intents.join("active-intent"), "gone\n").expect("cursor");

        assert_eq!(
            Layout::resolve(root.path()).record_dir(),
            Some(intents.join("rec-0001").as_path())
        );
    }

    #[test]
    fn without_a_cursor_a_lone_record_is_selected_and_two_records_are_not() {
        let root = workspace();
        record_with_state(root.path(), "rec-0001");
        let intents = root.path().join("aidlc/spaces/default/intents");
        // カーソル・registry・迷子のファイルは記録ではない。
        fs::write(intents.join("intents.json"), "[]\n").expect("registry");
        fs::create_dir_all(intents.join("no-state-here")).expect("stray dir");
        assert_eq!(
            Layout::resolve(root.path()).record_dir(),
            Some(intents.join("rec-0001").as_path())
        );

        record_with_state(root.path(), "rec-0002");
        assert_eq!(
            Layout::resolve(root.path()).record_dir(),
            None,
            "2 つ以上でカーソルが無ければ推測しない (upstream は handler 層が問う)"
        );
    }

    #[test]
    fn a_cursor_naming_a_real_record_wins_over_the_lone_intent_fallback() {
        let root = workspace();
        record_with_state(root.path(), "rec-0001");
        record_with_state(root.path(), "rec-0002");
        let intents = root.path().join("aidlc/spaces/default/intents");
        fs::write(intents.join("active-intent"), "rec-0002\n").expect("cursor");

        assert_eq!(
            Layout::resolve(root.path()).record_dir(),
            Some(intents.join("rec-0002").as_path())
        );
    }

    /// カーソルが名指すディレクトリに `aidlc-state.md` が無ければ記録ではない — upstream
    /// `activeIntent` の `existsSync(join(dir, raw, "aidlc-state.md"))` どおり、カーソルは
    /// 無視され、唯一記録の後退にも数えない (`listIntentDirs` も状態ファイルで数える)。
    /// 裁定 F-H1 = B (`lone-intent-fallback-questions.md`)。
    #[test]
    fn a_cursor_naming_a_directory_without_a_state_file_is_not_a_record() {
        let root = workspace();
        let intents = root.path().join("aidlc/spaces/default/intents");
        fs::create_dir_all(intents.join("half-made")).expect("dir");
        fs::write(intents.join("active-intent"), "half-made\n").expect("cursor");

        assert_eq!(Layout::resolve(root.path()).record_dir(), None);

        // 状態ファイルを持つ記録が他に 1 つあれば、カーソルを無視してそちらへ後退する。
        record_with_state(root.path(), "rec-0001");
        assert_eq!(
            Layout::resolve(root.path()).record_dir(),
            Some(intents.join("rec-0001").as_path())
        );
    }

    const SESSION: &str = "11111111-2222-4333-8444-555555555555";

    fn handoff_workspace() -> (tempfile::TempDir, PathBuf) {
        let root = tempfile::tempdir().expect("一時ディレクトリ");
        let intents = root.path().join("aidlc/spaces/default/intents");
        fs::create_dir_all(intents.join("260101-target-aaaaaaaa")).expect("記録");
        fs::write(
            intents.join("260101-target-aaaaaaaa/aidlc-state.md"),
            "# state\n",
        )
        .expect("状態ファイル");
        fs::write(intents.join("active-intent"), "260101-target-aaaaaaaa\n").expect("カーソル");
        fs::write(
            intents.join("intents.json"),
            r#"[{"uuid":"to-uuid","dirName":"260101-target-aaaaaaaa","slug":"target"}]"#,
        )
        .expect("registry");
        let sessions = root.path().join("aidlc/.aidlc-sessions");
        fs::create_dir_all(&sessions).expect("sessions");
        (root, sessions)
    }

    fn write_handoff(sessions: &Path, from: &str, to: &str, issued_at_ms: &str) {
        fs::write(
            sessions.join(format!("{SESSION}.handoff.json")),
            format!(
                r#"{{"fromIntentUuid":"{from}","toIntentUuid":"{to}","issuedAtMs":{issued_at_ms}}}"#
            ),
        )
        .expect("切替印");
    }

    /// 切替印は 5 分以内・印の uuid 一致・registry の行一致が揃って初めて「直後」である。
    /// 壊れた印（同一 intent、空欄、非数）は読めないものとして扱う。
    #[test]
    fn a_handoff_counts_only_when_fresh_stamped_and_registered() {
        let (root, sessions) = handoff_workspace();
        let layout = Layout::resolve(root.path());
        let now = chrono::Utc::now();
        let issued = now.timestamp_millis().to_string();
        assert!(!layout.has_current_handoff(SESSION, now), "印が無い");
        write_handoff(&sessions, "from-uuid", "to-uuid", &issued);
        assert!(
            !layout.has_current_handoff(SESSION, now),
            "session の印（stamp）が別なら切替ではない"
        );
        fs::write(sessions.join(SESSION), "to-uuid\n").expect("stamp");
        assert!(layout.has_current_handoff(SESSION, now));
        assert!(
            !layout.has_current_handoff("bad session", now),
            "文法外の session は読まない"
        );
        // 期限切れ（5 分より前）と未来の印は新鮮ではない。
        let stale = (now.timestamp_millis() - 301_000).to_string();
        write_handoff(&sessions, "from-uuid", "to-uuid", &stale);
        assert!(!layout.has_current_handoff(SESSION, now));
        let future = (now.timestamp_millis() + 1_000).to_string();
        write_handoff(&sessions, "from-uuid", "to-uuid", &future);
        assert!(!layout.has_current_handoff(SESSION, now));
        // 壊れた印。
        write_handoff(&sessions, "same", "same", &issued);
        assert!(!layout.has_current_handoff(SESSION, now));
        write_handoff(&sessions, "from-uuid", "to-uuid", "\"soon\"");
        assert!(!layout.has_current_handoff(SESSION, now));
        // registry が読めなければ照合できない。
        write_handoff(&sessions, "from-uuid", "to-uuid", &issued);
        let registry = root
            .path()
            .join("aidlc/spaces/default/intents/intents.json");
        fs::write(&registry, "{broken").expect("壊す");
        assert!(!layout.has_current_handoff(SESSION, now));
        fs::remove_file(&registry).expect("消す");
        assert!(!layout.has_current_handoff(SESSION, now));
        // 記録が無い配置では照合の対象が無い。
        let bare = Layout::new(root.path().to_path_buf(), "default".into(), None);
        assert!(!bare.has_current_handoff(SESSION, now));
    }

    /// 期限外の印だけを清掃し、新鮮な印と読めない印は残す。消費は文法外の session を無視する。
    #[test]
    fn expired_handoffs_are_discarded_and_fresh_ones_are_kept() {
        let (root, sessions) = handoff_workspace();
        let layout = Layout::resolve(root.path());
        let now = chrono::Utc::now();
        let marker = sessions.join(format!("{SESSION}.handoff.json"));
        layout.clear_handoff(SESSION).expect("無い印の消費は成功");
        layout.clear_handoff("bad session").expect("文法外は無視");
        write_handoff(
            &sessions,
            "from-uuid",
            "to-uuid",
            &now.timestamp_millis().to_string(),
        );
        layout
            .discard_expired_handoff(SESSION, now)
            .expect("新鮮な印は残す");
        assert!(marker.exists());
        write_handoff(
            &sessions,
            "from-uuid",
            "to-uuid",
            &(now.timestamp_millis() - 600_000).to_string(),
        );
        layout
            .discard_expired_handoff(SESSION, now)
            .expect("期限切れは消す");
        assert!(!marker.exists());
        fs::write(&marker, "not json").expect("壊れた印");
        layout
            .discard_expired_handoff(SESSION, now)
            .expect("読めない印は触らない");
        assert!(marker.exists());
        // 印がディレクトリなら消費は失敗を隠さない。
        fs::remove_file(&marker).expect("消す");
        fs::create_dir(&marker).expect("塞ぐ");
        assert!(layout.clear_handoff(SESSION).is_err());
    }

    /// 会話に固定された配置は、`boundAt` が空なら無効で、`intent` が null なら space だけの
    /// 配置になる。
    #[test]
    fn a_session_binding_needs_a_bound_at_and_may_select_only_a_space() {
        let (root, sessions) = handoff_workspace();
        let binding = sessions.join(format!("{SESSION}.binding.json"));
        fs::write(
            &binding,
            r#"{"space":"default","intent":"260101-target-aaaaaaaa","boundAt":""}"#,
        )
        .expect("binding");
        assert!(Layout::bound_selection(root.path(), SESSION).is_none());
        fs::write(
            &binding,
            r#"{"space":"default","intent":null,"boundAt":"2026-09-11T00:00:00Z"}"#,
        )
        .expect("binding");
        let bound = Layout::bound_selection(root.path(), SESSION).expect("space だけの固定");
        assert_eq!(bound.space(), "default");
        assert_eq!(bound.record_dir(), None);
        fs::write(
            &binding,
            r#"{"space":"default","intent":"260101-target-aaaaaaaa","boundAt":"2026-09-11T00:00:00Z"}"#,
        )
        .expect("binding");
        let bound = Layout::bound_selection(root.path(), SESSION).expect("記録の固定");
        assert!(bound.record_dir().is_some());
        assert_eq!(
            Layout::resolve_for_session(root.path(), Some(SESSION)).record_dir(),
            bound.record_dir()
        );
        assert_eq!(
            bound.stage_library_dir(),
            root.path().join(".claude/aidlc-common/stages")
        );
    }
}
