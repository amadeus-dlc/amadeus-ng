//! `read_next_answer` の行を組む投影 — `next` の答えを要求の形ごとに [`NextAnswerRow`] へ
//! 焼き込む。

use std::collections::BTreeSet;

use core_command_domain::orchestration::{GateDecision, Intent, IntentExecution, NextDecision};

use super::read_tables_error::ReadTablesError;
use super::request_kind::RequestKind;
use super::row_id;
use super::spelling;
use super::stage_lookup::slug_of;
use crate::orchestration::NextAnswerRow;

/// 1 つの要求の形に対する答えを 1 行へ写す。
///
/// `intent` は run-stage の材料を指す FK を組むためだけに要る — `read_run_stage` の
/// 行は定義 × scope × ステージで決まり、そのうち定義と scope は静的な intent が持つ。
/// `run_stage_ids` はそのスナップショットに実在する `read_run_stage.id` の集合であり、
/// **在る行しか指さない**ことをここで担保する (存在の照合であって判断ではない)。
///
/// # Errors
///
/// 渡された intent がこの実行のものでないとき ([`ReadTablesError::IntentUnavailable`])。
/// 呼出側は実行の `intent_id` で引いた intent を渡すので実運用では起きないが、集約の
/// 取り違えガード (BR2.6) の `Err` を握り潰さずそのまま材料不足として上へ流す。
pub(super) fn row(
    execution: &IntentExecution,
    intent: &Intent,
    kind: RequestKind,
    run_stage_ids: &BTreeSet<&str>,
) -> Result<NextAnswerRow, ReadTablesError> {
    let decision = execution
        .next_decision(intent, &kind.to_request())
        .map_err(|_| ReadTablesError::IntentUnavailable {
            execution_id: execution.id().as_str().to_string(),
            intent_id: execution.intent_id().as_str().to_string(),
        })?;
    let (stage_index, gate, checkbox) = match decision {
        NextDecision::RunStage { stage, gate } => (
            Some(stage.to_usize()),
            Some(GateDecision::spelling(gate).to_string()),
            None,
        ),
        NextDecision::Parked { stage } => (Some(stage.to_usize()), None, None),
        NextDecision::RecoverSkipInconsistency { stage, checkbox }
        | NextDecision::InconsistentSkip { stage, checkbox } => (
            Some(stage.to_usize()),
            None,
            Some(spelling::checkbox(checkbox).to_string()),
        ),
        NextDecision::Done
        | NextDecision::UnparkThenResume
        | NextDecision::ResumeMenu
        | NextDecision::NewWorkRouting => (None, None, None),
    };
    let stage_slug = stage_index.and_then(|index| slug_of(execution, index));
    let run_stage_id = if matches!(decision, NextDecision::RunStage { .. }) {
        stage_slug
            .as_deref()
            .map(|slug| row_id::run_stage(intent.definition_id().as_str(), intent.scope(), slug))
            .filter(|id| run_stage_ids.contains(id.as_str()))
    } else {
        None
    };
    Ok(NextAnswerRow::new(
        row_id::next_answer(execution.id().as_str(), kind.as_str()),
        execution.id().as_str().to_string(),
        kind.as_str().to_string(),
        spelling::decision_kind(&decision).to_string(),
        stage_index,
        stage_slug,
        gate,
        checkbox,
        run_stage_id,
    ))
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, Utc};
    use core_command_domain::orchestration::{
        ArtifactPaths, Created, IntentEventId, IntentExecutionId, IntentId, StageDisplay,
        StageEntries, StageEntry, StartRequest, WorkspaceScan,
    };
    use core_command_domain::workflow_definition::{
        BrownfieldGreenfield, DefinitionRevision, PhaseId, PlanAction, StageNumber, StageSlug,
        WorkflowDefinitionId,
    };

    use super::*;

    fn at() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-09-02T00:00:00Z")
            .expect("固定の ISO 8601 UTC")
            .with_timezone(&Utc)
    }

    fn slug(value: &str) -> StageSlug {
        StageSlug::parse(value).expect("テストの slug は文法内")
    }

    fn definition_id() -> WorkflowDefinitionId {
        WorkflowDefinitionId::parse("claude").expect("テストの定義 id")
    }

    fn intent_id() -> IntentId {
        IntentId::parse("01a02785-1bd8-76eb-aeea-5aa303ebd5b6")
            .expect("テストの IntentId は UUIDv7")
    }

    fn execution_a() -> IntentExecutionId {
        IntentExecutionId::parse("0190aaaa-bbbb-7ccc-9ddd-eeeeffff0000")
            .expect("テストの IntentExecutionId は UUIDv7")
    }

    fn revision(fill: char) -> DefinitionRevision {
        DefinitionRevision::parse(&format!("sha256:{}", fill.to_string().repeat(64)))
            .expect("テストの定義 revision")
    }

    fn stages() -> StageEntries {
        let entries: Vec<StageEntry> = {
            let display = |number: &str, name: &str, agent: &str| {
                StageDisplay::new(StageNumber::parse(number).expect("番号"), name, agent)
                    .expect("単一行")
            };
            vec![
                StageEntry::new(
                    slug("state-init"),
                    PhaseId::Initialization,
                    PlanAction::Execute,
                    false,
                    display("0.1", "State Init", "orchestrator"),
                ),
                StageEntry::new(
                    slug("intent-capture"),
                    PhaseId::Ideation,
                    PlanAction::Execute,
                    false,
                    display("1.1", "Intent Capture", "aidlc-product-agent"),
                ),
                StageEntry::new(
                    slug("scope-definition"),
                    PhaseId::Ideation,
                    PlanAction::Execute,
                    true,
                    display("1.4", "Scope Definition", "aidlc-product-agent"),
                ),
                StageEntry::new(
                    slug("requirements-analysis"),
                    PhaseId::Inception,
                    PlanAction::Execute,
                    false,
                    display("2.1", "Requirements Analysis", "aidlc-product-agent"),
                ),
                // Construction の最初の EXECUTE — walking-skeleton ゲートの位置 (b47 / #73)。
                StageEntry::new(
                    slug("functional-design"),
                    PhaseId::Construction,
                    PlanAction::Execute,
                    false,
                    display("3.1", "Functional Design", "aidlc-architect-agent"),
                ),
            ]
        };
        StageEntries::new(entries).expect("フィクスチャの計画は不変条件を満たす")
    }

    fn scan() -> WorkspaceScan {
        WorkspaceScan::new(BrownfieldGreenfield::Brownfield, "Rust", "tokio", "cargo")
            .expect("単一行")
    }

    /// intent を起こす (`id` だけを差し替えられる — 照合の取り違えを作るため)。
    fn intent_with(event_id: &str, id: IntentId) -> Intent {
        Intent::from((
            Created::new(
                IntentEventId::parse(event_id).expect("UUIDv7"),
                id,
                definition_id(),
                revision('1'),
                StartRequest::new("classic", "build the thing")
                    .with_depth("standard")
                    .with_test_strategy("standard")
                    .with_review("adversarial"),
                stages(),
                scan(),
            ),
            at(),
        ))
    }

    fn intent() -> Intent {
        intent_with("0191aaaa-bbbb-7ccc-9ddd-eeeeffff0001", intent_id())
    }

    /// 稼働中の実行 (ゲートを開けて承認 → カーソルが前進する)。
    fn running_execution() -> IntentExecution {
        let intent = intent();
        let (mut aggregate, _started) = IntentExecution::start(execution_a(), &intent, at());
        let _opened = aggregate
            .open_gate(
                &intent,
                ArtifactPaths::new(vec!["intent.md".to_string()]),
                at(),
            )
            .expect("ゲートは開く");
        let _approved = aggregate
            .approve_gate(&intent, None, Some("ok".to_string()), at())
            .expect("ゲートは承認される");
        aggregate
    }

    /// 別 intent を渡した行の組み立ては答えを持たず、材料不足として `Err` を返す。
    ///
    /// 集約のクエリ `next_decision` は 2026-09-06 の切替で取り違えガード (BR2.6) を持ち
    /// `Err(IntentMismatch)` を返すようになった。RMU はその `Err` を握り潰して部分的な行を
    /// 書かず、既存の材料不足 (`IntentUnavailable`) にそのまま写して上へ流す。
    #[test]
    fn a_foreign_intent_yields_no_answer_row_but_a_missing_material_error() {
        let execution = running_execution();
        // 同じ計画で別 ID の intent を起こす (照合だけが違う)。
        let foreign = intent_with(
            "0191aaaa-bbbb-7ccc-9ddd-eeeeffff0009",
            IntentId::parse("01a02785-1bd8-76eb-aeea-5aa303ebd5b7").expect("UUIDv7"),
        );
        let error = row(&execution, &foreign, RequestKind::Bare, &BTreeSet::new())
            .expect_err("取り違えは行にならない");
        assert_eq!(
            error,
            ReadTablesError::IntentUnavailable {
                execution_id: execution_a().as_str().to_string(),
                intent_id: intent_id().as_str().to_string(),
            }
        );
        // 自分の intent なら従来どおり行になる (照合は行の前段のガードにすぎない)。
        assert!(row(&execution, &intent(), RequestKind::Bare, &BTreeSet::new()).is_ok());
    }
}
