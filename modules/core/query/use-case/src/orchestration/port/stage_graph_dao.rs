//! `StageGraphDao` ポート — コンパイル済みステージグラフを引く DAO。

use super::read_model_read_error::ReadModelReadError;
use super::read_view::StageGraphEntryView;

/// コンパイル済みステージグラフの引当 (**読取専用**)。
///
/// `stage-graph.json` は compile コンテキストのイベント投影であり、クエリ側はこれを**読む
/// だけ**である (`coding-rules/cqrs-boundaries.md` 規則 7)。媒体 (JSON ファイル) は実装の
/// 内部詳細でポート面には現れない (同規則 6 の DAO 項)。
pub trait StageGraphDao {
    /// slug で 1 ノードを引き、無ければ番号 (`0.3` 等) で引く (upstream `resolveStage`)。
    ///
    /// **不在は失敗ではない** — グラフに無い slug/番号は正常な観測なので `Ok(None)` で返す。
    ///
    /// # Errors
    ///
    /// リードモデルを引けない ([`ReadModelReadError`])。
    fn find(&self, slug_or_number: &str)
    -> Result<Option<StageGraphEntryView>, ReadModelReadError>;

    /// 有効な全ノードを**グラフ順**で引く (upstream `loadStageGraph` — `enabled: false` は除く)。
    ///
    /// # Errors
    ///
    /// リードモデルを引けない ([`ReadModelReadError`])。
    fn find_all(&self) -> Result<Vec<StageGraphEntryView>, ReadModelReadError>;
}
