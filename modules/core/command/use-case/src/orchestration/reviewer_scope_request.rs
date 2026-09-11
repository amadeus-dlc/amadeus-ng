//! `ReviewerScopeRequest` — 読み取り範囲の判定 1 回分の入力。
use core_command_domain::orchestration::{
    InspectedTool, ReviewerDispatch, ReviewerScopeCandidates,
};

/// 進行中のレビューと、いま判定にかける呼出しを 1 つに束ねた入力。
///
/// 差し向け記録の読取りと経路の基点の決定はハーネス側の責務なので、ここへは**正規化済みの
/// 値**として届く (`coding-rules/use-case-rules.md`)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewerScopeRequest {
    dispatch: ReviewerDispatch,
    record_root: Option<String>,
    cwd: Option<String>,
    tool: InspectedTool,
    candidates: ReviewerScopeCandidates,
}
impl ReviewerScopeRequest {
    /// 判定 1 回分の材料を束ねる (**この型の唯一の構築経路**)。
    ///
    /// `record_root` は差し向け記録が置かれたディレクトリ、`cwd` はハーネスが渡す作業
    /// ディレクトリである。どちらも無ければ字句だけの照合になる。
    #[must_use]
    pub const fn new(
        dispatch: ReviewerDispatch,
        record_root: Option<String>,
        cwd: Option<String>,
        tool: InspectedTool,
        candidates: ReviewerScopeCandidates,
    ) -> ReviewerScopeRequest {
        ReviewerScopeRequest {
            dispatch,
            record_root,
            cwd,
            tool,
            candidates,
        }
    }
    /// 進行中のレビュー。
    #[must_use]
    pub const fn dispatch(&self) -> &ReviewerDispatch {
        &self.dispatch
    }
    /// 差し向け記録が置かれたディレクトリ。
    #[must_use]
    pub fn record_root(&self) -> Option<&str> {
        self.record_root.as_deref()
    }
    /// ハーネスが渡した作業ディレクトリ。
    #[must_use]
    pub fn cwd(&self) -> Option<&str> {
        self.cwd.as_deref()
    }
    /// 判定にかける工具。
    #[must_use]
    pub const fn tool(&self) -> InspectedTool {
        self.tool
    }
    /// 判定にかける候補 (掲載順)。
    #[must_use]
    pub const fn candidates(&self) -> &ReviewerScopeCandidates {
        &self.candidates
    }
}
