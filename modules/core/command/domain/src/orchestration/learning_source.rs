//! `LearningSource` — 学びの出所（監査行 `Source`）。

/// 候補の出所。surface が並べた候補か、人がその場で足した記述か。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LearningSource {
    /// surface が日誌から並べた候補を人が残した。
    Orchestrator,
    /// 「次回に向けて足すこと」で人が書き足した。
    UserAddition,
}

impl LearningSource {
    /// 選択ファイルの綴りから読む。`user_addition` 以外はすべて既定の
    /// `orchestrator` になる（本家と同じ倒し方）。
    #[must_use]
    pub fn of_spelling(spelling: &str) -> LearningSource {
        if spelling == "user_addition" {
            LearningSource::UserAddition
        } else {
            LearningSource::Orchestrator
        }
    }

    /// 監査行 `**Source**:` の逐語。
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            LearningSource::Orchestrator => "orchestrator",
            LearningSource::UserAddition => "user_addition",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::LearningSource;

    #[test]
    fn only_the_exact_user_addition_spelling_marks_a_human_addition() {
        assert_eq!(
            LearningSource::of_spelling("user_addition"),
            LearningSource::UserAddition
        );
        assert_eq!(
            LearningSource::of_spelling("orchestrator"),
            LearningSource::Orchestrator
        );
        assert_eq!(
            LearningSource::of_spelling(""),
            LearningSource::Orchestrator
        );
    }

    #[test]
    fn the_audit_spelling_is_the_upstream_verbatim() {
        assert_eq!(LearningSource::Orchestrator.as_str(), "orchestrator");
        assert_eq!(LearningSource::UserAddition.as_str(), "user_addition");
    }
}
