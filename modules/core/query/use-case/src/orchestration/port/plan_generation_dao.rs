//! 実装開始の結果行を読むポート。
use super::{PlanGenerationView, ReadModelReadError};
/// 指定した操作IDの結果だけを読む。
pub trait PlanGenerationDao {
    /// 投影済み結果の検索。
    /// # Errors
    /// リードモデルを読めない場合。
    fn find(&self, id: &str) -> Result<Option<PlanGenerationView>, ReadModelReadError>;
}
