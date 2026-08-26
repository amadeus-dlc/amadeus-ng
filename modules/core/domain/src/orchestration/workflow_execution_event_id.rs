//! `WorkflowExecutionEventId` — ドメインイベントの識別子 (本家 `Event::ID`)。

use std::fmt;
use std::num::NonZeroUsize;

use serde::{Deserialize, Serialize};

use super::intent_id::IntentId;

/// `IntentId` の正準形 (16 進小文字とハイフン) に現れない区切り。
const SEPARATOR: char = '#';

/// 1 つの [`WorkflowExecutionEvent`] を一意に指す識別子。
///
/// **採番は決定的** — 集約識別子と集約内の順序番号の組そのものである。集約は時計も乱数も
/// 持たない (NFR3.1 / ADR-002) ので、ULID や UUID のような外部の採番機構をイベント生成の
/// 経路に持ち込まない。ジャーナルの `UNIQUE (aggregate_id, seq_nr)` がこの組の一意性を
/// すでに保証しているため、別立ての採番を足しても一意性は増えない。
///
/// `Display` は `<intent_id><SEPARATOR><seq_nr>`。区切りは `IntentId` の正準表記に現れない
/// ので、綴りから組へ一意に戻せる。
///
/// [`WorkflowExecutionEvent`]: super::workflow_execution_event::WorkflowExecutionEvent
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct WorkflowExecutionEventId {
    intent_id: IntentId,
    // NonZeroUsize なので「seq_nr = 0 の識別子」は表現不能 (AVDM)。serde の復号も
    // 型が 0 を拒否する — 検証コンストラクタを別立てしなくても両経路が塞がる。
    seq_nr: NonZeroUsize,
}

impl WorkflowExecutionEventId {
    /// 集約識別子と集約内の順序番号から組み立てる (イベント生成時に封筒が採番する)。
    ///
    /// 順序番号は 1 始まり (BR2.1) なので `NonZeroUsize` で受ける — 0 の識別子は型レベルで
    /// 構成できない。
    #[must_use]
    pub const fn new(intent_id: IntentId, seq_nr: NonZeroUsize) -> WorkflowExecutionEventId {
        WorkflowExecutionEventId { intent_id, seq_nr }
    }

    /// このイベントが属する集約の識別子。
    #[must_use]
    pub const fn intent_id(&self) -> &IntentId {
        &self.intent_id
    }

    /// 集約内で 1 から単調増加する順序番号 (適用後の集約 `seq_nr` と一致 — BR2.1)。
    #[must_use]
    pub const fn seq_nr(&self) -> usize {
        self.seq_nr.get()
    }
}

impl fmt::Display for WorkflowExecutionEventId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{SEPARATOR}{}", self.intent_id, self.seq_nr)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "01a02785-1bd8-76eb-aeea-5aa303ebd5b6";

    fn intent() -> IntentId {
        IntentId::parse(SAMPLE).unwrap()
    }

    #[test]
    fn the_identifier_is_the_aggregate_and_the_sequence_number() {
        let id = WorkflowExecutionEventId::new(intent(), NonZeroUsize::new(7).unwrap());
        assert_eq!(id.intent_id(), &intent());
        assert_eq!(id.seq_nr(), 7);
        assert_eq!(id.to_string(), format!("{SAMPLE}#7"));
    }

    #[test]
    fn two_events_of_the_same_aggregate_differ_by_their_sequence_number() {
        assert_ne!(
            WorkflowExecutionEventId::new(intent(), NonZeroUsize::new(1).unwrap()),
            WorkflowExecutionEventId::new(intent(), NonZeroUsize::new(2).unwrap())
        );
        assert_eq!(
            WorkflowExecutionEventId::new(intent(), NonZeroUsize::new(1).unwrap()),
            WorkflowExecutionEventId::new(intent(), NonZeroUsize::new(1).unwrap())
        );
    }

    #[test]
    fn the_identifier_round_trips_through_serde() {
        let id = WorkflowExecutionEventId::new(intent(), NonZeroUsize::new(3).unwrap());
        // 本家 trait の serde 境界の往復確認であり、契約 JSON (BR1.7) の直列化経路では
        // ないため、canon-json を経ない素の serde_json を使う。
        #[allow(
            clippy::disallowed_methods,
            reason = "契約 JSON ではなく serde 境界そのものの往復確認 (BR1.7 の射程外)"
        )]
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(
            serde_json::from_str::<WorkflowExecutionEventId>(&json).unwrap(),
            id
        );
    }
}
