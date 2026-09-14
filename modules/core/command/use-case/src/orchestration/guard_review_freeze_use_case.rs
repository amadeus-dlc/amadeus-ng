//! `GuardReviewFreezeUseCase` — 終端受領証を無効化する書込みを拒否し、その事実を残す。
use chrono::{DateTime, Utc};
use core_command_domain::orchestration::{
    IntentExecutionId, ReviewFreezeBlock, ReviewFreezeVerdict, WriteTargets,
};
use core_command_domain::workspace::{
    EventType, HookHealthTarget, SessionAudit, SessionAuditObservation, SessionAuditObservationId,
    SessionAuditRecord,
};

use super::port::{
    IntentExecutionRepository, IntentRepository, RepositoryError, SessionAuditRepository,
    WorkflowDefinitionRepository,
};
use super::review_freeze_error::ReviewFreezeError;

/// 監査側の失敗を 1 つの原因へ畳む。
fn audit(error: impl Into<super::SessionAuditCommandError>) -> ReviewFreezeError {
    ReviewFreezeError::Audit(error.into())
}

/// 書込み前の凍結判定を行い、拒否したときだけ `REVIEW_FREEZE_BLOCKED` を保存する。
///
/// # なぜ判定結果を返すのか
///
/// このユースケースの産物は**拒否そのもの**である — ハーネスの PreToolUse 契約は
/// 「拒否なら exit 2 と理由」であり、呼出側がそれを知らなければ保護が成立しない。
/// 返すのは値オブジェクト [`ReviewFreezeVerdict`] であって表示材料ではない。拒否文言は
/// 合成ルートの Presenter が [`ReviewFreezeBlock`] から組む。
///
/// # 許可のときは何も書かない
///
/// upstream の hook も許可の監査行を持たない (`REVIEW_FREEZE_BLOCKED` は拒否のときだけ)。
/// 「読むだけの実行がある」ことは読取専用ユースケースの新設とは別である — 本ユースケースは
/// 拒否という書込みを持ち、その前提として同じ材料を読む。
#[derive(Debug)]
pub struct GuardReviewFreezeUseCase<
    E: IntentExecutionRepository,
    I: IntentRepository,
    D: WorkflowDefinitionRepository,
    S: SessionAuditRepository,
> {
    intent_execution_repository: E,
    intent_repository: I,
    workflow_definition_repository: D,
    session_audit_repository: S,
}

impl<
    E: IntentExecutionRepository,
    I: IntentRepository,
    D: WorkflowDefinitionRepository,
    S: SessionAuditRepository,
> GuardReviewFreezeUseCase<E, I, D, S>
{
    /// ポートの実装を 4 つ注入する。
    #[must_use]
    pub const fn new(
        intent_execution_repository: E,
        intent_repository: I,
        workflow_definition_repository: D,
        session_audit_repository: S,
    ) -> GuardReviewFreezeUseCase<E, I, D, S> {
        GuardReviewFreezeUseCase {
            intent_execution_repository,
            intent_repository,
            workflow_definition_repository,
            session_audit_repository,
        }
    }

    /// 書込み先の列を判定し、拒否なら 1 件の事実を保存して拒否を返す。
    ///
    /// `occurred_at` は呼出側が持つ時計の読みである — 集約は時計を持たない。
    ///
    /// # Errors
    /// 実行・計画・定義の再構成失敗、または拒否の記録失敗。
    pub async fn execute(
        &mut self,
        execution_id: &IntentExecutionId,
        target: &HookHealthTarget,
        workflow_status: &str,
        tool: &str,
        targets: &WriteTargets,
        occurred_at: DateTime<Utc>,
    ) -> Result<ReviewFreezeVerdict, ReviewFreezeError> {
        let execution = self
            .intent_execution_repository
            .find_by_id(execution_id)
            .await
            .map_err(ReviewFreezeError::Execution)?;
        let intent = self
            .intent_repository
            .find_by_id(execution.intent_id())
            .await
            .map_err(ReviewFreezeError::Intent)?;
        let definition = self
            .workflow_definition_repository
            .find_by_id(intent.definition_id())
            .await
            .map_err(ReviewFreezeError::Definition)?;
        let verdict = execution.judge_review_freeze(&intent, &definition, targets);
        let ReviewFreezeVerdict::Blocked(block) = &verdict else {
            return Ok(verdict);
        };
        self.record(target, workflow_status, tool, block, occurred_at)
            .await?;
        Ok(verdict)
    }

    /// 拒否 1 件を監査事実として保存する。
    async fn record(
        &mut self,
        target: &HookHealthTarget,
        workflow_status: &str,
        tool: &str,
        block: &ReviewFreezeBlock,
        occurred_at: DateTime<Utc>,
    ) -> Result<(), ReviewFreezeError> {
        let record = SessionAuditRecord::new(
            EventType::ReviewFreezeBlocked,
            block
                .audit_fields(tool)
                .map_err(ReviewFreezeError::AuditField)?,
        )
        .map_err(|error| ReviewFreezeError::Audit(error.into()))?;
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
