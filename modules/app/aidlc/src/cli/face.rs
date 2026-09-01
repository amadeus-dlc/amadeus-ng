//! 起動名（`argv[0]`）が指すツール面。

/// 起動名が指すツール面。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Face {
    /// `aidlc-orchestrate` / 素の `aidlc`。
    Orchestrate,
    /// `aidlc-utility`。
    Utility,
}

impl Face {
    /// `argv[0]` の basename から面を決める。
    ///
    /// 未知の名前は `Orchestrate` に倒す — 配布物の既定の顔がエンジンだからである。
    #[must_use]
    pub fn of(argv0: &str) -> Face {
        let name = argv0.rsplit(['/', '\\']).next().unwrap_or(argv0);
        // Windows の `.exe` 接尾辞を落としてから比べる。
        let name = name.strip_suffix(".exe").unwrap_or(name);
        match name {
            "aidlc-utility" => Face::Utility,
            _ => Face::Orchestrate,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_launch_name_selects_the_tool_face() {
        assert_eq!(Face::of("aidlc-utility"), Face::Utility);
        assert_eq!(Face::of("/usr/local/bin/aidlc-utility"), Face::Utility);
        assert_eq!(Face::of("aidlc-utility.exe"), Face::Utility);
        assert_eq!(Face::of("aidlc-orchestrate"), Face::Orchestrate);
        assert_eq!(Face::of("aidlc"), Face::Orchestrate);
        // 未知の名前はエンジンに倒す（配布物の既定の顔）。
        assert_eq!(Face::of("something-else"), Face::Orchestrate);
    }
}
