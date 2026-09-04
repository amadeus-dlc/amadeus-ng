//! `ScopeDao` ポート — scope カタログ 1 列を引く DAO。

use super::read_model_read_error::ReadModelReadError;
use super::read_view::ScopeView;

/// scope カタログの引当 (**読取専用**)。
pub trait ScopeDao {
    /// compose 提案が目安に使う既製 scope の綴り (upstream の定数、この順で並べる)。
    ///
    /// 定数がポート面に在るのは、これが**要求パラメータの正本**だからである — 呼び手が
    /// 3 つの綴りを自分で書き下すと、upstream の定数が 2 か所に散る。
    const STOCK_SCOPES: [&'static str; 3] = ["express", "classic", "feature"];

    /// 定義 × scope 名で 1 列を引く。
    ///
    /// **行が返ること自体が「その scope は有効」の答え**である (有効な scope にしか行が
    /// 無い)。不在は失敗ではないので `Ok(None)` で返す。
    ///
    /// # Errors
    ///
    /// リードモデルを引けない ([`ReadModelReadError`])。
    fn find(
        &self,
        definition_id: &str,
        scope: &str,
    ) -> Result<Option<ScopeView>, ReadModelReadError>;

    /// 定義 1 本の scope 列を**綴り順**でまとめて引く。
    ///
    /// 有効な scope にしか行が無いので、これが「この定義で名乗れる scope の全体」である。
    /// 未知 scope の拒否文言が並べる一覧の材料であり、**呼び手は選ばない** — 引いた行を
    /// そのまま並べる。並びを綴り順に固定するのは、行に順序の列が無いためである
    /// (`read_definition_scope` は位置を持たない)。
    ///
    /// # Errors
    ///
    /// リードモデルを引けない ([`ReadModelReadError`])。
    fn find_all(&self, definition_id: &str) -> Result<Vec<ScopeView>, ReadModelReadError>;

    /// 既製 3 scope をまとめて引く ([`ScopeDao::STOCK_SCOPES`] の順、引けたものだけ)。
    ///
    /// [`ScopeDao::find`] を定数の並び順に 3 回呼ぶだけの引当である。どれが引けたかは
    /// [`ScopeView::scope`] が名乗るので、呼び手は 3 件揃ったかを数で見分けられる。
    /// 既定実装で与えるのは**鍵の並びを 1 か所に閉じる**ためであり、実装ごとに順序が
    /// 揺れる余地を作らない。
    ///
    /// # Errors
    ///
    /// リードモデルを引けない ([`ReadModelReadError`])。
    fn find_stock(&self, definition_id: &str) -> Result<Vec<ScopeView>, ReadModelReadError> {
        let mut found = Vec::new();
        for scope in Self::STOCK_SCOPES {
            if let Some(view) = self.find(definition_id, scope)? {
                found.push(view);
            }
        }
        Ok(found)
    }
}
