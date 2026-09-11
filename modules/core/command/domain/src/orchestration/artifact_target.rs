//! `ArtifactTarget` — 書込み先が宣言成果物のどこに当たるか。
use super::UnitName;
/// 1 つの書込み先を 1 つのステージ宣言に照合した結果。
///
/// upstream `producesArtifactUnit` の 3 値 (`undefined` / `null` / `string`) を、
/// 意味の読める閉集合として持つ — 生の `Option<Option<String>>` は「宣言外」と
/// 「ステージ水準」を呼出側に読み分けさせるので使わない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArtifactTarget {
    /// このステージの宣言成果物ではない。
    Foreign,
    /// このステージの成果物で、Unit を名指していない (ステージ水準の書込み)。
    Stage,
    /// per-unit ステージの、名指しされた Unit ディレクトリ配下の書込み。
    Unit(UnitName),
}
impl ArtifactTarget {
    /// このステージの宣言成果物か (`Foreign` でないか)。
    #[must_use]
    pub const fn is_declared(&self) -> bool {
        !matches!(self, ArtifactTarget::Foreign)
    }
    /// 名指しされた Unit (ステージ水準・宣言外は `None`)。
    #[must_use]
    pub const fn unit(&self) -> Option<&UnitName> {
        match self {
            ArtifactTarget::Unit(unit) => Some(unit),
            ArtifactTarget::Foreign | ArtifactTarget::Stage => None,
        }
    }
}
#[cfg(test)]
mod tests {
    use super::{ArtifactTarget, UnitName};
    #[test]
    fn a_foreign_target_is_neither_declared_nor_unit_scoped() {
        let target = ArtifactTarget::Foreign;
        assert!(!target.is_declared());
        assert_eq!(target.unit(), None);
    }
    #[test]
    fn a_stage_level_target_is_declared_without_a_unit() {
        let target = ArtifactTarget::Stage;
        assert!(target.is_declared());
        assert_eq!(target.unit(), None);
    }
    #[test]
    fn a_unit_target_carries_the_named_unit() {
        let unit = UnitName::parse("u2-workflow-authority").unwrap();
        let target = ArtifactTarget::Unit(unit.clone());
        assert!(target.is_declared());
        assert_eq!(target.unit(), Some(&unit));
    }
}
