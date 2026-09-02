//! `NextAnswerRow` — `read_next_answer` の 1 行 (`next` の答えを要求の形ごとに焼き込む)。

use core_command_domain::orchestration::{IntentExecution, NextDecision};

use super::request_kind::RequestKind;
use super::spelling;
use super::stage_lookup::slug_of;

/// `read_next_answer` の 1 行。主キーは (`execution_id`, `request_kind`)。
///
/// 値は集約のクエリ [`IntentExecution::next_decision`] の答えである。**クエリ側は
/// 21 分岐のラダーを持たない** — どの要求の形にどの答えが対応するかは書込側の集約が
/// 決め、RMU はその答えを 4 行に焼き込むだけである
/// (`coding-rules/cqrs-boundaries.md` 規則 6 の 2026-09-02 追記)。
///
/// 逐語文言と directive の綴りはここに無い。それは行の `decision_kind` に従って
/// **出す側 (プレゼンタ)** が描く。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NextAnswerRow {
    execution_id: String,
    request_kind: String,
    decision_kind: String,
    stage_index: Option<usize>,
    stage_slug: Option<String>,
    gated: Option<bool>,
    checkbox: Option<String>,
}

impl NextAnswerRow {
    /// 1 つの要求の形に対する答えを 1 行へ写す (**この型の唯一の構築経路**)。
    #[must_use]
    pub fn of(execution: &IntentExecution, kind: RequestKind) -> NextAnswerRow {
        let decision = execution.next_decision(&kind.to_request());
        let (stage_index, gated, checkbox) = match decision {
            NextDecision::RunStage { stage, gate } => (Some(stage.to_usize()), Some(gate), None),
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
        NextAnswerRow {
            execution_id: execution.id().as_str().to_string(),
            request_kind: kind.as_str().to_string(),
            decision_kind: spelling::decision_kind(&decision).to_string(),
            stage_index,
            stage_slug: stage_index.and_then(|index| slug_of(execution, index)),
            gated,
            checkbox,
        }
    }

    /// 実行の識別子。
    #[must_use]
    pub fn execution_id(&self) -> &str {
        &self.execution_id
    }

    /// 要求の形の綴り (`bare` / `resume` / `free-text` / `reentry`)。
    #[must_use]
    pub fn request_kind(&self) -> &str {
        &self.request_kind
    }

    /// 答えの分類子 (`run-stage` … `inconsistent-skip`)。
    #[must_use]
    pub fn decision_kind(&self) -> &str {
        &self.decision_kind
    }

    /// 答えが名指すステージ位置 (名指さない分岐は NULL)。
    #[must_use]
    pub const fn stage_index(&self) -> Option<usize> {
        self.stage_index
    }

    /// 答えが名指すステージの slug。
    #[must_use]
    pub fn stage_slug(&self) -> Option<&str> {
        self.stage_slug.as_deref()
    }

    /// `run-stage` のときだけ在る — そのステージがゲート付きか。
    #[must_use]
    pub const fn gated(&self) -> Option<bool> {
        self.gated
    }

    /// 不整合 2 形のときだけ在る — 観測 checkbox の綴り。
    #[must_use]
    pub fn checkbox(&self) -> Option<&str> {
        self.checkbox.as_deref()
    }
}
