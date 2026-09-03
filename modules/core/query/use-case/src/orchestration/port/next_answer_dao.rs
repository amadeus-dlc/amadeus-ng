//! `NextAnswerDao` ポート — `next` の答えを要求の形ごとに引く DAO。

use super::read_model_read_error::ReadModelReadError;
use super::read_view::NextAnswerView;

/// `next` 1 要求ぶんの引当 (**読取専用**)。
///
/// 更新の動詞は存在しない。リードモデルは RMU が投影で作るものであり、その不能をポート面に
/// 更新動詞を置かないことで型保証する (`coding-rules/cqrs-boundaries.md` 規則 6)。
///
/// 読取元は実装の内部詳細である — ポート面が語るのは DTO ([`NextAnswerView`]) だけで、
/// 媒体名も格納形式も現れない (`coding-rules/gateway-taxonomy.md` §3)。
///
/// 返るのは `read_next_answer` の**1 行の写し**である。実行の現在地も run-stage の材料も
/// 含まれない — 行が運ぶのは FK 列だけで、それをたどるのはユースケースの仕事である
/// (オーナー裁定 2026-09-03 — `coding-rules/cqrs-boundaries.md` 規則 6)。
pub trait NextAnswerDao {
    /// 実行 1 本 × 要求の形 1 つの答えを引く。
    ///
    /// `request_kind` は行のキーになる 4 値の綴り (`bare` / `resume` / `free-text` /
    /// `reentry`)。どの綴りで引くかは要求の形で決まるので**コントローラのルーティング**で
    /// あり、この口は渡された鍵で引くだけである (設計 §0-3)。
    ///
    /// **不在は失敗ではない** — まだ投影されていない実行は正常な観測なので `Ok(None)` で
    /// 返す。
    ///
    /// # Errors
    ///
    /// リードモデルを引けない ([`ReadModelReadError`])。
    fn find(
        &self,
        execution_id: &str,
        request_kind: &str,
    ) -> Result<Option<NextAnswerView>, ReadModelReadError>;
}
