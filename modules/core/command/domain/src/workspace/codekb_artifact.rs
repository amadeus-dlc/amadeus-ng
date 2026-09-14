//! `CodekbArtifact` — codekb の成果物 1 枚 (名前と中身)。

use super::codekb_artifact_name::CodekbArtifactName;

/// codekb の成果物 1 枚。
///
/// 中身は**バイト列のまま**運ぶ — 公開はバイト単位の差し替えであり、途中で正規化・整形を
/// 挟むと staged の候補と公開後のストアが別物になる。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodekbArtifact {
    name: CodekbArtifactName,
    bytes: Vec<u8>,
}

impl CodekbArtifact {
    /// 名前と中身を束ねる (**この型の唯一の構築経路**)。
    #[must_use]
    pub const fn new(name: CodekbArtifactName, bytes: Vec<u8>) -> CodekbArtifact {
        CodekbArtifact { name, bytes }
    }

    /// 成果物の名前。
    #[must_use]
    pub const fn name(&self) -> &CodekbArtifactName {
        &self.name
    }

    /// 成果物の中身 (公開の境界へ渡すバイト)。
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}
