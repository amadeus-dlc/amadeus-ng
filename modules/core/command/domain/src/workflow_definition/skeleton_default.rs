//! `SkeletonDefault` — scope frontmatter の `skeleton:` 既定値 (`on` / `off` の 2 値)。

use super::unknown_skeleton_default::UnknownSkeletonDefault;

/// `skeleton:` の既定値。scope frontmatter では `on` / `off` の 2 値のみ
/// (orchestration の `SkeletonStance` が持つ `scope-dependent` はここには現れない)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SkeletonDefault {
    /// スコープ既定で walking skeleton Bolt を先に走らせる (upstream 01 §6.4)。
    On,
    /// スコープ既定では walking skeleton Bolt を走らせない。
    Off,
}

impl SkeletonDefault {
    /// 宣言順の全値 (2 値の網羅走査の正本)。
    pub const ALL: [SkeletonDefault; 2] = [SkeletonDefault::On, SkeletonDefault::Off];

    /// # Errors
    ///
    /// `on` / `off` 以外は `UnknownSkeletonDefault` で拒否する。
    pub fn parse(s: &str) -> Result<SkeletonDefault, UnknownSkeletonDefault> {
        Ok(match s {
            "on" => SkeletonDefault::On,
            "off" => SkeletonDefault::Off,
            other => return Err(UnknownSkeletonDefault::new(other)),
        })
    }

    /// frontmatter 上の正準綴り (`parse` の逆写像)。
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            SkeletonDefault::On => "on",
            SkeletonDefault::Off => "off",
        }
    }
}
