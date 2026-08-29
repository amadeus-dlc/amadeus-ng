//! `WireDecodeError` — 行のバイトをドメインへ写せない理由 (材料のみ)。

/// 復号の失敗。文言は持たず**材料だけ**を運ぶ (`coding-rules/error-handling.md`)。
///
/// 呼出側 (Repository) はこれを `Corrupt` の `source` 連鎖に載せる (裁定 6 — 分類はポート
/// 契約に載せず、診断表示だけを原因連鎖で残す)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WireDecodeError {
    /// 閉集合の外の綴り、または Domain Primitive の文法外の値。
    Malformed {
        /// 問題のあった DTO のフィールド名。
        field: &'static str,
        /// 拒否された生値 (正規化しない)。
        found: String,
    },
    /// 形は読めたが、組み上げるとドメインの不変条件を破る。
    InvariantViolation,
}

impl std::fmt::Display for WireDecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WireDecodeError::Malformed { field, found } => {
                write!(f, "malformed field {field}: {found}")
            }
            WireDecodeError::InvariantViolation => f.write_str("invariant violation"),
        }
    }
}

impl std::error::Error for WireDecodeError {}

impl WireDecodeError {
    /// 綴り・文法の拒否を組む。
    pub fn malformed(field: &'static str, found: impl Into<String>) -> WireDecodeError {
        WireDecodeError::Malformed {
            field,
            found: found.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_rejection_renders_its_material() {
        assert_eq!(
            WireDecodeError::malformed("id", "not-a-uuid").to_string(),
            "malformed field id: not-a-uuid"
        );
        assert_eq!(
            WireDecodeError::InvariantViolation.to_string(),
            "invariant violation"
        );
    }

    #[test]
    fn the_rejection_is_a_std_error() {
        let error: Box<dyn std::error::Error> = Box::new(WireDecodeError::InvariantViolation);
        assert_eq!(error.to_string(), "invariant violation");
    }
}
