//! コード生成の現在の試行を識別する境界。
use super::RunBoundaryKind;
use chrono::{DateTime, SecondsFormat, Utc};
/// 監査ファイルを権限の正本にせず、集約の事実から持つ実行境界。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeGenerationRunFloor {
    workflow_started: u64,
    stage_started: u64,
    stage_jumped: u64,
    gate_rejected: u64,
    latest: Option<(RunBoundaryKind, DateTime<Utc>)>,
}
impl Default for CodeGenerationRunFloor {
    fn default() -> Self {
        Self::of_counts(0, 0, 0, 0, None)
    }
}
impl CodeGenerationRunFloor {
    const fn of_counts(
        workflow_started: u64,
        stage_started: u64,
        stage_jumped: u64,
        gate_rejected: u64,
        latest: Option<(RunBoundaryKind, DateTime<Utc>)>,
    ) -> Self {
        Self {
            workflow_started,
            stage_started,
            stage_jumped,
            gate_rejected,
            latest,
        }
    }

    /// 保存された境界を検査して再構成する。
    /// # Errors
    /// 最新境界と発生回数が対応しない場合。
    pub const fn new(
        workflow_started: u64,
        stage_started: u64,
        stage_jumped: u64,
        gate_rejected: u64,
        latest: Option<(RunBoundaryKind, DateTime<Utc>)>,
    ) -> Result<Self, super::RunFloorError> {
        let value = Self::of_counts(
            workflow_started,
            stage_started,
            stage_jumped,
            gate_rejected,
            latest,
        );
        let valid = match value.latest {
            Some((kind, _)) => value.count(kind) > 0,
            None => {
                workflow_started == 0
                    && stage_started == 0
                    && stage_jumped == 0
                    && gate_rejected == 0
            }
        };
        if valid {
            Ok(value)
        } else {
            Err(super::RunFloorError)
        }
    }
    /// 境界種類ごとの通し番号。
    #[must_use]
    pub const fn count(&self, kind: RunBoundaryKind) -> u64 {
        match kind {
            RunBoundaryKind::WorkflowStarted => self.workflow_started,
            RunBoundaryKind::StageStarted => self.stage_started,
            RunBoundaryKind::StageJumped => self.stage_jumped,
            RunBoundaryKind::GateRejected => self.gate_rejected,
        }
    }
    /// 最後に発生した境界。
    #[must_use]
    pub const fn latest(&self) -> Option<(RunBoundaryKind, DateTime<Utc>)> {
        self.latest
    }
    /// 現在の境界の公開識別子。
    #[must_use]
    pub fn rendered(&self) -> String {
        self.latest.map_or_else(
            || "unstarted#0".to_string(),
            |(kind, at)| {
                format!(
                    "{}:{}#{}",
                    kind.as_str(),
                    at.to_rfc3339_opts(SecondsFormat::Secs, true),
                    self.count(kind)
                )
            },
        )
    }
    pub(crate) fn record(
        &mut self,
        kind: RunBoundaryKind,
        at: DateTime<Utc>,
    ) -> Result<(), super::apply_error::ApplyError> {
        let count = self.count(kind).checked_add(1).ok_or_else(|| {
            super::apply_error::ApplyError::InvariantViolation(
                "run boundary count exhausted".to_string(),
            )
        })?;
        match kind {
            RunBoundaryKind::WorkflowStarted => self.workflow_started = count,
            RunBoundaryKind::StageStarted => self.stage_started = count,
            RunBoundaryKind::StageJumped => self.stage_jumped = count,
            RunBoundaryKind::GateRejected => self.gate_rejected = count,
        }
        self.latest = Some((kind, at));
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn run_boundaries_match_the_fixed_upstream_observations() {
        let corpus: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../../../tests/golden/selfhost-stage1/run-floor.json"
        ))
        .unwrap();
        assert_eq!(
            corpus
                .get("source")
                .unwrap()
                .get("commit")
                .unwrap()
                .as_str(),
            Some("a277af218f0df7f325d3b8be7b6d90fce2c5bd40")
        );
        let cases = corpus.get("observations").unwrap().as_array().unwrap();
        assert!(!cases.is_empty());
        for case in cases {
            let mut floor = CodeGenerationRunFloor::default();
            for event in case.get("events").unwrap().as_array().unwrap() {
                let spelling = event.get("event").unwrap().as_str().unwrap();
                let kind = [
                    RunBoundaryKind::WorkflowStarted,
                    RunBoundaryKind::StageStarted,
                    RunBoundaryKind::StageJumped,
                    RunBoundaryKind::GateRejected,
                ]
                .into_iter()
                .find(|kind| kind.as_str() == spelling)
                .unwrap();
                let at =
                    DateTime::parse_from_rfc3339(event.get("timestamp").unwrap().as_str().unwrap())
                        .unwrap()
                        .with_timezone(&Utc);
                floor.record(kind, at).unwrap();
            }
            assert_eq!(
                floor.rendered(),
                case.get("floor").unwrap().as_str().unwrap(),
                "{}",
                case.get("id").unwrap()
            );
        }
    }
}
