//! `BrownfieldGreenfield` — 既存資産の有無による 2 値。

use super::unknown_brownfield_greenfield::UnknownBrownfieldGreenfield;

/// `consumes[].conditional_on` の閉集合。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
// ワイヤ綴りは `stage-graph.json` の正準綴り (小文字) に合わせる — `parse` / `as_str` と
// 同じ 1 つの綴りだけがワイヤに出るようにするため。
pub enum BrownfieldGreenfield {
    /// 既存コードベースの上で進むプロジェクト。
    Brownfield,
    /// 新規に起こすプロジェクト。
    Greenfield,
}

impl BrownfieldGreenfield {
    /// 宣言順の全値 (2 値の網羅走査の正本)。`always` に相当する第 3 の値は存在しない。
    pub const ALL: [BrownfieldGreenfield; 2] = [
        BrownfieldGreenfield::Brownfield,
        BrownfieldGreenfield::Greenfield,
    ];

    /// # Errors
    ///
    /// 2 値以外は `UnknownBrownfieldGreenfield` で拒否する。
    pub fn parse(s: &str) -> Result<BrownfieldGreenfield, UnknownBrownfieldGreenfield> {
        Ok(match s {
            "brownfield" => BrownfieldGreenfield::Brownfield,
            "greenfield" => BrownfieldGreenfield::Greenfield,
            other => return Err(UnknownBrownfieldGreenfield::new(other)),
        })
    }

    /// `stage-graph.json` 上の正準綴り (`parse` の逆写像)。
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            BrownfieldGreenfield::Brownfield => "brownfield",
            BrownfieldGreenfield::Greenfield => "greenfield",
        }
    }
}
