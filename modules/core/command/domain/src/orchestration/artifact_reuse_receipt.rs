//! 既存成果物を再利用すると決めたときの受領内容。
/// 保存と監査へ渡す、受理時点に確定した事実。
///
/// 決定の閉集合は配布実装の検証と同じ 3 語である
/// (`.claude/tools/aidlc-state.ts` の `handleReuseArtifact` が `keep` / `modify` / `redo` 以外を
/// 拒否する)。所有者は本家であり、こちらの都合で語を足さない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactReuseReceipt {
    stage: String,
    decision: String,
    artifacts: String,
    repo: Option<String>,
    single: bool,
}

/// 本家が受理する決定の 3 語。
const DECISIONS: [&str; 3] = ["keep", "modify", "redo"];

impl ArtifactReuseReceipt {
    /// 受理できる材料から構築する。
    /// # Errors
    /// ステージ・成果物が空、または決定が閉集合の外の場合。
    pub fn new(
        stage: String,
        decision: String,
        artifacts: String,
        repo: Option<String>,
        single: bool,
    ) -> Result<Self, super::ArtifactReuseError> {
        if stage.is_empty() {
            return Err(super::ArtifactReuseError::StageRequired);
        }
        if artifacts.is_empty() {
            return Err(super::ArtifactReuseError::ArtifactsRequired);
        }
        if !DECISIONS.contains(&decision.as_str()) {
            return Err(super::ArtifactReuseError::InvalidDecision { given: decision });
        }
        Ok(Self {
            stage,
            decision,
            artifacts,
            repo,
            single,
        })
    }

    /// 再利用を決めたステージ。
    #[must_use]
    pub fn stage(&self) -> &str {
        &self.stage
    }

    /// 決定（`keep` / `modify` / `redo`）。
    #[must_use]
    pub fn decision(&self) -> &str {
        &self.decision
    }

    /// 再利用する成果物の指定（本家と同じく区切り済みの 1 行）。
    #[must_use]
    pub fn artifacts(&self) -> &str {
        &self.artifacts
    }

    /// 対象 repo（指定が無ければ `None`）。
    #[must_use]
    pub fn repo(&self) -> Option<&str> {
        self.repo.as_deref()
    }

    /// 独立実行か。
    #[must_use]
    pub const fn is_single(&self) -> bool {
        self.single
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use crate::orchestration::ArtifactReuseError;

    /// 受理できる材料は、与えた値をそのまま持つ。
    #[test]
    fn an_accepted_receipt_keeps_the_material_it_was_built_from() {
        let receipt = ArtifactReuseReceipt::new(
            "reverse-engineering".to_string(),
            "keep".to_string(),
            "docs/a.md".to_string(),
            Some("app".to_string()),
            true,
        )
        .unwrap();
        assert_eq!(receipt.stage(), "reverse-engineering");
        assert_eq!(receipt.decision(), "keep");
        assert_eq!(receipt.artifacts(), "docs/a.md");
        assert_eq!(receipt.repo(), Some("app"));
        assert!(receipt.is_single());
        // 任意欄は省略できる。
        let bare = ArtifactReuseReceipt::new(
            "reverse-engineering".to_string(),
            "redo".to_string(),
            "docs/a.md".to_string(),
            None,
            false,
        )
        .unwrap();
        assert_eq!(bare.repo(), None);
        assert!(!bare.is_single());
    }

    /// 3 語すべてが受理される — 閉集合は本家の `keep` / `modify` / `redo` である。
    #[test]
    fn the_three_upstream_decisions_are_the_whole_closed_set() {
        for decision in DECISIONS {
            assert!(
                ArtifactReuseReceipt::new(
                    "reverse-engineering".to_string(),
                    decision.to_string(),
                    "docs/a.md".to_string(),
                    None,
                    false,
                )
                .is_ok(),
                "{decision}"
            );
        }
    }

    /// 空のステージ・空の成果物・閉集合の外の決定は、それぞれ別の理由で拒否される。
    #[test]
    fn an_empty_stage_an_empty_artifact_list_and_an_unknown_decision_are_refused_apart() {
        assert!(matches!(
            ArtifactReuseReceipt::new(
                String::new(),
                "keep".to_string(),
                "docs/a.md".to_string(),
                None,
                false,
            ),
            Err(ArtifactReuseError::StageRequired)
        ));
        assert!(matches!(
            ArtifactReuseReceipt::new(
                "reverse-engineering".to_string(),
                "keep".to_string(),
                String::new(),
                None,
                false,
            ),
            Err(ArtifactReuseError::ArtifactsRequired)
        ));
        let invalid = ArtifactReuseReceipt::new(
            "reverse-engineering".to_string(),
            "bogus".to_string(),
            "docs/a.md".to_string(),
            None,
            false,
        );
        assert!(
            matches!(invalid, Err(ArtifactReuseError::InvalidDecision { ref given }) if given == "bogus"),
            "{invalid:?}"
        );
    }
}
