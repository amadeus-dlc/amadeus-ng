//! 開始結果を依頼IDで引く読取りポート。
use super::{InitializationView, ReadModelReadError};
/// 作業開始の保存済み結果だけを読む。
pub trait InitializationDao {
    /// 指定した依頼の開始結果。不在はNone。
    /// # Errors
    /// リードモデルを取得できない場合。
    fn find(&self, intent_id: &str) -> Result<Option<InitializationView>, ReadModelReadError>;
}
