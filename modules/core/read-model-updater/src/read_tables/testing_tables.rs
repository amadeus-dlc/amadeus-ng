//! 人が編集するテスト方針と、依頼の条件を組み合わせる参照投影。
use super::TestingContractRow;
use crate::orchestration::{GlobalSeqNr, JournalBatch};
use core_command_domain::orchestration::TestingSections;
use core_infrastructure::canon_json::{JsonValue, hash_canonical};
/// 全依頼のテスト契約と参照入力の照合子。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestingTables {
    rows: Vec<TestingContractRow>,
    source_digest: String,
    as_of: GlobalSeqNr,
}
impl TestingTables {
    /// 履歴が持つ依頼条件と規則の原文から、ドメインの計算結果を投影する。
    #[must_use]
    pub fn project(history: &JournalBatch, sections: &TestingSections) -> Self {
        let mut rows = vec![TestingContractRow::of(None, sections)];
        rows.extend(
            history
                .intents()
                .iter()
                .map(|intent| TestingContractRow::of(Some(intent), sections)),
        );
        rows.sort_by(|left, right| left.id().cmp(right.id()));
        let mut inputs: Vec<JsonValue> = [sections.org(), sections.team(), sections.project()]
            .into_iter()
            .map(|text| JsonValue::String(text.to_string()))
            .collect();
        inputs.extend(rows.iter().map(|row| {
            JsonValue::Array(
                [
                    row.id(),
                    row.contract().unwrap_or_default(),
                    row.error().unwrap_or_default(),
                ]
                .into_iter()
                .map(|text| JsonValue::String(text.to_string()))
                .collect(),
            )
        }));
        let source_digest = hash_canonical(&JsonValue::Array(inputs)).rendered();
        Self {
            rows,
            source_digest,
            as_of: history.scanned_to().unwrap_or(GlobalSeqNr::ZERO),
        }
    }
    /// 依頼条件を取得した履歴位置。規則自体の版はsource_digestで識別する。
    #[must_use]
    pub const fn as_of(&self) -> GlobalSeqNr {
        self.as_of
    }
    /// 全行。
    #[must_use]
    pub fn rows(&self) -> &[TestingContractRow] {
        &self.rows
    }
    /// 規則と依頼条件の照合子。
    #[must_use]
    pub fn source_digest(&self) -> &str {
        &self.source_digest
    }
}
