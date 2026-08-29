//! `RehydratedWorkflowExecution` — 再構成した集約と、ストアが載せた楽観 version。

use core_domain::orchestration::WorkflowExecution;

/// [`WorkflowExecutionRepository::find_by_id`] が返す再水和の結果。
///
/// 集約そのものと、**次の書込に提示する楽観 version** を対で運ぶ。
///
/// # なぜ集約が version を持たないのか (ADR-010 / B7)
///
/// 楽観ロックの版数を採番するのはストアであり、正本はスナップショット行の列
/// (本家 v3 の `SnapshotEnvelope::version()`) である。集約が持つ順序番号は `seq_nr` だけで、
/// ストアの採番トークンと混ざらない (オーナー裁定「seq_nr と version を混ぜない」)。
/// その代わり、**版を握るのは再水和した呼出側**になる — 読んだ時点の版を書込に提示するから
/// こそ、その間に他者が書いた場合に競合として弾ける。version をストアの中で読み直すと、
/// 常に最新値が提示されることになり楽観ロックが成立しない。
///
/// `version` は**不透明なトークン**である。我々は解釈も比較も算術もしない — 読んだ値を
/// そのまま [`WorkflowExecutionRepository::store`] へ返すだけである (BR5.3)。
///
/// フィールドは private。読取は境界越えのアクセサで公開する (field-visibility.md)。
///
/// [`WorkflowExecutionRepository::find_by_id`]: super::workflow_execution_repository::WorkflowExecutionRepository::find_by_id
/// [`WorkflowExecutionRepository::store`]: super::workflow_execution_repository::WorkflowExecutionRepository::store
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RehydratedWorkflowExecution {
    aggregate: WorkflowExecution,
    version: usize,
}

impl RehydratedWorkflowExecution {
    /// 再構成した集約と、その時点でストアに載っていた版を束ねる。
    #[must_use]
    pub const fn new(aggregate: WorkflowExecution, version: usize) -> RehydratedWorkflowExecution {
        RehydratedWorkflowExecution { aggregate, version }
    }

    /// 再構成した集約。
    #[must_use]
    pub const fn aggregate(&self) -> &WorkflowExecution {
        &self.aggregate
    }

    /// 次の書込に提示する楽観 version (ストアが採番した不透明トークン)。
    ///
    /// `usize` で運ぶが数ではない — 解釈も比較も算術もせず、そのまま
    /// [`WorkflowExecutionRepository::store`] へ渡す。`seq_nr` と混同してはならず、集約へ
    /// 入れてもならない。
    ///
    /// [`WorkflowExecutionRepository::store`]: super::workflow_execution_repository::WorkflowExecutionRepository::store
    #[must_use]
    pub const fn version(&self) -> usize {
        self.version
    }

    /// 集約の所有権を取り出す (コマンドを打つ側は `&mut` が要る)。
    #[must_use]
    pub fn into_aggregate(self) -> WorkflowExecution {
        self.aggregate
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core_domain::orchestration::{StageDisplay, WorkspaceScan};
    use core_domain::workflow_definition::{BrownfieldGreenfield, StageNumber};
    fn display(number: &str) -> StageDisplay {
        StageDisplay::new(StageNumber::parse(number).unwrap(), "Stage", "orchestrator").unwrap()
    }

    fn scan() -> WorkspaceScan {
        WorkspaceScan::new(
            BrownfieldGreenfield::Greenfield,
            "Unknown",
            "Unknown",
            "Unknown",
        )
        .unwrap()
    }

    use chrono::{DateTime, Utc};
    use core_domain::orchestration::{IntentId, StageEntry, StartRequest};
    use core_domain::workflow_definition::{
        DefinitionRevision, PhaseId, PlanAction, StageSlug, WorkflowDefinitionId,
    };

    fn aggregate() -> WorkflowExecution {
        WorkflowExecution::start_from_plan_unchecked(
            IntentId::parse("01a02785-1bd8-76eb-aeea-5aa303ebd5b6").unwrap(),
            WorkflowDefinitionId::parse("claude").unwrap(),
            DefinitionRevision::parse(&format!("sha256:{}", "0".repeat(64))).unwrap(),
            &StartRequest::new("classic", "rehydration"),
            vec![StageEntry::new(
                StageSlug::parse("state-init").unwrap(),
                PhaseId::Initialization,
                PlanAction::Execute,
                false,
                display("0.1"),
            )],
            scan(),
            DateTime::<Utc>::UNIX_EPOCH,
        )
        .unwrap()
        .0
    }

    #[test]
    fn the_result_carries_the_aggregate_and_the_version_the_store_assigned() {
        let rehydrated = RehydratedWorkflowExecution::new(aggregate(), 4);
        assert_eq!(rehydrated.aggregate(), &aggregate());
        assert_eq!(rehydrated.version(), 4);
    }

    #[test]
    fn the_aggregate_can_be_taken_out_to_receive_commands() {
        let rehydrated = RehydratedWorkflowExecution::new(aggregate(), 4);
        let mut taken = rehydrated.into_aggregate();
        assert!(taken.complete_stage(DateTime::<Utc>::UNIX_EPOCH).is_ok());
    }

    #[test]
    fn results_compare_by_value() {
        assert_eq!(
            RehydratedWorkflowExecution::new(aggregate(), 1),
            RehydratedWorkflowExecution::new(aggregate(), 1)
        );
        assert_ne!(
            RehydratedWorkflowExecution::new(aggregate(), 1),
            RehydratedWorkflowExecution::new(aggregate(), 2)
        );
    }
}
