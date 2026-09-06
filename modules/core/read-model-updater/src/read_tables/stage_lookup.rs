//! 実行の**添字帳**から slug を引く (公開型ゼロの内部モジュール)。
//!
//! 行の多くはステージ位置 (`StageIndex`) を答えとして受け取るが、読取側が 1 回の引当で
//! 答えを得るには slug も同じ行に要る (裁定 §10-1 の非正規化)。位置から slug への写像を
//! 持っているのは集約のスロット帳 ([`IntentExecution::stage_key`]) なので、引き方をここに
//! 1 つだけ置いて、行ごとに書き直さない。
//!
//! 生の添字アクセスをせず問い合わせで引くのは、`clippy::indexing_slicing` を避けるためだけ
//! ではない — 範囲外の位置は「答えが無い」であって panic ではないからである。
//!
//! [`IntentExecution::stage_key`]: core_command_domain::orchestration::IntentExecution::stage_key

use core_command_domain::orchestration::{IntentExecution, StageIndex, StageKey};

/// ステージ位置の slug。
pub(crate) fn slug_at(execution: &IntentExecution, stage: StageIndex) -> Option<String> {
    slug_of(execution, stage.to_usize())
}

/// 文書順の索引の slug。
pub(crate) fn slug_of(execution: &IntentExecution, index: usize) -> Option<String> {
    execution
        .stage_index(index)
        .and_then(|stage| execution.stage_key(stage))
        .map(|key| StageKey::slug(key).as_str().to_string())
}
