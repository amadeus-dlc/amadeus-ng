//! 状態版の 4 分類 (U2 の分類器の答えの写し)。

/// `ok / unparseable / past / future`。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateVersionKindView {
    /// この build が理解する版。
    Ok,
    /// 版の行が無い・空・解釈不能。
    Unparseable,
    /// この build より古い版。
    Past,
    /// この build より新しい版。
    Future,
}
