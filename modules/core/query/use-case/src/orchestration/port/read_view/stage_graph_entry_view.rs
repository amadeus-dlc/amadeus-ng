//! `StageGraphEntryView` — コンパイル済みステージグラフ 1 ノードの写し。
//!
//! `stage-graph.json` は compile コンテキストのイベント投影 = リードモデルであり、その 1 行を
//! クエリ側が読んで写した DTO である (`coding-rules/cqrs-boundaries.md` 規則 7 —
//! 「クエリサイドはこれを読むだけ」)。媒体 (JSON ファイル) は DAO 実装の内部詳細であり、
//! この写しには現れない。
//!
//! 運ぶ列は `lookup phase-of` / `agent-for` / `validate-stage` と `stage-table` が読む 8 つに
//! 限る — 行は任意列を含むが、View は使う列だけを写す (`coding-rules/no-backward-compatibility.md`
//! — 使われない口を先回りで並べない)。表現 (フィールド) は隠し、契約 (アクセサ) だけを公開する
//! (`coding-rules/field-visibility.md` / `abstract-data-type.md`)。

/// コンパイル済みステージグラフの 1 ノード。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageGraphEntryView {
    slug: String,
    number: String,
    name: String,
    phase: String,
    execution: String,
    lead_agent: String,
    support_agents: Vec<String>,
    mode: String,
}

impl StageGraphEntryView {
    /// 8 列をそのまま束ねる (**この型の唯一の構築経路**)。
    #[expect(
        clippy::too_many_arguments,
        reason = "ノードの写しの完全コンストラクタ — 8 列がそのまま引数になる"
    )]
    #[must_use]
    pub const fn new(
        slug: String,
        number: String,
        name: String,
        phase: String,
        execution: String,
        lead_agent: String,
        support_agents: Vec<String>,
        mode: String,
    ) -> StageGraphEntryView {
        StageGraphEntryView {
            slug,
            number,
            name,
            phase,
            execution,
            lead_agent,
            support_agents,
            mode,
        }
    }

    /// ステージの slug。
    #[must_use]
    pub fn slug(&self) -> &str {
        &self.slug
    }

    /// グラフ上の番号 (`0.3` など)。
    #[must_use]
    pub fn number(&self) -> &str {
        &self.number
    }

    /// 表示名。
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 所属フェーズ (`initialization` など、行の綴りのまま)。
    #[must_use]
    pub fn phase(&self) -> &str {
        &self.phase
    }

    /// 実行区分 (`ALWAYS` / `CONDITIONAL`)。
    #[must_use]
    pub fn execution(&self) -> &str {
        &self.execution
    }

    /// 主担当エージェント (行の綴りのまま — 表示の読み替えは出す側)。
    #[must_use]
    pub fn lead_agent(&self) -> &str {
        &self.lead_agent
    }

    /// 支援エージェントの一覧 (無ければ空)。
    #[must_use]
    pub fn support_agents(&self) -> &[String] {
        &self.support_agents
    }

    /// 実行モード (`inline` / `subagent` など)。
    #[must_use]
    pub fn mode(&self) -> &str {
        &self.mode
    }
}
