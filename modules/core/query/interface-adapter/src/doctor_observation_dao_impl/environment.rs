//! D1 の観測 — Bun の所在と、この build の入口。

use std::path::Path;

use core_query_use_case::orchestration::{NativeEntryPointsView, ObservationFailure};

use super::super::doctor_environment::DoctorEnvironment;
use super::super::native_doctor_facts::NativeDoctorFacts;

/// Bun が `PATH` か `~/.bun/bin/bun` に在るか (本家 `Bun.which("bun") || ~/.bun/bin/bun`)。
pub(super) fn bun_found(environment: &DoctorEnvironment) -> bool {
    let executable = if cfg!(windows) { "bun.exe" } else { "bun" };
    let on_path = environment.path().is_some_and(|path| {
        std::env::split_paths(path).any(|dir| is_executable(&dir.join(executable)))
    });
    on_path
        || environment
            .home()
            .is_some_and(|home| home.join(".bun").join("bin").join("bun").exists())
}

/// 通常ファイルで、UNIX では実行ビットが立っているか (`Bun.which` の条件)。
fn is_executable(path: &Path) -> bool {
    let Ok(metadata) = std::fs::metadata(path) else {
        return false;
    };
    if !metadata.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
}

/// 合成ルートが写した入口の事実を View へ写す。
pub(super) fn entry_points(facts: &NativeDoctorFacts) -> NativeEntryPointsView {
    NativeEntryPointsView::new(
        facts
            .binary()
            .as_ref()
            .map(|path| path.to_string_lossy().into_owned())
            .map_err(|cause| ObservationFailure::new(cause.clone())),
        facts.missing_entry_points().to_vec(),
    )
}
