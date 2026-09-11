//! `ReadModelUpdater` の投影書込先 4 面。

use std::path::{Path, PathBuf};

/// 投影の書込先 4 面の場所。
///
/// 生の `PathBuf` をばらばらに引き回さないための束である — 片方だけ差し替わった
/// 取り合わせ（別 intent の状態ファイルと別 clone のシャード）を構成できなくする。
///
/// # メモリ層の 2 ファイル（b49）
///
/// `team.md` / `project.md` は**人が編集する正本**でもあるが、`PracticesAffirmed` の投影が
/// 節を置き換え・規則行を足す面でもある（設計 §5）。パスは active-space の memory
/// ディレクトリから導くので、束を組む側は**ディレクトリ 1 本**を渡す — 2 本を別々に渡せる
/// ようにすると、別の space の team.md と project.md を取り合わせられてしまう。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectionTargets {
    /// 記録が置かれたワークスペース根 (状態ファイルの配置から導く。導けない配置では `None`)。
    project_dir: Option<PathBuf>,
    state_file: PathBuf,
    active_directive_file: PathBuf,
    description_file: PathBuf,
    registry_file: PathBuf,
    human_turn_file: PathBuf,
    audit_shard: PathBuf,
    team_md: PathBuf,
    project_md: PathBuf,
}

impl ProjectionTargets {
    /// 開始時の比較基準。公開契約が定めるcode-generation配下だけを所有する。
    pub(super) fn source_baseline_file(&self, name: &str) -> PathBuf {
        self.state_file
            .with_file_name(".aidlc-source-review")
            .join("code-generation")
            .join(name)
    }
    /// 可変個数の基準ファイルも、内容とアドレスを照合して同じ記録へ束縛する。
    pub(super) fn owns_source_baseline(&self, path: &Path, after: &[u8]) -> bool {
        std::str::from_utf8(after)
            .ok()
            .and_then(|text| {
                core_command_domain::orchestration::SourceBaseline::new(Some(text.to_string())).ok()
            })
            .and_then(|baseline| baseline.snapshot_name())
            .is_some_and(|name| self.source_baseline_file(&name) == path)
    }

    /// 成果物・セッション監査が同じbatchへ積むrecord相対識別子。
    pub(super) fn audit_target(&self) -> Option<String> {
        let mut it = self.audit_shard.components().peekable();
        while let Some(c) = it.next() {
            if c.as_os_str() == "spaces" {
                let space = it.next()?.as_os_str().to_str()?;
                if it.next()?.as_os_str() != "intents" {
                    return None;
                }
                let record = it.next()?.as_os_str().to_str()?;
                return Some(format!("spaces/{space}/intents/{record}"));
            }
        }
        None
    }
    /// 状態ファイル・監査シャード・memory ディレクトリから組む（**唯一の構築経路**）。
    #[must_use]
    pub fn new(
        state_file: impl Into<PathBuf>,
        audit_shard: impl Into<PathBuf>,
        memory_dir: impl Into<PathBuf>,
    ) -> ProjectionTargets {
        let memory_dir = memory_dir.into();
        let state_file = state_file.into();
        let description_file = state_file.with_file_name("project-description.json");
        let registry_file = state_file
            .parent()
            .and_then(Path::parent)
            .unwrap_or(Path::new("."))
            .join("intents.json");
        ProjectionTargets {
            project_dir: project_dir_of(&state_file),
            human_turn_file: state_file.with_file_name(".aidlc-human-turn"),
            active_directive_file: state_file.with_file_name(".aidlc-active-directive.json"),
            state_file,
            description_file,
            registry_file,
            audit_shard: audit_shard.into(),
            team_md: memory_dir.join("team.md"),
            project_md: memory_dir.join("project.md"),
        }
    }

    /// 束縛ダイジェストと所有検査が共有する、順序付きの対象一覧。
    pub(super) fn owned_paths(&self) -> [&Path; 8] {
        [
            self.state_file(),
            self.active_directive_file(),
            self.description_file(),
            self.registry_file(),
            self.human_turn_file(),
            self.audit_shard(),
            self.project_md(),
            self.team_md(),
        ]
    }

    /// 所有対象を損失なく記録した束縛。ファイル変更がない計画にも同じ値を用いる。
    pub(super) fn binding(&self) -> Result<String, super::CatchUpError> {
        let mut paths = Vec::new();
        for path in self.owned_paths() {
            paths.push(core_infrastructure::canon_json::JsonValue::String(
                path.to_str()
                    .ok_or_else(|| super::CatchUpError::PublicationConflict {
                        path: path.to_path_buf(),
                    })?
                    .to_string(),
            ));
        }
        Ok(core_infrastructure::canon_json::hash_compact(
            &core_infrastructure::canon_json::JsonValue::Array(paths),
        )
        .rendered())
    }

    /// 指示発行記録の投影先。
    #[must_use]
    pub fn active_directive_file(&self) -> &Path {
        &self.active_directive_file
    }

    /// 人間応答の会話マーカー。承認権限の正本にはしない。
    #[must_use]
    pub fn human_turn_file(&self) -> &Path {
        &self.human_turn_file
    }

    /// 同じspaceの共有intent登録の投影先。
    #[must_use]
    pub fn registry_file(&self) -> &Path {
        &self.registry_file
    }

    /// 元の依頼文を保存する、同じ記録配下の投影先。
    #[must_use]
    pub fn description_file(&self) -> &Path {
        &self.description_file
    }

    /// 状態ファイル（`aidlc-state.md`）の場所。
    #[must_use]
    pub fn state_file(&self) -> &Path {
        &self.state_file
    }

    /// 記録が置かれたワークスペース根（導けない配置では `None`）。
    #[must_use]
    pub fn project_dir(&self) -> Option<&Path> {
        self.project_dir.as_deref()
    }

    /// この記録の監査値に掛ける `<project-dir>` の伏せ字規則。
    ///
    /// upstream `renderAuditBlock` は呼出側から project dir を受け取るが、本 build の投影は
    /// 書込先の束だけを知るので、状態ファイルの配置
    /// (`<project>/aidlc/spaces/<space>/intents/<record>/aidlc-state.md`) から根を導く。
    /// 導けない配置 (単体テストの仮の場所) では何も伏せない。
    #[must_use]
    pub fn audit_redaction(&self) -> crate::workspace::AuditRedaction {
        self.project_dir.as_deref().map_or_else(
            crate::workspace::AuditRedaction::none,
            crate::workspace::AuditRedaction::for_project_dir,
        )
    }

    /// 監査シャード（`<record>/audit/<host>-<clone>.md`）の場所。
    #[must_use]
    pub fn audit_shard(&self) -> &Path {
        &self.audit_shard
    }

    /// メモリ層のチーム規則（`<memory>/team.md`）の場所。
    #[must_use]
    pub fn team_md(&self) -> &Path {
        &self.team_md
    }

    /// メモリ層のプロジェクト規則（`<memory>/project.md`）の場所。
    #[must_use]
    pub fn project_md(&self) -> &Path {
        &self.project_md
    }
}

/// 状態ファイルの配置 `<project>/aidlc/spaces/<space>/intents/<record>/aidlc-state.md` から
/// ワークスペース根を導く。形が違えば `None`。相対配置 (`aidlc/spaces/…`) の根は `.`。
fn project_dir_of(state_file: &Path) -> Option<PathBuf> {
    let record = state_file.parent()?;
    let intents = record.parent()?;
    if intents.file_name()? != "intents" {
        return None;
    }
    let space = intents.parent()?;
    let spaces = space.parent()?;
    if spaces.file_name()? != "spaces" {
        return None;
    }
    let aidlc = spaces.parent()?;
    if aidlc.file_name()? != "aidlc" {
        return None;
    }
    let project = aidlc.parent()?;
    Some(if project.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        project.to_path_buf()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_project_dir_is_derived_from_the_record_layout() {
        let targets = ProjectionTargets::new(
            "/w/aidlc/spaces/default/intents/260910-x/aidlc-state.md",
            "/w/aidlc/spaces/default/intents/260910-x/audit/host-abcd1234.md",
            "/w/aidlc/spaces/default/memory",
        );
        assert_eq!(targets.project_dir(), Some(Path::new("/w")));
        let relative = ProjectionTargets::new(
            "aidlc/spaces/default/intents/260910-x/aidlc-state.md",
            "aidlc/spaces/default/intents/260910-x/audit/host-abcd1234.md",
            "aidlc/spaces/default/memory",
        );
        assert_eq!(relative.project_dir(), Some(Path::new(".")));
    }

    #[test]
    fn a_layout_that_is_not_a_record_has_no_project_dir_and_redacts_nothing() {
        let targets = ProjectionTargets::new(
            "/w/aidlc-state.md",
            "/w/audit/host-abcd1234.md",
            "/w/memory",
        );
        assert_eq!(targets.project_dir(), None);
        assert_eq!(targets.audit_redaction().redact("/w/file"), "/w/file");
    }

    #[test]
    fn the_targets_keep_every_path_together() {
        let targets = ProjectionTargets::new(
            "/w/aidlc-state.md",
            "/w/audit/host-abcd1234.md",
            "/w/memory",
        );
        assert_eq!(targets.state_file(), Path::new("/w/aidlc-state.md"));
        assert_eq!(
            targets.audit_shard(),
            Path::new("/w/audit/host-abcd1234.md")
        );
        assert_eq!(targets.team_md(), Path::new("/w/memory/team.md"));
        assert_eq!(targets.project_md(), Path::new("/w/memory/project.md"));
        assert_eq!(
            targets,
            ProjectionTargets::new(
                "/w/aidlc-state.md",
                "/w/audit/host-abcd1234.md",
                "/w/memory",
            )
        );
    }
}
