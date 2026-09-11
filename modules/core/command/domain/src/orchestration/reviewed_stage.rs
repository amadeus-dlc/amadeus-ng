//! `ReviewedStage` — 差し向け記録が名乗る、レビュー対象のステージの綴り。
/// 差し向け記録 (`.aidlc-reviewer-dispatch.json`) の `stage` の**逐語**。
///
/// [`StageSlug`] ではない — upstream `parseDispatchRecord` は `stage` が文字列であることしか
/// 見ず、空文字や `Functional Design` のような綴りもそのまま受けて `REVIEWER_SCOPE_BLOCKED`
/// の `Stage` へ逐語で載せる (裁定 2026-09-10 Q1 = A)。記録の文法を本 build が狭めると、
/// 規約外の記録に対して upstream が強制する越境を本 build は通してしまう。
///
/// [`StageSlug`]: crate::workflow_definition::StageSlug
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ReviewedStage(String);
impl ReviewedStage {
    /// 記録の綴りをそのまま持つ (**この型の唯一の構築経路**)。空文字も綴りである。
    #[must_use]
    pub const fn new(raw: String) -> ReviewedStage {
        ReviewedStage(raw)
    }
    /// 監査項目 `Stage` へ載せる逐語の綴り。
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
#[cfg(test)]
mod tests {
    use super::ReviewedStage;
    #[test]
    fn any_spelling_including_the_empty_one_is_carried_verbatim() {
        for raw in ["functional-design", "Functional Design", ""] {
            assert_eq!(ReviewedStage::new(raw.to_string()).as_str(), raw);
        }
    }
}
