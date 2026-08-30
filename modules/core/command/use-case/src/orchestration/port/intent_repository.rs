//! `IntentRepository` ポート — 集約 `Intent` の Repository (改訂 10 で前倒し新設)。

use core_command_domain::orchestration::{Intent, IntentId};

use super::repository_error::RepositoryError;

/// 集約 [`Intent`] の Repository。
///
/// 署名は**自集約の ID だけ**を取る (`coding-rules/gateway-taxonomy.md`)。動詞は本家
/// ライブラリの語彙に合わせて `find_by_id` である。
///
/// # なぜ書込 (`store`) が無いのか
///
/// 当面は読取だけで足りるからである。intent を**作る**のは upstream の `intent-create` で
/// あり、その実装は U7 の課題なので、`store(&IntentEvent, &Intent, ..)` はそのときに足す
/// (additive-safe)。今この口を先に開けておく理由が無い
/// (`coding-rules/no-backward-compatibility.md` — 使われない口を並立させない)。
///
/// # 実物の実装はまだ無い (U7 への申し送り)
///
/// 現時点で intent の完全な材料が永続化されているのは**各実行のジャーナル先頭の
/// `Started`** だけである。intent 自身のジャーナルをどう持つか (`IntentEvent::Created` の
/// 書き先) の設計ごと U7 の課題であり、B12 が用意するのは**ポートと結線の形**までである。
/// アダプタ側にあるのは結線テスト用のインメモリ実装 1 つだけである。
///
/// レシーバは CQS に従う (`coding-rules/command-query-separation.md`) — 読取なので `&self`。
#[allow(
    async_fn_in_trait,
    reason = "Send 境界を意図的に要求しない設計 (C3 / Q3 = A — tokio current_thread)。\
              `IntentExecutionRepository` と同じ方針である。"
)]
pub trait IntentRepository {
    /// intent を再構成して返す。
    ///
    /// 返るのは Always Valid な [`Intent`] である — 実装は復号したあと必ず検査付き再構成
    /// コンストラクタを通す (`coding-rules/domain-persistence-neutrality.md`)。
    ///
    /// # Errors
    ///
    /// intent が無い (`NotFound`)、ストア I/O (`Io`)、ストアの記録の破損 (`Corrupt` — 原因は
    /// `source` 連鎖) を返す。
    async fn find_by_id(&self, id: &IntentId) -> Result<Intent, RepositoryError<IntentId>>;
}
