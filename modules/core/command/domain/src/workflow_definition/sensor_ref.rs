//! `SensorRef` — ステージが発火させるセンサーの参照。

/// compile 時に manifest から逐語スナップショットしたセンサー適用宣言。
///
/// フック側は fire 時に manifest を再オープンしない (レポート §2.2 #28)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SensorRef {
    id: String,
    path: String,
    /// capability glob の逐語コピー (欠損しうる)。
    matches: Option<String>,
}

impl SensorRef {
    /// compile 時のスナップショットを組む。`matches` が manifest に無ければ `None`。
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        path: impl Into<String>,
        matches: Option<String>,
    ) -> SensorRef {
        SensorRef {
            id: id.into(),
            path: path.into(),
            matches,
        }
    }

    /// センサー id。directive 射影 (`StageNode::sensor_ids`) が残すのはこの欄だけ。
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
