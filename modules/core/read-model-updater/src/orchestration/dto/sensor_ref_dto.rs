//! `SensorRef` の永続化 DTO (**読む側**) — センサ参照 1 件の行の形。

use core_command_domain::workflow_definition::SensorRef;
use serde::{Deserialize, Serialize};

/// センサ参照 1 件の行の形。**フィールド名と並びが契約**である。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct SensorRefDto {
    id: String,
    path: String,
    matches: Option<String>,
}

impl SensorRefDto {
    /// ドメインの公開アクセサだけを読んで DTO を組む (書き — テストが行を用意する口)。
    pub(super) fn of(sensor: &SensorRef) -> SensorRefDto {
        SensorRefDto {
            id: sensor.id().to_string(),
            path: sensor.path().to_string(),
            matches: sensor.matches().map(str::to_string),
        }
    }

    /// ドメインの材料へ戻す (読み)。閉集合を持たないので失敗しない。
    pub(super) fn to_domain(&self) -> SensorRef {
        SensorRef::new(self.id.clone(), self.path.clone(), self.matches.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sensor_reference_survives_the_round_trip() {
        let sensor = SensorRef::new("linter", "sensors/linter.md", Some("*.rs".to_string()));
        assert_eq!(SensorRefDto::of(&sensor).to_domain(), sensor);

        let bare = SensorRef::new("linter", "sensors/linter.md", None);
        assert_eq!(SensorRefDto::of(&bare).to_domain(), bare);
    }

    #[expect(
        clippy::disallowed_methods,
        reason = "契約 JSON ではなくワイヤ形式そのものの逐語固定 (BR1.7 の射程外)"
    )]
    #[test]
    fn the_absent_match_pattern_stays_absent() {
        // `matches` は「宣言が無い」を `None` で表す — 空文字列へ潰さない。
        let dto = SensorRefDto::of(&SensorRef::new("linter", "sensors/linter.md", None));
        assert_eq!(
            serde_json::to_string(&dto).unwrap(),
            r#"{"id":"linter","path":"sensors/linter.md","matches":null}"#
        );
    }
}
