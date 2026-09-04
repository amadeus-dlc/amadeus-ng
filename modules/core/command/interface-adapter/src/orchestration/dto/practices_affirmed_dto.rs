//! `PracticesAffirmedDto` — `PracticesAffirmed` の材料。

use core_command_domain::workspace::PromotedSection;
use serde::{Deserialize, Serialize};

/// `PracticesAffirmed` の材料。**`id` (イベント自身の識別子) と `aggregate_id`
/// (どの集約の事実か) を先頭に置く並びが契約**である。
///
/// `mandated` / `forbidden` は**印付きの行**そのもの (`<rule> (affirmed YYYY-MM-DD)`) —
/// 投影はこの行をそのまま `## Mandated` / `## Forbidden` へ足す。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PracticesAffirmedDto {
    pub(super) id: String,
    pub(super) aggregate_id: String,
    pub(super) stage: String,
    pub(super) affirming_user: String,
    pub(super) sections: Vec<PromotedSectionDto>,
    pub(super) mandated: Vec<String>,
    pub(super) forbidden: Vec<String>,
}

/// 置き換える節 1 つのワイヤ形 (`heading` は `## ` を含まない裸の名前)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct PromotedSectionDto {
    pub(super) heading: String,
    pub(super) body: String,
}

impl PromotedSectionDto {
    /// ドメインの読取面から行の形を組む (書き)。
    pub(super) fn of(section: &PromotedSection) -> PromotedSectionDto {
        PromotedSectionDto {
            heading: section.heading().to_string(),
            body: section.body().to_string(),
        }
    }

    /// ドメインの値オブジェクトへ戻す (読み — 節の綴りに文法検査は無い)。
    pub(super) fn to_domain(&self) -> PromotedSection {
        PromotedSection::new(self.heading.clone(), self.body.clone())
    }
}
