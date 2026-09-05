//! `AutonomySwitchRequest` — `SwitchAutonomyUseCase` の入力（正規化済み）。

use core_command_domain::orchestration::AutonomyMode;
use core_command_domain::workspace::HumanTurns;

/// `aidlc-bolt set-autonomy` 1 回分の入力。
///
/// 構文段（`--mode` の有無と閉集合）と外部の材料の読取（監査台帳から
/// [`HumanTurns::find_in`] で組む・env `AIDLC_SKIP_HUMAN_PRESENCE_GUARD` の判定）は合成ルートが
/// 済ませているので、ここに届くのは**値**だけである（`coding-rules/use-case-rules.md` —
/// 入力は型付きの値で受ける）。**判断（昇格を受理してよいか）はこの型にもユースケースにも
/// 無い** — 集約 `IntentExecution::switch_autonomy` のガードが持つ。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutonomySwitchRequest {
    mode: AutonomyMode,
    turns: HumanTurns,
    human_presence_guard: bool,
}

impl AutonomySwitchRequest {
    /// 切替先・台帳の証拠・ガードの有無を束ねる（**この型の唯一の構築経路**）。
    #[must_use]
    pub const fn new(
        mode: AutonomyMode,
        turns: HumanTurns,
        human_presence_guard: bool,
    ) -> AutonomySwitchRequest {
        AutonomySwitchRequest {
            mode,
            turns,
            human_presence_guard,
        }
    }

    /// 切替先のモード（`--mode` の 2 値）。
    #[must_use]
    pub const fn mode(&self) -> AutonomyMode {
        self.mode
    }

    /// 監査台帳から読み取った「人が居た」証拠。
    #[must_use]
    pub const fn turns(&self) -> &HumanTurns {
        &self.turns
    }

    /// human presence ガードが有効か（env で外れていれば偽 — I11）。
    #[must_use]
    pub const fn is_human_presence_guard(&self) -> bool {
        self.human_presence_guard
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_request_carries_the_three_materials_it_was_given() {
        let turns = HumanTurns::find_in(
            "\n## H\n**Timestamp**: 2026-08-23T00:00:01Z\n**Event**: HUMAN_TURN\n",
        );
        let request = AutonomySwitchRequest::new(AutonomyMode::Autonomous, turns.clone(), true);
        assert_eq!(request.mode(), AutonomyMode::Autonomous);
        assert_eq!(request.turns(), &turns);
        assert!(request.is_human_presence_guard());

        let disabled =
            AutonomySwitchRequest::new(AutonomyMode::Gated, HumanTurns::default(), false);
        assert_eq!(disabled.mode(), AutonomyMode::Gated);
        assert!(!disabled.is_human_presence_guard());
    }
}
