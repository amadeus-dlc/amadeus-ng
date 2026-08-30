//! `RehydratedIntentExecution` — 再構成した集約と、ストアが載せた楽観 version。

use core_command_domain::orchestration::IntentExecution;

/// [`IntentExecutionRepository::find_by_id`] が返す再水和の結果。
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
/// そのまま [`IntentExecutionRepository::store`] へ返すだけである (BR5.3)。
///
/// フィールドは private。読取は境界越えのアクセサで公開する (field-visibility.md)。
///
/// [`IntentExecutionRepository::find_by_id`]: super::intent_execution_repository::IntentExecutionRepository::find_by_id
/// [`IntentExecutionRepository::store`]: super::intent_execution_repository::IntentExecutionRepository::store
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RehydratedIntentExecution {
    aggregate: IntentExecution,
    version: usize,
}

impl RehydratedIntentExecution {
    /// 再構成した集約と、その時点でストアに載っていた版を束ねる。
    #[must_use]
    pub const fn new(aggregate: IntentExecution, version: usize) -> RehydratedIntentExecution {
        RehydratedIntentExecution { aggregate, version }
    }

    /// 再構成した集約。
    #[must_use]
    pub const fn aggregate(&self) -> &IntentExecution {
        &self.aggregate
    }

    /// 次の書込に提示する楽観 version (ストアが採番した不透明トークン)。
    ///
    /// `usize` で運ぶが数ではない — 解釈も比較も算術もせず、そのまま
    /// [`IntentExecutionRepository::store`] へ渡す。`seq_nr` と混同してはならず、集約へ
    /// 入れてもならない。
    ///
    /// [`IntentExecutionRepository::store`]: super::intent_execution_repository::IntentExecutionRepository::store
    #[must_use]
    pub const fn version(&self) -> usize {
        self.version
    }

    /// 集約の所有権を取り出す (コマンドを打つ側は `&mut` が要る)。
    #[must_use]
    pub fn into_aggregate(self) -> IntentExecution {
        self.aggregate
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core_command_domain::orchestration::{Created, StageDisplay, WorkspaceScan};
    use core_command_domain::workflow_definition::{BrownfieldGreenfield, StageNumber};
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
    use core_command_domain::orchestration::{
        Intent, IntentExecutionId, IntentId, StageEntry, StartRequest,
    };
    use core_command_domain::workflow_definition::{
        DefinitionRevision, PhaseId, PlanAction, StageSlug, WorkflowDefinitionId,
    };

    fn intent() -> Intent {
        Intent::from(Created::new(
            IntentId::parse("01a02785-1bd8-76eb-aeea-5aa303ebd5b6").unwrap(),
            WorkflowDefinitionId::parse("claude").unwrap(),
            DefinitionRevision::parse(&format!("sha256:{}", "0".repeat(64))).unwrap(),
            StartRequest::new("classic", "rehydration"),
            vec![StageEntry::new(
                StageSlug::parse("state-init").unwrap(),
                PhaseId::Initialization,
                PlanAction::Execute,
                false,
                display("0.1"),
            )],
            scan(),
        ))
    }

    fn aggregate() -> IntentExecution {
        IntentExecution::start(
            IntentExecutionId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000").unwrap(),
            intent(),
            DateTime::<Utc>::UNIX_EPOCH,
        )
        .0
    }

    #[test]
    fn the_result_carries_the_aggregate_and_the_version_the_store_assigned() {
        let rehydrated = RehydratedIntentExecution::new(aggregate(), 4);
        assert_eq!(rehydrated.aggregate(), &aggregate());
        assert_eq!(rehydrated.version(), 4);
    }

    #[test]
    fn the_aggregate_can_be_taken_out_to_receive_commands() {
        let rehydrated = RehydratedIntentExecution::new(aggregate(), 4);
        let mut taken = rehydrated.into_aggregate();
        assert!(
            taken
                .complete_stage(&intent(), DateTime::<Utc>::UNIX_EPOCH)
                .is_ok()
        );
    }

    #[test]
    fn results_compare_by_value() {
        assert_eq!(
            RehydratedIntentExecution::new(aggregate(), 1),
            RehydratedIntentExecution::new(aggregate(), 1)
        );
        assert_ne!(
            RehydratedIntentExecution::new(aggregate(), 1),
            RehydratedIntentExecution::new(aggregate(), 2)
        );
    }
}
