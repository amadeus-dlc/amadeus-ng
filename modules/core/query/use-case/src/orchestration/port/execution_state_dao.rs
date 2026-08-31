//! `ExecutionStateDao` ポート — 実行状態リードモデルを読む DAO。

use crate::orchestration::ExecutionStateView;

use super::execution_state_read_error::ExecutionStateReadError;

/// 実行状態リードモデルを読む DAO (**読取専用**)。
///
/// **更新の動詞は存在しない。** リードモデルは RMU が投影で作成・更新するものであり、
/// クエリ側から書き換えることはできない — その不能をポート面に更新動詞を置かないことで
/// 型保証する (オーナー裁定 2026-08-31、`coding-rules/cqrs-boundaries.md` 規則 6)。
///
/// **読取元 (ファイル / SQLite のテーブル) は実装の内部詳細である** — ポート面が語るのは
/// DTO ([`ExecutionStateView`]) だけで、媒体名も格納形式もポート名にもシグネチャにも
/// 現れない (オーナー追補裁定 2026-08-31。`coding-rules/gateway-taxonomy.md` §2 の
/// 媒体名禁止と同じ理屈)。
pub trait ExecutionStateDao {
    /// 現在の実行状態リードモデルを読み、クエリモデルへ写して返す。
    ///
    /// **不在は失敗ではない** — active-intent がまだ無いワークフローは正常な観測なので
    /// `Ok(None)` で返し、ユースケースは誕生分岐の群へ落とす。「無い」と「読めない」で
    /// 行き先が違うのは観測可能な契約であり、`Option` に潰してはならない。
    ///
    /// 呼出のたびに読み直す (キャッシュ戦略は観測不能なので実装の自由 — 12 §10)。
    ///
    /// # Errors
    ///
    /// 読取対象は在るのに読めない (`NotReadable`)、読めたが復号できない (`Malformed`)。
    fn find(&self) -> Result<Option<ExecutionStateView>, ExecutionStateReadError>;
}
