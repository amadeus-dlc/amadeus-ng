//! 永続化モデル (DTO) — **ジャーナル行とスナップショット行のバイトを決めるのはここ**である。
//!
//! ドメインは永続化知識から中立であり、serde の記述もストアの trait 実装も持たない
//! (`coding-rules/domain-persistence-neutrality.md`)。直列化属性は「このフィールドはこの
//! 名前・この形でバイトになる」という**ストアとの契約**であって、ドメインの語彙ではない。
//!
//! # 往復の形
//!
//! - **書き**: ドメインの公開アクセサ → DTO → serde。
//! - **読み**: serde → DTO → ドメインの**検査付き再構成コンストラクタ**。
//!
//! Always Valid の担保は落ちない — 担保の場所がドメインの serde 属性からこの層の変換関数へ
//! 移るだけで、検査を迂回する構築口は存在しない (復元は必ず `Intent::from_material` /
//! `IntentExecution::from_snapshot` を通る)。
//!
//! # 綴りの正本はここにある
//!
//! 閉集合の綴りは [`wire_vocabulary`] が持つ。ドメイン側の `as_str` / `parse` を**流用しない** —
//! 同じ値でも面ごとに綴りが違うからである (例: `PhaseId` はジャーナル上 `"Ideation"` だが
//! `stage-graph.json` 上は `"ideation"`、`BrownfieldGreenfield` はどちらも `"greenfield"`)。
//! 流用すると片方の綴りを変えた瞬間にもう片方のバイトが壊れる。
//!
//! 読む側 (RMU) は**自前の**復号 DTO を持つ (`coding-rules/cqrs-boundaries.md` — 共有部品は
//! 側の独立を DRY に優先する)。書き手と読み手のワイヤ形式の一致は横断適合テストが固定する。

mod aggregate_key;
mod wire_error;
mod wire_event;
mod wire_intent;
mod wire_snapshot;
mod wire_vocabulary;

pub use aggregate_key::AggregateKey;
pub use wire_error::WireDecodeError;
pub use wire_event::{
    WireAutonomyModeSet, WireEvent, WireGateApproved, WireGateOpened, WireGateRejected, WireJumped,
    WireParked, WireRecomposed, WireStageCompleted, WireStageRevised, WireStageSkipped,
    WireStarted,
};
pub use wire_snapshot::WireSnapshot;

#[cfg(test)]
mod tests;
