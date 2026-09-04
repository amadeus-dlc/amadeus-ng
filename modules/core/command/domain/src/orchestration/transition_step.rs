//! `TransitionStep` — [`ReportDecision`] が名指しする遷移サブコマンド 1 段。
//!
//! [`ReportDecision`]: super::ReportDecision

/// upstream `handleReport` が `sequence` に積む遷移サブコマンド 1 段。
///
/// 綴り ([`TransitionStep::subcommand`]) は upstream の Published Language であり、成功の
/// 逐語 `Committed <subs joined by " + "> for "<slug>"` の材料になる (ピン `:5921-5926`)。
///
/// # 段とイベントは 1:1 ではない
///
/// 状態遷移を起こすのは列の**最後の段**である。先行する [`TransitionStep::GateStartRecovered`]
/// は監査の見え方 (`STAGE_AWAITING_APPROVAL` の `Recovered` 行) を決めるだけで、遷移そのものは
/// 続く [`TransitionStep::Approve`] の 1 イベント (BR1.3 — `[-]` からの承認) が担う。
/// upstream は `gate-start --recovered` と `approve` の 2 プロセスに分かれるが、こちらは
/// 「1 コマンド 1 イベント」(`coding-rules/aggregate-commands.md`) を崩さない。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TransitionStep {
    /// `gate-start <slug> --recovered` — 明示 `--stage` 復旧で開き直すゲート (監査だけの段)。
    GateStartRecovered,
    /// `gate-start <slug>` — ゲートの開放。
    GateStart,
    /// `approve <slug>` — ゲートの通過。
    Approve,
    /// `reject <slug> --feedback <text>` — ゲートでの差し戻し。
    Reject,
    /// `revise <slug>` — 差し戻し後のゲート再入。
    Revise,
    /// `skip <slug> --reason <text> --route` — ルーティングされた読み飛ばし。
    Skip,
    /// `advance <slug>` — 非ゲート完了後のカーソル前進。
    ///
    /// **この build に対応する集約コマンドは無い** — 非ゲート完了のパイプラインは b42 で
    /// 撤去した (#85 = A)。ディスパッチ表の完全性のために列挙する。
    Advance,
    /// `complete-workflow <slug>` — 最終ステージの完了。
    ///
    /// [`TransitionStep::Advance`] と同じ理由で対応する集約コマンドが無い。
    CompleteWorkflow,
}

impl TransitionStep {
    /// upstream `aidlc-state.ts` のサブコマンド綴り (逐語 — `committed.push(subArgs[0])`)。
    #[must_use]
    pub const fn subcommand(self) -> &'static str {
        match self {
            // `--recovered` は引数であってサブコマンド名ではない (ピン `:5876` / `:5908`)。
            TransitionStep::GateStartRecovered | TransitionStep::GateStart => "gate-start",
            TransitionStep::Approve => "approve",
            TransitionStep::Reject => "reject",
            TransitionStep::Revise => "revise",
            TransitionStep::Skip => "skip",
            TransitionStep::Advance => "advance",
            TransitionStep::CompleteWorkflow => "complete-workflow",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_step_spells_its_upstream_subcommand() {
        for (step, spelled) in [
            (TransitionStep::GateStartRecovered, "gate-start"),
            (TransitionStep::GateStart, "gate-start"),
            (TransitionStep::Approve, "approve"),
            (TransitionStep::Reject, "reject"),
            (TransitionStep::Revise, "revise"),
            (TransitionStep::Skip, "skip"),
            (TransitionStep::Advance, "advance"),
            (TransitionStep::CompleteWorkflow, "complete-workflow"),
        ] {
            assert_eq!(step.subcommand(), spelled, "{step:?}");
        }
    }

    #[test]
    fn the_recovered_gate_start_is_spelled_like_an_organic_one() {
        // `--recovered` は引数であってサブコマンド名ではない — 逐語
        // `Committed gate-start + approve for ...` がその証拠である。
        assert_eq!(
            TransitionStep::GateStartRecovered.subcommand(),
            TransitionStep::GateStart.subcommand()
        );
    }
}
