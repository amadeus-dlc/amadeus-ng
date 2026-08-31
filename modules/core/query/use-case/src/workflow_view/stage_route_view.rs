//! `StageRouteView` — 「どのステージが、どの scope 経路の上で走るか」の同一性。
//!
//! steering 連鎖の route 束縛 (`r`) が指す**対象そのもの**を名前付きで運ぶ。素材文字列の
//! 組み立てはこの型の知識ではない — ダイジェストの計算は
//! [`crate::orchestration`] の `steering_digest` モジュールが本型の関連メソッドとして持つ
//! (`coding-rules/domain-services.md`)。

use super::stage_slug_view::StageSlugView;

/// ステージと、その scope の in-scope ステージ列 (グラフ順) の対。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageRouteView {
    stage: StageSlugView,
    stages_in_scope: Vec<StageSlugView>,
}

impl StageRouteView {
    /// 対象ステージと in-scope ステージ列を束ねる。
    #[must_use]
    pub const fn new(stage: StageSlugView, stages_in_scope: Vec<StageSlugView>) -> StageRouteView {
        StageRouteView {
            stage,
            stages_in_scope,
        }
    }

    /// 対象ステージ。
    #[must_use]
    pub const fn stage(&self) -> &StageSlugView {
        &self.stage
    }

    /// scope の in-scope ステージ列 (グラフ順)。
    #[must_use]
    pub fn stages_in_scope(&self) -> &[StageSlugView] {
        &self.stages_in_scope
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_route_carries_its_stage_and_the_scope_membership() {
        let route = StageRouteView::new(
            StageSlugView::parse("functional-design").unwrap(),
            vec![
                StageSlugView::parse("intent-capture").unwrap(),
                StageSlugView::parse("functional-design").unwrap(),
            ],
        );
        assert_eq!(route.stage().as_str(), "functional-design");
        assert_eq!(route.stages_in_scope().len(), 2);
    }

    #[test]
    fn routes_compare_by_value() {
        let one = StageRouteView::new(StageSlugView::parse("a").unwrap(), Vec::new());
        let same = StageRouteView::new(StageSlugView::parse("a").unwrap(), Vec::new());
        let other = StageRouteView::new(
            StageSlugView::parse("a").unwrap(),
            vec![StageSlugView::parse("b").unwrap()],
        );
        assert_eq!(one, same);
        assert_ne!(one, other);
    }
}
