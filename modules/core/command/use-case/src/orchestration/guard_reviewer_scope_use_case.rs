//! `GuardReviewerScopeUseCase` — 兄弟 Unit へ届く呼出しを拒否し、その事実を残す。
use chrono::{DateTime, Utc};
use core_command_domain::orchestration::{ReviewerScope, ReviewerScopeBlock, ReviewerScopeVerdict};
use core_command_domain::workspace::{
    EventType, HookHealthTarget, SessionAudit, SessionAuditObservation, SessionAuditObservationId,
    SessionAuditRecord,
};

use super::port::{RepositoryError, SessionAuditRepository};
use super::reviewer_scope_cause::ReviewerScopeCause;
use super::reviewer_scope_error::ReviewerScopeError;
use super::reviewer_scope_request::ReviewerScopeRequest;

/// 監査側の失敗を 1 つの原因へ畳む。
fn audit(error: impl Into<super::SessionAuditCommandError>) -> ReviewerScopeCause {
    ReviewerScopeCause::Audit(error.into())
}

/// 呼出し前の読み取り範囲の判定を行い、拒否したときだけ `REVIEWER_SCOPE_BLOCKED` を保存する。
///
/// # なぜ集約を読まないのか
///
/// 読み取り範囲は差し向け記録 (誰・どのステージ・どの Unit・何が許可か) と呼出しの内容
/// だけで決まる。workflow の進行状態は材料に入らないので、実行・計画・定義の再構成を
/// 行わない。凍結 ([`GuardReviewFreezeUseCase`]) が受領証という**集約の状態**を見るのと
/// 対照的である。
///
/// # 許可のときは何も書かない
///
/// upstream の hook も許可の監査行を持たない (`REVIEWER_SCOPE_BLOCKED` は拒否のときだけ)。
///
/// [`GuardReviewFreezeUseCase`]: super::GuardReviewFreezeUseCase
#[derive(Debug)]
pub struct GuardReviewerScopeUseCase<S: SessionAuditRepository> {
    session_audit_repository: S,
}

impl<S: SessionAuditRepository> GuardReviewerScopeUseCase<S> {
    /// 監査のポートを注入する。
    #[must_use]
    pub const fn new(session_audit_repository: S) -> GuardReviewerScopeUseCase<S> {
        GuardReviewerScopeUseCase {
            session_audit_repository,
        }
    }

    /// 呼出し 1 件を判定し、拒否なら 1 件の事実を保存して拒否を返す。
    ///
    /// `occurred_at` は呼出側が持つ時計の読みである — 集約は時計を持たない。
    ///
    /// # Errors
    /// 拒否は成立したのに記録できなかった場合。誤りは記録の失敗だけなので、戻る
    /// [`ReviewerScopeError`] は拒否の材料を運ぶ — 呼出側は原因を残したうえで拒否する。
    pub async fn execute(
        &mut self,
        request: &ReviewerScopeRequest,
        target: &HookHealthTarget,
        workflow_status: &str,
        occurred_at: DateTime<Utc>,
    ) -> Result<ReviewerScopeVerdict, ReviewerScopeError> {
        let verdict = ReviewerScope::of(request.dispatch(), request.record_root(), request.cwd())
            .judge(request.tool(), request.candidates());
        let ReviewerScopeVerdict::Blocked(block) = &verdict else {
            return Ok(verdict);
        };
        self.record(target, workflow_status, block, occurred_at)
            .await
            .map_err(|cause| ReviewerScopeError::new(block.clone(), cause))?;
        Ok(verdict)
    }

    /// 拒否 1 件を監査事実として保存する。
    async fn record(
        &mut self,
        target: &HookHealthTarget,
        workflow_status: &str,
        block: &ReviewerScopeBlock,
        occurred_at: DateTime<Utc>,
    ) -> Result<(), ReviewerScopeCause> {
        let record = SessionAuditRecord::new(
            EventType::ReviewerScopeBlocked,
            block
                .audit_fields()
                .map_err(ReviewerScopeCause::AuditField)?,
        )
        .map_err(|error| ReviewerScopeCause::Audit(error.into()))?;
        let observation = SessionAuditObservation::new(
            SessionAuditObservationId::generate(),
            target.clone(),
            record,
            workflow_status.to_string(),
        );
        let id = SessionAudit::id_for(&observation);
        match self.session_audit_repository.find_by_id(&id).await {
            Ok(mut aggregate) => {
                if let Some(event) = aggregate.record(&observation, occurred_at).map_err(audit)? {
                    self.session_audit_repository
                        .store(&event, &aggregate)
                        .await
                        .map_err(audit)?;
                }
            }
            Err(RepositoryError::NotFound { .. }) => {
                if let Some((aggregate, event)) =
                    SessionAudit::start(&observation, occurred_at).map_err(audit)?
                {
                    self.session_audit_repository
                        .store(&event, &aggregate)
                        .await
                        .map_err(audit)?;
                }
            }
            Err(error) => return Err(audit(error)),
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    // テストでは想定外バリアントの即時失敗に panic! を使う。
    #![allow(clippy::panic)]

    use super::GuardReviewerScopeUseCase;
    use crate::orchestration::test_support::InMemorySessionAuditRepository;
    use crate::orchestration::{ReviewerScopeRequest, SessionAuditRepository as _};
    use chrono::{TimeZone as _, Utc};
    use core_command_domain::orchestration::{
        ExemptPaths, InspectedTool, ReviewedStage, ReviewedUnit, ReviewerDispatch,
        ReviewerScopeCandidate, ReviewerScopeCandidates, ReviewerScopeVerdict, ScopeToken,
    };
    use core_command_domain::workspace::{
        EventType, HookHealthTarget, SessionAudit, SessionAuditObservation,
        SessionAuditObservationId, SessionAuditRecord, SpaceName,
    };

    const SIBLING: &str = "/r/construction/u2-beta/design.md";

    fn dispatch() -> ReviewerDispatch {
        ReviewerDispatch::new(
            "aidlc-architecture-reviewer-agent".to_string(),
            ReviewedStage::new("functional-design".to_string()),
            ReviewedUnit::parse("u1-alpha").unwrap(),
            ExemptPaths::default(),
        )
    }

    fn read_of(path: &str) -> ReviewerScopeRequest {
        ReviewerScopeRequest::new(
            dispatch(),
            Some("/r".to_string()),
            Some("/w".to_string()),
            InspectedTool::parse("Read").unwrap(),
            ReviewerScopeCandidates::new(vec![ReviewerScopeCandidate::Target(
                ScopeToken::parse(path).unwrap(),
            )]),
        )
    }

    fn target() -> HookHealthTarget {
        HookHealthTarget::new(SpaceName::parse("default").unwrap(), None)
    }

    fn at() -> chrono::DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 9, 10, 0, 0, 0).unwrap()
    }

    #[tokio::test]
    async fn a_crossing_call_is_refused_and_recorded_once() {
        let mut audits = InMemorySessionAuditRepository::empty();
        let verdict = GuardReviewerScopeUseCase::new(&mut audits)
            .execute(&read_of(SIBLING), &target(), "Running", at())
            .await
            .expect("記録できる");
        let ReviewerScopeVerdict::Blocked(block) = verdict else {
            panic!("兄弟 Unit への読取りは拒否される");
        };
        assert_eq!(block.target().as_str(), SIBLING);
        let [event] = audits.stored() else {
            panic!("拒否は 1 件の事実として保存される: {:?}", audits.stored());
        };
        assert_eq!(event.record().kind(), EventType::ReviewerScopeBlocked);
        assert!(
            event
                .record()
                .fields()
                .iter()
                .any(|(key, value)| key.as_str() == "Target" && value.as_str() == SIBLING),
            "{:?}",
            event.record().fields()
        );
    }

    #[tokio::test]
    async fn a_call_inside_the_unit_is_allowed_and_nothing_is_recorded() {
        let mut audits = InMemorySessionAuditRepository::empty();
        let verdict = GuardReviewerScopeUseCase::new(&mut audits)
            .execute(
                &read_of("/r/construction/u1-alpha/design.md"),
                &target(),
                "Running",
                at(),
            )
            .await
            .expect("許可に記録は要らない");
        assert_eq!(verdict, ReviewerScopeVerdict::Allowed);
        assert!(
            audits.stored().is_empty(),
            "許可の監査行は無い (upstream と同じ)"
        );
    }

    #[tokio::test]
    async fn a_second_refusal_appends_to_the_existing_audit_aggregate() {
        // 同じ記録先に既に監査集約があれば、genesis ではなく追記になる。
        let first = SessionAuditObservation::new(
            SessionAuditObservationId::generate(),
            target(),
            SessionAuditRecord::new(
                EventType::SessionStarted,
                core_command_domain::workspace::AuditFields::new().with(
                    core_command_domain::workspace::AuditFieldKey::parse("Source").unwrap(),
                    "startup",
                ),
            )
            .unwrap(),
            "Running".to_string(),
        );
        let (aggregate, _) = SessionAudit::start(&first, at()).unwrap().unwrap();
        let mut audits = InMemorySessionAuditRepository::holding(aggregate);
        GuardReviewerScopeUseCase::new(&mut audits)
            .execute(&read_of(SIBLING), &target(), "Running", at())
            .await
            .expect("追記できる");
        let [event] = audits.stored() else {
            panic!("追記は 1 件: {:?}", audits.stored());
        };
        assert_eq!(event.record().kind(), EventType::ReviewerScopeBlocked);
        assert_eq!(
            audits
                .find_by_id(event.aggregate_id())
                .await
                .unwrap()
                .seq_nr(),
            2,
            "既存の集約の通番が進む"
        );
    }

    #[tokio::test]
    async fn a_recording_failure_still_carries_the_refusal() {
        // 監査の失敗は判定を変えない (upstream「an audit failure never changes the block
        // decision」) — 誤りは拒否の材料を運んだまま呼出側へ戻る。
        let mut audits = InMemorySessionAuditRepository::failing_on_store();
        let error = GuardReviewerScopeUseCase::new(&mut audits)
            .execute(&read_of(SIBLING), &target(), "Running", at())
            .await
            .expect_err("保存が失敗した");
        assert_eq!(error.block().target().as_str(), SIBLING);
        assert_eq!(error.block().unit().as_str(), "u1-alpha");
        assert!(error.to_string().contains("was not recorded"), "{error}");
        assert!(audits.stored().is_empty());
    }
}
