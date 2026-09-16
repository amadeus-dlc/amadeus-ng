//! `DocumentInputDao` ポート — 直接入力の 2 面 (転送ファイルと、名指された 1 ファイル) を
//! 生バイトで引く DAO。
//!
//! この面は `StateFileDao` / `ProjectDescriptionDao` と同じ「人間が書く・git で交換する」
//! リードモデルの一部であり、構造化リードモデル (`read_*` 表) ではない。契約がバイト逐語
//! (UTF-8 の妥当性・種別・文字数上限の判定材料) なので、ポートが返すのも**生バイト**で
//! あって列の写しではない。
//!
//! # 媒体はポート契約に漏らさない
//!
//! 実装はプロジェクトルートからパスを解決してファイルを開くが、その事実はここに現れない
//! (`port/mod.rs` の「媒体はポート契約に漏らさない」)。ポート面が語るのは、顧客が書いた
//! **要求パスの綴り**と、封じ込めに成功したときの**可搬パス**だけである。

use super::document_input_bytes::DocumentInputBytes;
use super::document_input_read_error::DocumentInputReadError;

/// 直接入力の 2 面を引く。
pub trait DocumentInputDao {
    /// 活動記録直下の転送ファイルの生バイトを引く。
    ///
    /// **不在も失敗である** — 直接入力は「1 つの正確なパスがそこに書かれている」ことを
    /// 前提にした面なので、書かれていなければ読む対象が決まらない。`Ok(None)` を持たない
    /// のはそのためである。
    ///
    /// # Errors
    ///
    /// 引けない ([`DocumentInputReadError::Unreadable`])。パス長の上限を超える転送ファイルは、
    /// 1 行検査の前に上限超過として拒む。
    fn find_request(&self) -> Result<Vec<u8>, DocumentInputReadError>;

    /// 顧客が名指した 1 ファイルを、プロジェクトルートへ封じ込めてから引く。
    ///
    /// # Errors
    ///
    /// 解決先が外にある ([`DocumentInputReadError::OutsideProject`])、または直接読めない
    /// ([`DocumentInputReadError::Unreadable`])。
    fn find_document(&self, requested: &str) -> Result<DocumentInputBytes, DocumentInputReadError>;
}
