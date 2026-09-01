//! `SerializationProfile` が選ぶインデントの単位 (BR1.5)。

/// インデントの単位 (BR1.5)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Indent {
    /// 1 段につき半角スペース 2 個。
    TwoSpaces,
    /// インデントなし (空白を一切入れない)。
    None,
}

impl Indent {
    /// 1 段分のインデント文字列。
    #[must_use]
    pub const fn unit(self) -> &'static str {
        match self {
            Indent::TwoSpaces => "  ",
            Indent::None => "",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indent_unit_is_two_spaces_or_empty() {
        assert_eq!(Indent::TwoSpaces.unit(), "  ");
        assert_eq!(Indent::None.unit(), "");
    }
}
