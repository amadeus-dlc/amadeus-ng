//! D4.a の観測 — 読めた状態ファイルの版分類と、本家分類の message。

use super::StateVersionKind;

/// 分類は runtime と同一の判定 ([`StateVersionClassification::classify`]) の答えの**写し**で
/// あり、message は本家 `classifyStateVersion` の逐語 (`Ok` では `None`) である。逐語は
/// 出す側 (合成ルートの `wording`) が組み、ここは観測した事実として運ぶだけである。
///
/// [`StateVersionClassification::classify`]: super::StateVersionClassification::classify
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateVersionObservation {
    kind: StateVersionKind,
    message: Option<String>,
}

impl StateVersionObservation {
    /// 分類結果と message を束ねる。
    #[must_use]
    pub const fn new(kind: StateVersionKind, message: Option<String>) -> Self {
        Self { kind, message }
    }

    /// runtime と同じ分類器の答え。
    #[must_use]
    pub const fn kind(&self) -> StateVersionKind {
        self.kind
    }

    /// 本家分類の message (`Ok` では `None`)。
    #[must_use]
    pub fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }
}
