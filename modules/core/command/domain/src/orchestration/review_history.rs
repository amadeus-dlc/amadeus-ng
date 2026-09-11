//! 現試行のレビュー内容結合の記録列。
use super::{ReviewBinding, ReviewCompletion, ReviewRecord};
/// 個々のiterationが確定した内容を保持する一級コレクション。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewHistory {
    records: Vec<ReviewRecord>,
}
impl Default for ReviewHistory {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}
impl ReviewHistory {
    /// 保存された全記録で構築する。
    #[must_use]
    pub const fn new(records: Vec<ReviewRecord>) -> Self {
        Self { records }
    }
    /// 永続化境界へ記録順を渡す。
    pub fn iter(&self) -> impl Iterator<Item = &ReviewRecord> {
        self.records.iter()
    }
    /// 当該iterationの要求内容。
    #[must_use]
    pub fn request(&self, iteration: u32) -> Option<&ReviewBinding> {
        self.records.iter().rev().find_map(|record| match record {
            ReviewRecord::Requested {
                iteration: found,
                binding,
                ..
            } if *found == iteration => Some(binding),
            _ => None,
        })
    }
    /// この要求が再試行済みか。
    #[must_use]
    pub fn retried(&self, iteration: u32) -> bool {
        self.records.iter().any(|record|matches!(record,ReviewRecord::Requested { iteration: found,retry:true,.. } if *found==iteration))
    }
    /// 要求を記録する。
    pub(super) fn record_request(&mut self, iteration: u32, binding: ReviewBinding, retry: bool) {
        self.records.push(ReviewRecord::Requested {
            iteration,
            binding,
            retry,
        });
    }
    /// 完了を記録する。
    pub(super) fn record_completion(&mut self, iteration: u32, completion: ReviewCompletion) {
        self.records.push(ReviewRecord::Completed {
            iteration,
            completion,
        });
    }
}
