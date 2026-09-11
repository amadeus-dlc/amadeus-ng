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
    fire_on: Option<String>,
    default_severity: Option<String>,
    category: Option<String>,
}

impl SensorRef {
    /// compile 時のスナップショットを組む。`matches` が manifest に無ければ `None`。
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        path: impl Into<String>,
        matches: Option<String>,
        fire_on: Option<String>,
        default_severity: Option<String>,
        category: Option<String>,
    ) -> SensorRef {
        SensorRef {
            id: id.into(),
            path: path.into(),
            matches,
            fire_on,
            default_severity,
            category,
        }
    }

    /// 配布資産が指定する発火時機。実行機能はここには持たない。
    #[must_use]
    pub fn fire_on(&self) -> Option<&str> {
        self.fire_on.as_deref()
    }

    /// 配布資産が指定する既定の重要度。
    #[must_use]
    pub fn default_severity(&self) -> Option<&str> {
        self.default_severity.as_deref()
    }

    /// 配布資産が指定する分類。
    #[must_use]
    pub fn category(&self) -> Option<&str> {
        self.category.as_deref()
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
