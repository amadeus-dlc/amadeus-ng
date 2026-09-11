//! 実行と対象で計画指紋を読むポート。
use super::PlanFingerprintView;
use crate::orchestration::ReadModelReadError;
/// 指定した自然キーに対応する1行だけを取得する。
pub trait PlanFingerprintDao {
    /// 計画指紋を読む。
    /// # Errors
    /// 読取りモデルを取得できない場合。
    fn find(
        &self,
        execution_id: &str,
        target_id: &str,
    ) -> Result<Option<PlanFingerprintView>, ReadModelReadError>;
}
