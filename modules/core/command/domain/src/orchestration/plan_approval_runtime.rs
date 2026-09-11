//! 全intent・spaceで共有する計画承認の集約。
use super::{
    PlanApprovalEvent, PlanApprovalEventId, PlanApprovalOperationId, PlanApprovalRuntimeId,
    PlanChallenge, PlanChallengeIssued, PlanChallengeOccurrence, PlanChallenges,
    PlanRuntimeCreated, PlanRuntimeError,
};
use chrono::{DateTime, Utc};
/// 提示と人間応答の有効性を、共有の発行履歴で判断する所有者。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanApprovalRuntime {
    generations: super::PlanGenerations,
    answers: super::PlanAnswers,
    receipts: super::PlanReceipts,
    pending_responses: super::PlanPendingResponses,
    id: PlanApprovalRuntimeId,
    challenges: PlanChallenges,
    invalidations: super::PlanInvalidations,
    applied: super::PlanAppliedOperations,
    seq_nr: usize,
    version: usize,
    last_updated_at: DateTime<Utc>,
}
impl PlanApprovalRuntime {
    /// 公開直後のソースを照合し、開始を確定または失効する。
    /// # Errors
    /// 対応する公開待ち開始操作がない場合。
    pub fn certify_generation(
        &mut self,
        id: &PlanApprovalOperationId,
        source: Option<&str>,
        at: DateTime<Utc>,
    ) -> Result<PlanApprovalEvent, PlanRuntimeError> {
        let pending = self
            .generations
            .get(id)
            .filter(|generation| generation.state() == super::PlanGenerationState::Pending)
            .ok_or(PlanRuntimeError::NoPendingGeneration)?;
        if self.receipts.get(&pending.receipt().key()) != Some(pending.receipt()) {
            return Err(PlanRuntimeError::InvalidState);
        }
        let event = if source == Some(pending.receipt().certified_source()) {
            PlanApprovalEvent::GenerationCertified(super::PlanGenerationCertified::new(
                PlanApprovalEventId::generate(),
                self.id,
                id.clone(),
            ))
        } else {
            PlanApprovalEvent::GenerationRevoked(super::PlanGenerationRevoked::new(
                PlanApprovalEventId::generate(),
                self.id,
                id.clone(),
            ))
        };
        self.apply_event(&event, self.seq_nr + 1, at);
        Ok(event)
    }

    /// 現在の開始判断と直前ソースを照合し、実装開始を記録する。
    /// # Errors
    /// 文書・受領・直前ソースが一致しない場合。
    pub fn request_generation(
        &mut self,
        id: PlanApprovalOperationId,
        approval: &super::CodeGenerationApproval,
        source: Option<&str>,
        at: DateTime<Utc>,
    ) -> Result<PlanApprovalEvent, super::PlanApprovalError> {
        use super::{PlanApprovalError, PlanGenerationState, PlanGenerationStatus};
        self.require_unused(&id)
            .map_err(|error| PlanApprovalError::new(error.to_string()))?;
        if !self.invalidations.is_empty()
            || !self.pending_responses.is_empty()
            || self.answers.pending().next().is_some()
            || self.generations.pending().next().is_some()
        {
            return Err(PlanApprovalError::new(
                "pending approval operations must be recovered before generation",
            ));
        }
        if !approval.ok() {
            return Err(PlanApprovalError::new(approval.reason()));
        }
        let receipt = approval
            .receipt_key()
            .and_then(|key| self.receipts.get(key))
            .ok_or_else(|| {
                PlanApprovalError::new("Code Generation has no protected approval receipt")
            })?;
        let state = if receipt.status() == PlanGenerationStatus::Generation {
            PlanGenerationState::Active
        } else {
            if source != Some(receipt.certified_source()) {
                return Err(PlanApprovalError::new(
                    "workspace source changed after Plan Approval and before generation began",
                ));
            }
            PlanGenerationState::Pending
        };
        let generation = super::PlanGeneration::new(id, receipt.for_generation()?, state)?;
        let event =
            PlanApprovalEvent::GenerationRequested(Box::new(super::PlanGenerationRequested::new(
                PlanApprovalEventId::generate(),
                self.id,
                generation,
            )));
        self.apply_event(&event, self.seq_nr + 1, at);
        Ok(event)
    }
    /// 開始操作の履歴。
    #[must_use]
    pub const fn generations(&self) -> &super::PlanGenerations {
        &self.generations
    }

    /// 認証中にソースが変わった受領を取り消す。
    /// # Errors
    /// 対応する監査待ち回答がない場合。
    pub fn abort_answer(
        &mut self,
        id: &PlanApprovalOperationId,
        space: &crate::workspace::SpaceName,
        execution: &super::IntentExecution,
        source: Option<&str>,
        at: DateTime<Utc>,
    ) -> Result<PlanApprovalEvent, PlanRuntimeError> {
        if self.answer_delivery(id, space, execution, source)?
            != super::PlanAnswerDelivery::RejectCertification
        {
            return Err(PlanRuntimeError::InvalidState);
        }
        let event = PlanApprovalEvent::AnswerAborted(super::PlanAnswerAborted::new(
            PlanApprovalEventId::generate(),
            self.id,
            id.clone(),
        ));
        self.apply_event(&event, self.seq_nr + 1, at);
        Ok(event)
    }

    /// 受領のソースと、元の実行の監査記録を照合する。
    /// # Errors
    /// 操作や対象が一致しない場合。
    pub fn answer_delivery(
        &self,
        id: &PlanApprovalOperationId,
        space: &crate::workspace::SpaceName,
        execution: &super::IntentExecution,
        source: Option<&str>,
    ) -> Result<super::PlanAnswerDelivery, PlanRuntimeError> {
        let answer = self
            .answers
            .get(id)
            .filter(|answer| answer.state() == &super::PlanAnswerState::Pending)
            .ok_or(PlanRuntimeError::NoPendingAnswer)?;
        if answer.input().origin().space() != space
            || answer.input().origin().execution_id() != execution.id()
        {
            return Err(PlanRuntimeError::AnswerTargetMismatch);
        }
        Ok(if execution.has_plan_answer(id) {
            super::PlanAnswerDelivery::Recorded
        } else if answer
            .receipt()
            .is_some_and(|receipt| source != Some(receipt.certified_source()))
        {
            super::PlanAnswerDelivery::RejectCertification
        } else {
            super::PlanAnswerDelivery::RecordRequired
        })
    }
    /// 元の実行へ監査まで記録した計画回答を完了する。
    /// # Errors
    /// 操作・対象・監査記録が成立しない場合。
    pub fn complete_answer(
        &mut self,
        id: &PlanApprovalOperationId,
        space: &crate::workspace::SpaceName,
        execution: &super::IntentExecution,
        at: DateTime<Utc>,
    ) -> Result<PlanApprovalEvent, PlanRuntimeError> {
        if self.answer_delivery(id, space, execution, None)? != super::PlanAnswerDelivery::Recorded
        {
            return Err(PlanRuntimeError::AnswerNotRecorded);
        }
        let event = PlanApprovalEvent::AnswerCompleted(super::PlanAnswerCompleted::new(
            PlanApprovalEventId::generate(),
            self.id,
            id.clone(),
        ));
        self.apply_event(&event, self.seq_nr + 1, at);
        Ok(event)
    }

    /// 実際の提示と応答に一致した計画回答を受領する。
    /// # Errors
    /// 提示・応答・対象・ソースが一致しない場合。
    pub fn record_answer(
        &mut self,
        id: PlanApprovalOperationId,
        input: super::PlanAnswerInput,
        at: DateTime<Utc>,
    ) -> Result<PlanApprovalEvent, super::PlanApprovalError> {
        use super::{
            PlanAnswer, PlanAnswerState, PlanApprovalError, PlanApprovalReceipt, PlanChoice,
            PlanGenerationStatus,
        };
        self.require_unused(&id)
            .map_err(|error| PlanApprovalError::new(error.to_string()))?;
        if !self.invalidations.is_empty()
            || !self.pending_responses.is_empty()
            || self.answers.pending().next().is_some()
            || self.generations.pending().next().is_some()
        {
            return Err(PlanApprovalError::new(
                "pending approval operations must be recovered before recording an answer",
            ));
        }
        // 提示 (offered) と、その提示に対する同じ選択の応答 (response) が揃うときだけ有効。
        let matched = self
            .challenges
            .for_session(input.decision().session())
            .and_then(|offered| {
                let same_prompt = offered
                    .challenge()
                    .evidence()
                    .same_prompt(input.decision().evidence());
                offered
                    .response()
                    .filter(|response| {
                        same_prompt
                            && response.choice() == input.choice()
                            && response.occurrence_id() == offered.id()
                    })
                    .map(|response| (offered, response))
            });
        let Some((offered, response)) = matched else {
            return Err(PlanApprovalError::new(
                "Plan Approval requires the actual offered choice from this prompt and session",
            ));
        };
        let receipt = if input.choice() == PlanChoice::ApprovePlan {
            let source=input.source().filter(|source|*source==input.decision().evidence().authority().source_floor()).ok_or_else(||PlanApprovalError::new("Plan Approval requires workspace source to match the Code Generation directive's pre-planning source floor"))?;
            Some(PlanApprovalReceipt::new(
                input.decision().clone(),
                offered.challenge().id().to_string(),
                source.to_string(),
                PlanGenerationStatus::Approved,
            )?)
        } else {
            None
        };
        let answer = PlanAnswer::new(
            id,
            input,
            offered.id().clone(),
            offered.challenge().id().to_string(),
            response.id().clone(),
            receipt,
            PlanAnswerState::Pending,
        )?;
        let event = PlanApprovalEvent::AnswerRecorded(Box::new(super::PlanAnswerRecorded::new(
            PlanApprovalEventId::generate(),
            self.id,
            answer,
        )));
        self.apply_event(&event, self.seq_nr + 1, at);
        Ok(event)
    }
    /// 受領した回答と配送状態。
    #[must_use]
    pub const fn answers(&self) -> &super::PlanAnswers {
        &self.answers
    }
    /// 現在の受領。
    #[must_use]
    pub const fn receipts(&self) -> &super::PlanReceipts {
        &self.receipts
    }

    /// 元の観測先の記録が必要か、保存済みの事実から判断する。
    /// # Errors
    /// 準備がない場合や、対象が一致しない場合。
    pub fn response_delivery(
        &self,
        id: &PlanApprovalOperationId,
        space: &crate::workspace::SpaceName,
        execution: &super::IntentExecution,
    ) -> Result<super::PlanResponseDelivery, PlanRuntimeError> {
        let pending = self
            .pending_responses
            .get(id)
            .ok_or(PlanRuntimeError::NoPreparedResponse)?;
        if pending.origin().space() != space || pending.origin().execution_id() != execution.id() {
            return Err(PlanRuntimeError::ResponseTargetMismatch);
        }
        Ok(if execution.has_approval_observation(id) {
            super::PlanResponseDelivery::Recorded
        } else {
            super::PlanResponseDelivery::RecordRequired
        })
    }
    /// 実行側の記録が確認できた応答を、観測時の提示へ反映する。
    /// # Errors
    /// 準備・対象・実行側の記録が成立しない場合。
    pub fn complete_response(
        &mut self,
        id: &PlanApprovalOperationId,
        space: &crate::workspace::SpaceName,
        execution: &super::IntentExecution,
        at: DateTime<Utc>,
    ) -> Result<PlanApprovalEvent, PlanRuntimeError> {
        if self.response_delivery(id, space, execution)? != super::PlanResponseDelivery::Recorded {
            return Err(PlanRuntimeError::ResponseNotRecorded);
        }
        let pending = self
            .pending_responses
            .get(id)
            .ok_or(PlanRuntimeError::NoPreparedResponse)?;
        let current = self
            .challenges
            .for_session(pending.session())
            .is_some_and(|offered| offered.id() == pending.occurrence_id());
        let event = PlanApprovalEvent::ResponseObserved(super::PlanResponseObserved::new(
            PlanApprovalEventId::generate(),
            self.id,
            id.clone(),
            pending.occurrence_id().clone(),
            pending.session().clone(),
            core_infrastructure::hash::sha256_hex(
                core_infrastructure::ecmascript::trim(pending.response()).as_bytes(),
            ),
            if current { pending.choice() } else { None },
        ));
        self.apply_event(&event, self.seq_nr + 1, at);
        Ok(event)
    }
    /// 観測した応答を、提示の発行回へ固定して先に保存する。
    /// # Errors
    /// 提示がない場合、または先行操作が未完了の場合。
    pub fn prepare_response(
        &mut self,
        id: PlanApprovalOperationId,
        origin: super::PlanApprovalOrigin,
        session: super::PlanSession,
        response: &str,
        at: DateTime<Utc>,
    ) -> Result<PlanApprovalEvent, PlanRuntimeError> {
        if self.generations.pending().next().is_some() {
            return Err(PlanRuntimeError::PendingGeneration);
        }
        if self.answers.pending().next().is_some() {
            return Err(PlanRuntimeError::PendingAnswer);
        }
        if !self.invalidations.is_empty() {
            return Err(PlanRuntimeError::PendingInvalidation);
        }
        if !self.pending_responses.is_empty() {
            return Err(PlanRuntimeError::PendingResponse);
        }
        self.require_unused(&id)?;
        let offered = self
            .challenges
            .for_session(&session)
            .ok_or(PlanRuntimeError::NoPendingChallenge)?;
        let preparation = super::PlanResponsePreparation::new(
            id,
            origin,
            offered.id().clone(),
            session,
            response.to_string(),
            offered.challenge().offered_choice(response),
        );
        let event = PlanApprovalEvent::ResponsePrepared(super::PlanResponsePrepared::new(
            PlanApprovalEventId::generate(),
            self.id,
            preparation,
        ));
        self.apply_event(&event, self.seq_nr + 1, at);
        Ok(event)
    }

    /// 回復待ちの応答。
    #[must_use]
    pub const fn pending_responses(&self) -> &super::PlanPendingResponses {
        &self.pending_responses
    }

    /// 初期化の完了前に、承認操作がまだ始まっていないことを確認する。
    /// # Errors
    /// 誕生以外の事実や承認状態が既にある場合。
    pub fn require_initialization_only(&self) -> Result<(), PlanRuntimeError> {
        if self.seq_nr == 1
            && self.generations.iter().next().is_none()
            && self.answers.iter().next().is_none()
            && self.receipts.iter().next().is_none()
            && self.challenges.iter().next().is_none()
            && self.pending_responses.is_empty()
            && self.invalidations.is_empty()
            && self.applied.iter().next().is_none()
        {
            Ok(())
        } else {
            Err(PlanRuntimeError::AlreadyInUse)
        }
    }

    /// 元の実行が保存した発行事実を照合して、失効の保留を解く。
    /// # Errors
    /// 準備がない場合、または渡された実行が対象と異なる場合。
    pub fn resolve_for_publication(
        &mut self,
        id: &PlanApprovalOperationId,
        space: &crate::workspace::SpaceName,
        execution: &super::IntentExecution,
        at: DateTime<Utc>,
    ) -> Result<PlanApprovalEvent, PlanRuntimeError> {
        let pending = self
            .invalidations
            .iter()
            .find(|pending| pending.id() == id)
            .ok_or(PlanRuntimeError::UnknownInvalidation)?;
        if pending.space() != space || pending.execution_id() != execution.id() {
            return Err(PlanRuntimeError::InvalidationTargetMismatch);
        }
        Ok(self.resolve_invalidation(id, execution.has_approval_publication(id), at))
    }

    /// 同じ集約の完全な状態から、不変条件を検査して再構成する。
    /// # Errors
    /// 通番や保持中の操作・提示の対応が不正な場合。
    #[allow(
        clippy::too_many_arguments,
        reason = "集約の完全な状態を唯一の検査付き構築口へ渡す（domain-persistence-neutrality.md）"
    )]
    pub fn new(
        generations: super::PlanGenerations,
        answers: super::PlanAnswers,
        receipts: super::PlanReceipts,
        pending_responses: super::PlanPendingResponses,
        id: PlanApprovalRuntimeId,
        challenges: PlanChallenges,
        invalidations: super::PlanInvalidations,
        applied: super::PlanAppliedOperations,
        seq_nr: usize,
        at: DateTime<Utc>,
    ) -> Result<Self, PlanRuntimeError> {
        if answers.pending().any(|answer| {
            answer
                .receipt()
                .is_some_and(|receipt| receipts.get(&receipt.key()) != Some(receipt))
        }) || receipts.iter().any(|receipt| {
            !answers.iter().any(|answer| {
                !matches!(answer.state(), super::PlanAnswerState::Aborted(_))
                    && answer
                        .receipt()
                        .is_some_and(|approved| approved.same_certification(receipt))
            })
        }) {
            return Err(PlanRuntimeError::InvalidState);
        }
        if pending_responses
            .iter()
            .any(|pending| applied.contains(pending.id()) || invalidations.contains(pending.id()))
        {
            return Err(PlanRuntimeError::InvalidState);
        }
        if seq_nr == 0
            || invalidations
                .iter()
                .any(|entry| applied.contains(entry.id()))
            || challenges.iter().any(|issued| {
                !applied.contains(issued.id())
                    || issued.response().is_some_and(|response| {
                        response.occurrence_id() != issued.id() || !applied.contains(response.id())
                    })
            })
        {
            return Err(PlanRuntimeError::InvalidState);
        }
        Ok(Self {
            generations,
            answers,
            receipts,
            pending_responses,
            id,
            challenges,
            invalidations,
            applied,
            seq_nr,
            version: 0,
            last_updated_at: at,
        })
    }

    /// ストアから受けた不透明な版を持ち回る。採番しない。
    #[must_use]
    pub const fn with_version(mut self, version: usize) -> Self {
        self.version = version;
        self
    }
    /// ストアが付けた版。
    #[must_use]
    pub const fn version(&self) -> usize {
        self.version
    }

    /// 元の指示の確定結果を反映して、共有承認の保留を解く。
    ///
    /// 対応する準備の存在は唯一の呼出し元 `resolve_for_publication` が先に検査している。
    fn resolve_invalidation(
        &mut self,
        id: &PlanApprovalOperationId,
        published: bool,
        at: DateTime<Utc>,
    ) -> PlanApprovalEvent {
        let event = PlanApprovalEvent::InvalidationResolved(super::PlanInvalidationResolved::new(
            PlanApprovalEventId::generate(),
            self.id,
            id.clone(),
            published,
        ));
        self.apply_event(&event, self.seq_nr + 1, at);
        event
    }

    /// 通常指示の発行前に、共有承認の判断を保留する。
    /// # Errors
    /// 同じ操作が既に確定した場合。
    pub fn prepare_invalidation(
        &mut self,
        invalidation: super::PlanInvalidation,
        at: DateTime<Utc>,
    ) -> Result<PlanApprovalEvent, PlanRuntimeError> {
        if self.generations.pending().next().is_some() {
            return Err(PlanRuntimeError::PendingGeneration);
        }
        if self.answers.pending().next().is_some() {
            return Err(PlanRuntimeError::PendingAnswer);
        }
        if !self.pending_responses.is_empty() {
            return Err(PlanRuntimeError::PendingResponse);
        }
        self.require_unused(invalidation.id())?;
        let event = PlanApprovalEvent::InvalidationPrepared(super::PlanInvalidationPrepared::new(
            PlanApprovalEventId::generate(),
            self.id,
            invalidation,
        ));
        self.apply_event(&event, self.seq_nr + 1, at);
        Ok(event)
    }

    /// 回復すべき先行発行の集合。
    #[must_use]
    pub const fn invalidations(&self) -> &super::PlanInvalidations {
        &self.invalidations
    }

    /// 観測時の発行回を指定し、人間応答をその提示だけへ照合する。
    /// # Errors
    /// 応答記録が成立しない場合。
    pub fn observe_response(
        &mut self,
        id: PlanApprovalOperationId,
        occurrence: &PlanApprovalOperationId,
        session: &super::PlanSession,
        response: &str,
        at: DateTime<Utc>,
    ) -> Result<PlanApprovalEvent, PlanRuntimeError> {
        if self.generations.pending().next().is_some() {
            return Err(PlanRuntimeError::PendingGeneration);
        }
        if self.answers.pending().next().is_some() {
            return Err(PlanRuntimeError::PendingAnswer);
        }
        if !self.invalidations.is_empty() {
            return Err(PlanRuntimeError::PendingInvalidation);
        }
        self.require_unused(&id)?;
        if !self.pending_responses.is_empty() {
            return Err(PlanRuntimeError::PendingResponse);
        }
        let choice = self
            .challenges
            .for_session(session)
            .filter(|issued| issued.id() == occurrence)
            .and_then(|issued| issued.challenge().offered_choice(response));
        let event = PlanApprovalEvent::ResponseObserved(super::PlanResponseObserved::new(
            PlanApprovalEventId::generate(),
            self.id,
            id,
            occurrence.clone(),
            session.clone(),
            core_infrastructure::hash::sha256_hex(
                core_infrastructure::ecmascript::trim(response).as_bytes(),
            ),
            choice,
        ));
        self.apply_event(&event, self.seq_nr + 1, at);
        Ok(event)
    }

    /// 初めて共有承認の所有者を作る。
    #[must_use]
    pub fn create(at: DateTime<Utc>) -> (Self, PlanApprovalEvent) {
        let created = PlanRuntimeCreated::new(
            PlanApprovalEventId::generate(),
            PlanApprovalRuntimeId::Workspace,
        );
        (
            Self::from((created.clone(), at)),
            PlanApprovalEvent::Created(created),
        )
    }
    /// 検証済みの提示を、新しい発行回として記録する。
    /// # Errors
    /// 発行が成立しない場合。
    pub fn issue_challenge(
        &mut self,
        operation_id: PlanApprovalOperationId,
        challenge: PlanChallenge,
        at: DateTime<Utc>,
    ) -> Result<PlanApprovalEvent, PlanRuntimeError> {
        if self.generations.pending().next().is_some() {
            return Err(PlanRuntimeError::PendingGeneration);
        }
        if self.answers.pending().next().is_some() {
            return Err(PlanRuntimeError::PendingAnswer);
        }
        if !self.invalidations.is_empty() {
            return Err(PlanRuntimeError::PendingInvalidation);
        }
        if !self.pending_responses.is_empty() {
            return Err(PlanRuntimeError::PendingResponse);
        }
        self.require_unused(&operation_id)?;
        let event = PlanApprovalEvent::ChallengeIssued(Box::new(PlanChallengeIssued::new(
            PlanApprovalEventId::generate(),
            self.id,
            PlanChallengeOccurrence::new(operation_id, challenge),
        )));
        self.apply_event(&event, self.seq_nr + 1, at);
        Ok(event)
    }
    /// 記録済みの事実を同じ状態遷移へ適用する。
    /// # Panics
    /// 別集約・通番の欠落・誕生の二重適用という壊れた履歴を拒否する。
    #[allow(
        clippy::panic,
        clippy::expect_used,
        reason = "再生中の誕生重複や不正な応答は破損した履歴なのでクラッシュする（aggregate-commands.md）"
    )]
    pub fn apply_event(&mut self, event: &PlanApprovalEvent, seq_nr: usize, at: DateTime<Utc>) {
        assert_eq!(event.aggregate_id(), &self.id, "foreign approval runtime");
        assert_eq!(seq_nr, self.seq_nr + 1, "approval sequence gap");
        match event {
            PlanApprovalEvent::Created(_) => panic!("approval runtime already exists"),
            PlanApprovalEvent::GenerationCertified(certified) => {
                let pending = self
                    .generations
                    .get(certified.operation_id())
                    .expect("generation request exists");
                assert_eq!(
                    pending.state(),
                    super::PlanGenerationState::Pending,
                    "generation is pending"
                );
                self.generations
                    .apply_certification(certified)
                    .expect("recorded generation certification matches its pending operation");
                self.applied.insert(certified.operation_id().clone());
            }
            PlanApprovalEvent::GenerationRevoked(revoked) => {
                let pending = self
                    .generations
                    .get(revoked.operation_id())
                    .expect("generation request exists");
                assert_eq!(
                    pending.state(),
                    super::PlanGenerationState::Pending,
                    "generation is pending"
                );
                self.receipts.remove(&pending.receipt().key());
                self.generations
                    .apply_revocation(revoked)
                    .expect("recorded generation revocation matches its pending operation");
                self.applied.insert(revoked.operation_id().clone());
            }
            PlanApprovalEvent::GenerationRequested(requested) => {
                let generation = requested.generation();
                assert!(
                    self.require_unused(generation.id()).is_ok(),
                    "generation operation already applied"
                );
                self.receipts.insert(generation.receipt().clone());
                if generation.state() == super::PlanGenerationState::Active {
                    self.applied.insert(generation.id().clone());
                }
                self.generations.insert(generation.clone());
            }
            PlanApprovalEvent::ChallengeIssued(issued) => {
                assert!(
                    self.require_unused(issued.occurrence().id()).is_ok(),
                    "approval operation already applied"
                );
                self.applied.insert(issued.occurrence().id().clone());
                self.challenges.issue(issued.occurrence().clone());
            }
            PlanApprovalEvent::InvalidationPrepared(prepared) => {
                assert!(
                    self.require_unused(prepared.invalidation().id()).is_ok(),
                    "approval operation already applied"
                );
                self.invalidations.insert(prepared.invalidation().clone())
            }
            PlanApprovalEvent::InvalidationResolved(resolved) => {
                assert!(
                    self.invalidations.contains(resolved.operation_id()),
                    "invalidation was prepared"
                );
                self.invalidations.remove(resolved.operation_id());
                if *resolved.published() {
                    self.receipts.clear();
                    self.challenges = PlanChallenges::default();
                }
                self.applied.insert(resolved.operation_id().clone());
            }
            PlanApprovalEvent::AnswerAborted(aborted) => {
                let answer = self
                    .answers
                    .get(aborted.operation_id())
                    .expect("aborted answer exists");
                assert_eq!(
                    answer.state(),
                    &super::PlanAnswerState::Pending,
                    "answer remains pending"
                );
                let receipt = answer
                    .receipt()
                    .expect("aborted answer has a source certification");
                self.receipts.remove(&receipt.key());
                self.answers.abort(
                    aborted.operation_id(),
                    PlanRuntimeError::ReceiptSourceChanged.to_string(),
                );
                self.applied.insert(aborted.operation_id().clone());
            }
            PlanApprovalEvent::AnswerCompleted(completed) => {
                let answer = self
                    .answers
                    .get(completed.operation_id())
                    .expect("completed answer exists");
                assert_eq!(
                    answer.state(),
                    &super::PlanAnswerState::Pending,
                    "answer remains pending"
                );
                self.challenges
                    .clear_occurrence(answer.input().decision().session(), answer.occurrence_id());
                self.answers.record(completed.operation_id());
                self.applied.insert(completed.operation_id().clone());
            }
            PlanApprovalEvent::AnswerRecorded(recorded) => {
                assert!(
                    self.require_unused(recorded.answer().id()).is_ok(),
                    "approval operation already applied"
                );
                if let Some(receipt) = recorded.answer().receipt() {
                    self.receipts.insert(receipt.clone());
                }
                self.answers.insert(recorded.answer().clone());
            }
            PlanApprovalEvent::ResponsePrepared(prepared) => {
                assert!(
                    self.require_unused(prepared.preparation().id()).is_ok(),
                    "approval operation already applied"
                );
                self.pending_responses
                    .insert(prepared.preparation().clone());
            }
            PlanApprovalEvent::ResponseObserved(observed) => {
                if let Some(pending) = self.pending_responses.get(observed.observation_id()) {
                    assert_eq!(
                        pending.occurrence_id(),
                        observed.occurrence_id(),
                        "observed issuance remains fixed"
                    );
                    assert_eq!(
                        pending.session(),
                        observed.session(),
                        "observed session remains fixed"
                    );
                    assert_eq!(
                        core_infrastructure::hash::sha256_hex(
                            core_infrastructure::ecmascript::trim(pending.response()).as_bytes()
                        ),
                        *observed.response_sha256(),
                        "observed response remains fixed"
                    );
                    if let Some(choice) = observed.choice() {
                        assert_eq!(
                            pending.choice(),
                            Some(*choice),
                            "observed choice remains fixed"
                        );
                    }
                    self.pending_responses.remove(observed.observation_id());
                } else {
                    assert!(
                        self.require_unused(observed.observation_id()).is_ok(),
                        "approval operation already applied"
                    );
                }
                self.applied.insert(observed.observation_id().clone());
                if let Some(choice) = observed.choice() {
                    self.challenges.observe(
                        observed.session(),
                        super::PlanHumanResponse::new(
                            observed.observation_id().clone(),
                            observed.occurrence_id().clone(),
                            *choice,
                            observed.response_sha256().clone(),
                        )
                        .expect("recorded human response is valid"),
                    );
                }
            }
        }
        self.seq_nr = seq_nr;
        self.last_updated_at = at;
    }
    /// 保存済みの基底へ、それ以後の事実だけを適用する。
    /// # Panics
    /// [`Self::apply_event`] と同じ壊れた履歴の場合。
    #[must_use]
    pub fn replay(
        mut base: Self,
        events: impl IntoIterator<Item = (PlanApprovalEvent, usize, DateTime<Utc>)>,
    ) -> Self {
        for (event, sequence, at) in events {
            base.apply_event(&event, sequence, at);
        }
        base
    }
    fn require_unused(&self, id: &PlanApprovalOperationId) -> Result<(), PlanRuntimeError> {
        if self.generations.get(id).is_some()
            || self.applied.contains(id)
            || self.invalidations.contains(id)
            || self.pending_responses.get(id).is_some()
            || self.answers.get(id).is_some()
        {
            Err(PlanRuntimeError::OperationAlreadyApplied)
        } else {
            Ok(())
        }
    }
    /// 確定済み操作。回復で同じ操作を再実行しないための履歴。
    #[must_use]
    pub const fn applied_operations(&self) -> &super::PlanAppliedOperations {
        &self.applied
    }
    /// 現在の提示集合。
    #[must_use]
    pub const fn challenges(&self) -> &PlanChallenges {
        &self.challenges
    }
    /// 集約の識別子。
    #[must_use]
    pub const fn id(&self) -> &PlanApprovalRuntimeId {
        &self.id
    }
    /// 適用済み事実の通番。
    #[must_use]
    pub const fn seq_nr(&self) -> usize {
        self.seq_nr
    }
    /// 最後の発生時刻。
    #[must_use]
    pub const fn last_updated_at(&self) -> DateTime<Utc> {
        self.last_updated_at
    }
}
impl From<(PlanRuntimeCreated, DateTime<Utc>)> for PlanApprovalRuntime {
    #[allow(
        clippy::expect_used,
        reason = "誕生イベントから導く空状態は構築規則を満たす。破損した履歴は停止する（aggregate-commands.md）"
    )]
    fn from((created, at): (PlanRuntimeCreated, DateTime<Utc>)) -> Self {
        Self::new(
            super::PlanGenerations::default(),
            super::PlanAnswers::default(),
            super::PlanReceipts::default(),
            super::PlanPendingResponses::default(),
            *created.aggregate_id(),
            PlanChallenges::default(),
            super::PlanInvalidations::default(),
            super::PlanAppliedOperations::default(),
            1,
            at,
        )
        .expect("recorded approval genesis is valid")
    }
}
