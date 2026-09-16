//! 起動名（`argv[0]`）が指すツール面。

/// 起動名が指すツール面。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Face {
    /// 本家aidlc-jumpのresolve/execute面。
    Jump,
    /// テスト契約と計画承認の入口。
    TestingPosture,
    /// `aidlc-orchestrate` / 素の `aidlc`。
    Orchestrate,
    /// `aidlc-utility`。
    Utility,
    /// `aidlc-log`（対話イベントの記録面 — b48 で `review` 動詞だけが配線されている）。
    Log,
    /// `aidlc-state`（状態ファイルの書込面 — b49 で `practices-promote` 動詞だけが
    /// 配線されている）。
    State,
    /// `aidlc-bolt`（Construction の Bolt 面 — b50 で `set-autonomy` 動詞だけが
    /// 配線されている）。
    Bolt,
    /// `aidlc-learnings`（§13 の学びの儀式 — `surface` と `persist`）。
    Learnings,
    /// `aidlc-review-brief`（レビュー判断の文脈 — `review` / `context` / `summary`）。
    ///
    /// 本家 2.8.2 の ROUTES 表も noun `review-brief` をこの面（`TOOLS.reviewBrief`）へ
    /// 委譲する。engine 自身が受ける `routeOnly` ルートではない。
    ReviewBrief,
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
            "aidlc-jump" => Face::Jump,
            "aidlc-testing-posture" => Face::TestingPosture,
            "aidlc-utility" => Face::Utility,
            "aidlc-log" => Face::Log,
            "aidlc-state" => Face::State,
            "aidlc-bolt" => Face::Bolt,
            "aidlc-learnings" => Face::Learnings,
            "aidlc-review-brief" => Face::ReviewBrief,
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
        assert_eq!(Face::of("aidlc-log"), Face::Log);
        assert_eq!(Face::of("/usr/local/bin/aidlc-log"), Face::Log);
        assert_eq!(Face::of("aidlc-log.exe"), Face::Log);
        assert_eq!(Face::of("aidlc-state"), Face::State);
        assert_eq!(Face::of("/usr/local/bin/aidlc-state"), Face::State);
        assert_eq!(Face::of("aidlc-state.exe"), Face::State);
        assert_eq!(Face::of("aidlc-bolt"), Face::Bolt);
        assert_eq!(Face::of("/usr/local/bin/aidlc-bolt"), Face::Bolt);
        assert_eq!(Face::of("aidlc-bolt.exe"), Face::Bolt);
        assert_eq!(Face::of("aidlc-learnings"), Face::Learnings);
        assert_eq!(Face::of("/usr/local/bin/aidlc-learnings"), Face::Learnings);
        assert_eq!(Face::of("aidlc-learnings.exe"), Face::Learnings);
        assert_eq!(Face::of("aidlc-review-brief"), Face::ReviewBrief);
        assert_eq!(
            Face::of("/usr/local/bin/aidlc-review-brief"),
            Face::ReviewBrief
        );
        assert_eq!(Face::of("aidlc-review-brief.exe"), Face::ReviewBrief);
        assert_eq!(Face::of("aidlc-orchestrate"), Face::Orchestrate);
        assert_eq!(Face::of("aidlc"), Face::Orchestrate);
        // 未知の名前はエンジンに倒す（配布物の既定の顔）。
        assert_eq!(Face::of("something-else"), Face::Orchestrate);
    }
}
