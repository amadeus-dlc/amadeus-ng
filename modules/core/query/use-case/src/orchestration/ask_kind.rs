//! `AskKind` — 構造化質問の種別。
//!
//! 種別が conductor の**応答契約**を選ぶ (どの答えを期待するか)。閉集合なので enum で運ぶ。

/// 構造化質問の種別 — conductor の応答契約を選ぶ。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AskKind {
    /// state ありでの `--resume` (分岐 6) — 再開メニュー。
    ResumeMenu,
    /// 稼働中の自由記述 (分岐 9c) — `new-work-routing`。回答は `next` 経由で、stage report に
    /// 記録してはならない (§4.5)。
    NewWorkRouting,
    /// state なし・キーワードヒットの scope 確認 (分岐 8)。
    ScopeConfirm,
    /// state なし・キーワード非ヒットの compose 提案 (分岐 8)。
    ComposeOffer,
    /// fresh clone の intent 選択 (分岐 7b — records はあるが active-intent カーソルなし)。
    IntentPick,
}
