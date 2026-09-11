//! 保存後に観測した成果物書込の事実。
use super::HookHealthTarget;
/// RMUへ渡す保存観測。ファイルI/Oは呼出側が済ませ、ここは材料と分類だけを持つ。
#[derive(Debug, Clone, PartialEq, Eq)]
/// 完成した成果物監査のドメイン型。
pub struct ArtifactWriteObservation {
    target: HookHealthTarget,
    tool: String,
    file: String,
    context: String,
    created: bool,
}
impl ArtifactWriteObservation {
    /// 全観測材料を受け取る完全コンストラクタ。
    #[must_use]
    /// 検査済みの値を返す。
    pub const fn new(
        target: HookHealthTarget,
        tool: String,
        file: String,
        context: String,
        created: bool,
    ) -> Self {
        Self {
            target,
            tool,
            file,
            context,
            created,
        }
    }
    /// 観測対象。
    #[must_use]
    /// 検査済みの値を返す。
    pub const fn target(&self) -> &HookHealthTarget {
        &self.target
    }
    /// 使用したツール。
    #[must_use]
    /// 検査済みの値を返す。
    pub fn tool(&self) -> &str {
        &self.tool
    }
    /// 書き込んだ絶対パス。
    #[must_use]
    /// 検査済みの値を返す。
    pub fn file(&self) -> &str {
        &self.file
    }
    /// 記録上の相対文脈。
    #[must_use]
    /// 検査済みの値を返す。
    pub fn context(&self) -> &str {
        &self.context
    }
    /// 新規作成か。
    #[must_use]
    /// 検査済みの値を返す。
    pub const fn created(&self) -> bool {
        self.created
    }
}
