//! 1 つの作業（intent）の集計と、その中の session 別集計。
use super::ordered_map::OrderedMap;
use super::priced_row::PricedRow;
use super::usage_aggregate::UsageAggregate;
use core_infrastructure::canon_json::{JsonValue, ObjectMembers};

/// 作業単位の集計。`sessions` が権威ある workflow×session の交差である。
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct WorkflowUsage {
    aggregate: UsageAggregate,
    sessions: OrderedMap<UsageAggregate>,
}

impl WorkflowUsage {
    /// 集計と session 別内訳から組む。
    pub(crate) const fn new(
        aggregate: UsageAggregate,
        sessions: OrderedMap<UsageAggregate>,
    ) -> Self {
        Self {
            aggregate,
            sessions,
        }
    }

    /// 1 行を、作業全体と当該 session の両方へ足し込む。
    pub(crate) fn fold(&mut self, row: &PricedRow, stage: Option<&str>, session_key: &str) {
        self.aggregate.fold(row, stage);
        self.sessions
            .update(session_key, |session| session.fold(row, stage));
    }

    /// この作業の中の 1 session ぶんの集計。
    pub(crate) fn session(&self, session_key: &str) -> Option<&UsageAggregate> {
        self.sessions.get(session_key)
    }

    /// JSON の `{totals, byStage, byModel, byAgent, sessions}`。
    pub(crate) fn to_json(&self) -> JsonValue {
        let mut fields = self.aggregate.to_members();
        fields.insert(
            "sessions",
            JsonValue::Object(self.sessions.fold_left(
                ObjectMembers::new(),
                |mut members, key, session| {
                    members.insert(key, JsonValue::Object(session.to_members()));
                    members
                },
            )),
        );
        JsonValue::Object(fields)
    }

    /// JSON から読む。
    pub(crate) fn of_json(value: &JsonValue) -> Self {
        Self::new(
            UsageAggregate::of_json(Some(value)),
            match value {
                JsonValue::Object(members) => match members.get("sessions") {
                    Some(JsonValue::Object(sessions)) => {
                        sessions.fold_left(OrderedMap::new(), |mut map, key, session| {
                            map.insert(key, UsageAggregate::of_json(Some(session)));
                            map
                        })
                    }
                    _ => OrderedMap::new(),
                },
                _ => OrderedMap::new(),
            },
        )
    }
}
