//! `BrownfieldGreenfieldView` — `consumes[].conditional_on` の閉集合。

use super::unknown_value::UnknownValue;

/// プロジェクト種別。`always` に相当する第 3 の値は存在しない。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BrownfieldGreenfieldView {
    /// 既存コードベースの上で進むプロジェクト。
    Brownfield,
    /// 新規に起こすプロジェクト。
    Greenfield,
}

impl BrownfieldGreenfieldView {
    /// 宣言順の全値 (2 値の網羅走査の正本)。
    pub const ALL: [BrownfieldGreenfieldView; 2] = [
        BrownfieldGreenfieldView::Brownfield,
        BrownfieldGreenfieldView::Greenfield,
    ];

    /// # Errors
    ///
    /// 2 値以外は [`UnknownValue`] で拒否する。
    pub fn parse(s: &str) -> Result<BrownfieldGreenfieldView, UnknownValue> {
        Ok(match s {
            "brownfield" => BrownfieldGreenfieldView::Brownfield,
            "greenfield" => BrownfieldGreenfieldView::Greenfield,
            other => return Err(UnknownValue::new(other)),
        })
    }

    /// `stage-graph.json` 上の正準綴り (`parse` の逆写像)。
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            BrownfieldGreenfieldView::Brownfield => "brownfield",
            BrownfieldGreenfieldView::Greenfield => "greenfield",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_values_round_trip_and_unknown_is_rejected() {
        for b in BrownfieldGreenfieldView::ALL {
            assert_eq!(BrownfieldGreenfieldView::parse(b.as_str()).unwrap(), b);
        }
        let rejected = BrownfieldGreenfieldView::parse("bluefield").unwrap_err();
        assert_eq!(rejected.as_str(), "bluefield");
    }
}
