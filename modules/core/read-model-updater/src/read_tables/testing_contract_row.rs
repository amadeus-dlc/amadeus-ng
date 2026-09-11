//! 規則と依頼からドメインが解決したテスト契約の行。
use core_command_domain::orchestration::{Intent, TestingContext, TestingPosture, TestingSections};
use core_infrastructure::canon_json::{SerializationProfile, serialize};
/// 成功した契約か、現在の規則を受理できない理由のどちらかを保持する。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestingContractRow {
    id: String,
    contract: Option<String>,
    rendered: Option<String>,
    error: Option<String>,
}
impl TestingContractRow {
    const fn new(
        id: String,
        contract: Option<String>,
        rendered: Option<String>,
        error: Option<String>,
    ) -> Self {
        Self {
            id,
            contract,
            rendered,
            error,
        }
    }

    pub(super) fn of(intent: Option<&Intent>, sections: &TestingSections) -> Self {
        let id = intent
            .map_or("bare-space", |intent| intent.id().as_str())
            .to_string();
        match TestingPosture::resolve(sections, &TestingContext::for_intent(intent)) {
            Ok(posture) => {
                let contract = serialize(posture.value(), SerializationProfile::ContractPretty);
                let rendered = format!("## Testing Contract\n\n```json\n{contract}```\n");
                Self::new(id, Some(contract), Some(rendered), None)
            }
            Err(error) => Self::new(id, None, None, Some(error.to_string())),
        }
    }
    /// 依頼ID。依頼前の既定行はbare-space。
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }
    /// 解決済み契約の公開JSON。
    #[must_use]
    pub fn contract(&self) -> Option<&str> {
        self.contract.as_deref()
    }
    /// 計画へ載せる公開Markdown。
    #[must_use]
    pub fn rendered(&self) -> Option<&str> {
        self.rendered.as_deref()
    }
    /// 規則の拒否理由。
    #[must_use]
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }
}
