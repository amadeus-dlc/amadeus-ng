//! 現試行で記録したレビューの内容結合。
use super::{ReviewBinding, ReviewCompletion};
/// 要求と完了の記録。試行をまたいで再利用しない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReviewRecord {
    /// 要求した原文。retryも同じ結合を持つ。
    Requested {
        /// 対象iteration。
        iteration: u32,
        /// 要求内容。
        binding: ReviewBinding,
        /// 未完了要求の再試行。
        retry: bool,
    },
    /// 完了時の原文。
    Completed {
        /// 対象iteration。
        iteration: u32,
        /// 検証された内容結合。
        completion: ReviewCompletion,
    },
}
