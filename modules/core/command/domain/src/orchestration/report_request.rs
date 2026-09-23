//! `ReportRequest` — 集約のクエリ [`IntentExecution::report_dispatch`] が受け取る観測。
//!
//! [`IntentExecution::report_dispatch`]: super::IntentExecution::report_dispatch

use super::verdict::Verdict;
use crate::workflow_definition::StageSlug;
use core_infrastructure::ecmascript::{collapse_whitespace, is_whitespace, trim};

/// `report` 1 回ぶんの観測 (段 5〜13 の材料)。
///
/// 合成ルートが構文的な段 (値の有無・既知値・env) を通したあとに組む値である。ここに来る
/// [`Verdict`] に `Resume` は無い — 再開は遷移をコミットせず、合成ルートが手前でルーティング
/// する (`coding-rules/use-case-rules.md` §3)。構成上その腕を弾く型は作らない (`Verdict` の
/// 6 値をそのまま持つ) が、`report_dispatch` は `Resume` を**拒否として**扱う。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportRequest {
    source_baseline: Option<super::SourceBaseline>,
    workspace_stages: super::StageSlugSet,
    validation: Option<super::StageValidation>,
    pipeline_handoff: Option<super::PipelineHandoff>,
    pipeline_disabled: bool,
    verdict: Verdict,
    stage: Option<StageSlug>,
    user_input: Option<String>,
    reason: Option<String>,
    human_presence_guard: bool,
    turns: crate::workspace::HumanTurns,
}

impl ReportRequest {
    /// 採取時のソースと、配布定義がソースを必要とする工程の観測を伴う要求。
    #[must_use]
    pub fn with_source_baseline(
        mut self,
        baseline: Option<super::SourceBaseline>,
        stages: super::StageSlugSet,
    ) -> Self {
        self.source_baseline = baseline;
        self.workspace_stages = stages;
        self
    }
    pub(super) fn source_for(&self, stage: &StageSlug) -> Option<&super::SourceBaseline> {
        self.workspace_stages
            .contains(stage)
            .then_some(self.source_baseline.as_ref())
            .flatten()
    }

    /// 報告時に採取した検証根拠を伴う要求。
    #[must_use]
    pub fn with_validation(mut self, validation: Option<super::StageValidation>) -> Self {
        self.validation = validation;
        self
    }
    pub(super) const fn validation(&self) -> Option<&super::StageValidation> {
        self.validation.as_ref()
    }

    /// 呼出境界で得た現在のhandoffと、明示された回復switchを伴う要求を作る。
    #[must_use]
    pub fn with_pipeline_observation(
        mut self,
        handoff: Option<super::PipelineHandoff>,
        disabled: bool,
    ) -> Self {
        self.pipeline_handoff = handoff;
        self.pipeline_disabled = disabled;
        self
    }
    /// 段階の完了根拠が必要な報告か。
    #[must_use]
    pub const fn requires_completion_evidence(&self) -> bool {
        matches!(
            self.verdict,
            Verdict::Forward | Verdict::AwaitingApproval | Verdict::Revised
        )
    }
    pub(super) const fn pipeline_handoff(&self) -> Option<&super::PipelineHandoff> {
        self.pipeline_handoff.as_ref()
    }
    pub(super) const fn pipeline_disabled(&self) -> bool {
        self.pipeline_disabled
    }

    /// 5 観測を束ねる (**この型の唯一の構築経路**)。
    ///
    /// `stage` は明示された `--stage` (空白のみは合成ルートが `None` に畳む)、`user_input` は
    /// `--user-input`、`reason` は `--reason`、`human_presence_guard` は環境変数
    /// `AIDLC_SKIP_HUMAN_PRESENCE_GUARD` が `"1"` **でない**こと (= ガードが効く) である。
    #[must_use]
    pub fn new(
        verdict: Verdict,
        stage: Option<StageSlug>,
        user_input: Option<String>,
        reason: Option<String>,
        human_presence_guard: bool,
    ) -> ReportRequest {
        ReportRequest {
            source_baseline: None,
            workspace_stages: super::StageSlugSet::empty(),
            validation: None,
            pipeline_handoff: None,
            pipeline_disabled: false,
            verdict,
            stage,
            user_input,
            reason,
            human_presence_guard,
            turns: crate::workspace::HumanTurns::default(),
        }
    }

    /// 監査台帳から読んだ人間の turn を伴う要求（承認・差し戻しの human presence の材料 —
    /// 2.8.2 `humanActedSinceGate`）。
    #[must_use]
    pub const fn with_human_turns(mut self, turns: crate::workspace::HumanTurns) -> Self {
        self.turns = turns;
        self
    }
    pub(super) const fn turns(&self) -> &crate::workspace::HumanTurns {
        &self.turns
    }

    /// 楽観競合の再試行を、最初に判断した対象へ固定する。
    #[must_use]
    pub fn for_retry_at(&self, stage: StageSlug) -> ReportRequest {
        Self::new(
            self.verdict,
            Some(stage),
            self.user_input.clone(),
            self.reason.clone(),
            self.human_presence_guard,
        )
        .with_pipeline_observation(self.pipeline_handoff.clone(), self.pipeline_disabled)
        .with_validation(self.validation.clone())
        .with_source_baseline(self.source_baseline.clone(), self.workspace_stages.clone())
        .with_human_turns(self.turns.clone())
    }

    /// 報告された結末の分類。
    #[must_use]
    pub const fn verdict(&self) -> Verdict {
        self.verdict
    }

    /// 明示されたステージ (`None` はカーソルに作用する — **有無それ自体が契約**)。
    #[must_use]
    pub const fn stage(&self) -> Option<&StageSlug> {
        self.stage.as_ref()
    }

    /// 承認時の人間入力 (逐語保持)。
    #[must_use]
    pub fn user_input(&self) -> Option<&str> {
        self.user_input.as_deref()
    }

    /// 読み飛ばし理由 / 差し戻しフィードバックの代替。
    #[must_use]
    pub fn reason(&self) -> Option<&str> {
        self.reason.as_deref()
    }

    /// human-presence ガードが効いているか (段 13)。
    #[must_use]
    pub const fn human_presence_guard(&self) -> bool {
        self.human_presence_guard
    }

    /// 差し戻しのフィードバック (2.8.2 `aidlc-orchestrate.ts` report `rejected` の組み立て)。
    ///
    /// `--reason` が在ればそれ (空白だけなら「無い」 — `--user-input` へは落ちない)。無ければ、
    /// ゲートが人間の決定を保護している (`protected_gate`) 間は `--user-input` は人間の選択
    /// (`Request Changes`) であってフィードバックではないので「無い」。保護されていなければ
    /// `--user-input` を使う。どちらも両端の空白を除き、空なら「無い」である。
    pub(super) fn feedback(&self, protected_gate: bool) -> Option<&str> {
        let chosen = match (self.reason(), protected_gate) {
            (Some(reason), _) => Some(reason),
            (None, true) => None,
            (None, false) => self.user_input(),
        };
        chosen.map(trim).filter(|text| !text.is_empty())
    }

    /// `--user-input` がゲートの `Request Changes` の選択か (2.8.2 `aidlc-lib.ts`
    /// `isRequestChangesChoice`)。
    ///
    /// 人が打つ形を同じ選択とみなす — 大文字小文字、選択肢の前置き (`B.` / `2)`)、
    /// 前後の引用符、末尾の `.` `!`、空白の連なり。言い換え (`please change it`) は選択では
    /// ない。
    pub(super) fn is_request_changes_choice(&self) -> bool {
        let text = trim(self.user_input().unwrap_or_default());
        let text = without_option_prefix(text);
        let text = text
            .trim_start_matches(['"', '\'', '`'])
            .trim_end_matches(['"', '\'', '`'])
            .trim_end_matches(['.', '!']);
        trim(&collapse_whitespace(text)).to_lowercase() == "request changes"
    }

    /// 空白でない `--user-input` があるか (段 13 の判定材料)。
    #[must_use]
    pub fn has_user_input(&self) -> bool {
        self.user_input()
            .is_some_and(|text| !text.trim().is_empty())
    }

    /// 空白でない `--reason` があるか (段 9 の判定材料)。
    #[must_use]
    pub fn has_reason(&self) -> bool {
        self.reason().is_some_and(|text| !text.trim().is_empty())
    }
}

/// 選択肢の前置き (英字 1 文字か数字の列に `.` か `)`、続く空白) を外す
/// (2.8.2 `/^(?:[A-Za-z]|\d+)[.)]\s*/`)。
fn without_option_prefix(text: &str) -> &str {
    let bytes = text.as_bytes();
    let head = match bytes.first() {
        Some(byte) if byte.is_ascii_alphabetic() => 1,
        Some(byte) if byte.is_ascii_digit() => bytes
            .iter()
            .take_while(|byte| byte.is_ascii_digit())
            .count(),
        _ => return text,
    };
    match bytes.get(head) {
        Some(b'.' | b')') => text
            .get(head.saturating_add(1)..)
            .map_or(text, |rest| rest.trim_start_matches(is_whitespace)),
        _ => text,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn slug(value: &str) -> StageSlug {
        StageSlug::parse(value).expect("フィクスチャの slug は文法内")
    }

    #[test]
    fn retry_names_the_original_target_without_losing_observations() {
        let original = ReportRequest::new(
            Verdict::Rejected,
            None,
            Some(" input ".into()),
            Some(" reason ".into()),
            false,
        );
        let retry = original.for_retry_at(slug("domain-design"));
        assert_eq!(
            retry,
            ReportRequest::new(
                Verdict::Rejected,
                Some(slug("domain-design")),
                Some(" input ".into()),
                Some(" reason ".into()),
                false
            )
        );
        assert_eq!(original.stage(), None);
    }

    #[test]
    fn the_request_carries_the_five_observations() {
        let request = ReportRequest::new(
            Verdict::Forward,
            Some(slug("domain-design")),
            Some("A".to_string()),
            None,
            true,
        );
        assert_eq!(request.verdict(), Verdict::Forward);
        assert_eq!(request.stage(), Some(&slug("domain-design")));
        assert_eq!(request.user_input(), Some("A"));
        assert_eq!(request.reason(), None);
        assert!(request.human_presence_guard());
    }

    fn rejected(user_input: Option<&str>, reason: Option<&str>) -> ReportRequest {
        ReportRequest::new(
            Verdict::Rejected,
            None,
            user_input.map(str::to_string),
            reason.map(str::to_string),
            true,
        )
    }

    /// `--reason` が在ればそれがフィードバックである (保護の有無を問わない)。
    #[test]
    fn the_feedback_is_the_reason_when_one_is_given() {
        let request = rejected(Some("Request Changes"), Some(" 直して "));
        assert_eq!(request.feedback(true), Some("直して"));
        assert_eq!(request.feedback(false), Some("直して"));
    }

    /// 保護されたゲートでは `--user-input` は選択であってフィードバックではない。
    #[test]
    fn a_protected_gate_does_not_take_the_user_input_as_feedback() {
        let request = rejected(Some("直して"), None);
        assert_eq!(request.feedback(true), None);
        assert_eq!(request.feedback(false), Some("直して"));
    }

    /// 空白だけの `--reason` は「無い」であり、`--user-input` へは落ちない (2.8.2 は
    /// `flags.reason !== undefined` で分岐する)。
    #[test]
    fn a_blank_reason_does_not_fall_back_to_the_user_input() {
        let request = rejected(Some("実のある返答"), Some("   "));
        assert_eq!(request.feedback(false), None);
        assert!(request.has_user_input());
        assert!(!request.has_reason());
    }

    /// 人が打つ揺れは同じ `Request Changes` の選択として読む。
    #[test]
    fn the_request_changes_choice_tolerates_how_a_person_types_it() {
        for reply in [
            "Request Changes",
            "request changes",
            "  REQUEST   CHANGES  ",
            "B. Request Changes",
            "2) Request Changes",
            "12) request changes",
            "\"Request Changes\"",
            "`Request Changes`",
            "Request Changes.",
            "Request Changes!!",
            "b.Request Changes",
        ] {
            assert!(
                rejected(Some(reply), None).is_request_changes_choice(),
                "{reply:?}"
            );
        }
    }

    /// 言い換え・別の選択・空は `Request Changes` の選択ではない。
    #[test]
    fn a_paraphrase_or_another_choice_is_not_request_changes() {
        for reply in [
            "",
            "   ",
            "please change it",
            "Approve",
            "Request",
            "Request Changes please",
            "AB. Request Changes",
            "\"Request Changes\".",
        ] {
            assert!(
                !rejected(Some(reply), None).is_request_changes_choice(),
                "{reply:?}"
            );
        }
        assert!(!rejected(None, Some("理由")).is_request_changes_choice());
    }

    #[test]
    fn blank_material_counts_as_absent() {
        let blank = ReportRequest::new(Verdict::Skipped, None, None, Some("\t".to_string()), true);
        assert_eq!(blank.feedback(false), None);
        assert!(!blank.has_user_input());
        assert!(!blank.has_reason());
    }
}
