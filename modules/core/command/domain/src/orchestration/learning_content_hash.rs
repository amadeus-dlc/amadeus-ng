//! `LearningContentHash` — 学びの本文そのものから導く同一性。

/// 学びの本文の SHA-256（小文字 16 進 64 桁）。
///
/// **候補番号 (`c1`) は同一性ではない** — surface は起動のたびに `c1` から採番し直すので、
/// 同じ番号が別の学びを指しうる。重複抑止の鍵は本文のハッシュであり、切り詰めない
/// （固定本家 2.7.1 `a277af21` `aidlc-learnings.ts:632-654` の裁定）。
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LearningContentHash {
    hex: String,
}

/// SHA-256 の 16 進表記の桁数。
const HEX_LEN: usize = 64;

impl LearningContentHash {
    // 検査済みの綴りは、この構築口で全状態を初期化する。
    const fn of_hex(hex: String) -> Self {
        Self { hex }
    }

    /// 学びの本文から導く（**この型の唯一の算出経路**）。
    #[must_use]
    pub fn of_text(text: &str) -> LearningContentHash {
        LearningContentHash::of_hex(core_infrastructure::hash::sha256_hex(text.as_bytes()))
    }

    /// 既存の綴りを読み戻す（保存往復・監査照合）。
    ///
    /// # Errors
    /// 64 桁の小文字 16 進でない場合。
    pub fn parse(raw: &str) -> Result<LearningContentHash, super::LearningContentHashError> {
        if raw.len() != HEX_LEN || !raw.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(super::LearningContentHashError::new(raw));
        }
        if raw.bytes().any(|byte| byte.is_ascii_uppercase()) {
            return Err(super::LearningContentHashError::new(raw));
        }
        Ok(LearningContentHash::of_hex(raw.to_string()))
    }

    /// 監査行 `**Content-Hash**:` の綴り。
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.hex
    }
}

#[cfg(test)]
mod tests {
    use super::LearningContentHash;

    /// ゴールデン `tests/golden/upstream-a277af21/learnings/cases.json` の実測値。
    const GOLDEN_TEXT: &str = "ALWAYS 採取用の検証結果を記録する。";
    const GOLDEN_HASH: &str = "f543ed24a72a9b57b8fac723a95fa0a2c04240a25c8a6a67229322acb3de9bd3";

    #[test]
    fn the_hash_is_the_full_sha256_of_the_utf8_text() {
        assert_eq!(
            LearningContentHash::of_text(GOLDEN_TEXT).as_str(),
            GOLDEN_HASH
        );
    }

    #[test]
    fn different_texts_never_share_a_hash() {
        assert_ne!(
            LearningContentHash::of_text("a"),
            LearningContentHash::of_text("b")
        );
        assert_eq!(
            LearningContentHash::of_text("a"),
            LearningContentHash::of_text("a")
        );
    }

    #[test]
    fn only_a_full_lowercase_hex_digest_parses() {
        assert_eq!(
            LearningContentHash::parse(GOLDEN_HASH).map(|hash| hash.as_str().to_string()),
            Ok(GOLDEN_HASH.to_string())
        );
        assert!(LearningContentHash::parse(&GOLDEN_HASH[..8]).is_err());
        assert!(LearningContentHash::parse(&GOLDEN_HASH.to_uppercase()).is_err());
        assert!(LearningContentHash::parse(&"g".repeat(64)).is_err());
    }
}
