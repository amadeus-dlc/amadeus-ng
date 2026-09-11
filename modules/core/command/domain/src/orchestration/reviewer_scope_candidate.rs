//! `ReviewerScopeCandidate` — 呼出し 1 件が名指した、判定にかける綴り。
use super::{InspectedCommand, ScopeToken};

/// 工具の入力から取り出した候補 1 件と、その読み方 (upstream `candidateStrings` の `kind`)。
///
/// 読み方が別なのは、**同じ綴りでも許否が変わる**からである。探索の根は「その下を
/// すべて掃く」ので `construction/` の**上**にあるだけで越境になるが、開く対象は
/// その経路自身しか触らない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReviewerScopeCandidate {
    /// 開く・書く対象そのもの。
    Target(ScopeToken),
    /// 再帰的に掃く根 (`LS` の `path`、`Glob`/`Grep` の `path`)。
    SearchRoot(ScopeToken),
    /// 経路の形をしたパターン (`Glob` の `pattern`、`Grep` の `glob`)。
    Glob(ScopeToken),
    /// `Bash` の 1 呼出し。
    Command(InspectedCommand),
}
