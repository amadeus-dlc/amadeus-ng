//! 保存済みテスト契約の引当ポート。
use super::TestingContractView;
use crate::orchestration::ReadModelReadError;
/// 依頼IDに対応する1行を取得する。
pub trait TestingContractDao {
    /// 指定した依頼の契約を読む。
    /// # Errors
    /// 読取り媒体から取得できない場合。
    fn find(&self, intent_id: &str) -> Result<Option<TestingContractView>, ReadModelReadError>;
}
