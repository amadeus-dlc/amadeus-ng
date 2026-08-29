//! `WireDecodeError` — 行のバイトをドメインへ写せない理由 (材料のみ、**読む側の写し**)。

/// 復号の失敗。文言は持たず**材料だけ**を運ぶ (`coding-rules/error-handling.md`)。
///
/// 呼出側 (`JournalReaderImpl`) はこれを RMU の `CorruptCause` へ写す — 「読めない」か
/// 「読めたが不変条件を破る」かの区別だけが上位の関心事だからである。
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

impl WireDecodeError {
    /// 綴り・文法の拒否を組む。
    pub fn malformed(field: &'static str, found: impl Into<String>) -> WireDecodeError {
        WireDecodeError::Malformed {
            field,
            found: found.into(),
        }
    }
}
