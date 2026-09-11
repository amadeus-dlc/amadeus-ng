//! 実行別pipelineの参照投影を引くポート。
use super::{PipelineProgressView, ReadModelReadError};
/// キーで引くだけの読取り境界。
pub trait PipelineProgressDao {
    /// 実行・stage・単独実行のキーで1行を読む。
    /// # Errors
    /// リードモデルを読めない場合。
    fn find(
        &self,
        execution: &str,
        stage: &str,
        single: bool,
    ) -> Result<Option<PipelineProgressView>, ReadModelReadError>;
}
