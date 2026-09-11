//! 回答識別子による結果の読取りポート。
use super::{AnswerResultView, ReadModelReadError};
/// 回答時点の結果を読み、現在状態から作り直さない。
pub trait AnswerResultDao {
    /// 指定した回答結果。不在はNone。
    /// # Errors
    /// リードモデルが読めない場合。
    fn find(&self, answer_id: &str) -> Result<Option<AnswerResultView>, ReadModelReadError>;
}
