//! `NextAnswerRow` — `read_next_answer` の 1 行 (`next` の答えを要求の形ごとに焼き込む)。

use std::collections::BTreeSet;

use core_command_domain::orchestration::{GateDecision, Intent, IntentExecution, NextDecision};

use super::read_tables_error::ReadTablesError;
use super::request_kind::RequestKind;
use super::row_id;
use super::spelling;
use super::stage_lookup::slug_of;

/// `read_next_answer` の 1 行。主キーは 1 列 `id` (自然キー
/// (`execution_id`, `request_kind`) から導いた代理キー)。`execution_id` は
/// `read_execution.id` を、`run_stage_id` は `read_run_stage.id` を指す FK である。
///
/// 値は集約のクエリ [`IntentExecution::next_decision`] の答えである。**クエリ側は
/// 21 分岐のラダーを持たない** — どの要求の形にどの答えが対応するかは書込側の集約が
/// 決め、RMU はその答えを 4 行に焼き込むだけである
/// (`coding-rules/cqrs-boundaries.md` 規則 6 の 2026-09-02 追記)。
///
/// 逐語文言と directive の綴りはここに無い。それは行の `decision_kind` に従って
/// **出す側 (プレゼンタ)** が描く。
///
/// `run_stage_id` は**指す先の行が同じスナップショットに在るときだけ**値を持つ。「答えは
/// run-stage なので、材料はこの行にある」を FK 1 本で言うためであり、NULL は「材料の行が
/// 無い」を意味する (判断ではなく不在である)。値が無くなるのは 2 つの場合である —
/// 決定が run-stage ではない、または決定が名指すステージの run-stage 行が**この定義には
/// 無い** (計画は誕生時の内容版に対して解決されるので、その後の改訂でステージが消えると
/// 実行の計画だけが古いステージを名乗り続ける)。届かない FK を書かないのは、読み手に
/// 「引いたが無かった」の後始末をさせないためである。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NextAnswerRow {
    id: String,
    execution_id: String,
    request_kind: String,
    decision_kind: String,
    stage_index: Option<usize>,
    stage_slug: Option<String>,
    gate: Option<String>,
    checkbox: Option<String>,
    run_stage_id: Option<String>,
}

impl NextAnswerRow {
    /// 1 つの要求の形に対する答えを 1 行へ写す (**この型の唯一の構築経路**)。
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
    pub fn of(
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
                .map(|slug| {
                    row_id::run_stage(intent.definition_id().as_str(), intent.scope(), slug)
                })
                .filter(|id| run_stage_ids.contains(id.as_str()))
        } else {
            None
        };
        Ok(NextAnswerRow {
            id: row_id::next_answer(execution.id().as_str(), kind.as_str()),
            execution_id: execution.id().as_str().to_string(),
            request_kind: kind.as_str().to_string(),
            decision_kind: spelling::decision_kind(&decision).to_string(),
            stage_index,
            stage_slug,
            gate,
            checkbox,
            run_stage_id,
        })
    }

    /// 主キー — 自然キー (`execution_id`, `request_kind`) から導いた代理キー。
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// `read_run_stage.id` を指す FK (指す先の行が同じスナップショットに在るときだけ)。
    #[must_use]
    pub fn run_stage_id(&self) -> Option<&str> {
        self.run_stage_id.as_deref()
    }

    /// `read_execution.id` を指す FK。
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

    /// `run-stage` のときだけ在る — そのステージのゲート判断の綴り
    /// (`gated` / `ungated` / `unresolved`)。
    ///
    /// 綴りの正本はドメインの [`GateDecision::spelling`] である — 3 値であって真偽値では
    /// ないので、列も `INTEGER` ではなく `TEXT` である (b47 / #73)。
    #[must_use]
    pub fn gate(&self) -> Option<&str> {
        self.gate.as_deref()
    }

    /// 不整合 2 形のときだけ在る — 観測 checkbox の綴り。
    #[must_use]
    pub fn checkbox(&self) -> Option<&str> {
        self.checkbox.as_deref()
    }
}
