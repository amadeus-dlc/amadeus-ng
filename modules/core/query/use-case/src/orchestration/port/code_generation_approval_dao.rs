//! 実行と対象で開始可否を読むポート。
use super::CodeGenerationApprovalView;
use crate::orchestration::ReadModelReadError;

/// 指定した自然キーに対応する 1 行だけを取得する。
pub trait CodeGenerationApprovalDao {
    /// 開始可否を読む。
    /// # Errors
    /// 読取りモデルを取得できない場合。
    fn find(
        &self,
        execution_id: &str,
        target_id: &str,
    ) -> Result<Option<CodeGenerationApprovalView>, ReadModelReadError>;
}
