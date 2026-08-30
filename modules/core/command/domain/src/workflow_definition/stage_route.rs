//! `StageRoute` — 「どのステージが、どの scope 経路の上で走るか」の同一性。
//!
//! steering 連鎖の route 束縛 (`r`) が指す**対象そのもの**を名前付きで運ぶ。素材文字列の
//! 連結はドメインの知識ではない — ダイジェストの計算と直列化は codec (アダプタ層) が持ち、
//! ドメインはこの VO だけを渡す。

use super::stage_slug::StageSlug;

/// ステージと、その scope の in-scope ステージ列 (グラフ順) の対。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageRoute {
    stage: StageSlug,
    stages_in_scope: Vec<StageSlug>,
}

impl StageRoute {
    /// 対象ステージと in-scope ステージ列を束ねる。
    #[must_use]
    pub const fn new(stage: StageSlug, stages_in_scope: Vec<StageSlug>) -> StageRoute {
        StageRoute {
            stage,
            stages_in_scope,
        }
    }

    /// 対象ステージ。
    #[must_use]
    pub const fn stage(&self) -> &StageSlug {
        &self.stage
    }

    /// scope の in-scope ステージ列 (グラフ順)。
    #[must_use]
    pub fn stages_in_scope(&self) -> &[StageSlug] {
        &self.stages_in_scope
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn slug(s: &str) -> StageSlug {
        StageSlug::parse(s).unwrap()
    }

    #[test]
    fn the_route_carries_the_stage_and_its_scope_membership() {
        let route = StageRoute::new(slug("domain-design"), vec![slug("intent-capture")]);
        assert_eq!(route.stage(), &slug("domain-design"));
        assert_eq!(route.stages_in_scope(), [slug("intent-capture")]);
        assert_eq!(route.clone(), route);
    }
}
