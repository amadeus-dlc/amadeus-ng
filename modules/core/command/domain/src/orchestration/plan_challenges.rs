//! セッションの公開キーごとの、現在の計画承認提示。
use super::{PlanChallengeOccurrence, PlanSession};
use std::collections::BTreeMap;
/// 同じ公開キーでも元のセッション名を確認する提示集合。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanChallenges {
    entries: BTreeMap<String, PlanChallengeOccurrence>,
}
impl Default for PlanChallenges {
    fn default() -> Self {
        Self::of_entries(Default::default())
    }
}
impl PlanChallenges {
    // 検査済みの索引と空の既定値を、同じ完全な構築口へ集める。
    const fn of_entries(entries: BTreeMap<String, PlanChallengeOccurrence>) -> Self {
        Self { entries }
    }
    /// 記録済みの各要素を重複なく組む。
    /// # Errors
    /// 同じキーが複数ある場合。
    pub fn new(
        values: impl IntoIterator<Item = PlanChallengeOccurrence>,
    ) -> Result<Self, super::PlanApprovalError> {
        let mut entries = BTreeMap::new();
        for value in values {
            if entries
                .insert(value.challenge().session().key().to_string(), value)
                .is_some()
            {
                return Err(super::PlanApprovalError::new(
                    "duplicate approval collection entry",
                ));
            }
        }
        Ok(Self::of_entries(entries))
    }

    /// 現在の各提示。
    pub fn iter(&self) -> impl Iterator<Item = &PlanChallengeOccurrence> {
        self.entries.values()
    }

    /// 指定セッションに提示した現在の発行回。
    #[must_use]
    pub fn for_session(&self, session: &PlanSession) -> Option<&PlanChallengeOccurrence> {
        self.entries
            .get(session.key())
            .filter(|entry| entry.challenge().session() == session)
    }
    #[allow(
        clippy::expect_used,
        reason = "検証済みイベントの再生で提示が欠ける履歴は破損としてクラッシュする（aggregate-commands.md）"
    )]
    pub(super) fn observe(&mut self, session: &PlanSession, response: super::PlanHumanResponse) {
        let current = self
            .entries
            .get_mut(session.key())
            .expect("observed challenge exists");
        assert_eq!(
            current.challenge().session(),
            session,
            "observed session matches"
        );
        assert_eq!(
            current.id(),
            response.occurrence_id(),
            "observed issuance matches"
        );
        current.observe(response);
    }
    pub(super) fn clear_occurrence(
        &mut self,
        session: &PlanSession,
        id: &super::PlanApprovalOperationId,
    ) {
        if self
            .for_session(session)
            .is_some_and(|current| current.id() == id)
        {
            self.entries.remove(session.key());
        }
    }
    pub(super) fn issue(&mut self, occurrence: PlanChallengeOccurrence) {
        self.entries.insert(
            occurrence.challenge().session().key().to_string(),
            occurrence,
        );
    }
}
