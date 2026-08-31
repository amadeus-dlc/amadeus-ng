//! `MemoryRulesDao` ポート — active-space の memory 層 (決定論的 steering) を読む DAO。

use super::memory_rules_read_error::MemoryRulesReadError;
use crate::orchestration::MemoryRules;

/// memory 層ルール束を読む DAO (**読取専用**)。
///
/// **更新の動詞は存在しない。** memory 層のルールはワークフローの入力であってクエリ側の
/// 書込対象ではない — その不能をポート面に更新動詞を置かないことで型保証する
/// (オーナー裁定 2026-08-31、`coding-rules/cqrs-boundaries.md` 規則 6)。
///
/// **読取元 (ファイル / SQLite のテーブル) は実装の内部詳細である** — ポート面が語るのは
/// DTO ([`MemoryRules`]) だけで、媒体名も格納形式もポート名にもシグネチャにも現れない
/// (オーナー追補裁定 2026-08-31。`coding-rules/gateway-taxonomy.md` §2 の媒体名禁止と
/// 同じ理屈)。
pub trait MemoryRulesDao {
    /// memory 層を解決順 (`org → team → project → phases/<phase>`、strict-additive) で読み、
    /// ルール束を返す。層の順序は本ポートの約束だが、**層が何に格納されているかは約束しない**。
    ///
    /// **不在は失敗ではない** — ルール未整備は正常なので、無い層は単に束の列に現れない
    /// (空束も正常で、bare run-stage になる)。フェーズの選択と配信計画への分割・パックは
    /// 読取ではなく純計算なので、[`MemoryRules::plan_for`] が行う。
    ///
    /// # Errors
    ///
    /// 読取対象が在るのに読めない (権限・UTF-8 破損など)。呼出側は blocking で `error`
    /// directive を出す (02 §10)。
    fn find(&self) -> Result<MemoryRules, MemoryRulesReadError>;
}
