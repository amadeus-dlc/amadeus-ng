//! `CodekbCandidate` — 公開を待つ codekb の候補 (9 成果物 + 走査範囲の主張)。

use core_infrastructure::collections::FirstClassCollection as _;

use super::codekb_artifacts::CodekbArtifacts;
use super::codekb_scope_path::CodekbScopePath;
use super::codekb_scope_paths::CodekbScopePaths;

/// staged された公開候補。
///
/// 鮮度印が主張する「深く読んだパス」と、そこに書かれた内容指紋を運ぶ。指紋は記録されて
/// いないことがある (`unknown` / 空欄) ので [`Option`] である — 「指紋が無い」と「指紋が
/// 違う」は別の観測であり、畳むと公開の可否を取り違える。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodekbCandidate {
    artifacts: CodekbArtifacts,
    analyzed_paths: CodekbScopePaths,
    recorded_fingerprint: Option<String>,
}

impl CodekbCandidate {
    /// 候補の全材料を束ねる (**この型の唯一の構築経路**)。
    #[must_use]
    pub const fn new(
        artifacts: CodekbArtifacts,
        analyzed_paths: CodekbScopePaths,
        recorded_fingerprint: Option<String>,
    ) -> CodekbCandidate {
        CodekbCandidate {
            artifacts,
            analyzed_paths,
            recorded_fingerprint,
        }
    }

    /// snapshot が取った範囲が、この候補の主張を覆っていない**最初の**パスを答える。
    ///
    /// 覆えていれば `None`。snapshot がリポジトリ根 (`./`) を取っていれば、個々のパスを
    /// 見るまでもなく覆う (upstream の短絡と同じ)。
    ///
    /// これは公開前の検査であり、ストアの状態を見ない — だから集約ではなく候補が答える。
    #[must_use]
    pub fn uncovered_by(&self, snapshot_paths: &CodekbScopePaths) -> Option<&CodekbScopePath> {
        if snapshot_paths.claims_root() {
            return None;
        }
        let mut index = 0;
        while let Some(path) = self.analyzed_paths.at(index) {
            if !snapshot_paths.covers(path) {
                return Some(path);
            }
            index += 1;
        }
        None
    }

    /// 公開するバイト。
    #[must_use]
    pub const fn artifacts(&self) -> &CodekbArtifacts {
        &self.artifacts
    }

    /// 公開するバイトを取り出して候補を使い切る。
    ///
    /// 公開は候補の**最後の使い道**なので、借りて複製するのではなく所有ごと渡す
    /// (9 成果物のバイトを二重に持たない)。
    #[must_use]
    pub fn into_artifacts(self) -> CodekbArtifacts {
        self.artifacts
    }

    /// 候補が「深く読んだ」と主張する走査範囲。
    ///
    /// 公開の前に、snapshot が取った範囲がこれを覆っているかを合成ルートが実測する
    /// (境界での読取であり、判断の持ち出しではない)。
    #[must_use]
    pub const fn analyzed_paths(&self) -> &CodekbScopePaths {
        &self.analyzed_paths
    }

    /// 鮮度印が記録している内容指紋 (記録が無ければ `None`)。
    #[must_use]
    pub fn recorded_fingerprint(&self) -> Option<&str> {
        self.recorded_fingerprint.as_deref()
    }
}
