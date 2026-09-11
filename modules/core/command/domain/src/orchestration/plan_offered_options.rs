//! 計画承認で提示する、順序付きの2択。
use super::PlanApprovalError;
/// 本家のCSV区切りとJavaScript空白規則で解釈した選択肢。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanOfferedOptions {
    values: [String; 2],
}
impl PlanOfferedOptions {
    /// 空の項目を除いた後に、必ず2択であることを確認する。
    /// # Errors
    /// 選択肢が2件でない場合。
    pub fn parse(raw: &str) -> Result<Self, PlanApprovalError> {
        let values: Vec<_> = raw
            .split(',')
            .map(core_infrastructure::ecmascript::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
            .collect();
        let values = <[String; 2]>::try_from(values).map_err(|_| {
            PlanApprovalError::new("Plan Approval decision requires exactly two offered options")
        })?;
        Ok(Self { values })
    }
    /// 意味の位置を維持した2択。
    #[must_use]
    pub const fn values(&self) -> &[String; 2] {
        &self.values
    }
}
