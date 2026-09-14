//! D2.e の観測 — このリポジトリ固有の接続定義が語る、フックごとの結び先。
//!
//! 定義の置き場 (`scripts/aidlc-selfhost/hook-binding.json`) は観測側の実装詳細であり、
//! ここへは「何が宣言されていたか」だけが届く。

use super::ObservationFailure;

/// 接続定義が語る「どのフックをどちらの面へ結ぶか」。
///
/// 配布そのままの作業ツリーには定義が無い。そこでは native と配布の混ざり方を判定せず
/// (照合する宣言が無い)、未知の native 名と解決できない呼出しだけを見る。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HookBindingDeclaration {
    /// 定義が無い作業ツリー。宣言と登録の照合は行わない。
    Absent,
    /// 定義を読めた。
    Declared {
        /// この build のフック面 (`aidlc hook <name>`) へ結ぶと宣言した名前。
        native: Vec<String>,
        /// 配布 TypeScript のまま残すと宣言した名前。
        distributed: Vec<String>,
    },
    /// 定義はあるが読めない・壊れている。
    Unreadable(ObservationFailure),
}
