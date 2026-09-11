//! 稼働観測リードモデルのID読取ポート。
use super::{HookHealthView, ReadModelReadError};
/// IDを条件に行を1件だけ読む。
pub trait HookHealthDao {
    /// 指定IDの投影行を読む。
    /// # Errors
    /// リードモデルを読めない場合。
    fn find(&self, id: &str) -> Result<Option<HookHealthView>, ReadModelReadError>;
}
