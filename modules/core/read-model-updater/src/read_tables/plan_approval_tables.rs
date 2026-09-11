//! 共有承認の事実から構築する操作のリードモデル。
use super::PlanApprovalOperationRow;
use crate::orchestration::{CorruptCause, JournalReadError, PlanApprovalJournalEntry};
use core_command_domain::orchestration::{PlanApprovalEvent, PlanApprovalRuntime};
/// 操作IDで読む行集合と、その出所のイベント位置。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanApprovalTables {
    directory_present: bool,
    generations: Vec<super::PlanGenerationRow>,
    answers: Vec<super::PlanAnswerRow>,
    rows: Vec<PlanApprovalOperationRow>,
    files: Vec<super::PlanApprovalFile>,
    sequence: usize,
    anchor_event_id: Option<String>,
}
impl PlanApprovalTables {
    const fn new(
        rows: Vec<PlanApprovalOperationRow>,
        files: Vec<super::PlanApprovalFile>,
        answers: Vec<super::PlanAnswerRow>,
        generations: Vec<super::PlanGenerationRow>,
        sequence: usize,
        anchor_event_id: Option<String>,
        directory_present: bool,
    ) -> Self {
        Self {
            directory_present,
            generations,
            answers,
            rows,
            files,
            sequence,
            anchor_event_id,
        }
    }
    /// 操作IDを主キーとする実装開始結果。
    #[must_use]
    pub fn generations(&self) -> &[super::PlanGenerationRow] {
        &self.generations
    }
    /// 操作IDを主キーとする回答結果の投影行。
    #[must_use]
    pub fn answers(&self) -> &[super::PlanAnswerRow] {
        &self.answers
    }

    /// 履歴から集約を再生して操作を投影する。
    /// # Errors
    /// 誕生のない履歴・通番の欠落。
    /// # Panics
    /// ドメインの不変条件を破る履歴は、集約の再生規則に従って停止する。
    pub fn project(entries: &[PlanApprovalJournalEntry]) -> Result<Self, JournalReadError> {
        let corrupt = || JournalReadError::Corrupt {
            aggregate_id: "workspace".to_string(),
            seq_nr: None,
            cause: CorruptCause::InvariantViolation,
        };
        let Some((first, delta)) = entries.split_first() else {
            return Ok(Self::new(
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                0,
                None,
                false,
            ));
        };
        let PlanApprovalEvent::Created(created) = first.event() else {
            return Err(corrupt());
        };
        if first.sequence() != 1
            || entries
                .iter()
                .enumerate()
                .any(|(index, entry)| entry.sequence() != index + 1)
        {
            return Err(corrupt());
        }
        let runtime = PlanApprovalRuntime::replay(
            PlanApprovalRuntime::from((created.clone(), first.occurred_at())),
            delta
                .iter()
                .map(|entry| (entry.event().clone(), entry.sequence(), entry.occurred_at())),
        );
        let mut rows: Vec<_> = runtime
            .applied_operations()
            .iter()
            .map(|id| {
                PlanApprovalOperationRow::new(
                    id.to_string(),
                    "applied".to_string(),
                    None,
                    None,
                    "completed".to_string(),
                )
            })
            .collect();
        rows.extend(runtime.invalidations().iter().map(|pending| {
            PlanApprovalOperationRow::new(
                pending.id().to_string(),
                "prepared".to_string(),
                Some(pending.space().as_str().to_string()),
                Some(pending.execution_id().as_str().to_string()),
                "publication".to_string(),
            )
        }));
        rows.extend(runtime.pending_responses().iter().map(|pending| {
            PlanApprovalOperationRow::new(
                pending.id().to_string(),
                "prepared".to_string(),
                Some(pending.origin().space().as_str().to_string()),
                Some(pending.origin().execution_id().as_str().to_string()),
                "response".to_string(),
            )
        }));
        rows.extend(runtime.answers().pending().map(|answer| {
            PlanApprovalOperationRow::new(
                answer.id().to_string(),
                "prepared".to_string(),
                Some(answer.input().origin().space().as_str().to_string()),
                Some(answer.input().origin().execution_id().as_str().to_string()),
                "answer".to_string(),
            )
        }));
        rows.extend(runtime.generations().pending().map(|generation| {
            PlanApprovalOperationRow::new(
                generation.id().to_string(),
                "prepared".to_string(),
                None,
                None,
                "generation".to_string(),
            )
        }));
        rows.sort_by(|a, b| a.id().cmp(b.id()));
        let files = runtime
            .challenges()
            .iter()
            .flat_map(|occurrence| {
                let mut files = vec![super::PlanApprovalFile::challenge(occurrence.challenge())];
                if let Some(response) = occurrence.response() {
                    files.push(super::PlanApprovalFile::response(
                        occurrence.challenge(),
                        response,
                    ));
                }
                files
            })
            .chain(
                runtime
                    .receipts()
                    .iter()
                    .map(super::PlanApprovalFile::receipt),
            )
            .collect();
        let answers = runtime
            .answers()
            .iter()
            .map(|answer| {
                use core_command_domain::orchestration::{PlanAnswerState, PlanChoice};
                let (status, emitted, error) = match answer.state() {
                    PlanAnswerState::Pending => ("pending", None, None),
                    PlanAnswerState::Aborted(error) => ("aborted", None, Some(error.clone())),
                    PlanAnswerState::Recorded => (
                        "recorded",
                        Some(
                            match answer.input().choice() {
                                PlanChoice::ApprovePlan => "PLAN_APPROVAL_RECORDED",
                                PlanChoice::RequestChanges => "QUESTION_ANSWERED",
                            }
                            .to_string(),
                        ),
                        None,
                    ),
                };
                super::PlanAnswerRow::new(
                    answer.id().to_string(),
                    status.to_string(),
                    emitted,
                    answer.input().stage().to_string(),
                    error,
                )
            })
            .collect();
        let generations = runtime
            .generations()
            .iter()
            .map(|generation| {
                use core_command_domain::orchestration::PlanGenerationState;
                let (status, error) = match generation.state() {
                    PlanGenerationState::Pending => ("pending", None),
                    PlanGenerationState::Active => ("generation", None),
                    PlanGenerationState::Revoked => (
                        "revoked",
                        Some(
                            "workspace source changed while Code Generation authority was starting"
                                .to_string(),
                        ),
                    ),
                };
                super::PlanGenerationRow::new(
                    generation.id().to_string(),
                    status.to_string(),
                    generation
                        .receipt()
                        .decision()
                        .evidence()
                        .authority()
                        .unit()
                        .map(str::to_string),
                    error,
                )
            })
            .collect();
        Ok(Self::new(
            rows,
            files,
            answers,
            generations,
            runtime.seq_nr(),
            entries.last().map(|entry| entry.event().id().to_string()),
            directory_present(entries),
        ))
    }
    /// 受領単体の除去後は空ディレクトリを残し、全体失効後は除く。
    #[must_use]
    pub const fn directory_present(&self) -> bool {
        self.directory_present
    }
    /// 本家互換の機械ローカルなファイル投影。
    #[must_use]
    pub fn files(&self) -> &[super::PlanApprovalFile] {
        &self.files
    }
    /// 投影する行。
    #[must_use]
    pub fn rows(&self) -> &[PlanApprovalOperationRow] {
        &self.rows
    }
    /// 投影元の通番。
    #[must_use]
    pub const fn sequence(&self) -> usize {
        self.sequence
    }
    /// 投影元の最後のイベント。
    #[must_use]
    pub fn anchor_event_id(&self) -> Option<&str> {
        self.anchor_event_id.as_deref()
    }
}

// 固定本家 aidlc-lib.ts の writePlanApprovalReceipt / clearPlanApprovalReceipt /
// resetPlanApprovalRuntime。空かどうかだけでは削除と保持を区別できない。
fn directory_present(entries: &[PlanApprovalJournalEntry]) -> bool {
    entries
        .iter()
        .fold(false, |present, entry| match entry.event() {
            PlanApprovalEvent::Created(_) => false,
            PlanApprovalEvent::InvalidationResolved(resolved) => present && !*resolved.published(),
            PlanApprovalEvent::ChallengeIssued(_) | PlanApprovalEvent::GenerationRequested(_) => {
                true
            }
            PlanApprovalEvent::AnswerRecorded(recorded) => {
                present || recorded.answer().receipt().is_some()
            }
            PlanApprovalEvent::InvalidationPrepared(_)
            | PlanApprovalEvent::ResponsePrepared(_)
            | PlanApprovalEvent::ResponseObserved(_)
            | PlanApprovalEvent::AnswerCompleted(_)
            | PlanApprovalEvent::AnswerAborted(_)
            | PlanApprovalEvent::GenerationCertified(_)
            | PlanApprovalEvent::GenerationRevoked(_) => present,
        })
}
