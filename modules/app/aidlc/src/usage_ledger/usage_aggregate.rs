//! 1 つの所有境界（workspace 全体・workflow・session）での集計。
use super::ordered_map::OrderedMap;
use super::priced_row::PricedRow;
use super::stage_bucket::{StageBucket, totals_map_of_json, totals_map_to_json};
use super::totals::Totals;
use core_infrastructure::canon_json::{JsonValue, ObjectMembers};

/// 合計・ステージ別・モデル別・エージェント別。
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct UsageAggregate {
    totals: Totals,
    by_stage: OrderedMap<StageBucket>,
    by_model: OrderedMap<Totals>,
    by_agent: OrderedMap<Totals>,
}

impl UsageAggregate {
    /// 4 つの集計から組む。
    pub(crate) const fn new(
        totals: Totals,
        by_stage: OrderedMap<StageBucket>,
        by_model: OrderedMap<Totals>,
        by_agent: OrderedMap<Totals>,
    ) -> Self {
        Self {
            totals,
            by_stage,
            by_model,
            by_agent,
        }
    }

    /// 1 行を足し込む。ステージが分かるときだけステージ別も更新する。
    pub(crate) fn fold(&mut self, row: &PricedRow, stage: Option<&str>) {
        self.totals.add(row.counts(), row.usd());
        self.by_model.update(row.model_bucket(), |totals| {
            totals.add(row.counts(), row.usd())
        });
        self.by_agent.update(row.agent_bucket(), |totals| {
            totals.add(row.counts(), row.usd())
        });
        if let Some(stage) = stage {
            self.by_stage.update(stage, |bucket| bucket.fold(row));
        }
    }

    /// この境界の合計。
    pub(crate) const fn totals(&self) -> Totals {
        self.totals
    }

    /// `{totals, byStage, byModel, byAgent}` のメンバ列（上位が `sessions` を足せる形）。
    pub(crate) fn to_members(&self) -> ObjectMembers {
        let mut fields = ObjectMembers::new();
        fields.insert("totals", self.totals.to_json());
        fields.insert(
            "byStage",
            JsonValue::Object(self.by_stage.fold_left(
                ObjectMembers::new(),
                |mut members, slug, bucket| {
                    members.insert(slug, bucket.to_json());
                    members
                },
            )),
        );
        fields.insert("byModel", totals_map_to_json(&self.by_model));
        fields.insert("byAgent", totals_map_to_json(&self.by_agent));
        fields
    }

    /// JSON から読む。
    pub(crate) fn of_json(value: Option<&JsonValue>) -> Self {
        let Some(JsonValue::Object(members)) = value else {
            return Self::default();
        };
        Self::new(
            Totals::of_json(members.get("totals")),
            match members.get("byStage") {
                Some(JsonValue::Object(stages)) => {
                    stages.fold_left(OrderedMap::new(), |mut map, slug, bucket| {
                        map.insert(slug, StageBucket::of_json(bucket));
                        map
                    })
                }
                _ => OrderedMap::new(),
            },
            totals_map_of_json(members.get("byModel")),
            totals_map_of_json(members.get("byAgent")),
        )
    }
}
