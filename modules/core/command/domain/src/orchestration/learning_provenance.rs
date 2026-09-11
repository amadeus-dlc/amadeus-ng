//! `LearningProvenance` — surface の時点で固定した書込先の素性。

use crate::workspace::{IntentDirName, SpaceName};

/// 候補を並べた時点の space と intent 記録。
///
/// **persist はこれを固定値として受け取り、実行時のカーソルを読み直さない** — surface と
/// persist のあいだに利用者が別の作業へ移っても、書込みは並べた時点の作業へ落ちる
/// （固定本家 2.7.1 `a277af21` `aidlc-learnings.ts:703-711` の LOCAL FIX #2）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LearningProvenance {
    space: SpaceName,
    intent: IntentDirName,
}

impl LearningProvenance {
    /// 固定した 2 つを束ねる完全コンストラクタ。
    #[must_use]
    pub const fn new(space: SpaceName, intent: IntentDirName) -> LearningProvenance {
        LearningProvenance { space, intent }
    }

    /// 固定した space。
    #[must_use]
    pub const fn space(&self) -> &SpaceName {
        &self.space
    }

    /// 固定した intent 記録ディレクトリ名。
    #[must_use]
    pub const fn intent(&self) -> &IntentDirName {
        &self.intent
    }

    /// 監査行 `**Destination**:` の綴り（本家は絶対パスを書き、`<project-dir>` へ伏せる）。
    #[must_use]
    pub fn destination(&self, scope: super::LearningScope) -> String {
        format!(
            "<project-dir>/aidlc/spaces/{}/memory/{}",
            self.space.as_str(),
            scope.method_file()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::LearningProvenance;
    use crate::orchestration::LearningScope;
    use crate::workspace::{IntentDirName, SpaceName};

    fn provenance() -> LearningProvenance {
        LearningProvenance::new(
            SpaceName::parse("default").expect("space"),
            IntentDirName::parse("260908-learnings").expect("intent"),
        )
    }

    #[test]
    fn the_provenance_keeps_the_pinned_space_and_intent() {
        let pinned = provenance();
        assert_eq!(pinned.space().as_str(), "default");
        assert_eq!(pinned.intent().as_str(), "260908-learnings");
    }

    /// ゴールデン `learnings/persist-one` の `**Destination**:` 実測。
    #[test]
    fn the_destination_is_the_redacted_method_file_path() {
        assert_eq!(
            provenance().destination(LearningScope::Project),
            "<project-dir>/aidlc/spaces/default/memory/project.md"
        );
        assert_eq!(
            provenance().destination(LearningScope::Team),
            "<project-dir>/aidlc/spaces/default/memory/team.md"
        );
    }
}
