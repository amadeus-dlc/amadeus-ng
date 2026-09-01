//! `CheckboxUpdateError` — `Checkboxes` の行編集 (marker / suffix writer) の拒否理由。

/// marker writer (`Checkboxes::with_marker`) の拒否理由。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckboxUpdateError {
    /// 対象 slug の行が存在しない。
    MissingStage(String),
    /// 対象行の末尾が EXECUTE / SKIP のどちらでもない (書き換え先が無い)。
    MissingSuffix(String),
}
