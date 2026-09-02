//! `IntentRepository` ポート — 集約 `Intent` の Repository (改訂 10 で前倒し新設)。

use core_command_domain::orchestration::{Intent, IntentEvent, IntentId};

use super::repository_error::RepositoryError;

/// 集約 [`Intent`] の Repository (イベントソーシング形 — ADR-010 / issue #50)。
///
/// 署名は**自集約の ID だけ**を取る (`coding-rules/gateway-taxonomy.md`)。動詞は本家
/// ライブラリ (event-store-adapter-rs) の語彙に従い `store` / `find_by_id` である。
///
/// # intent 自身のジャーナルを持つ (issue #50)
///
/// intent は静的な集約 (変異コマンドが現状無い) だが、永続化は他の集約と同じ規律に従う —
/// 誕生イベント [`IntentEvent::Created`] をジャーナルへ書き、読取はイベントからの再構成
/// だけである (オーナー裁定 2026-08-30 — コマンド側の読取規律)。各実行のジャーナル先頭
/// (`Started`) に埋め込まれた intent はこの正本の写しであり、イベント痩身 (issue #56) で
/// `intent_id` 参照へ置き換わる。
///
/// # 発生時刻は集約から読む
///
/// 他のリポジトリと同じく `store` は (イベント, 集約) の 2 引数である (オーナー裁定
/// 2026-09-02 — 発生時刻を引数で運ぶ変則を撤去)。ジャーナル封筒の時刻は集約の
/// `created_at` (genesis の `occurred_at`) から組む — `IntentExecutionRepositoryImpl` が
/// `last_updated_at` から組むのと対である。
///
/// レシーバは CQS に従う (`coding-rules/command-query-separation.md`) — 読取は `&self`、
/// 永続化は `&mut self`。
#[allow(
    async_fn_in_trait,
    reason = "Send 境界を意図的に要求しない設計 (C3 / Q3 = A — tokio current_thread)。\
              `IntentExecutionRepository` と同じ方針である。"
)]
pub trait IntentRepository {
    /// intent を再構成して返す。
    ///
    /// 返るのは Always Valid な [`Intent`] である — 実装は復号したあと必ず検査付き再構成
    /// 経路 (誕生記録の変換 + [`Intent::replay`]) を通す
    /// (`coding-rules/domain-persistence-neutrality.md`)。
    ///
    /// # Errors
    ///
    /// intent が無い (`NotFound`)、ストア I/O (`Io`)、ストアの記録の破損 (`Corrupt` — 原因は
    /// `source` 連鎖) を返す。
    async fn find_by_id(&self, id: &IntentId) -> Result<Intent, RepositoryError<IntentId>>;

    /// イベントを 1 件と、適用後の集約を永続化する。
    ///
    /// 呼出側は [`Intent::create`] が返す (集約, 誕生イベント) の対をそのまま渡すだけで
    /// よい — いつスナップショットを書くかは実装の内部政策である (オーナー裁定 2026-08-30。
    /// 現状イベントは `Created` 1 種 = 必ず初回なので、実装は本家の作成規約どおり
    /// journal と snapshot を原子的に書く)。
    ///
    /// # Errors
    ///
    /// 同じ intent が既に存在する (`Conflict`)、ストア I/O (`Io`)、書込契約の違反
    /// (`Corrupt`) を返す。
    async fn store(
        &mut self,
        event: &IntentEvent,
        intent: &Intent,
    ) -> Result<(), RepositoryError<IntentId>>;
}
