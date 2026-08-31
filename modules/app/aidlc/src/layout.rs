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
const DEFAULT_SPACE: &str = "default";

/// ワークスペース根から導いた配置。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Layout {
    project_dir: PathBuf,
    space: String,
    record_dir: Option<PathBuf>,
}

impl Layout {
    /// カーソルを読んで配置を決める。
    ///
    /// active-intent カーソルが無ければ `record_dir` は `None` になる — intent がまだ
    /// 生まれていない fresh なワークスペースの正常な姿である。
    #[must_use]
    pub fn resolve(project_dir: &Path) -> Layout {
        let aidlc = project_dir.join("aidlc");
        let space =
            read_cursor(&aidlc.join("active-space")).unwrap_or_else(|| DEFAULT_SPACE.into());
        let intents = aidlc.join("spaces").join(&space).join("intents");
        let record_dir = read_cursor(&intents.join("active-intent")).map(|slug| intents.join(slug));
        Layout {
            project_dir: project_dir.to_path_buf(),
            space,
            record_dir,
        }
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
    /// 空のカーソルを見る — [`read_cursor`] は空を「無い」と読むので、鋳造直後の `next` が
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

    #[test]
    fn the_active_intent_cursor_names_the_record_directory() {
        let root = workspace();
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
}
