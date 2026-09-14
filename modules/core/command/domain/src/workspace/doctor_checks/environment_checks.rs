//! D1.a / D1.b — Bun の所在と、この build の入口。

use crate::workspace::{DoctorCheck, DoctorCheckId, DoctorObservation};

/// 本家 2266–2278 のラベル。
const BUN_LABEL: &str = "bun installed (required for CLI tools and hooks)";
/// 本家 2276–2277 の fix (Windows 以外)。
#[cfg(not(windows))]
const BUN_FIX: &str = "install via `curl -fsSL https://bun.sh/install | bash`";
/// 本家 2275 の fix (Windows)。
#[cfg(windows)]
const BUN_FIX: &str =
    "install via `npm install -g bun` or `powershell -c \"irm bun.sh/install.ps1 | iex\"`";
/// 独自の必須診断のラベル (C7 D1.b)。
const ENTRY_POINTS_LABEL: &str = "Native engine entry points";

/// D1.a と D1.b の 2 行。
pub(super) fn evaluate(observed: &DoctorObservation) -> Vec<DoctorCheck> {
    let bun = if observed.bun_found() {
        DoctorCheck::passed(DoctorCheckId::D1a, BUN_LABEL.to_string())
    } else {
        DoctorCheck::failed(
            DoctorCheckId::D1a,
            BUN_LABEL.to_string(),
            BUN_FIX.to_string(),
        )
    };
    let entry_points = observed.entry_points();
    let mut causes = Vec::new();
    if let Err(cause) = entry_points.binary() {
        causes.push(format!("current executable: unreadable ({cause})"));
    }
    if !entry_points.missing_entry_points().is_empty() {
        causes.push(format!(
            "aidlc: missing entry points ({})",
            entry_points.missing_entry_points().join(", ")
        ));
    }
    let native = if causes.is_empty() {
        DoctorCheck::passed(DoctorCheckId::D1b, ENTRY_POINTS_LABEL.to_string())
    } else {
        DoctorCheck::failed(
            DoctorCheckId::D1b,
            ENTRY_POINTS_LABEL.to_string(),
            causes.join("; "),
        )
    };
    vec![bun, native]
}
