//! `StageKey` — イベント適用の添字帳 1 行 (slug + phase)。
//!
//! 集約 [`IntentExecution`](super::IntentExecution) が本家 `UserAccount::replay(events,
//! snapshot)` と同型の**自己完結 replay** を持つために要る最小の静的材料である
//! (オーナー裁定 2026-08-30「replay や apply_event が集約側に必要」)。イベントはステージを
//! `StageSlug` で名指すので、適用には slug→索引の解決が要り、不変条件 I7 (no_gate_bypass)
//! には is_gated (= phase から導出) が要る。この 2 つ**だけ**を複製する — [`Intent`] 全体は
//! 従来どおり ID 参照であり、表示属性・計画・条件フラグは複製しない
//! (`coding-rules/aggregate-references.md` の趣旨を最小侵襲で維持)。
//!
//! [`Intent`]: super::Intent

use crate::workflow_definition::{PhaseId, StageSlug};

/// 計画 1 ステージの適用用ヘッダ (slug + phase)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageKey {
    slug: StageSlug,
    phase: PhaseId,
}

impl StageKey {
    /// slug と phase を束ねる。
    #[must_use]
    pub const fn new(slug: StageSlug, phase: PhaseId) -> StageKey {
        StageKey { slug, phase }
    }

    /// ステージの slug (イベントのステージ参照の解決先)。
    #[must_use]
    pub const fn slug(&self) -> &StageSlug {
        &self.slug
    }

    /// ステージのフェーズ (ジャーナル面の綴りは wire 側が持つ)。
    #[must_use]
    pub const fn phase(&self) -> PhaseId {
        self.phase
    }

    /// 承認ゲート付きか (initialization フェーズだけが非ゲート — BR1.3 の静的既定)。
    #[must_use]
    pub fn is_gated(&self) -> bool {
        self.phase != PhaseId::Initialization
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(phase: PhaseId) -> StageKey {
        StageKey::new(StageSlug::parse("intent-capture").unwrap(), phase)
    }

    #[test]
    fn the_key_carries_slug_and_phase() {
        let key = key(PhaseId::Ideation);
        assert_eq!(key.slug().as_str(), "intent-capture");
        assert_eq!(key.phase(), PhaseId::Ideation);
    }

    #[test]
    fn only_initialization_is_ungated() {
        assert!(!key(PhaseId::Initialization).is_gated());
        assert!(key(PhaseId::Ideation).is_gated());
        assert!(key(PhaseId::Construction).is_gated());
    }
}
