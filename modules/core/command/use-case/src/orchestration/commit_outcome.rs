//! `CommitOutcome` — [`CommitVerdictUseCase::execute`] の成功 2 形。
//!
//! [`CommitVerdictUseCase::execute`]: super::CommitVerdictUseCase::execute

use core_command_domain::orchestration::{ReportNoOp, TransitionStep};
use core_command_domain::workflow_definition::StageSlug;

/// 報告が着地した形と、逐語文言を組むのに要る材料。
///
/// **ユースケースは文言を組まない** (`coding-rules/error-handling.md`) ので、出す側が
/// `Committed <subs> for "<slug>" (scope: <scope>)` を綴れるだけの材料をここで運ぶ。
/// 材料 (slug・scope・段の列・no-op の種別) はすべて集約の判断と intent から来ており、
/// 合成ルートがリードモデルを引き直す必要は無い。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommitOutcome {
    /// 遷移をコミットした。
    Committed {
        /// 作用したステージ。
        stage: StageSlug,
        /// その実行が進めている scope。
        scope: String,
        /// コミットした段の列 (逐語 `<subs joined by " + ">` の材料)。
        steps: Vec<TransitionStep>,
    },
    /// 何もコミットしなかった (**失敗ではない**)。
    NoOp {
        /// 報告されたステージ。
        stage: StageSlug,
        /// その実行が進めている scope。
        scope: String,
        /// 何もしなかった理由。
        no_op: ReportNoOp,
    },
}

#[cfg(test)]
mod tests {
    // panic! は「想定した変種でなければ即失敗」という検証用途で使っており、テスト失敗の
    // シグナルとして妥当なので許容する。
    #![allow(clippy::panic)]

    use super::*;

    fn slug(value: &str) -> StageSlug {
        StageSlug::parse(value).expect("フィクスチャの slug は文法内")
    }

    #[test]
    fn a_commit_carries_the_steps_in_the_order_they_were_committed() {
        let outcome = CommitOutcome::Committed {
            stage: slug("domain-design"),
            scope: "classic".to_string(),
            steps: vec![TransitionStep::GateStartRecovered, TransitionStep::Approve],
        };
        let CommitOutcome::Committed { steps, scope, .. } = &outcome else {
            panic!("Committed を期待した")
        };
        assert_eq!(scope, "classic");
        assert_eq!(
            steps
                .iter()
                .map(|step| step.subcommand())
                .collect::<Vec<_>>(),
            ["gate-start", "approve"]
        );
    }

    #[test]
    fn a_no_op_carries_the_reason_it_committed_nothing() {
        let outcome = CommitOutcome::NoOp {
            stage: slug("domain-design"),
            scope: "classic".to_string(),
            no_op: ReportNoOp::AlreadyAwaiting {
                stage: slug("domain-design"),
            },
        };
        assert!(matches!(
            outcome,
            CommitOutcome::NoOp {
                no_op: ReportNoOp::AlreadyAwaiting { .. },
                ..
            }
        ));
    }
}
