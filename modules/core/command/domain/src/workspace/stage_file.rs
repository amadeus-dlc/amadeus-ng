//! ステージ本体ファイル 1 本の写し (frontmatter の解釈は判断側)。

use super::ObservationFailure;

/// `<stages>/<phase>/<slug>.md` の所在と中身。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageFile {
    phase: String,
    slug: String,
    content: Result<String, ObservationFailure>,
}

impl StageFile {
    /// 観測を束ねる。
    #[must_use]
    pub const fn new(
        phase: String,
        slug: String,
        content: Result<String, ObservationFailure>,
    ) -> Self {
        Self {
            phase,
            slug,
            content,
        }
    }

    /// 置かれているフェーズディレクトリ名。
    #[must_use]
    pub fn phase(&self) -> &str {
        &self.phase
    }

    /// ファイル名の幹。
    #[must_use]
    pub fn slug(&self) -> &str {
        &self.slug
    }

    /// 中身 (読めなければ原因)。
    pub const fn content(&self) -> &Result<String, ObservationFailure> {
        &self.content
    }
}
