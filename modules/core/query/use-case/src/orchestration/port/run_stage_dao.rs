//! `RunStageDao` ポート — run-stage の材料 1 行を引く DAO。

use super::read_model_read_error::ReadModelReadError;
use super::read_view::RunStageView;

/// run-stage 材料の引当 (**読取専用**)。
///
/// 行は定義 × scope で決まり、実行には依らない (ゲートの上書き・`unit`・`single` の
/// ピンは要求と token が運ぶ)。したがって state を持たない要求 (`--single`・state なし
/// jump) からも同じ鍵で引ける。
///
/// 3 つの動詞はいずれも `read_run_stage` の**同じ 1 表**を引く — 違うのは鍵だけである
/// (自然キー / 代理キー / 自然キー + 束縛)。
pub trait RunStageDao {
    /// 定義 × scope × ステージ slug で run-stage の材料を引く。
    ///
    /// **不在は失敗ではない** — その scope でそのステージが実行されない (グリッドが SKIP)
    /// のは正常な観測なので `Ok(None)` で返す。
    ///
    /// # Errors
    ///
    /// リードモデルを引けない ([`ReadModelReadError`])。
    fn find(
        &self,
        definition_id: &str,
        scope: &str,
        stage_slug: &str,
    ) -> Result<Option<RunStageView>, ReadModelReadError>;

    /// 行の識別子で引く (`read_next_answer.run_stage_id` の FK をたどる口)。
    ///
    /// 答えが名指す材料はこの鍵で引く。`stage_slug` から自然キーで引き直すと、RMU が
    /// 「材料は無い」と書いた答え (park・不整合 2 形) にまで材料が付いてしまう —
    /// それは行に無い事実を作ることである。
    ///
    /// # Errors
    ///
    /// リードモデルを引けない ([`ReadModelReadError`])。
    fn find_by_id(&self, id: &str) -> Result<Option<RunStageView>, ReadModelReadError>;

    /// 自然キーに token の 2 束縛 (経路 / directive) を加えて引く (`continue` の照合)。
    ///
    /// 束縛は**鍵の一部**である。1 つでもずれた token は行に当たらず `Ok(None)` になるので、
    /// 「合っているか」を判定する経路はクエリ側に無い。自然キーを鍵に含めるのは、束縛
    /// 2 列だけでは索引が UNIQUE でなく 1 行に定まらないからである (in-scope 列が同じ
    /// 2 つの scope は同じ経路ダイジェストを持ちうる)。
    ///
    /// # Errors
    ///
    /// リードモデルを引けない ([`ReadModelReadError`])。
    fn find_bound(
        &self,
        definition_id: &str,
        scope: &str,
        stage_slug: &str,
        route_digest: &str,
        directive_digest: &str,
    ) -> Result<Option<RunStageView>, ReadModelReadError>;
}
