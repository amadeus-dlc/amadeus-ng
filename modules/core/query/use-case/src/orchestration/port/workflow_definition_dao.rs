//! `WorkflowDefinitionDao` ポート — ワークフロー定義リードモデルを読む DAO。

use crate::workflow_view::DefinitionView;

use super::workflow_definition_read_error::WorkflowDefinitionReadError;

/// ワークフロー定義リードモデルを読む DAO (**読取専用**)。
///
/// **更新の動詞は存在しない。** ワークフロー定義は compile コンテキストのイベントから RMU が
/// 作成・更新するリードモデルであり、クエリ側はこれを読むだけである — その不能をポート面に
/// 更新動詞を置かないことで型保証する (オーナー裁定 2026-08-31、
/// `coding-rules/cqrs-boundaries.md` 規則 6 / 規則 7)。
///
/// **読取元 (ファイル / SQLite のテーブル) は実装の内部詳細である** — ポート面が語るのは
/// DTO ([`DefinitionView`]) だけで、媒体名も格納形式もポート名にもシグネチャにも現れない
/// (オーナー追補裁定 2026-08-31。`coding-rules/gateway-taxonomy.md` §2 の媒体名禁止と
/// 同じ理屈)。**例外はエラーの材料だけ**である: `InvalidJson` と `ScopeFile` は upstream の
/// 逐語文言 (12 §4 #1 / #2 — 「Stage graph not readable at {path}」「... is not valid JSON」)
/// がファイルと JSON を名指すので、その文言を組む材料としてだけ形式語が残る。観測互換は
/// 設計規則より上位であり (`coding-rules/README.md` の衝突優先順 1)、これは媒体の選択が
/// 契約に漏れているのではなく**契約そのものが媒体を名指している**ケースである。
///
/// コマンド側の `WorkflowDefinitionRepository` (集約 `WorkflowDefinition` を `find_by_id` /
/// `store` する) とは**別物**である。同じ Published Language を読むが、写す先も変更理由も
/// 独立している (同規則「共有部品は側の独立を DRY に優先」)。
pub trait WorkflowDefinitionDao {
    /// ワークフロー定義とその scope カタログを読み、クエリモデルへ写して返す。
    ///
    /// 引数で定義を名指ししないのは、**どの定義を読むかが DAO の構築時に決まる**ためである
    /// (1 つのハーネスが提供できる定義は 1 つだけ — BR2.6 / ADR-008)。名指しできなかった
    /// 呼び手は [`WorkflowDefinitionReadError::Unidentified`] を返す DAO を注入する。
    ///
    /// 呼出のたびに読み直す。キャッシュ戦略は**観測不能なので実装の自由** (12 §10) —
    /// upstream のモジュールレベル可変シングルトンと `_reset*ForTests()` は模倣しない。
    ///
    /// # Errors
    ///
    /// 読取対象を名指しできない (`Unidentified`)、ハーネス identity の読取・検証失敗
    /// (`HarnessIdentity`)、グラフの読取失敗 (`NotReadable`)、不正 JSON (`InvalidJson`)、
    /// scope identity の列挙・読取・検証失敗 (`ScopeFile`)、ビュー型への写像失敗
    /// (`Malformed`)。**グリッドの欠損・不正はエラーにしない** — 転置導出へフォールバック
    /// する (12 §4 #3)。
    fn find(&self) -> Result<DefinitionView, WorkflowDefinitionReadError>;
}
