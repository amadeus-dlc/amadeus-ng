//! Pipeline参照投影の取得だけを行う。
use super::{PipelineProgressDao, PipelineProgressView, ReadModelReadError};
/// 状態の判断・更新を持たない取得UseCase。
#[derive(Debug)]
pub struct PipelineProgressUseCase<D: PipelineProgressDao> {
    dao: D,
}
impl<D: PipelineProgressDao> PipelineProgressUseCase<D> {
    /// 読取りポートを注入する。
    #[must_use]
    pub const fn new(dao: D) -> Self {
        Self { dao }
    }
    /// 指定したキーの行を返す。
    /// # Errors
    /// 読取りに失敗した場合。
    pub fn execute(
        &self,
        execution: &str,
        stage: &str,
        single: bool,
    ) -> Result<Option<PipelineProgressView>, ReadModelReadError> {
        self.dao.find(execution, stage, single)
    }
}
