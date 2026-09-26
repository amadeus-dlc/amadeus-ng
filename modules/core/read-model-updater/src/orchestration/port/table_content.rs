//! `TableContent` — 表 1 つに格納された全行の生の値。

use rusqlite::types::Value;

/// 表 1 つに格納された全行の生の値 (主キー `id` の昇順、列は表の定義順)。
///
/// 構造化面の表の DAO が `find_content` で返す。値の格納型 (整数・文字列・NULL …) まで含めて
/// 保存されたとおりに運ぶので、同じ行を同じ DDL で書いた 2 つの断面は等しくなる。
/// 20 表を束ねた同一性 (ダイジェスト・一致) は更新器の側が持つ — 表をまたぐ照合は DAO の
/// 仕事ではない (`coding-rules/read-model-updater-structure.md` 原則 3)。
#[derive(Debug, Clone, PartialEq)]
pub struct TableContent {
    rows: Vec<Vec<Value>>,
}

impl TableContent {
    /// 読んだ行を束ねる (**この型の唯一の構築経路**)。
    #[must_use]
    pub const fn new(rows: Vec<Vec<Value>>) -> Self {
        Self { rows }
    }

    /// 全行 (1 行は列の値の並び)。
    #[must_use]
    pub fn rows(&self) -> &[Vec<Value>] {
        &self.rows
    }
}
