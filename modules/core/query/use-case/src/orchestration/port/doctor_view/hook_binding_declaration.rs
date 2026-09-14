//! D2.e の観測 — 接続定義が語るフックごとの結び先。

use super::ObservationFailure;

/// 接続定義 (`scripts/aidlc-selfhost/hook-binding.json`) が語る結び先。
///
/// 定義の無い作業ツリー (配布そのまま) では [`HookBindingDeclaration::Absent`] になる。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HookBindingDeclaration {
    /// 定義が無い作業ツリー。
    Absent,
    /// 定義を読めた。
    Declared {
        /// native 面へ結ぶと宣言した名前。
        native: Vec<String>,
        /// 配布 TypeScript のまま残すと宣言した名前。
        distributed: Vec<String>,
    },
    /// 定義はあるが読めない・壊れている。
    Unreadable(ObservationFailure),
}
