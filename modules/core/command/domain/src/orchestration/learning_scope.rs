//! `LearningScope` — 確定した学びが実践として落ちるメモリ層の層。

/// 学びの書込先の層 (`project.md` / `team.md`)。
///
/// org 層への昇格路は無い (`stage-protocol.md` §13「no org tier」)。綴りは選択ファイルの
/// `scope` と同じ逐語である。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LearningScope {
    /// 既定の層 — `aidlc/spaces/<space>/memory/project.md`。
    Project,
    /// 人が「チームへ広げる」を選んだ層 — `aidlc/spaces/<space>/memory/team.md`。
    Team,
}

impl LearningScope {
    /// 選択ファイルの綴りから読む。`team` 以外はすべて既定の `project` になる
    /// (本家 `raw.scope === "team" ? "team" : "project"` と同じ倒し方)。
    #[must_use]
    pub fn of_spelling(spelling: &str) -> LearningScope {
        if spelling == "team" {
            LearningScope::Team
        } else {
            LearningScope::Project
        }
    }

    /// 選択ファイルと同じ逐語。
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            LearningScope::Project => "project",
            LearningScope::Team => "team",
        }
    }

    /// メモリ層のファイル名。
    #[must_use]
    pub const fn method_file(&self) -> &'static str {
        match self {
            LearningScope::Project => "project.md",
            LearningScope::Team => "team.md",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::LearningScope;

    #[test]
    fn only_the_exact_team_spelling_widens_the_scope() {
        assert_eq!(LearningScope::of_spelling("team"), LearningScope::Team);
        assert_eq!(
            LearningScope::of_spelling("project"),
            LearningScope::Project
        );
        assert_eq!(LearningScope::of_spelling("Team"), LearningScope::Project);
        assert_eq!(LearningScope::of_spelling("org"), LearningScope::Project);
    }

    #[test]
    fn each_scope_names_its_method_file() {
        assert_eq!(LearningScope::Project.method_file(), "project.md");
        assert_eq!(LearningScope::Team.method_file(), "team.md");
        assert_eq!(LearningScope::Project.as_str(), "project");
        assert_eq!(LearningScope::Team.as_str(), "team");
    }
}
