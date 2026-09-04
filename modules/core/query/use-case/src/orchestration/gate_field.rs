//! `GateField` — `run-stage` の `gate` フィールド (公開言語 B14)。
//!
//! ワイヤ上は boolean か `"unresolved"` の 3 値しか取らない (E2)。閉集合なので enum で運び、
//! 未知の値は構成不能にする。

/// `run-stage` の `gate` フィールド — boolean か `"unresolved"` のみ (E2)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateField {
    /// 承認ゲート付き (`true`)。
    Gated,
    /// ゲートなし (`false` — 初期化ステージ)。
    Ungated,
    /// walking-skeleton 判定が要る非決定ケース (`"unresolved"`)。
    Unresolved,
}

impl GateField {
    /// リードモデル `read_next_answer.gate` の綴りから起こす。
    ///
    /// 綴りの正本はコマンド側の `GateDecision::spelling` である — 型は側ごとに別だが
    /// (`coding-rules/cqrs-boundaries.md`)、行に書かれる 3 語は同じ 1 つの語彙である。
    /// 閉集合の外は `None` — 行が語彙の外の値を持つのは RMU の破損であり、読む側は
    /// 「値が無い」として扱って定義側の静的既定へ落ちる。
    #[must_use]
    pub fn parse(raw: &str) -> Option<GateField> {
        match raw {
            "gated" => Some(GateField::Gated),
            "ungated" => Some(GateField::Ungated),
            "unresolved" => Some(GateField::Unresolved),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_three_spellings_parse_and_anything_else_is_absent() {
        assert_eq!(GateField::parse("gated"), Some(GateField::Gated));
        assert_eq!(GateField::parse("ungated"), Some(GateField::Ungated));
        assert_eq!(GateField::parse("unresolved"), Some(GateField::Unresolved));
        assert_eq!(GateField::parse("true"), None);
        assert_eq!(GateField::parse(""), None);
    }
}
