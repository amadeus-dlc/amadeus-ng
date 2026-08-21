//! directive プロトコル (Published Language、B14) — `DirectiveKind` 10 種の閉集合。
//! Directive 本体の判別共用体・28KiB 上限・continue_token ペイロード型は後続スライス。
//! 出典: upstream `aidlc-directive.ts:419-430` (02 §4.1)。

#![forbid(unsafe_code)]

/// 10 種の閉集合。`PresentGate` と `DispatchSubagent` は upstream の placeholder —
/// 「Do not implement those two placeholder behaviours speculatively.」(02 §4.1)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DirectiveKind {
    LoadSteering,
    RunStage,
    DispatchSubagent,
    InvokeSwarm,
    PresentGate,
    Ask,
    Print,
    Error,
    Done,
    Parked,
}

impl DirectiveKind {
    pub const ALL: &'static [DirectiveKind] = &[
        DirectiveKind::LoadSteering,
        DirectiveKind::RunStage,
        DirectiveKind::DispatchSubagent,
        DirectiveKind::InvokeSwarm,
        DirectiveKind::PresentGate,
        DirectiveKind::Ask,
        DirectiveKind::Print,
        DirectiveKind::Error,
        DirectiveKind::Done,
        DirectiveKind::Parked,
    ];

    /// ワイヤ上の正準綴り。
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            DirectiveKind::LoadSteering => "load-steering",
            DirectiveKind::RunStage => "run-stage",
            DirectiveKind::DispatchSubagent => "dispatch-subagent",
            DirectiveKind::InvokeSwarm => "invoke-swarm",
            DirectiveKind::PresentGate => "present-gate",
            DirectiveKind::Ask => "ask",
            DirectiveKind::Print => "print",
            DirectiveKind::Error => "error",
            DirectiveKind::Done => "done",
            DirectiveKind::Parked => "parked",
        }
    }

    #[must_use]
    pub fn parse(s: &str) -> Option<DirectiveKind> {
        Self::ALL.iter().copied().find(|k| k.as_str() == s)
    }

    #[must_use]
    pub const fn is_placeholder(self) -> bool {
        matches!(
            self,
            DirectiveKind::PresentGate | DirectiveKind::DispatchSubagent
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ten_kinds_round_trip_and_unknown_is_rejected() {
        assert_eq!(DirectiveKind::ALL.len(), 10);
        for k in DirectiveKind::ALL {
            assert_eq!(DirectiveKind::parse(k.as_str()), Some(*k));
        }
        assert_eq!(DirectiveKind::parse("gate"), None);
    }

    #[test]
    fn exactly_the_two_placeholders_are_marked() {
        let placeholders: Vec<_> = DirectiveKind::ALL
            .iter()
            .filter(|k| k.is_placeholder())
            .collect();
        assert_eq!(placeholders.len(), 2);
    }
}
