//! `JumpDao` ポート — ジャンプ先ごとの受理判定を引く DAO。

use super::read_model_read_error::ReadModelReadError;
use super::read_view::JumpView;

/// ジャンプの引当 (**読取専用**)。
///
/// 行は全 target を網羅する非正規化であり、**拒否も 1 つの答え**として行になっている。
/// 「跳べるか」を計算する経路はクエリ側に無い。
pub trait JumpDao {
    /// 実行 × ジャンプ先 slug で受理判定を引く。
    ///
    /// **不在は失敗ではない** — 計画に無い slug は正常な観測なので `Ok(None)` で返す。
    ///
    /// # Errors
    ///
    /// リードモデルを引けない ([`ReadModelReadError`])。
    fn find(
        &self,
        execution_id: &str,
        target_slug: &str,
    ) -> Result<Option<JumpView>, ReadModelReadError>;

    /// 実行 × ジャンプ先の**位置**で受理判定を引く。
    ///
    /// 鍵はフェーズ表 ([`super::JumpPhaseView::target_index`]) がたどらせる目的地の位置で
    /// ある。2 つの動詞は同じ 1 表を別の鍵で引くだけで、結合はしない。
    ///
    /// **不在は失敗ではない** — 計画に無い位置は `Ok(None)` で返す。
    ///
    /// # Errors
    ///
    /// リードモデルを引けない ([`ReadModelReadError`])。
    fn find_by_target(
        &self,
        execution_id: &str,
        target_index: u32,
    ) -> Result<Option<JumpView>, ReadModelReadError>;
}
