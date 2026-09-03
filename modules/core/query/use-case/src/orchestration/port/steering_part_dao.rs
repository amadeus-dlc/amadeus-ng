//! `SteeringPartDao` ポート — 配信計画の 1 部を引く DAO。

use super::read_model_read_error::ReadModelReadError;
use super::read_view::SteeringPartView;

/// 配信の 1 部の引当 (**読取専用**)。
pub trait SteeringPartDao {
    /// 最初に届ける部の番号 (upstream の数え方は 1 始まり)。
    ///
    /// 定数がポート面に在るのは、これが**要求パラメータの正本**だからである — `next` が
    /// 1 部目から配るというのは要求の形の話であって、DAO の SQL に焼くと「行に無い事実」を
    /// 引当条件に埋めることになる ([`super::ScopeDao::STOCK_SCOPES`] と同じ置き方)。
    const FIRST_PART: u32 = 1;

    /// 計画の識別子と部番号で 1 部を引く。
    ///
    /// **不在は失敗ではない** — その番号の行が無いのは「もう配る部が無い」(終端) であり、
    /// 行の有無がそのまま答えである。
    ///
    /// # Errors
    ///
    /// リードモデルを引けない ([`ReadModelReadError`])。
    fn find(
        &self,
        steering_plan_id: &str,
        part_index: u32,
    ) -> Result<Option<SteeringPartView>, ReadModelReadError>;
}
