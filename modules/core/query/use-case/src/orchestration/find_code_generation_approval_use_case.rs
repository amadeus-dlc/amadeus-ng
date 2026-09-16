//! 投影済みの開始可否を指定対象で読む。
use super::{CodeGenerationApprovalDao, CodeGenerationApprovalView, ReadModelReadError};

/// 文書も規則も受領も解釈し直さず、DAO の行を返す。
#[derive(Debug)]
pub struct FindCodeGenerationApprovalUseCase<D: CodeGenerationApprovalDao> {
    dao: D,
}

impl<D: CodeGenerationApprovalDao> FindCodeGenerationApprovalUseCase<D> {
    /// 読取りポートを注入する。
    #[must_use]
    pub const fn new(dao: D) -> Self {
        Self { dao }
    }

    /// 対象の行を取得する。
    /// # Errors
    /// DAO の読取りが失敗した場合。
    pub fn execute(
        &self,
        execution_id: &str,
        target_id: &str,
    ) -> Result<Option<CodeGenerationApprovalView>, ReadModelReadError> {
        self.dao.find(execution_id, target_id)
    }
}
