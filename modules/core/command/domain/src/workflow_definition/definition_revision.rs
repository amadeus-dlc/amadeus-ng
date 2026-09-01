//! `DefinitionRevision` — `WorkflowDefinition` の内容版 (ADR-008)。識別子ではなく**値属性**。

use std::fmt;

use super::definition_revision_error::DefinitionRevisionError;

/// 正準ダイジェストの接頭辞 (canon-json の正準族 `Digest::rendered()` と同じ表記)。
const PREFIX: &str = "sha256:";
/// sha256 の 16 進表記の桁数。`DefinitionRevisionError` の Display も同じ桁数を文言に
/// 載せるため、値の正本をここ 1 箇所に置いたまま兄弟モジュールへ見せる。
pub(super) const HEX_LEN: usize = 64;

/// 3 入力 (コンパイル済み `stage-graph.json` / `scope-grid.json` / scope identity 群) の
/// 正準 JSON の sha256 ダイジェスト (Always Valid)。
///
/// 同じ内容なら同じ revision、ピン更新で変わる。**識別子ではない**ため、これが変わっても
/// 定義の系譜 (`WorkflowDefinitionId`) は変わらない。来歴と drift 検出の材料。
///
/// 値の計算はアダプタ層 (Repository 実装 + canon-json) が行う。ドメインは形の検証と
/// 保持だけを担う (core-command-domain は canon-json に依存しない — NFR4.1)。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DefinitionRevision(String);

impl DefinitionRevision {
    /// `sha256:<hex64>` (16 進は小文字) のみを受理する。
    ///
    /// # Errors
    ///
    /// 接頭辞の欠落・桁数違い・16 進小文字以外の文字を拒否する。
    pub fn parse(s: &str) -> Result<DefinitionRevision, DefinitionRevisionError> {
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
        Ok(DefinitionRevision(s.to_string()))
    }

    /// `sha256:` 接頭辞付きの生表記。
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// 接頭辞を除いた 16 進 64 桁。
    #[must_use]
    pub fn hex(&self) -> &str {
        &self.0[PREFIX.len()..]
    }
}

impl TryFrom<String> for DefinitionRevision {
    type Error = DefinitionRevisionError;

    fn try_from(value: String) -> Result<DefinitionRevision, DefinitionRevisionError> {
        DefinitionRevision::parse(&value)
    }
}

impl fmt::Display for DefinitionRevision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// canon-json の正準族 `Digest::rendered()` が返す形の代表値。
    const SAMPLE: &str = "sha256:303d9bb7b5d777d54a6761be9ed154d85d5bb3f2d6b9cce02f71f4ed1b3a4ff3";

    #[test]
    fn parse_round_trips_a_canonical_digest_rendering() {
        let revision = DefinitionRevision::parse(SAMPLE).unwrap();
        assert_eq!(revision.as_str(), SAMPLE);
        assert_eq!(revision.to_string(), SAMPLE);
        assert_eq!(
            revision.hex(),
            "303d9bb7b5d777d54a6761be9ed154d85d5bb3f2d6b9cce02f71f4ed1b3a4ff3"
        );
    }

    #[test]
    fn the_all_zero_digest_used_by_synthetic_fixtures_is_accepted() {
        let raw = format!("sha256:{}", "0".repeat(HEX_LEN));
        assert_eq!(DefinitionRevision::parse(&raw).unwrap().as_str(), raw);
    }

    #[test]
    fn a_bare_hex_digest_is_rejected_because_the_family_is_part_of_the_form() {
        // 非正準族 (`hash_compact`) は生 hex を返す。取り違えを型で止める。
        let bare = "303d9bb7b5d777d54a6761be9ed154d85d5bb3f2d6b9cce02f71f4ed1b3a4ff3";
        assert_eq!(
            DefinitionRevision::parse(bare),
            Err(DefinitionRevisionError::MissingPrefix)
        );
        assert_eq!(
            DefinitionRevision::parse(""),
            Err(DefinitionRevisionError::MissingPrefix)
        );
        assert_eq!(
            DefinitionRevision::parse("md5:abcd"),
            Err(DefinitionRevisionError::MissingPrefix)
        );
    }

    #[test]
    fn a_digest_of_the_wrong_width_is_rejected() {
        assert_eq!(
            DefinitionRevision::parse("sha256:abc"),
            Err(DefinitionRevisionError::InvalidLength { actual: 3 })
        );
        let too_long = format!("sha256:{}", "a".repeat(HEX_LEN + 1));
        assert_eq!(
            DefinitionRevision::parse(&too_long),
            Err(DefinitionRevisionError::InvalidLength {
                actual: HEX_LEN + 1
            })
        );
    }

    #[test]
    fn uppercase_hex_and_non_hex_characters_are_rejected() {
        // canon-json は小文字 hex を返すので、大文字は「別経路で作った値」の印。
        let upper = format!("sha256:{}", "A".repeat(HEX_LEN));
        assert_eq!(
            DefinitionRevision::parse(&upper),
            Err(DefinitionRevisionError::InvalidHexDigit('A'))
        );
        let with_g = format!("sha256:g{}", "0".repeat(HEX_LEN - 1));
        assert_eq!(
            DefinitionRevision::parse(&with_g),
            Err(DefinitionRevisionError::InvalidHexDigit('g'))
        );
    }

    #[test]
    fn ordering_and_equality_follow_the_raw_rendering() {
        let a = DefinitionRevision::parse(&format!("sha256:{}", "0".repeat(HEX_LEN))).unwrap();
        let b = DefinitionRevision::parse(&format!("sha256:{}", "1".repeat(HEX_LEN))).unwrap();
        assert!(a < b);
        assert_eq!(a, DefinitionRevision::parse(a.as_str()).unwrap());
        assert_ne!(a, b);
    }

    #[test]
    fn the_rejection_carries_material_not_wording() {
        assert_eq!(
            DefinitionRevisionError::MissingPrefix.to_string(),
            "missing sha256: prefix"
        );
        assert_eq!(
            DefinitionRevisionError::InvalidLength { actual: 3 }.to_string(),
            "expected 64 hex digits, got 3"
        );
        assert_eq!(
            DefinitionRevisionError::InvalidHexDigit('A').to_string(),
            "not a lowercase hex digit: 'A'"
        );
    }
}
