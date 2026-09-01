//! `GateField` — `run-stage` の `gate` フィールド (公開言語 B14)。
//!
//! ワイヤ上は boolean か `"unresolved"` の 3 値しか取らない (E2)。閉集合なので enum で運び、
//! 未知の値は構成不能にする。

/// `run-stage` の `gate` フィールド — boolean か `"unresolved"` のみ (E2)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateField {
    /// 承認ゲート付き (`true`)。
    Gated,
    /// ゲートなし (`false` — 初期化ステージ)。
    Ungated,
    /// walking-skeleton 判定が要る非決定ケース (`"unresolved"`)。
    Unresolved,
}
