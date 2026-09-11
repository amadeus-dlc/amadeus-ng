//! 2026-09-09の利用者裁定: nativeの対象外としたセンサー3種だけを期待監査から外す。
//! 原本は変更しない。native側の出力は削らず、架空のセンサー行の発生も検出する。
pub(super) fn expected_native_audit(source: &str) -> String {
    source
        .split_inclusive("\n---\n")
        .fold(String::new(), |mut result, block| {
            let mut events = block
                .lines()
                .filter_map(|line| line.strip_prefix("**Event**: "));
            let excluded = matches!(
                events.next(),
                Some("SENSOR_FIRED" | "SENSOR_PASSED" | "SENSOR_FAILED")
            );
            // 不正・曖昧なブロックまで比較から落とさない。
            if excluded && events.next().is_none() && block.ends_with("\n---\n") {
                // 最初のセンサーブロックに先行するシャード見出しは残す。
                if let Some((prefix, _)) = block.split_once("\n## ") {
                    result.push_str(prefix);
                } else {
                    result.push_str(block);
                }
            } else {
                result.push_str(block);
            }
            result
        })
}

#[cfg(test)]
mod tests {
    use super::expected_native_audit;

    #[test]
    fn only_the_three_approved_sensor_events_are_excluded() {
        let sensor = "\n## Sensor\n**Event**: SENSOR_FIRED\n\n---\n\n## Sensor\n**Event**: SENSOR_PASSED\n\n---\n\n## Sensor\n**Event**: SENSOR_FAILED\n\n---\n";
        let lifecycle =
            "\n## Stage\n**Event**: STAGE_COMPLETED\n**Details**: SENSOR_FIRED\n\n---\n";
        assert_eq!(
            expected_native_audit(&format!("{sensor}{lifecycle}")),
            lifecycle
        );
        let header = "# AI-DLC Audit Log\n";
        assert_eq!(
            expected_native_audit(&format!("{header}{sensor}{lifecycle}")),
            format!("{header}{lifecycle}")
        );
    }

    #[test]
    fn unknown_events_ambiguous_blocks_and_missing_terminators_remain_visible() {
        for source in [
            "\n## Sensor\n**Event**: SENSOR_UNKNOWN\n\n---\n",
            "\n## Mixed\n**Event**: SENSOR_FIRED\n**Event**: STAGE_COMPLETED\n\n---\n",
            "\n## Sensor\n**Event**: SENSOR_FIRED\n",
            "# AI-DLC Audit Log\n",
        ] {
            assert_eq!(expected_native_audit(source), source);
        }
    }
}
