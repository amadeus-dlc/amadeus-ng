//! 人が編集するテスト方針と、依頼の条件を組み合わせる参照投影。
use crate::orchestration::{GlobalSeqNr, JournalBatch, TestingContractRow};
use core_command_domain::orchestration::{Intent, TestingContext, TestingPosture, TestingSections};
use core_infrastructure::canon_json::{JsonValue, SerializationProfile, hash_canonical, serialize};
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
        let mut rows = vec![Self::contract_row(None, sections)];
        rows.extend(
            history
                .intents()
                .iter()
                .map(|intent| Self::contract_row(Some(intent), sections)),
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
    /// 依頼 1 件 (無ければ依頼前の既定行 `bare-space`) の契約を、ドメインに解決させて行へ写す。
    ///
    /// 解決に成功すれば契約の公開 JSON と計画へ載せる Markdown を、失敗すれば拒否理由を持つ。
    fn contract_row(intent: Option<&Intent>, sections: &TestingSections) -> TestingContractRow {
        let id = intent
            .map_or("bare-space", |intent| intent.id().as_str())
            .to_string();
        match TestingPosture::resolve(sections, &TestingContext::for_intent(intent)) {
            Ok(posture) => {
                let contract = serialize(posture.value(), SerializationProfile::ContractPretty);
                let rendered = format!("## Testing Contract\n\n```json\n{contract}```\n");
                TestingContractRow::new(id, Some(contract), Some(rendered), None)
            }
            Err(error) => TestingContractRow::new(id, None, None, Some(error.to_string())),
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
