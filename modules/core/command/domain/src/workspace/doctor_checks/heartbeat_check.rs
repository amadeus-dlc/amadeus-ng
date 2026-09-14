//! D2.f — hook heartbeat (本家 `aidlc-utility.ts:3073-3218` の 5 分岐、許容差 1832 の 300000ms)。

use crate::workspace::{DoctorCheck, DoctorCheckId, HeartbeatObservation};

/// 本家 1826 の復旧手順。
const HOOK_EXECUTION_RECOVERY: &str = "1. Run /hooks to check hook approval and policy state. 2. If hooks need approval, approve them and fully restart the CLI; approval does not take effect until a full restart. 3. If /hooks says hooks are restricted by policy, only your Claude Code administrator can lift allowManagedHooksOnly in managed-settings.json. Until then, for an attended session, launch the CLI with AIDLC_SKIP_HUMAN_PRESENCE_GUARD=1 and AIDLC_SKIP_SUMMARY_CONFIRMATION_GUARD=1";
/// 本家 1832 — 最終進行より 5 分を超えて古い heartbeat だけを遅延とする。
const HOOK_HEARTBEAT_STALE_SLACK_MS: i64 = 5 * 60 * 1000;

/// heartbeat の 1 行。
pub(super) fn evaluate(heartbeat: &HeartbeatObservation) -> DoctorCheck {
    let id = DoctorCheckId::D2f;
    if !heartbeat.entries().is_empty() {
        let newest_heartbeat = heartbeat
            .entries()
            .iter()
            .filter_map(|entry| entry.timestamp().millis().map(|ms| (ms, entry)))
            .max_by_key(|(ms, _)| *ms);
        let newest_progress = heartbeat
            .newest_progress()
            .and_then(|progress| progress.millis().map(|ms| (ms, progress)));
        if let (Some((heartbeat_ms, entry)), Some((progress_ms, progress))) =
            (newest_heartbeat, newest_progress)
            && progress_ms - heartbeat_ms > HOOK_HEARTBEAT_STALE_SLACK_MS
        {
            return DoctorCheck::failed(
                id,
                format!(
                    "Hooks last fired {}, but the workflow last advanced {}",
                    entry.timestamp().raw(),
                    progress.raw()
                ),
                HOOK_EXECUTION_RECOVERY.to_string(),
            );
        }
        let entries: Vec<String> = heartbeat
            .entries()
            .iter()
            .map(|entry| format!("{} {}", entry.hook(), entry.timestamp().raw()))
            .collect();
        return DoctorCheck::passed(id, format!("Hooks last fired: {}", entries.join(", ")));
    }
    let progressed = heartbeat.progressed_stage_count();
    if !heartbeat.health_dir_exists() && progressed > 0 {
        let stages = if progressed == 1 { "stage" } else { "stages" };
        return DoctorCheck::failed(
            id,
            format!(
                "Hooks have never executed although this workflow has progressed {progressed} {stages}"
            ),
            HOOK_EXECUTION_RECOVERY.to_string(),
        );
    }
    if heartbeat.health_dir_exists()
        && !heartbeat.has_heartbeat_files()
        && heartbeat.stage_started()
    {
        return DoctorCheck::failed(
            id,
            "Hook heartbeat data".to_string(),
            "health dir exists and the ledger shows STAGE_STARTED, but no hook has ever fired — verify hooks are registered in settings.json".to_string(),
        );
    }
    if !heartbeat.health_dir_exists()
        || (!heartbeat.has_heartbeat_files() && !heartbeat.stage_started())
    {
        return DoctorCheck::passed(
            id,
            "Hook heartbeats: not yet fired (first workflow stage will populate)".to_string(),
        );
    }
    DoctorCheck::failed(
        id,
        "Hook heartbeat data".to_string(),
        "health dir exists but heartbeat files are unreadable — verify permissions and hook registration".to_string(),
    )
}
