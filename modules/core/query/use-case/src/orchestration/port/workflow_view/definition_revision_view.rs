//! `DefinitionRevisionView` — リードモデル 3 入力の**内容版** (ADR-008)。識別子ではなく値属性。

use super::definition_revision_error::DefinitionRevisionError;

/// 正準ダイジェストの接頭辞 (canon-json の正準族 `Digest::rendered()` と同じ表記)。
const PREFIX: &str = "sha256:";
/// sha256 の 16 進表記の桁数。
///
/// 拒否の文言も同じ桁数を述べるので、主たる従属先である本ファイルに置いて
/// `definition_revision_error` から参照する。
pub(super) const HEX_LEN: usize = 64;

/// 3 入力の正準 JSON の sha256 ダイジェスト。
///
/// 同じ内容なら同じ revision、ピン更新で変わる。**識別子ではない**ため、これが変わっても
/// 定義の系譜 ([`super::DefinitionIdView`]) は変わらない。値の計算はアダプタ層
/// (canon-json) が行い、本型は形の検証と保持だけを担う。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DefinitionRevisionView(String);

impl DefinitionRevisionView {
    /// `sha256:<hex64>` (16 進は小文字) のみを受理する。
    ///
    /// # Errors
    ///
    /// 接頭辞の欠落・桁数違い・16 進小文字以外の文字を拒否する。
    pub fn parse(s: &str) -> Result<DefinitionRevisionView, DefinitionRevisionError> {
        let hex = s
            .strip_prefix(PREFIX)
            .ok_or(DefinitionRevisionError::MissingPrefix)?;
        if hex.len() != HEX_LEN {
            return Err(DefinitionRevisionError::InvalidLength { actual: hex.len() });
        }
        if let Some(c) = hex
            .chars()
            .find(|c| !(c.is_ascii_digit() || matches!(c, 'a'..='f')))
        {
            return Err(DefinitionRevisionError::InvalidHexDigit(c));
        }
        Ok(DefinitionRevisionView(s.to_string()))
    }

    /// `sha256:` 接頭辞付きの生表記。
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// 接頭辞を除いた 16 進 64 桁。`parse` が接頭辞を保証するので剥がしは必ず成功する。
    #[must_use]
    pub fn hex(&self) -> &str {
        self.0.strip_prefix(PREFIX).unwrap_or(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "sha256:303d9bb7b5d777d54a6761be9ed154d85d5bb3f2d6b9cce02f71f4ed1b3a4ff3";

    #[test]
    fn parse_round_trips_a_canonical_digest_rendering() {
        let revision = DefinitionRevisionView::parse(SAMPLE).unwrap();
        assert_eq!(revision.as_str(), SAMPLE);
        assert_eq!(
            revision.hex(),
            "303d9bb7b5d777d54a6761be9ed154d85d5bb3f2d6b9cce02f71f4ed1b3a4ff3"
        );
    }

    #[test]
    fn only_the_canonical_family_rendering_is_accepted() {
        assert_eq!(
            DefinitionRevisionView::parse(&"0".repeat(HEX_LEN)),
            Err(DefinitionRevisionError::MissingPrefix)
        );
        assert_eq!(
            DefinitionRevisionView::parse("sha256:abc"),
            Err(DefinitionRevisionError::InvalidLength { actual: 3 })
        );
        let upper = format!("sha256:{}", "A".repeat(HEX_LEN));
        assert_eq!(
            DefinitionRevisionView::parse(&upper),
            Err(DefinitionRevisionError::InvalidHexDigit('A'))
        );
    }
}
