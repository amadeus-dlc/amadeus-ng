//! `ProjectDescriptionDao` ポート — 依頼原文のサイドカーを生テキストで引く DAO。

use super::read_model_read_error::ReadModelReadError;

/// record 直下の依頼原文サイドカー (`project-description.json`) の**生テキスト**を引く。
///
/// この面は `StateFileDao` と同じ「upstream 互換・人間可読・git 交換用」のリードモデルの
/// 一部であり、構造化リードモデル (`read_*` 表) ではない。契約はバイト逐語そのもの
/// (JSON 文字列 1 個) なので、ポートが返すのも**生テキスト**であって列の写しではない。
/// 中身が JSON 文字列として妥当かを判定するのは読んだ側 (ユースケース) である。
///
/// # 媒体はポート契約に漏らさない
///
/// 実装はファイルを読むが、その事実はここに現れない (`port/mod.rs` の「媒体はポート契約に
/// 漏らさない」)。鍵を取らないのは、この面が **record ごとに 1 つ**しか無いからである —
/// どの record を見るかは実装が握る (合成ルートが結線する)。
pub trait ProjectDescriptionDao {
    /// サイドカーの生テキストを引く。
    ///
    /// **不在は失敗ではない** — サイドカーを持たない legacy record は正常な観測なので
    /// `Ok(None)` で返す。状態ファイルが `Project Description Source` で正本を名指して
    /// いるのに不在、という**取り合わせ**が拒否になるかどうかは読んだ側が決める。
    ///
    /// # Errors
    ///
    /// リードモデルを引けない ([`ReadModelReadError`])。通常ファイル以外 (シンボリックリンク・
    /// FIFO・ディレクトリ) は引けない対象として扱う。
    fn find(&self) -> Result<Option<String>, ReadModelReadError>;
}
