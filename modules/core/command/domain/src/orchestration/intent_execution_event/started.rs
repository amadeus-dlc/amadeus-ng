//! `Started` — `IntentExecutionEvent::Started` のペイロード。

use crate::orchestration::IntentId;

/// `Started` のペイロード — 起きた事実 (どの intent の実行が始まったか) だけを運ぶ。
///
/// **イベントはそのイベントを説明するプロパティだけに絞る** (オーナー裁定 2026-08-30)。
/// かつて丸ごと運んでいた `Intent` の複製 (解決済み計画・表示属性・走査結果・依頼文) は
/// 撤去した — それらは実行開始という事実の説明ではなく **intent 自身の誕生の記録**であり、
/// 正本は intent 自身のジャーナルの `Created` にある (issue #50 / #56)。RMU が状態ファイル
/// の骨格を描く材料もそこから取る。集約参照は ID で行う
/// (coding-rules/aggregate-references.md)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Started {
    intent_id: IntentId,
}

impl Started {
    /// 開始された intent の識別子を束ねる。
    #[must_use]
    pub const fn new(intent_id: IntentId) -> Started {
        Started { intent_id }
    }

    /// 開始された intent の識別子。
    #[must_use]
    pub const fn intent_id(&self) -> &IntentId {
        &self.intent_id
    }
}
