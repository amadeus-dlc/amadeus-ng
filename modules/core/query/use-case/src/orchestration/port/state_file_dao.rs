//! `StateFileDao` ポート — record の状態ファイル 1 面を生テキストで引く DAO。

use super::read_model_read_error::ReadModelReadError;

/// upstream 互換の人間可読リードモデル (`aidlc-state.md`) の**生テキスト**を引く。
///
/// この面は RMU が投影した 2 つのリードモデルのうち「upstream 互換・人間可読・git 交換用」の
/// ほうである。構造化リードモデル (`read_*` 表) と違い、契約はバイト逐語そのものなので、
/// ポートが返すのも**生テキスト**であって列の写しではない。読んだ側 (合成ルート) が
/// `State Version` を分類する (段 1 の state-version guard)。
///
/// # 媒体はポート契約に漏らさない
///
/// 実装はファイルを読むが、その事実はここに現れない (`port/mod.rs` の「媒体はポート契約に
/// 漏らさない」)。鍵を取らないのは、この面が **record ごとに 1 つ**しか無いからである —
/// どの record を見るかは実装が握る (合成ルートが結線する)。
pub trait StateFileDao {
    /// 状態ファイルの生テキストを引く。
    ///
    /// **不在は失敗ではない** — record がまだ無い / 状態ファイルがまだ書かれていないのは
    /// 正常な観測なので `Ok(None)` で返す。**0 バイトのファイルは「在る」**であり
    /// `Ok(Some(""))` になる (upstream `loadStateFileIfPresent` の `!== null` 判定と同じ —
    /// 空の状態ファイルは不在ではなく「版が読めない状態ファイル」である)。
    ///
    /// # Errors
    ///
    /// リードモデルを引けない ([`ReadModelReadError`])。
    fn find(&self) -> Result<Option<String>, ReadModelReadError>;
}
