//! `LearningDisposition` — その学びについて何を書き足すか。

/// 実践行と監査行のどちらが欠けているか。
///
/// 本家 `aidlc-learnings.ts:822-865` は `hasRow` / `hasLine` の 2 値から 4 通りを導く。
/// 両方在るものはそもそも事実にならないので、この列挙は残る 3 通りだけを持つ。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LearningDisposition {
    /// どちらも無い — 実践行を足し、監査行を立てる。
    Fresh,
    /// 監査行は在るが実践行が無い — 実践行だけを補う（監査は二重に立てない）。
    PracticeLineOnly,
    /// 実践行は在るが監査行が無い — 監査行だけを補う（実践行は二重に足さない）。
    AuditRowOnly,
}

impl LearningDisposition {
    /// 実践行と監査行の実測から決める。両方在るなら `None`（書くものが無い）。
    #[must_use]
    pub const fn of_presence(
        recorded_in_audit: bool,
        present_in_practice_file: bool,
    ) -> Option<LearningDisposition> {
        match (recorded_in_audit, present_in_practice_file) {
            (true, true) => None,
            (false, false) => Some(LearningDisposition::Fresh),
            (true, false) => Some(LearningDisposition::PracticeLineOnly),
            (false, true) => Some(LearningDisposition::AuditRowOnly),
        }
    }

    /// 実践行を足すか。
    #[must_use]
    pub const fn writes_practice_line(&self) -> bool {
        matches!(
            self,
            LearningDisposition::Fresh | LearningDisposition::PracticeLineOnly
        )
    }

    /// 監査行を立てるか（`rule_learned` に数えるのはこちら）。
    #[must_use]
    pub const fn writes_audit_row(&self) -> bool {
        matches!(
            self,
            LearningDisposition::Fresh | LearningDisposition::AuditRowOnly
        )
    }
}

#[cfg(test)]
mod tests {
    use super::LearningDisposition;

    #[test]
    fn both_sides_present_means_there_is_nothing_to_write() {
        assert_eq!(LearningDisposition::of_presence(true, true), None);
    }

    #[test]
    fn each_missing_side_names_what_to_repair() {
        assert_eq!(
            LearningDisposition::of_presence(false, false),
            Some(LearningDisposition::Fresh)
        );
        assert_eq!(
            LearningDisposition::of_presence(true, false),
            Some(LearningDisposition::PracticeLineOnly)
        );
        assert_eq!(
            LearningDisposition::of_presence(false, true),
            Some(LearningDisposition::AuditRowOnly)
        );
    }

    #[test]
    fn the_disposition_answers_which_side_it_writes() {
        assert!(LearningDisposition::Fresh.writes_practice_line());
        assert!(LearningDisposition::Fresh.writes_audit_row());
        assert!(LearningDisposition::PracticeLineOnly.writes_practice_line());
        assert!(!LearningDisposition::PracticeLineOnly.writes_audit_row());
        assert!(!LearningDisposition::AuditRowOnly.writes_practice_line());
        assert!(LearningDisposition::AuditRowOnly.writes_audit_row());
    }
}
