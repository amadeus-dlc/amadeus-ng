//! D4.a / D4.b の観測 — 状態ファイルの有無・可読性・版分類。

use super::{ObservationFailure, StateVersionObservation};

/// `aidlc-state.md` の 3 態。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StateFileObservation {
    /// ファイルが無い。
    Absent,
    /// ファイルはあるが読めない。
    Unreadable(ObservationFailure),
    /// 読めて、runtime と同じ分類器が版を判定した。
    Classified(StateVersionObservation),
}
