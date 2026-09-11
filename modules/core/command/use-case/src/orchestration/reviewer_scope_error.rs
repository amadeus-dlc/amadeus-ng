//! `ReviewerScopeError` — 成立した拒否を残せなかったこと。
use super::reviewer_scope_cause::ReviewerScopeCause;
use core_command_domain::orchestration::ReviewerScopeBlock;

/// 拒否は成立したが、その事実を台帳へ残せなかった。
///
/// **拒否そのものは失敗していない。** 監査の失敗が判定を変えてはならない (upstream
/// `emitReviewerScopeBlocked` の「an audit failure never changes the block decision」) ので、
/// この値は**拒否の材料を運んだまま**呼出側へ戻る。呼出側は原因を drop へ残し、拒否は
/// そのまま実行する。判定の結末 (許可・拒否) は誤りではないのでここに無い。
#[derive(Debug)]
pub struct ReviewerScopeError {
    block: ReviewerScopeBlock,
    cause: ReviewerScopeCause,
}
impl ReviewerScopeError {
    /// 記録できなかった拒否と、その原因を束ねる (**この型の唯一の構築経路**)。
    #[must_use]
    pub const fn new(block: ReviewerScopeBlock, cause: ReviewerScopeCause) -> ReviewerScopeError {
        ReviewerScopeError { block, cause }
    }
    /// 記録できなかった拒否 (これは成立している)。
    #[must_use]
    pub const fn block(&self) -> &ReviewerScopeBlock {
        &self.block
    }
    /// 記録に失敗した理由。
    #[must_use]
    pub const fn cause(&self) -> &ReviewerScopeCause {
        &self.cause
    }
}
impl core::fmt::Display for ReviewerScopeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "the reviewer-scope refusal stands but was not recorded: {}",
            self.cause
        )
    }
}
impl std::error::Error for ReviewerScopeError {}
