//! 保存済みの移動結果を取得する。
use super::{JumpResultDao, JumpResultView, ReadModelReadError};
#[derive(Debug)]
/// 移動結果の読取り境界。
pub struct JumpResultUseCase<D: JumpResultDao> {
    dao: D,
}
impl<D: JumpResultDao> JumpResultUseCase<D> {
    /// 読取ポートを注入する。
    #[must_use]
    pub const fn new(dao: D) -> Self {
        Self { dao }
    }
    /// イベントIDで結果を取得する。
    /// # Errors
    /// 読取り失敗を伝播する。
    pub fn execute(&self, id: &str) -> Result<Option<JumpResultView>, ReadModelReadError> {
        self.dao.find(id)
    }
}
