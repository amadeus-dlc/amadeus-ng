//! 1 ステージ分の集計。
use super::ordered_map::OrderedMap;
use super::priced_row::PricedRow;
use super::totals::Totals;
use core_infrastructure::canon_json::{JsonValue, ObjectMembers};

/// ステージ内の合計と、そのステージ **自身の** モデル別・エージェント別内訳。
///
/// 全ステージを足した大域の内訳を使うと `Cost USD` と食い違うので、ここは分けて持つ。
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct StageBucket {
    totals: Totals,
    by_model: OrderedMap<Totals>,
    by_agent: OrderedMap<Totals>,
}

impl StageBucket {
    /// 3 つの集計から組む。
    pub(crate) const fn new(
        totals: Totals,
        by_model: OrderedMap<Totals>,
        by_agent: OrderedMap<Totals>,
    ) -> Self {
        Self {
            totals,
            by_model,
            by_agent,
        }
    }

    /// 1 行を足し込む。
    pub(crate) fn fold(&mut self, row: &PricedRow) {
        self.totals.add(row.counts(), row.usd());
        self.by_model.update(row.model_bucket(), |totals| {
            totals.add(row.counts(), row.usd())
        });
        self.by_agent.update(row.agent_bucket(), |totals| {
            totals.add(row.counts(), row.usd())
        });
    }

    /// JSON の `{totals, byModel, byAgent}`。
    pub(crate) fn to_json(&self) -> JsonValue {
        let mut fields = ObjectMembers::new();
        fields.insert("totals", self.totals.to_json());
        fields.insert("byModel", totals_map_to_json(&self.by_model));
        fields.insert("byAgent", totals_map_to_json(&self.by_agent));
        JsonValue::Object(fields)
    }

    /// JSON から読む。旧形（平坦な `{tokens, usd}`）は内訳なしとして包む。
    pub(crate) fn of_json(value: &JsonValue) -> Self {
        let JsonValue::Object(members) = value else {
            return Self::default();
        };
        if members.get("totals").is_some() {
            return Self::new(
                Totals::of_json(members.get("totals")),
                totals_map_of_json(members.get("byModel")),
                totals_map_of_json(members.get("byAgent")),
            );
        }
        if members.get("tokens").is_some() {
            return Self::new(
                Totals::of_json(Some(value)),
                OrderedMap::new(),
                OrderedMap::new(),
            );
        }
        Self::default()
    }
}

/// `Record<string, Totals>` を JSON へ。
pub(crate) fn totals_map_to_json(map: &OrderedMap<Totals>) -> JsonValue {
    JsonValue::Object(
        map.fold_left(ObjectMembers::new(), |mut members, key, totals| {
            members.insert(key, totals.to_json());
            members
        }),
    )
}

/// JSON から `Record<string, Totals>` を読む。
pub(crate) fn totals_map_of_json(value: Option<&JsonValue>) -> OrderedMap<Totals> {
    match value {
        Some(JsonValue::Object(members)) => {
            members.fold_left(OrderedMap::new(), |mut map, key, entry| {
                map.insert(key, Totals::of_json(Some(entry)));
                map
            })
        }
        _ => OrderedMap::new(),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    use super::*;
    use core_infrastructure::canon_json::parse;

    fn bucket(text: &str) -> StageBucket {
        StageBucket::of_json(&parse(text).expect("JSON"))
    }

    /// 旧形（平坦な `{tokens, usd}`）は内訳なしの合計として包み、object でなければ空、
    /// どちらの鍵も無ければ空である。内訳の欄が object でなければ空の内訳になる。
    #[test]
    fn legacy_flat_buckets_are_wrapped_and_malformed_ones_are_empty() {
        let legacy = bucket(
            r#"{"tokens":{"input":3,"output":4,"cacheCreate5m":0,"cacheCreate1h":0,"cacheRead":1},"usd":0.5}"#,
        );
        let expected_totals = Totals::of_json(Some(
            &parse(r#"{"tokens":{"input":3,"output":4,"cacheRead":1},"usd":0.5}"#).expect("JSON"),
        ));
        assert_eq!(legacy.totals, expected_totals);
        assert!(legacy.by_model.keys().is_empty());
        assert!(legacy.by_agent.keys().is_empty());
        assert_eq!(bucket("[1,2]"), StageBucket::default());
        assert_eq!(bucket(r#"{"other":1}"#), StageBucket::default());
        let scalar_breakdown = bucket(r#"{"totals":{"usd":1},"byModel":7,"byAgent":"x"}"#);
        assert_eq!(
            scalar_breakdown.totals,
            Totals::of_json(Some(&parse(r#"{"usd":1}"#).expect("JSON")))
        );
        assert!(scalar_breakdown.by_model.keys().is_empty());
        assert!(scalar_breakdown.by_agent.keys().is_empty());
        assert!(totals_map_of_json(None).keys().is_empty());
    }
}
