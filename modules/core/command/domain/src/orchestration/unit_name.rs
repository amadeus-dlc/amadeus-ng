//! `UnitName` — Construction の作業単位ディレクトリ 1 階層分の名前。
use super::UnitNameError;
/// `construction/<unit>/<stage>/…` の `<unit>` に現れる 1 階層の名前。
///
/// 経路の一部でありながら**経路ではない** — 区切りを含む綴りは Unit を名指していないので、
/// 構築の時点で拒否する (upstream `producesArtifactUnit` の
/// `unit.length > 0 && !unit.includes("/")`)。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UnitName(String);
impl UnitName {
    /// 生の綴りを検査して名前にする (**この型の唯一の構築経路**)。
    ///
    /// # Errors
    /// 空、または経路区切り (`/`) を含む場合。
    pub fn parse(raw: &str) -> Result<UnitName, UnitNameError> {
        if raw.is_empty() {
            return Err(UnitNameError::Empty);
        }
        if raw.contains('/') {
            return Err(UnitNameError::Separated);
        }
        Ok(UnitName(raw.to_string()))
    }
    /// 監査項目・照合に渡す正準綴り。
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
#[cfg(test)]
mod tests {
    use super::{UnitName, UnitNameError};
    #[test]
    fn a_single_directory_name_is_a_unit() {
        let unit = UnitName::parse("u2-workflow-authority").unwrap();
        assert_eq!(unit.as_str(), "u2-workflow-authority");
    }
    #[test]
    fn an_empty_spelling_names_no_unit() {
        assert_eq!(UnitName::parse(""), Err(UnitNameError::Empty));
    }
    #[test]
    fn a_nested_path_is_not_a_unit_name() {
        assert_eq!(
            UnitName::parse("u2/inner"),
            Err(UnitNameError::Separated),
            "区切りを含む綴りは 1 階層の Unit ではない"
        );
    }
}
