//! シェル文字列の観測投影で発生する技術的失敗。
/// 固定パターンの不正と、文字位置の不整合を区別する。
#[derive(Debug)]
pub enum ShellParseError {
    /// 正規表現の構築失敗。
    Pattern(regex::Error),
    /// 取得した文字位置の不整合。
    Boundary {
        /// 不整合を検出した位置。
        offset: usize,
    },
}
impl std::fmt::Display for ShellParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pattern(error) => error.fmt(f),
            Self::Boundary { offset } => write!(f, "offset={offset}"),
        }
    }
}
impl std::error::Error for ShellParseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Pattern(error) => Some(error),
            Self::Boundary { .. } => None,
        }
    }
}
impl From<regex::Error> for ShellParseError {
    fn from(error: regex::Error) -> Self {
        Self::Pattern(error)
    }
}

#[cfg(test)]
mod tests {
    use super::ShellParseError;
    use std::error::Error as _;

    fn pattern_error() -> regex::Error {
        let unclosed = String::from("(");
        regex::Regex::new(&unclosed).expect_err("閉じない括弧は不正なパターン")
    }

    #[test]
    fn a_boundary_error_names_its_offset_and_has_no_source() {
        let error = ShellParseError::Boundary { offset: 7 };
        assert_eq!(error.to_string(), "offset=7");
        assert!(error.source().is_none());
    }

    #[test]
    fn a_pattern_error_wraps_the_regex_error_as_its_source_and_message() {
        let inner = pattern_error();
        let expected = inner.to_string();
        let error = ShellParseError::from(inner);
        assert!(matches!(error, ShellParseError::Pattern(_)), "{error:?}");
        assert_eq!(error.to_string(), expected);
        assert!(
            error
                .source()
                .is_some_and(|source| source.to_string() == expected)
        );
    }
}
