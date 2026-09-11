//! `LearningCandidateId` — surface が採番した候補の番号。

/// surface のその回の候補番号（`c1`, `c2`, …）。
///
/// **同一性ではない** — 起動のたびに `c1` から採り直すので、同じ番号が別の学びを指しうる。
/// 監査行 `**Candidate-ID**:` に残すのは、どの回のどの候補だったかを人が辿るためである
/// （同一性は [`LearningContentHash`] が持つ）。
///
/// [`LearningContentHash`]: super::LearningContentHash
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LearningCandidateId {
    id: String,
}

impl LearningCandidateId {
    // 検査済みの綴りは、この構築口で全状態を初期化する。
    const fn of_id(id: String) -> Self {
        Self { id }
    }

    /// 選択ファイルの綴りを読む（**この型の唯一の構築経路**）。
    ///
    /// 監査行の 1 ラベル 1 行を壊さないことだけを見る — 空・改行・制御文字を拒否する。
    ///
    /// # Errors
    /// 空、または改行・制御文字を含む場合。
    pub fn parse(raw: &str) -> Result<LearningCandidateId, super::LearningCandidateIdError> {
        if raw.is_empty() || raw.chars().any(char::is_control) || raw.trim() != raw {
            return Err(super::LearningCandidateIdError::new(raw));
        }
        Ok(LearningCandidateId::of_id(raw.to_string()))
    }

    /// 監査行 `**Candidate-ID**:` の綴り。
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.id
    }
}

#[cfg(test)]
mod tests {
    use super::LearningCandidateId;

    #[test]
    fn a_plain_candidate_number_parses() {
        assert_eq!(
            LearningCandidateId::parse("c1").map(|id| id.as_str().to_string()),
            Ok("c1".to_string())
        );
        assert_eq!(
            LearningCandidateId::parse("fixture-1").map(|id| id.as_str().to_string()),
            Ok("fixture-1".to_string())
        );
    }

    #[test]
    fn a_line_breaking_or_empty_id_is_refused() {
        assert!(LearningCandidateId::parse("").is_err());
        assert!(LearningCandidateId::parse("c1\nStage: other").is_err());
        assert!(LearningCandidateId::parse("c1\r").is_err());
        assert!(LearningCandidateId::parse(" c1").is_err());
    }
}
