//! `SensorRefView` — compile 時に manifest から逐語スナップショットしたセンサー適用宣言。

/// センサー適用宣言 1 件。フック側は fire 時に manifest を再オープンしない (12 §2.2 #28)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SensorRefView {
    id: String,
    path: String,
    matches: Option<String>,
}

impl SensorRefView {
    /// compile 時のスナップショットを組む。`matches` が manifest に無ければ `None`。
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        path: impl Into<String>,
        matches: Option<String>,
    ) -> SensorRefView {
        SensorRefView {
            id: id.into(),
            path: path.into(),
            matches,
        }
    }

    /// センサー id。directive 射影 ([`super::StageView::sensor_ids`]) が残すのはこの欄だけ。
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// センサー定義ファイルのパス。格納形にのみ存在し、directive 射影では落ちる。
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// capability glob の逐語コピー (欠損しうる)。
    #[must_use]
    pub fn matches(&self) -> Option<&str> {
        self.matches.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_three_fields_stay_distinct_and_matches_may_be_absent() {
        let sensor = SensorRefView::new(
            "no-todo",
            ".claude/sensors/no-todo.md",
            Some("**/*.rs".to_string()),
        );
        assert_eq!(sensor.id(), "no-todo");
        assert_eq!(sensor.path(), ".claude/sensors/no-todo.md");
        assert_eq!(sensor.matches(), Some("**/*.rs"));

        let without = SensorRefView::new("x", ".claude/sensors/x.md", None);
        assert_eq!(without.matches(), None);
    }
}
