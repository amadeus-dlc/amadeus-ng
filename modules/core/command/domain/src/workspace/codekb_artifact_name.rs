//! `CodekbArtifactName` — codekb が持つ 9 成果物の名前。

use std::fmt;

use super::codekb_artifact_name_error::CodekbArtifactNameError;

/// codekb ストアを構成する成果物の名前 (upstream `CODEKB_ARTIFACT_FILES` 逐語)。
///
/// **9 つで閉じた集合**である。開いた文字列にすると「9 つちょうど」という公開の前提を
/// 型で語れなくなり、綴り違いが実行時まで生き延びる。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CodekbArtifactName {
    /// `api-documentation.md`
    ApiDocumentation,
    /// `architecture.md`
    Architecture,
    /// `business-overview.md`
    BusinessOverview,
    /// `code-quality-assessment.md`
    CodeQualityAssessment,
    /// `code-structure.md`
    CodeStructure,
    /// `component-inventory.md`
    ComponentInventory,
    /// `dependencies.md`
    Dependencies,
    /// `reverse-engineering-timestamp.md` — 鮮度印。走査範囲ブロックを載せる 1 枚。
    ReverseEngineeringTimestamp,
    /// `technology-stack.md`
    TechnologyStack,
}

impl CodekbArtifactName {
    /// 正準の 9 綴りを**辞書順**で返す。
    ///
    /// upstream は `CODEKB_ARTIFACT_FILES` を宣言順のまま持ち、比較の直前に `sort()` する。
    /// こちらは最初から辞書順で持つので、並べ替えが要らない。
    #[must_use]
    pub const fn all() -> [CodekbArtifactName; 9] {
        [
            CodekbArtifactName::ApiDocumentation,
            CodekbArtifactName::Architecture,
            CodekbArtifactName::BusinessOverview,
            CodekbArtifactName::CodeQualityAssessment,
            CodekbArtifactName::CodeStructure,
            CodekbArtifactName::ComponentInventory,
            CodekbArtifactName::Dependencies,
            CodekbArtifactName::ReverseEngineeringTimestamp,
            CodekbArtifactName::TechnologyStack,
        ]
    }

    /// ファイル名の綴り (逐語)。
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            CodekbArtifactName::ApiDocumentation => "api-documentation.md",
            CodekbArtifactName::Architecture => "architecture.md",
            CodekbArtifactName::BusinessOverview => "business-overview.md",
            CodekbArtifactName::CodeQualityAssessment => "code-quality-assessment.md",
            CodekbArtifactName::CodeStructure => "code-structure.md",
            CodekbArtifactName::ComponentInventory => "component-inventory.md",
            CodekbArtifactName::Dependencies => "dependencies.md",
            CodekbArtifactName::ReverseEngineeringTimestamp => "reverse-engineering-timestamp.md",
            CodekbArtifactName::TechnologyStack => "technology-stack.md",
        }
    }

    /// 綴りから成果物名を起こす。
    ///
    /// # Errors
    ///
    /// 正準の 9 綴りのどれでもない綴りを拒否する。
    pub fn parse(s: &str) -> Result<CodekbArtifactName, CodekbArtifactNameError> {
        CodekbArtifactName::all()
            .into_iter()
            .find(|name| name.as_str() == s)
            .ok_or_else(|| CodekbArtifactNameError::Unknown(s.to_string()))
    }
}

impl fmt::Display for CodekbArtifactName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
