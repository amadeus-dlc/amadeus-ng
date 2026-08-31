//! `SkeletonDefaultView` — scope frontmatter の `skeleton:` (2 値)。

use super::unknown_value::UnknownValue;

/// スコープ既定の walking-skeleton 姿勢。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SkeletonDefaultView {
    /// スコープ既定で walking skeleton Bolt を先に走らせる (upstream 01 §6.4)。
    On,
    /// スコープ既定では walking skeleton Bolt を走らせない。
    Off,
}

impl SkeletonDefaultView {
    /// 宣言順の全値 (2 値の網羅走査の正本)。
    pub const ALL: [SkeletonDefaultView; 2] = [SkeletonDefaultView::On, SkeletonDefaultView::Off];

    /// # Errors
    ///
    /// `on` / `off` 以外は [`UnknownValue`] で拒否する。
    pub fn parse(s: &str) -> Result<SkeletonDefaultView, UnknownValue> {
        Ok(match s {
            "on" => SkeletonDefaultView::On,
            "off" => SkeletonDefaultView::Off,
            other => return Err(UnknownValue::new(other)),
        })
    }

    /// frontmatter 上の正準綴り (`parse` の逆写像)。
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            SkeletonDefaultView::On => "on",
            SkeletonDefaultView::Off => "off",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_on_and_off_are_accepted() {
        for s in SkeletonDefaultView::ALL {
            assert_eq!(SkeletonDefaultView::parse(s.as_str()).unwrap(), s);
        }
        let rejected = SkeletonDefaultView::parse("yes").unwrap_err();
        assert_eq!(rejected.as_str(), "yes");
        assert!(SkeletonDefaultView::parse("enabled").is_err());
    }
}
