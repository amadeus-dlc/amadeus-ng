//! `ProcessProbe` ポートの実プロセス判定実装 — infra-io `process_probe::process_alive` に委譲する。

use core_use_case::workspace::ProcessProbe;

/// OS のプロセステーブルに対する実判定。
#[derive(Debug, Clone, Copy, Default)]
pub struct OsProcessProbe;

impl OsProcessProbe {
    /// 単位型を作る (状態を持たないので設定項目は無い)。
    #[must_use]
    pub const fn new() -> OsProcessProbe {
        OsProcessProbe
    }
}

impl ProcessProbe for OsProcessProbe {
    fn is_alive(&self, pid: i32) -> bool {
        infra_io::process_probe::process_alive(pid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_current_process_is_alive() {
        let probe = OsProcessProbe::new();
        assert!(probe.is_alive(std::process::id() as i32));
    }
}
