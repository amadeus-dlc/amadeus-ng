//! `PracticesAffirmed` の永続化 DTO (**読む側**)。

use core_command_domain::orchestration::PracticesAffirmed;
use core_command_domain::workspace::{PromotedSection, PromotedSections, RuleLines};
use serde::{Deserialize, Serialize};

use super::dto_decode_error::DtoDecodeError;
use super::intent_execution_event_dto::{aggregate_id_of, event_id_of, slug_of, slug_spelling};

/// `PracticesAffirmed` の材料。**`id` (イベント自身の識別子) と `aggregate_id`
/// (どの集約の事実か) を先頭に置く並びが契約**である。
///
/// `mandated` / `forbidden` は**印付きの行**そのもの (`<rule> (affirmed YYYY-MM-DD)`) —
/// 投影はこの行をそのまま `## Mandated` / `## Forbidden` へ足す。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PracticesAffirmedDto {
    id: String,
    aggregate_id: String,
    stage: String,
    affirming_user: String,
    sections: Vec<PromotedSectionDto>,
    mandated: Vec<String>,
    forbidden: Vec<String>,
}

/// 置き換える節 1 つのワイヤ形 (`heading` は `## ` を含まない裸の名前)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct PromotedSectionDto {
    heading: String,
    body: String,
}

/// 規則行の列を行の列へ写す (順序・重複はそのまま)。
fn rule_column(lines: &RuleLines) -> Vec<String> {
    lines.fold_left(Vec::new(), |mut column, line| {
        column.push(line.to_string());
        column
    })
}

impl PracticesAffirmedDto {
    /// ドメインの公開アクセサだけを読んで DTO を組む (書き)。
    pub(super) fn of(payload: &PracticesAffirmed) -> PracticesAffirmedDto {
        PracticesAffirmedDto {
            id: payload.id().as_str().to_string(),
            aggregate_id: payload.aggregate_id().as_str().to_string(),
            stage: slug_spelling(payload.stage()),
            affirming_user: payload.affirming_user().to_string(),
            sections: payload
                .sections()
                .fold_left(Vec::new(), |mut rows, section| {
                    rows.push(PromotedSectionDto {
                        heading: section.heading().to_string(),
                        body: section.body().to_string(),
                    });
                    rows
                }),
            mandated: rule_column(payload.mandated()),
            forbidden: rule_column(payload.forbidden()),
        }
    }

    /// ドメインの材料へ戻す (読み)。
    pub(super) fn to_domain(&self) -> Result<PracticesAffirmed, DtoDecodeError> {
        Ok(PracticesAffirmed::new(
            event_id_of(&self.id)?,
            aggregate_id_of(&self.aggregate_id)?,
            slug_of(&self.stage, "stage")?,
            self.affirming_user.clone(),
            PromotedSections::new(
                self.sections
                    .iter()
                    .map(|section| {
                        PromotedSection::new(section.heading.clone(), section.body.clone())
                    })
                    .collect(),
            )
            .map_err(|_| DtoDecodeError::InvariantViolation)?,
            RuleLines::new(self.mandated.clone()),
            RuleLines::new(self.forbidden.clone()),
        ))
    }
}
