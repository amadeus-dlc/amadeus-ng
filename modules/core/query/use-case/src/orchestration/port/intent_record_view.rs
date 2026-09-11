//! 登録簿へ投影された依頼と記録ディレクトリの対応。
/// ドメインや配置導出の規則を持たない読取ビュー。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntentRecordView {
    intent_id: String,
    directory: String,
}
impl IntentRecordView {
    /// 保存された対応を束ねる。
    #[must_use]
    pub const fn new(intent_id: String, directory: String) -> Self {
        Self {
            intent_id,
            directory,
        }
    }
    /// 依頼の識別子。
    #[must_use]
    pub fn intent_id(&self) -> &str {
        &self.intent_id
    }
    /// 登録された記録ディレクトリ名。
    #[must_use]
    pub fn directory(&self) -> &str {
        &self.directory
    }
}
