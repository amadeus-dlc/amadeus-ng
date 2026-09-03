//! `DefinitionSummaryView` — `read_definition` 1 行の写し。

/// `read_definition` の 1 行 (主キー `definition_id`)。
///
/// 名前が `DefinitionView` でないのは、同じ `port` に配布 3 入力をまるごと写す既存の
/// `DefinitionView` が居るからである (b44 で退役するまで綴りを衝突させない)。本型が持つのは
/// **その 1 行の要約**だけである。
///
/// **行が引けないこと自体が「定義が未取込」の答え**である — 定義ファイルを読みに行く経路は
/// クエリ側に無い。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefinitionSummaryView {
    revision: String,
    stage_count: u32,
    scope_count: u32,
}

impl DefinitionSummaryView {
    /// 3 列をそのまま束ねる (**この型の唯一の構築経路**)。
    #[must_use]
    pub const fn new(
        revision: String,
        stage_count: u32,
        scope_count: u32,
    ) -> DefinitionSummaryView {
        DefinitionSummaryView {
            revision,
            stage_count,
            scope_count,
        }
    }

    /// 定義の内容版。
    #[must_use]
    pub fn revision(&self) -> &str {
        &self.revision
    }

    /// グラフのステージ総数。
    #[must_use]
    pub const fn stage_count(&self) -> u32 {
        self.stage_count
    }

    /// カタログの scope 総数。
    #[must_use]
    pub const fn scope_count(&self) -> u32 {
        self.scope_count
    }
}
