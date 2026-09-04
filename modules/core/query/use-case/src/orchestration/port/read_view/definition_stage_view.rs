//! `DefinitionStageView` — `read_definition_stage` 1 行の写し。

/// `read_definition_stage` の 1 行 (自然キー (`definition_id`, `stage_slug`))。
///
/// **行が引けないこと自体が「そのステージがグラフに無い」の答え**である — upstream の
/// `findStageBySlug("practices-discovery")` が `undefined` を返す形にあたる (b49)。
///
/// 運ぶ列は昇格の構文段が読む 2 つだけである。行は 32 列あるが、View は**使う列だけ**を
/// 写す — 使わない列を載せると「この行の写しは何のためにあるのか」が読めなくなるからで
/// ある。別の消費者が別の列を要るようになったら、そのときに足す
/// (`coding-rules/no-backward-compatibility.md` — 使われない口を先回りで並べない)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefinitionStageView {
    stage_slug: String,
    support_agents: String,
}

impl DefinitionStageView {
    /// 2 列をそのまま束ねる (**この型の唯一の構築経路**)。
    #[must_use]
    pub const fn new(stage_slug: String, support_agents: String) -> DefinitionStageView {
        DefinitionStageView {
            stage_slug,
            support_agents,
        }
    }

    /// ステージの slug。
    #[must_use]
    pub fn stage_slug(&self) -> &str {
        &self.stage_slug
    }

    /// 支援エージェントの 1 行 JSON 配列 (配列へ開くのは描く側)。
    #[must_use]
    pub fn support_agents(&self) -> &str {
        &self.support_agents
    }
}
