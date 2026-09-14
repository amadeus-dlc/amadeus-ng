//! D5.a の観測 — イベントストアの有無・可読性・形。

use super::{ObservationFailure, StoreSchemaView};

/// `.aidlc-store.sqlite` の 3 態。診断は存在しないストアを作らない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreObservationView {
    /// ファイルが無い。
    Absent,
    /// ファイルはあるが開けない。
    Unreadable(ObservationFailure),
    /// 読取専用で開けた。
    Opened(StoreSchemaView),
}
