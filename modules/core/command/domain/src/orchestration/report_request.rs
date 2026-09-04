//! `ReportRequest` — 集約のクエリ [`IntentExecution::report_dispatch`] が受け取る観測。
//!
//! [`IntentExecution::report_dispatch`]: super::IntentExecution::report_dispatch

use super::verdict::Verdict;
use crate::workflow_definition::StageSlug;

/// `report` 1 回ぶんの観測 (段 5〜13 の材料)。
///
/// 合成ルートが構文的な段 (値の有無・既知値・env) を通したあとに組む値である。ここに来る
/// [`Verdict`] に `Resume` は無い — 再開は遷移をコミットせず、合成ルートが手前でルーティング
/// する (`coding-rules/use-case-rules.md` §3)。構成上その腕を弾く型は作らない (`Verdict` の
/// 6 値をそのまま持つ) が、`report_dispatch` は `Resume` を**拒否として**扱う。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportRequest {
    verdict: Verdict,
    stage: Option<StageSlug>,
    user_input: Option<String>,
    reason: Option<String>,
    human_presence_guard: bool,
}

impl ReportRequest {
    /// 5 観測を束ねる (**この型の唯一の構築経路**)。
    ///
    /// `stage` は明示された `--stage` (空白のみは合成ルートが `None` に畳む)、`user_input` は
    /// `--user-input`、`reason` は `--reason`、`human_presence_guard` は環境変数
    /// `AIDLC_SKIP_HUMAN_PRESENCE_GUARD` が `"1"` **でない**こと (= ガードが効く) である。
    #[must_use]
    pub const fn new(
        verdict: Verdict,
        stage: Option<StageSlug>,
        user_input: Option<String>,
        reason: Option<String>,
        human_presence_guard: bool,
    ) -> ReportRequest {
        ReportRequest {
            verdict,
            stage,
            user_input,
            reason,
            human_presence_guard,
        }
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

    /// 差し戻しのフィードバック — `--user-input` が無ければ `--reason` (段 10、ピン `:5721`)。
    ///
    /// 空白のみは「無い」と同じである。
    #[must_use]
    pub fn feedback(&self) -> Option<&str> {
        self.user_input()
            .or_else(|| self.reason())
            .map(str::trim)
            .filter(|text| !text.is_empty())
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

#[cfg(test)]
mod tests {
    use super::*;

    fn slug(value: &str) -> StageSlug {
        StageSlug::parse(value).expect("フィクスチャの slug は文法内")
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

    #[test]
    fn the_feedback_prefers_user_input_and_falls_back_to_reason() {
        let with_input = ReportRequest::new(
            Verdict::Rejected,
            None,
            Some("直して".to_string()),
            Some("理由".to_string()),
            true,
        );
        assert_eq!(with_input.feedback(), Some("直して"));
        let with_reason = ReportRequest::new(
            Verdict::Rejected,
            None,
            None,
            Some("理由".to_string()),
            true,
        );
        assert_eq!(with_reason.feedback(), Some("理由"));
    }

    #[test]
    fn a_blank_user_input_does_not_fall_back_to_the_reason() {
        // upstream は `(flags.userInput ?? flags.reason)?.trim()` — nullish coalescing なので
        // **空白の `--user-input` は「在る」**であり、`--reason` へは落ちない (ピン `:5721`)。
        let blank_input = ReportRequest::new(
            Verdict::Rejected,
            None,
            Some("   ".to_string()),
            Some("実のある理由".to_string()),
            true,
        );
        assert_eq!(blank_input.feedback(), None);
        assert!(!blank_input.has_user_input());
        assert!(blank_input.has_reason());
    }

    #[test]
    fn blank_material_counts_as_absent() {
        let blank = ReportRequest::new(Verdict::Skipped, None, None, Some("\t".to_string()), true);
        assert_eq!(blank.feedback(), None);
        assert!(!blank.has_user_input());
        assert!(!blank.has_reason());
    }
}
