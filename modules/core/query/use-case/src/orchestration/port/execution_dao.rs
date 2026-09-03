//! `ExecutionDao` ポート — 実行 1 本の現在地を引く DAO。

use super::read_model_read_error::ReadModelReadError;
use super::read_view::ExecutionView;

/// 実行の現在地の引当 (**読取専用**)。
///
/// どちらの動詞も `read_execution` の**同じ 1 表**を引く — 違うのは鍵だけである。
pub trait ExecutionDao {
    /// 実行識別子で現在地を引く。
    ///
    /// **不在は失敗ではない** — その識別子の実行がまだ投影されていない (あるいは実行が
    /// 存在しない) のは正常な観測なので `Ok(None)` で返す。読取コマンドはそれを「state
    /// なし」の群として扱う。
    ///
    /// # Errors
    ///
    /// リードモデルを引けない ([`ReadModelReadError`])。
    fn find(&self, execution_id: &str) -> Result<Option<ExecutionView>, ReadModelReadError>;

    /// 状態の束縛ダイジェストで現在地を引く (`continue` の state 照合)。
    ///
    /// 束縛は実行ごとに違う値になるので、当たる行は高々 1 行である。**不在は失敗ではない**
    /// — 保存位置が動いていれば当たらず `Ok(None)` になる (fail-closed の材料)。
    ///
    /// # Errors
    ///
    /// リードモデルを引けない ([`ReadModelReadError`])。
    fn find_by_state_binding(
        &self,
        state_binding: &str,
    ) -> Result<Option<ExecutionView>, ReadModelReadError>;
}
