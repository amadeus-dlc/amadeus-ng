//! `JumpScope` — 直接 execute が `--scope` で名指した別 scope の実効計画 (本家 `handleExecute`)。

use super::StageSlugSet;
use crate::workflow_definition::StageSlug;

/// 直接 execute に渡された別 scope。
///
/// 到達可否と読み飛ばし・巻き戻しの導出に、この実行の実効計画ではなく **その scope の静的な列**
/// を使う (本家 `aidlc-jump.ts handleExecute`: 別 scope は state の suffix を見ず static grid だけ
/// を参照する — `foreign-scope` 観測)。適用 (再生) が定義を読まずに自己完結するよう、EXECUTE の
/// 集合そのものを事実として運ぶ。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JumpScope {
    name: String,
    executes: StageSlugSet,
}

impl JumpScope {
    /// scope 名と、その scope で EXECUTE の stage 集合。
    #[must_use]
    pub const fn new(name: String, executes: StageSlugSet) -> JumpScope {
        JumpScope { name, executes }
    }

    /// 名指された scope 名。
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// その scope で EXECUTE の stage 集合。
    #[must_use]
    pub const fn executes(&self) -> &StageSlugSet {
        &self.executes
    }

    /// その scope で EXECUTE か。
    #[must_use]
    pub fn contains(&self, slug: &StageSlug) -> bool {
        self.executes.contains(slug)
    }
}
