//! Pipelineの試行境界と完了受領を所有する一級コレクション。
use super::{
    IntentExecutionEvent, IntentExecutionId, PipelineLinkError, PipelineLinkRequest,
    PipelineReceipt, PipelineRecord,
};
use crate::workflow_definition::StageNode;
use chrono::{DateTime, Utc};
/// 順序・重複・現在の試行・引継ぎの鮮度を判断する履歴。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PipelineHistory {
    records: Vec<PipelineRecord>,
}
impl Default for PipelineHistory {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}
impl PipelineHistory {
    /// 直近の単独試行が開始済みで未完了か。
    #[must_use]
    pub fn single_is_open(&self, stage_name: &str) -> bool {
        self.records
            .iter()
            .rev()
            .find_map(|record| match record {
                PipelineRecord::Boundary {
                    stage: Some(stage),
                    single: true,
                    ..
                } if stage == stage_name => Some(true),
                PipelineRecord::Closed(stage) if stage == stage_name => Some(false),
                _ => None,
            })
            .unwrap_or(false)
    }

    /// 保存された時系列を復元する。
    #[must_use]
    pub const fn new(records: Vec<PipelineRecord>) -> Self {
        Self { records }
    }
    /// 最初の通常実行境界を記録した履歴。
    #[must_use]
    pub fn started(at: DateTime<Utc>) -> Self {
        Self::new(vec![PipelineRecord::Boundary {
            stage: None,
            single: false,
            at,
        }])
    }
    /// 保存境界で時系列を畳み込む。
    pub fn fold_left<T>(&self, initial: T, fold: impl FnMut(T, &PipelineRecord) -> T) -> T {
        self.records.iter().fold(initial, fold)
    }
    pub(super) fn belongs_to(&self, id: &IntentExecutionId) -> bool {
        self.records.iter().all(|record| match record {
            PipelineRecord::Completed(e) => e.aggregate_id() == id,
            _ => true,
        })
    }
    pub(super) fn observe(
        &mut self,
        event: &IntentExecutionEvent,
        next_stage: Option<String>,
        at: DateTime<Utc>,
    ) {
        match event {
            IntentExecutionEvent::PipelineLinkCompleted(e) => {
                self.records.push(PipelineRecord::Completed(e.clone()))
            }
            IntentExecutionEvent::Jumped(_) => self.boundary(None, false, at),
            IntentExecutionEvent::GateRejected(e) => {
                self.boundary(Some(e.stage().as_str().into()), false, at)
            }
            IntentExecutionEvent::Reported(e) => {
                if let super::ReportResult::Committed {
                    stage,
                    transition: super::ReportTransition::GateRejected { .. },
                    ..
                } = e.result()
                {
                    self.boundary(Some(stage.as_str().into()), false, at)
                }
            }
            IntentExecutionEvent::SingleStageRunStarted(e) => {
                self.boundary(Some(e.stage().as_str().into()), true, at)
            }
            IntentExecutionEvent::SingleStageRunCommitted(e) => {
                self.records
                    .push(PipelineRecord::Closed(e.stage().as_str().into()));
            }
            _ => {}
        }
        if event.advancing_stage().is_some()
            && let Some(stage) = next_stage
        {
            self.boundary(Some(stage), false, at);
        }
    }
    fn boundary(&mut self, stage: Option<String>, single: bool, at: DateTime<Utc>) {
        self.records
            .push(PipelineRecord::Boundary { stage, single, at });
    }
    fn current_chain<'a>(
        &'a self,
        node: &StageNode,
        stage_name: &str,
        repo: Option<&str>,
        single: bool,
        current: Option<&super::PipelineHandoff>,
    ) -> (Option<usize>, Vec<&'a PipelineReceipt>) {
        let links = std::iter::once(node.lead_agent())
            .chain(node.support_agents().iter().map(String::as_str))
            .collect::<Vec<_>>();
        let floor = self.records.iter().rposition(|entry| match entry {
            PipelineRecord::Boundary {
                stage,
                single: boundary_single,
                ..
            } => {
                *boundary_single == single
                    && (stage.as_deref() == Some(stage_name) || stage.is_none() && !single)
            }
            _ => false,
        });
        let mut chain: Vec<&PipelineReceipt> = Vec::new();
        for entry in self.records.iter().skip(floor.map_or(0, |i| i + 1)) {
            let PipelineRecord::Completed(event) = entry else {
                continue;
            };
            let receipt = event.receipt();
            if receipt.stage() != stage_name
                || receipt.is_single() != single
                || receipt.repo() != repo
            {
                continue;
            }
            if receipt.link() == node.lead_agent() {
                chain.clear();
                if stage_name != "reverse-engineering"
                    || receipt
                        .handoff()
                        .zip(current)
                        .is_some_and(|(old, now)| old.matches_current(now))
                {
                    chain.push(receipt);
                }
            } else if !chain.is_empty()
                && links
                    .get(chain.len())
                    .is_some_and(|link| *link == receipt.link())
            {
                chain.push(receipt);
            }
        }
        (floor, chain)
    }
    /// 現在のファイル観測に一致する受領だけを、投影境界で畳み込む。
    pub fn fold_completed<T>(
        &self,
        node: &StageNode,
        current: Option<&super::PipelineHandoff>,
        single: bool,
        initial: T,
        fold: impl FnMut(T, &PipelineReceipt) -> T,
    ) -> T {
        let (_, chain) = self.current_chain(node, node.slug().as_str(), None, single, current);
        chain.into_iter().fold(initial, fold)
    }
    /// 現試行で不足するlink名。完了していればNone。
    #[must_use]
    pub fn missing(
        &self,
        node: &StageNode,
        current: Option<&super::PipelineHandoff>,
        single: bool,
    ) -> Option<String> {
        let (_, chain) = self.current_chain(node, node.slug().as_str(), None, single, current);
        let missing = std::iter::once(node.lead_agent())
            .chain(node.support_agents().iter().map(String::as_str))
            .filter(|link| !chain.iter().any(|receipt| receipt.link() == *link))
            .collect::<Vec<_>>();
        (!missing.is_empty()).then(|| missing.join(", "))
    }
    pub(super) fn accept(
        &self,
        request: &PipelineLinkRequest,
        node: &StageNode,
    ) -> Result<PipelineReceipt, PipelineLinkError> {
        use PipelineLinkError as E;
        let links = std::iter::once(node.lead_agent())
            .chain(node.support_agents().iter().map(String::as_str))
            .collect::<Vec<_>>();
        let position = links
            .iter()
            .position(|link| *link == request.link())
            .ok_or_else(|| E::UnknownLink {
                stage: request.stage().into(),
                link: request.link().into(),
                declared: links.join(", "),
            })?;
        if request.repo().is_some_and(|repo| !repo.is_empty()) {
            return Err(E::UnregisteredRepo {
                stage: request.stage().into(),
            });
        }
        let (floor, chain) = self.current_chain(
            node,
            request.stage(),
            request.repo(),
            request.is_single(),
            request.handoff().current(),
        );
        if chain.iter().any(|receipt| receipt.link() == request.link()) {
            return Err(E::Duplicate {
                stage: request.stage().into(),
                link: request.link().into(),
                repo: request.repo().map(str::to_string),
            });
        }
        if let Some(previous) = position.checked_sub(1).and_then(|i| links.get(i))
            && !chain.iter().any(|receipt| receipt.link() == *previous)
        {
            return Err(E::OutOfOrder {
                stage: request.stage().into(),
                link: request.link().into(),
                previous: (*previous).into(),
                position: position + 1,
                total: links.len(),
                repo: request.repo().map(str::to_string),
            });
        }
        let handoff = if request.stage() == "reverse-engineering"
            && request.link() == node.lead_agent()
        {
            let handoff = request.handoff().require()?;
            let started = floor
                .and_then(|i| self.records.get(i))
                .and_then(|entry| match entry {
                    PipelineRecord::Boundary { at, .. } => Some(*at),
                    _ => None,
                });
            if started.is_none_or(|at| !handoff.was_written_since(at)) {
                return Err(E::ArtifactStale {
                    path: handoff.path().into(),
                });
            }
            let prior = self
                .records
                .iter()
                .filter_map(|entry| match entry {
                    PipelineRecord::Completed(e) => Some(e.receipt()),
                    _ => None,
                })
                .filter(|receipt| {
                    receipt.stage() == request.stage()
                        && receipt.link() == request.link()
                        && receipt.repo() == request.repo()
                        && receipt.is_single() == request.is_single()
                })
                .filter_map(PipelineReceipt::handoff)
                .map(super::PipelineHandoff::millis)
                .reduce(f64::max);
            if prior.is_some_and(|prior| handoff.millis() <= prior) {
                return Err(E::ArtifactNotRewritten {
                    path: handoff.path().into(),
                });
            }
            Some(handoff)
        } else {
            None
        };
        PipelineReceipt::new(
            request.stage().into(),
            request.link().into(),
            request
                .repo()
                .filter(|repo| !repo.is_empty())
                .map(str::to_string),
            request.is_single(),
            position + 1,
            links.len(),
            handoff,
        )
    }
}
