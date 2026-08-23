//! `StartRequest` — `start` に渡す呼出側の要求 (scope / request / depth / test_strategy)。

/// 実行開始時に呼出側 (birth ユースケース) が解決して渡す要求 (C5 `Started` の payload 材料)。
///
/// `depth` / `test_strategy` は upstream 状態ファイルの `Scope Configuration` 行 (`Depth` /
/// `Test Strategy`) を U4 が描くための材料であり、**集約はこの 4 値に意味論を持たない** —
/// フラグ上書きと scope metadata の既定のどちらを採るかの解決は呼出側の責務で、ここは素通しの
/// 投影材料である。`Started` が自己完結する (投影が定義を読み直さない) ためにイベントへ載せる。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StartRequest {
    scope: String,
    request: String,
    depth: Option<String>,
    test_strategy: Option<String>,
}

impl StartRequest {
    /// スコープ名と人間の要求から組む。`depth` / `test_strategy` は既定で「指定なし」。
    #[must_use]
    pub fn new(scope: impl Into<String>, request: impl Into<String>) -> StartRequest {
        StartRequest {
            scope: scope.into(),
            request: request.into(),
            depth: None,
            test_strategy: None,
        }
    }

    /// 解決済みの depth を載せる (再呼出は上書き)。
    #[must_use]
    pub fn with_depth(mut self, depth: impl Into<String>) -> StartRequest {
        self.depth = Some(depth.into());
        self
    }

    /// 解決済みの test strategy を載せる (再呼出は上書き)。
    #[must_use]
    pub fn with_test_strategy(mut self, test_strategy: impl Into<String>) -> StartRequest {
        self.test_strategy = Some(test_strategy.into());
        self
    }

    /// 選択されたスコープ名 (妥当性は `WorkflowExecution::start` が定義に照らして検査する)。
    #[must_use]
    pub fn scope(&self) -> &str {
        &self.scope
    }

    /// 人間の要求 (逐語保持)。
    #[must_use]
    pub fn request(&self) -> &str {
        &self.request
    }

    /// 解決済みの depth (`None` = 指定なし)。
    #[must_use]
    pub fn depth(&self) -> Option<&str> {
        self.depth.as_deref()
    }

    /// 解決済みの test strategy (`None` = 指定なし)。
    #[must_use]
    pub fn test_strategy(&self) -> Option<&str> {
        self.test_strategy.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_request_carries_the_scope_and_the_human_request() {
        let r = StartRequest::new("classic", "build it");
        assert_eq!(r.scope(), "classic");
        assert_eq!(r.request(), "build it");
    }

    #[test]
    fn depth_and_test_strategy_are_absent_unless_supplied() {
        let r = StartRequest::new("classic", "build it");
        assert_eq!(r.depth(), None);
        assert_eq!(r.test_strategy(), None);
    }

    #[test]
    fn the_optional_fields_can_be_supplied_independently() {
        let only_depth = StartRequest::new("classic", "build it").with_depth("standard");
        assert_eq!(only_depth.depth(), Some("standard"));
        assert_eq!(only_depth.test_strategy(), None);

        let only_strategy =
            StartRequest::new("classic", "build it").with_test_strategy("comprehensive");
        assert_eq!(only_strategy.depth(), None);
        assert_eq!(only_strategy.test_strategy(), Some("comprehensive"));

        let both = StartRequest::new("classic", "build it")
            .with_depth("standard")
            .with_test_strategy("comprehensive");
        assert_eq!(both.depth(), Some("standard"));
        assert_eq!(both.test_strategy(), Some("comprehensive"));
    }

    #[test]
    fn a_later_call_replaces_the_earlier_value() {
        let r = StartRequest::new("classic", "build it")
            .with_depth("quick")
            .with_depth("standard");
        assert_eq!(r.depth(), Some("standard"));
    }

    #[test]
    fn the_domain_does_not_validate_the_request() {
        // 集約はこの 4 値に意味論を持たない — 解決 (フラグ上書き or scope metadata の既定) は
        // 呼出側 (birth ユースケース) の責務で、ここは素通しの投影材料である。
        let r = StartRequest::new("", "")
            .with_depth("")
            .with_test_strategy("");
        assert_eq!(r.scope(), "");
        assert_eq!(r.request(), "");
        assert_eq!(r.depth(), Some(""));
        assert_eq!(r.test_strategy(), Some(""));
    }

    #[test]
    fn requests_compare_by_value() {
        let a = StartRequest::new("classic", "build it").with_depth("standard");
        assert_eq!(
            a,
            StartRequest::new("classic", "build it").with_depth("standard")
        );
        assert_ne!(a, StartRequest::new("classic", "build it"));
        assert_ne!(
            a,
            StartRequest::new("express", "build it").with_depth("standard")
        );
    }
}
