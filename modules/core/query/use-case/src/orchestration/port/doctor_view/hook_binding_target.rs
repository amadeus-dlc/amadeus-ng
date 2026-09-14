//! フック登録 1 件の呼出し先の分類。

/// `settings.json` の 1 登録がどの実装へ結ばれているか。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HookBindingTarget {
    /// この build のフック面 (`aidlc hook <name>`) へ結ばれている。
    Native(String),
    /// 配布物の TypeScript フック (`aidlc-<name>.ts`) へ結ばれている。
    Distributed(String),
    /// どちらとも識別できない呼出し。
    Unknown,
}
