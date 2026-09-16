//! `IntentListingDao` ポート — 空間の依頼一覧 (登録簿の行 ∪ 記録ディレクトリ) を引く DAO。
//!
//! この面は `IntentRecordDao` と**同じ登録簿を別の問いで**読む。あちらは 1 つの uuid から
//! 記録ディレクトリを引く鍵引きで、こちらは空間の全行を並べる一覧である。1 表 1 ポートの
//! 規則は「1 つの問いに 1 つの口」を言うものなので、鍵引きと一覧は別のポートになる
//! (`coding-rules/cqrs-boundaries.md` 規則 6)。
//!
//! # 媒体はポート契約に漏らさない
//!
//! 実装は JSON の登録簿とディレクトリの両方を見るが、その事実はここに現れない。ポート面が
//! 語るのは、行が持つ 5 つの値だけである。

use super::intent_listing_row_view::IntentListingRowView;
use super::read_model_read_error::ReadModelReadError;

/// 空間の依頼一覧を引く。
pub trait IntentListingDao {
    /// 登録簿の行と、登録簿に無い記録ディレクトリを 1 つの並びで引く。
    ///
    /// **登録簿が無い・壊れているのは失敗ではない** — 記録ディレクトリだけを並べた一覧が
    /// 正しい観測である (upstream `readIntentRegistry` の `catch` → `[]`)。
    ///
    /// # Errors
    ///
    /// 記録ディレクトリを並べられない ([`ReadModelReadError`])。
    fn find(&self) -> Result<Vec<IntentListingRowView>, ReadModelReadError>;
}
