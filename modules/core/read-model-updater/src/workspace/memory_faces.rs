//! `MemoryFaces` — 投影が読み書きするメモリ層 2 ファイルの**メモリ上の姿**。

/// メモリ層の 2 ファイル（`team.md` / `project.md`）の本文と、この回に書き替えたか。
///
/// 状態ファイルと同じ**置換**面である（現在値そのものを持つので、投影は読んで書き換える）。
/// 監査シャードのような追記面ではない。
///
/// `dirty` を持つのは、**触っていないなら 1 バイトも書かない**ためである — メモリ層は人が
/// 編集する正本でもあるので、`PracticesAffirmed` を含まない更新が mtime を
/// 動かすことがあってはならない（設計 §5）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryFaces {
    team: String,
    project: String,
    dirty: bool,
}

impl MemoryFaces {
    const fn of_contents(team: String, project: String, dirty: bool) -> Self {
        Self {
            team,
            project,
            dirty,
        }
    }

    /// 読み込んだ 2 本の本文から作る（**この型の唯一の構築経路**。まだ書き替えていない）。
    #[must_use]
    pub fn new(team: impl Into<String>, project: impl Into<String>) -> MemoryFaces {
        Self::of_contents(team.into(), project.into(), false)
    }

    /// `team.md` の現在の本文。
    #[must_use]
    pub fn team(&self) -> &str {
        &self.team
    }

    /// `project.md` の現在の本文。
    #[must_use]
    pub fn project(&self) -> &str {
        &self.project
    }

    /// この回に書き替えたか（偽なら書き戻す必要が無い）。
    #[must_use]
    pub const fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// 描画後の本文を持つ完成値を作る。新しい値はdirtyになる。
    pub(crate) const fn rewritten(team: String, project: String) -> Self {
        Self::of_contents(team, project, true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_freshly_read_pair_is_not_dirty() {
        let faces = MemoryFaces::new("# Team\n", "# Project\n");
        assert_eq!(faces.team(), "# Team\n");
        assert_eq!(faces.project(), "# Project\n");
        assert!(!faces.is_dirty());
    }

    #[test]
    fn a_replacement_marks_the_pair_dirty() {
        let faces = MemoryFaces::rewritten("# Team 2\n".to_string(), "# Project 2\n".to_string());
        assert_eq!(faces.team(), "# Team 2\n");
        assert_eq!(faces.project(), "# Project 2\n");
        assert!(faces.is_dirty());
    }
}
