//! 比較基準の構築拒否。
/// ソース一覧が公開構文を満たさない。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceBaselineError {
    /// 行形式・順序・エスケープが不正。
    MalformedListing,
}
impl std::fmt::Display for SourceBaselineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("malformed source listing")
    }
}
impl std::error::Error for SourceBaselineError {}
