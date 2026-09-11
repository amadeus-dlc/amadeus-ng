//! テスト契約が対象とする作業の観測値。
/// 入力境界で正規化されたスコープ・戦略・プロジェクト種別。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestingContext {
    scope: String,
    strategy: String,
    project_type: String,
}
impl TestingContext {
    /// 現在の作業の入力を束ねる。
    #[must_use]
    pub const fn new(scope: String, strategy: String, project_type: String) -> Self {
        Self {
            scope,
            strategy,
            project_type,
        }
    }
    /// 依頼の作業条件を正規化する。依頼前は本家の既定値を使う。
    #[must_use]
    pub fn for_intent(intent: Option<&super::Intent>) -> Self {
        let scope = intent
            .map_or("feature", super::Intent::scope)
            .trim()
            .to_lowercase();
        let strategy = intent
            .and_then(super::Intent::test_strategy)
            .unwrap_or("standard")
            .trim()
            .to_lowercase();
        let strategy = if matches!(strategy.as_str(), "minimal" | "standard" | "comprehensive") {
            strategy
        } else {
            "standard".to_string()
        };
        let project_type = intent
            .map_or("greenfield", |intent| intent.scan().project_kind().as_str())
            .to_string();
        Self::new(scope, strategy, project_type)
    }
    /// スコープ。
    #[must_use]
    pub fn scope(&self) -> &str {
        &self.scope
    }
    /// テスト戦略。
    #[must_use]
    pub fn strategy(&self) -> &str {
        &self.strategy
    }
    /// プロジェクト種別。
    #[must_use]
    pub fn project_type(&self) -> &str {
        &self.project_type
    }
}
