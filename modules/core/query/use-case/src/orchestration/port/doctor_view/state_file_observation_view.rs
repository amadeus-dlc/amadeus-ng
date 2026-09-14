//! D4.a / D4.b の観測 — 状態ファイルの有無・可読性・版分類。

use super::{ObservationFailure, StateVersionView};

/// `aidlc-state.md` の 3 態。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StateFileObservationView {
    /// ファイルが無い。
    Absent,
    /// ファイルはあるが読めない。
    Unreadable(ObservationFailure),
    /// 読めて、U2 の分類器が版を判定した。
    Classified(StateVersionView),
}
